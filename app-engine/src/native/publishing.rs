use crate::config::{
    APPROVAL_FILE_BYTES, FEE_HEADROOM, FEE_RATE, PLAN_ID_FILE_BYTES, PROGRESS_FILE_BYTES,
};
use crate::errors::NativeError;
use crate::native::service::Service;
use crate::native::wallet::Known;
use crate::record::{build, review};
use bitcoin::consensus::deserialize;
use bitcoin::{OutPoint, ScriptBuf, Transaction};
use std::path::{Path, PathBuf};
use urma_chain::observation::Chain as Network;
use urma_core::envelope;
use urma_core::format::PublicRecord;
use urma_identity::identity::{IdentityKey, IdentitySigner};
use urma_runtime::node::{Node, Presence};
use urma_runtime::plan::{self, PlanLimits, PublicationPlan};
use urma_runtime::publication::PublicPlan;
use urma_runtime::publish::{self, PublishReport, TransactionStatus};
use urma_runtime::wallet::Coin;
use urma_wallet::account::{Account, KeyPath};
use wire_app_core::publication::{Content, Pending, Publication};

enum Standing {
    Planned(PublicationPlan, PathBuf),
    Idle,
}

enum Journal {
    Written(PublishReport),
    Absent,
}

enum Journaled {
    Complete,
    Submitted,
    Unsent,
}

impl Service {
    fn standing(&self) -> Result<Standing, NativeError> {
        let path = self.directory.join(Self::ACTIVE_FILE);
        if !path.try_exists()? {
            return Ok(Standing::Idle);
        }
        match self.saved(&path) {
            Ok(standing) => Ok(standing),
            Err(error) => {
                tracing::warn!(error = %error, "saved publication cannot be loaded; it is dropped");
                self.forget(&path)?;
                Ok(Standing::Idle)
            }
        }
    }

    fn forget(&self, path: &Path) -> Result<(), NativeError> {
        let id: String =
            match serde_json::from_slice(&urma_io::read_bounded(path, PLAN_ID_FILE_BYTES)?) {
                Ok(id) => id,
                Err(error) => {
                    tracing::warn!(error = %error, "active publication pointer holds no plan id");
                    std::fs::remove_file(path)?;
                    return Ok(());
                }
            };
        let directory = self.directory.join(Self::PLANS_DIR).join(&id);
        if plan_id(&id) && directory.try_exists()? {
            std::fs::remove_dir_all(&directory)?;
        }
        std::fs::remove_file(path)?;
        Ok(())
    }

    fn saved(&self, path: &Path) -> Result<Standing, NativeError> {
        let id: String = serde_json::from_slice(&urma_io::read_bounded(path, PLAN_ID_FILE_BYTES)?)?;
        if !plan_id(&id) {
            return Err(NativeError::InvalidPlanId);
        }
        let directory = self.directory.join(Self::PLANS_DIR).join(&id);
        let plan = PublicationPlan::load(&directory.join(Self::PLAN_FILE))
            .map_err(NativeError::SavedPlan)?;
        if plan.id()? != id
            || !matches!(plan.chain, Network::LitecoinTestnet)
            || plan.records.len() != 1
        {
            return Err(NativeError::ForeignPlan);
        }
        Ok(Standing::Planned(plan, directory))
    }

    fn refuse_while_unfinished(&self) -> Result<(), NativeError> {
        let Standing::Planned(plan, directory) = self.standing()? else {
            return Ok(());
        };
        if !directory.join(Self::APPROVAL_FILE).exists() {
            return Ok(());
        }
        match journaled(&directory)? {
            Journaled::Complete => return Ok(()),
            Journaled::Submitted | Journaled::Unsent => (),
        }
        if self
            .with_node(|node| observe(node, &plan, &directory))?
            .complete
        {
            return Ok(());
        }
        match journaled(&directory)? {
            Journaled::Complete => Ok(()),
            Journaled::Submitted => Err(NativeError::Confirming),
            Journaled::Unsent => Err(NativeError::Unfinished),
        }
    }

    pub fn quote(&self, content: &Content) -> Result<Publication, NativeError> {
        let record = build(content)?;
        let signer = self.signer()?;
        let funder = self.funder()?;
        let fee = urma_runtime::publication::quote_record(
            &record.encode()?,
            signer.public_key().inner.x_only_public_key().0,
            &funder.receive_script()?,
            FEE_RATE.get(),
        )?;
        self.refuse_while_unfinished()?;
        let limits = PlanLimits {
            fee_rate: FEE_RATE.get(),
            max_fee: fee
                .checked_add(FEE_HEADROOM)
                .ok_or(NativeError::InvalidPlanFee)?,
            max_records: 1,
        };
        self.with_node(|node| {
            let plan = self.funded(node, &signer, &funder, &record, limits)?;
            let id = plan.id()?;
            let directory = self.directory.join(Self::PLANS_DIR).join(&id);
            std::fs::create_dir_all(&directory)?;
            let path = directory.join(Self::PLAN_FILE);
            if !path.exists() {
                plan.save_new(&path)?;
            }
            self.start(node)?;
            urma_io::write_replace(
                &self.directory.join(Self::ACTIVE_FILE),
                &serde_json::to_vec(&id)?,
            )?;
            described(&plan, &directory, 0, 0)
        })
    }

