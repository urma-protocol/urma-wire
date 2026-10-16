use crate::config::{
    APPROVAL_FILE_BYTES, FEE_HEADROOM, FEE_RATE, PLAN_ID_FILE_BYTES, PROGRESS_FILE_BYTES,
};
use crate::errors::NativeError;
use crate::native::service::Service;
use crate::record::{build, review};
use bitcoin::consensus::deserialize;
use bitcoin::Transaction;
use std::path::{Path, PathBuf};
use urma_chain::observation::Chain as Network;
use urma_core::envelope;
use urma_identity::identity::IdentitySigner;
use urma_runtime::node::{Node, Presence};
use urma_runtime::plan::{self, PlanLimits, PublicationPlan};
use urma_runtime::publication::PublicPlan;
use urma_runtime::publish::{self, PublishReport};
use wire_app_core::publication::{Content, Pending, Publication};

enum Standing {
    Planned(PublicationPlan, PathBuf),
    Idle,
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

    fn refuse_while_unfinished(&self, node: &Node) -> Result<(), NativeError> {
        let Standing::Planned(plan, directory) = self.standing()? else {
            return Ok(());
        };
        if directory.join(Self::APPROVAL_FILE).exists()
            && !observe(node, &plan, &directory)?.complete
        {
            return Err(NativeError::Unfinished);
        }
        Ok(())
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
        self.with_node(|node| {
            self.refuse_while_unfinished(node)?;
            let plan = plan::prepare_atomic(
                node,
                &signer,
                &funder,
                &record,
                PlanLimits {
                    fee_rate: FEE_RATE.get(),
                    max_fee: fee
                        .checked_add(FEE_HEADROOM)
                        .ok_or(NativeError::InvalidPlanFee)?,
                    max_records: 1,
                },
            )?;
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
            observe(node, &plan, &directory)
        })
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
            observe(node, &plan, &directory)
        })
    }
}

fn plan_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn paused(reason: &str) -> bool {
    !reason.is_empty() && !reason.starts_with("awaiting")
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
    let signed = envelope::verify_reveal(&reveal, &commit)?;
    let content = review(&signed.record)?;
    let mut confirmed = 0;
    let mut mempool = 0;
    for txid in [commit.compute_txid(), reveal.compute_txid()] {
        match node.presence(txid)? {
            Presence::Confirmed { .. } => confirmed += 1,
            Presence::Mempool => mempool += 1,
            Presence::Missing => (),
        }
    }
    if node.block_hash(anchor.0)?.to_string() != anchor.1 {
        return Err(NativeError::ChainChanged);
    }
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

fn never_submitted(directory: &Path) -> Result<bool, NativeError> {
    let journal = directory.join(Service::PROGRESS_FILE);
    if !journal.try_exists()? {
        return Ok(true);
    }
    let report: PublishReport =
        serde_json::from_slice(&urma_io::read_bounded(&journal, PROGRESS_FILE_BYTES)?)?;
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
