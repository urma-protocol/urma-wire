use crate::config::{APPROVAL_FILE_BYTES, FEE_RATE, PLAN_ID_FILE_BYTES};
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
use urma_runtime::publish;
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
        let id: String =
            serde_json::from_slice(&urma_io::read_bounded(&path, PLAN_ID_FILE_BYTES)?)?;
        if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(NativeError::InvalidPlanId);
        }
        let directory = self.directory.join(Self::PLANS_DIR).join(&id);
        let plan = PublicationPlan::load(&directory.join(Self::PLAN_FILE))?;
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
        if directory.join(Self::APPROVAL_FILE).exists() && !observe(node, &plan, &directory)?.complete
        {
            return Err(NativeError::Unfinished);
        }
        Ok(())
    }

    pub fn quote(&self, content: &Content) -> Result<Publication, NativeError> {
        let record = build(content)?;
        let node = self.settings.connect()?;
        self.refuse_while_unfinished(&node)?;
        let Some(wallet) = &self.wallet else {
            return Err(NativeError::SignerLocked);
        };
        let signer = wallet.signer()?;
        let fee = urma_runtime::publication::quote_record(
            &record.encode()?,
            signer.public_key().inner.x_only_public_key().0,
            &urma_wallet::signing::script(&signer)?,
            FEE_RATE.get(),
        )?;
        let plan = plan::prepare_atomic(
            &node,
            &signer,
            &record,
            PlanLimits {
                fee_rate: FEE_RATE.get(),
                max_fee: fee,
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
        self.start(&node)?;
        urma_io::write_replace(
            &self.directory.join(Self::ACTIVE_FILE),
            &serde_json::to_vec(&id)?,
        )?;
        observe(&node, &plan, &directory)
    }

    pub fn status(&self) -> Result<Pending, NativeError> {
        let Standing::Planned(plan, directory) = self.standing()? else {
            return Ok(Pending::Settled);
        };
        let node = self.settings.connect()?;
        Ok(Pending::Awaiting(observe(&node, &plan, &directory)?))
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
            urma_io::write_replace(&approval, id.as_bytes())?;
        }
        if urma_io::read_bounded(&approval, APPROVAL_FILE_BYTES)? != id.as_bytes() {
            return Err(NativeError::ApprovalRequired);
        }
        let node = self.settings.connect()?;
        let report = publish::publish(&node, &plan, id, &directory.join(Self::PROGRESS_FILE))?;
        if paused(&report.blocked_reason) {
            return Err(NativeError::Paused(report.blocked_reason));
        }
        observe(&node, &plan, &directory)
    }
}

fn paused(reason: &str) -> bool {
    !reason.is_empty()
        && !reason.starts_with("awaiting one confirmation")
        && reason != "awaiting confirmations for commit and reveal"
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
    let funding = pair.funding.prevout()?.1.value.to_sat();
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
    })
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