    fn funded(
        &self,
        node: &Node,
        signer: &IdentityKey,
        funder: &Account,
        record: &PublicRecord,
        limits: PlanLimits,
    ) -> Result<PublicationPlan, NativeError> {
        let receive = funder.receive_script()?;
        let mut known = self.coins();
        let remembered = match &*known {
            Known::Coins {
                receive: held,
                coins,
            } if *held == receive => coins.clone(),
            Known::Coins { .. } | Known::Nothing => Vec::new(),
        };
        if !remembered.is_empty() {
            match plan::prepare_atomic_from(node, signer, funder, record, limits, remembered) {
                Ok(plan) => return Ok(plan),
                Err(error) => {
                    tracing::warn!(error = %error, "remembered coins could not fund the plan; scanning the wallet")
                }
            }
        }
        let view = node.wallet(funder)?;
        *known = Known::Coins {
            receive,
            coins: view.coins.clone(),
        };
        Ok(plan::prepare_atomic_from(
            node, signer, funder, record, limits, view.coins,
        )?)
    }

    pub fn status(&self) -> Result<Pending, NativeError> {
        let Standing::Planned(plan, directory) = self.standing()? else {
            return Ok(Pending::Settled);
        };
        self.with_node(|node| Ok(Pending::Awaiting(observe(node, &plan, &directory)?)))
    }

    pub fn discard_quote(&self, id: &str) -> Result<(), NativeError> {
        let Standing::Planned(plan, directory) = self.standing()? else {
            return Ok(());
        };
        if plan.id()? != id {
            return Ok(());
        }
        if directory.join(Self::APPROVAL_FILE).exists() {
            let publication = self.with_node(|node| observe(node, &plan, &directory))?;
            if !publication.abandonable {
                return Err(NativeError::Unfinished);
            }
            tracing::warn!(
                id,
                "abandoning an approved plan that never reached the network"
            );
            std::fs::remove_file(directory.join(Self::APPROVAL_FILE))?;
        }
        std::fs::remove_file(self.directory.join(Self::ACTIVE_FILE))?;
        Ok(())
    }

    pub fn advance(&self, id: &str, approve: bool) -> Result<Publication, NativeError> {
        let Standing::Planned(plan, directory) = self.standing()? else {
            return Err(NativeError::NoPublication);
        };
        if plan.id()? != id {
            return Err(NativeError::PlanMismatch);
        }
        let approval = directory.join(Self::APPROVAL_FILE);
        if approve {
            plan.require_funder(&self.funder()?)?;
            urma_io::write_replace(&approval, id.as_bytes())?;
        }
        if urma_io::read_bounded(&approval, APPROVAL_FILE_BYTES)? != id.as_bytes() {
            return Err(NativeError::ApprovalRequired);
        }
        self.with_node(|node| {
            let report = publish::publish(node, &plan, id, &directory.join(Self::PROGRESS_FILE))?;
            if paused(&report.blocked_reason) {
                return Err(NativeError::Paused(report.blocked_reason));
            }
            if report.complete {
                self.remember_change(&plan.records[0], &report);
            }
            let (confirmed, mempool) = counted(&report);
            described(&plan, &directory, confirmed, mempool)
        })
    }

    fn remember_change(&self, pair: &PublicPlan, report: &PublishReport) {
        let mut known = self.coins();
        let Known::Coins { receive, coins } = &mut *known else {
            return;
        };
        let (spent, change) = match change_coin(pair, report, receive) {
            Ok(found) => found,
            Err(error) => {
                tracing::warn!(error = %error, "confirmed change is not remembered; the next quote scans the wallet");
                coins.clear();
                return;
            }
        };
        coins.retain(|coin| !spent.contains(&coin.outpoint));
        coins.push(change);
    }
}

fn plan_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn paused(reason: &str) -> bool {
    !reason.is_empty() && !reason.starts_with("awaiting")
}

fn counted(report: &PublishReport) -> (usize, usize) {
    counted_in(&report.transactions)
}

fn counted_in(statuses: &[TransactionStatus]) -> (usize, usize) {
    let mut confirmed = 0;
    let mut mempool = 0;
    for status in statuses {
        match status.presence {
            Presence::Confirmed { .. } => confirmed += 1,
            Presence::Mempool => mempool += 1,
            Presence::Missing => (),
        }
    }
    (confirmed, mempool)
}

fn record_completion(
    directory: &Path,
    plan: &PublicationPlan,
    transactions: Vec<TransactionStatus>,
) -> Result<(), NativeError> {
    let report = PublishReport {
        plan_id: plan.id()?,
        root_txid: plan.root_txid.clone(),
        complete: true,
        confirmed: true,
        blocked_reason: String::new(),
        transactions,
    };
    urma_io::write_replace(
        &directory.join(Service::PROGRESS_FILE),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(())
}

fn change_coin(
    pair: &PublicPlan,
    report: &PublishReport,
    receive: &ScriptBuf,
) -> Result<(Vec<OutPoint>, Coin), NativeError> {
    let commit: Transaction = deserialize(&hex::decode(&pair.commit)?)?;
    let txid = commit.compute_txid();
    let Some(status) = report
        .transactions
        .iter()
        .find(|status| status.txid == txid.to_string())
    else {
        return Err(NativeError::Change("the commit is not in the journal"));
    };
    let Presence::Confirmed { height, .. } = &status.presence else {
        return Err(NativeError::Change("the commit is not confirmed"));
    };
    let Some(change) = commit.output.get(1) else {
        return Err(NativeError::Change("the commit has no change output"));
    };
    if change.script_pubkey != *receive {
        return Err(NativeError::Change(
            "the change does not return to the receive address",
        ));
    }
    let spent = commit
        .input
        .iter()
        .map(|input| input.previous_output)
        .collect();
    Ok((
        spent,
        Coin {
            outpoint: OutPoint::new(txid, 1),
            value: change.value.to_sat(),
            height: *height,
            path: KeyPath::RECEIVE,
        },
    ))
}

fn observe(
    node: &Node,
    plan: &PublicationPlan,
    directory: &Path,
) -> Result<Publication, NativeError> {
    let anchor = node.tip()?;
    let pair = &plan.records[0];
    let commit: Transaction = deserialize(&hex::decode(&pair.commit)?)?;
    let reveal: Transaction = deserialize(&hex::decode(&pair.reveal)?)?;
    let mut statuses = Vec::new();
    for txid in [commit.compute_txid(), reveal.compute_txid()] {
        statuses.push(TransactionStatus {
            txid: txid.to_string(),
            presence: node.presence(txid)?,
        });
    }
    if node.block_hash(anchor.0)?.to_string() != anchor.1 {
        return Err(NativeError::ChainChanged);
    }
    let (confirmed, mempool) = counted_in(&statuses);
    if confirmed == 2 {
        record_completion(directory, plan, statuses)?;
    }
    described(plan, directory, confirmed, mempool)
}

fn described(
    plan: &PublicationPlan,
    directory: &Path,
    confirmed: usize,
    mempool: usize,
) -> Result<Publication, NativeError> {
    let pair = &plan.records[0];
    let commit: Transaction = deserialize(&hex::decode(&pair.commit)?)?;
    let reveal: Transaction = deserialize(&hex::decode(&pair.reveal)?)?;
    let signed = envelope::verify_reveal(&reveal, &commit)?;
    let content = review(&signed.record)?;
    let approved = directory.join(Service::APPROVAL_FILE).exists();
    let funding = selected_funding(pair)?;
    let Some(returned) = funding.checked_sub(plan.total_fee) else {
        return Err(NativeError::InvalidPlanFee);
    };
    Ok(Publication {
        content,
        id: plan.id()?,
        txid: plan.root_txid.clone(),
        fee: plan.total_fee,
        selected_funding: funding,
        returned,
        approved,
        confirmed,
        mempool,
        complete: confirmed == 2,
        message: observed(confirmed, approved).into(),
        abandonable: approved && confirmed == 0 && mempool == 0 && never_submitted(directory)?,
    })
}

fn selected_funding(pair: &PublicPlan) -> Result<u64, NativeError> {
    let mut total: u64 = 0;
    for funding in &pair.funding {
        total = total
            .checked_add(funding.prevout()?.1.value.to_sat())
            .ok_or(NativeError::InvalidPlanFee)?;
    }
    Ok(total)
}

fn journal(directory: &Path) -> Result<Journal, NativeError> {
    let path = directory.join(Service::PROGRESS_FILE);
    if !path.try_exists()? {
        return Ok(Journal::Absent);
    }
    Ok(Journal::Written(serde_json::from_slice(
        &urma_io::read_bounded(&path, PROGRESS_FILE_BYTES)?,
    )?))
}

fn journaled(directory: &Path) -> Result<Journaled, NativeError> {
    let Journal::Written(report) = journal(directory)? else {
        return Ok(Journaled::Unsent);
    };
    if report.complete {
        return Ok(Journaled::Complete);
    }
    let submitted = report.transactions.len() == 2
        && report
            .transactions
            .iter()
            .all(|status| !matches!(status.presence, Presence::Missing));
    Ok(match submitted {
        true => Journaled::Submitted,
        false => Journaled::Unsent,
    })
}

fn never_submitted(directory: &Path) -> Result<bool, NativeError> {
    let Journal::Written(report) = journal(directory)? else {
        return Ok(true);
    };
    Ok(report
        .transactions
        .iter()
        .all(|status| matches!(status.presence, Presence::Missing)))
}

fn observed(confirmed: usize, approved: bool) -> &'static str {
    if confirmed == 2 {
        return "Confirmed on Litecoin Testnet";
    }
    match approved {
        false => "Prepared. Review the exact record and fee before approving.",
        true => "Waiting for blockchain confirmation; publication continues while following.",
    }
}
