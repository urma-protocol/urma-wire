use crate::{
    config::Settings,
    connection::{safe_endpoint, ConnectionHealth},
    live_index::{self, NodeReader},
    live_model::{Feed, Publication},
    Chain, IdentityStatus, Post, WalletSummary, WireWallet,
};
use fs2::FileExt;
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use urma_chain::observation::Chain as Network;
use urma_core::topics::Topics;
use urma_core::{envelope, format::PublicRecord};
use urma_identity::{identity::IdentitySigner, vault::UnlockCredential};
use urma_runtime::{
    node::{Node, Presence},
    plan::{self, PlanLimits, PublicationPlan},
    publish,
};
use urma_wire::index::{self, Index};
use zeroize::Zeroizing;

pub struct Service {
    directory: PathBuf,
    settings: Settings,
    wallet: Option<WireWallet>,
    _lock: File,
}

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

impl Service {
    /// Checks source height and existing index metadata without starting sync.
    pub fn connection_health(&self) -> ConnectionHealth {
        let configured = self.settings.rpc_url.as_deref();
        let mut health = ConnectionHealth {
            network: "Litecoin Testnet".into(),
            source_type: if configured.is_some() {
                "Configured RPC"
            } else {
                "Public source pool"
            }
            .into(),
            endpoint: configured
                .map(safe_endpoint)
                .unwrap_or_else(|| "Public pool (provider varies by request)".into()),
            checked_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as u64),
            remote_height: None,
            local_start: None,
            local_height: None,
            error: None,
        };
        if self.index_path().exists() {
            match Index::load(&self.index_path()) {
                Ok(index) => {
                    health.local_start = Some(index.start);
                    health.local_height = index.blocks.last().map(|block| block.height);
                }
                Err(_) => health.error = Some("Local index unavailable".into()),
            }
        }
        match self.settings.connect().and_then(|node| node.tip_height()) {
            Ok(height) => health.remote_height = Some(height),
            Err(_) => {
                health.error = Some("Chain source unavailable or configuration invalid".into())
            }
        }
        health
    }

    pub fn open(directory: &Path, settings: Settings) -> Result<Self, String> {
        std::fs::create_dir_all(directory).map_err(error)?;
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join("instance.lock"))
            .map_err(error)?;
        lock.try_lock_exclusive()
            .map_err(|_| "Wire is already running with this data directory".to_string())?;
        Ok(Self {
            directory: directory.into(),
            settings,
            wallet: None,
            _lock: lock,
        })
    }

    pub fn unlock(&mut self, password: String, use_unlock_file: bool) -> Result<(), String> {
        let supplied_password = Zeroizing::new(password);
        let settings = &self.settings;
        let password = if use_unlock_file {
            urma_workflows::vault::read_secret(
                settings.unlock_file.as_ref().ok_or("Missing unlock file")?,
            )
            .map_err(error)?
        } else {
            supplied_password
        };
        let vault = urma_workflows::vault::load(
            &self
                .identity_path()
                .ok_or("Import an encrypted URMA vault first")?,
            UnlockCredential::Password(&password),
        )
        .map_err(error)?;
        let wallet = WireWallet::from_vault(vault);
        wallet.author_id().map_err(error)?;
        self.wallet = Some(wallet);
        Ok(())
    }

    fn identity_path(&self) -> Option<PathBuf> {
        let imported = self.directory.join("identity.urma");
        if imported.exists() {
            Some(imported)
        } else {
            self.settings.vault.clone()
        }
    }

    /// Reads session metadata only: never connects to a source or unlocks a vault.
    pub fn identity_status(&self) -> Result<IdentityStatus, String> {
        let imported = self.directory.join("identity.urma").exists();
        let vault_available = self.identity_path().is_some_and(|path| path.is_file());
        let author = self
            .wallet
            .as_ref()
            .map(WireWallet::author_id)
            .transpose()
            .map_err(error)?;
        Ok(IdentityStatus {
            unlocked: self.wallet.is_some(),
            vault_available,
            source: if imported {
                "imported"
            } else if vault_available {
                "configured"
            } else {
                "none"
            }
            .into(),
            author_pubkey: author.as_ref().map(|author| author.to_hex()),
            author_fingerprint: author.as_ref().map(|author| author.fingerprint()),
            configured_unlock_available: self
                .settings
                .unlock_file
                .as_ref()
                .is_some_and(|path| path.is_file()),
        })
    }

    /// Import is deliberately create-only. The configured CLI vault is never changed.
    pub fn import_identity(
        &mut self,
        bytes: Vec<u8>,
        password: String,
    ) -> Result<IdentityStatus, String> {
        let password = Zeroizing::new(password);
        if bytes.len() > urma_identity::vault::EncryptedVault::MAX_BYTES {
            return Err("Encrypted URMA vault exceeds the size limit".into());
        }
        let path = self.directory.join("identity.urma");
        if path.try_exists().map_err(error)? {
            return Err(
                "An imported identity already exists. Unlock it; importing cannot overwrite it."
                    .into(),
            );
        }
        let wallet = WireWallet::unlock(&bytes, UnlockCredential::Password(&password))
            .map_err(|_| "Cannot decrypt this URMA vault with the supplied password".to_string())?;
        wallet.author_id().map_err(error)?;
        urma_io::write_new(&path, &bytes).map_err(error)?;
        self.wallet = Some(wallet);
        self.identity_status()
    }

    pub fn lock(&mut self) {
        self.wallet = None;
    }

    pub fn wallet(&self) -> Result<WalletSummary, String> {
        let wallet = self
            .wallet
            .as_ref()
            .ok_or("Identity vault is locked. Unlock it in Identity.")?;
        let node = self.settings.connect().map_err(error)?;
        let outputs = node
            .available_utxos(&wallet.signer().map_err(error)?)
            .map_err(error)?;
        let balance = outputs
            .iter()
            .try_fold(0u64, |sum, output| sum.checked_add(output.value))
            .ok_or("Balance overflow")?;
        wallet
            .summary(
                Chain::Litecoin,
                balance,
                0,
                outputs.len(),
                node.tip_height().map_err(error)?,
            )
            .map_err(error)
    }

    fn index_path(&self) -> PathBuf {
        self.directory.join("ltc-testnet-index.json")
    }

    fn start(&self, node: &Node) -> Result<u64, String> {
        let path = self.directory.join("ltc-testnet-start.json");
        if path.try_exists().map_err(error)? {
            return serde_json::from_slice(&urma_io::read_bounded(&path, 64).map_err(error)?)
                .map_err(error);
        }
        let start = node.tip_height().map_err(error)?.saturating_sub(100);
        urma_io::write_new(&path, &serde_json::to_vec(&start).map_err(error)?).map_err(error)?;
        Ok(start)
    }

    pub fn sync(&self) -> Result<Feed, String> {
        let node = self.settings.connect().map_err(error)?;
        let start = self.start(&node)?;
        let report =
            index::sync(&NodeReader(node), &self.index_path(), start, 100).map_err(error)?;
        let feed = live_index::project(
            &Index::load(&self.index_path()).map_err(error)?,
            report.complete_to_tip,
        )?;
        Ok(feed)
    }

    pub fn post(&self, txid: &str) -> Result<Option<Post>, String> {
        if !self.index_path().exists() {
            return Ok(None);
        }
        Ok(
            live_index::project(&Index::load(&self.index_path()).map_err(error)?, false)?
                .posts
                .into_iter()
                .find(|post| post.txid.to_display_hex() == txid),
        )
    }

    pub fn replies(&self, txid: &str) -> Result<Vec<Post>, String> {
        if !self.index_path().exists() {
            return Ok(Vec::new());
        }
        live_index::replies(&Index::load(&self.index_path()).map_err(error)?, txid)
    }

    fn active(&self) -> Result<Option<(PublicationPlan, PathBuf)>, String> {
        let path = self.directory.join("active.json");
        if !path.try_exists().map_err(error)? {
            return Ok(None);
        }
        let id: String = serde_json::from_slice(&urma_io::read_bounded(&path, 128).map_err(error)?)
            .map_err(error)?;
        if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid active plan ID".into());
        }
        let directory = self.directory.join("plans").join(&id);
        let plan = PublicationPlan::load(&directory.join("plan.json")).map_err(error)?;
        if plan.id().map_err(error)? != id
            || !matches!(plan.chain, Network::LitecoinTestnet)
            || plan.records.len() != 1
        {
            return Err("Active publication does not match its Litecoin Testnet plan".into());
        }
        Ok(Some((plan, directory)))
    }

    pub fn prepare(&self, text: String, target: Option<String>) -> Result<Publication, String> {
        self.prepare_topics(text, target, String::new(), Vec::new())
    }

    pub fn prepare_topics(
        &self,
        text: String,
        target: Option<String>,
        wire: String,
        hashtags: Vec<String>,
    ) -> Result<Publication, String> {
        if text.trim().is_empty() {
            return Err("Write a post or reply first".into());
        }
        let topics = Topics { wire, hashtags };
        topics.validate().map_err(error)?;
        let node = self.settings.connect().map_err(error)?;
        if let Some((plan, directory)) = self.active()? {
            if directory.join("approved").exists() && !observe(&node, &plan, &directory)?.complete {
                return Err(
                    "Finish the existing publication before preparing another; use Resume.".into(),
                );
            }
        }
        let signer = self
            .wallet
            .as_ref()
            .ok_or("Unlock your identity in Identity first")?
            .signer()
            .map_err(error)?;
        let record = match target {
            Some(target) if !topics.wire.is_empty() || !topics.hashtags.is_empty() => {
                PublicRecord::WireReply {
                    target: target.parse().map_err(error)?,
                    topics,
                    text,
                }
            }
            None if !topics.wire.is_empty() || !topics.hashtags.is_empty() => {
                PublicRecord::WirePost { topics, text }
            }
            Some(target) => PublicRecord::Reply {
                target: target.parse().map_err(error)?,
                text,
            },
            None => PublicRecord::Post(text),
        };
        let fee = urma_runtime::publication::quote_record(
            &record.encode().map_err(error)?,
            signer.public_key().inner.x_only_public_key().0,
            &urma_wallet::signing::script(&signer).map_err(error)?,
            1,
        )
        .map_err(error)?;
        let plan = plan::prepare_atomic(
            &node,
            &signer,
            &record,
            PlanLimits {
                fee_rate: 1,
                max_fee: fee,
                max_records: 1,
            },
        )
        .map_err(error)?;
        let id = plan.id().map_err(error)?;
        let directory = self.directory.join("plans").join(&id);
        std::fs::create_dir_all(&directory).map_err(error)?;
        let path = directory.join("plan.json");
        if !path.exists() {
            plan.save_new(&path).map_err(error)?;
        }
        self.start(&node)?;
        urma_io::write_replace(
            &self.directory.join("active.json"),
            &serde_json::to_vec(&id).map_err(error)?,
        )
        .map_err(error)?;
        observe(&node, &plan, &directory)
    }

    pub fn status(&self) -> Result<Option<Publication>, String> {
        match self.active()? {
            None => Ok(None),
            Some((plan, directory)) => {
                let node = self.settings.connect().map_err(error)?;
                Ok(Some(observe(&node, &plan, &directory)?))
            }
        }
    }

    pub fn advance(&self, id: &str, approve: bool) -> Result<Publication, String> {
        let (plan, directory) = self.active()?.ok_or("No prepared publication")?;
        if plan.id().map_err(error)? != id {
            return Err("Approval does not match the current exact plan".into());
        }
        if approve {
            urma_io::write_replace(&directory.join("approved"), id.as_bytes()).map_err(error)?;
        }
        let approval = urma_io::read_bounded(&directory.join("approved"), 64).map_err(error)?;
        if approval != id.as_bytes() {
            return Err("Exact plan approval required before broadcast".into());
        }
        let node = self.settings.connect().map_err(error)?;
        let report =
            publish::publish(&node, &plan, id, &directory.join("progress.json")).map_err(error)?;
        if !report.blocked_reason.is_empty()
            && !report
                .blocked_reason
                .starts_with("awaiting one confirmation")
            && report.blocked_reason != "awaiting confirmations for commit and reveal"
        {
            return Err(format!("{}. Publication paused; approved fees unchanged. Resume explicitly after resolving it.", report.blocked_reason));
        }
        observe(&node, &plan, &directory)
    }
}

fn observe(node: &Node, plan: &PublicationPlan, directory: &Path) -> Result<Publication, String> {
    let anchor = node.tip().map_err(error)?;
    let pair = &plan.records[0];
    let commit: bitcoin::Transaction =
        bitcoin::consensus::deserialize(&hex::decode(&pair.commit).map_err(error)?)
            .map_err(error)?;
    let reveal: bitcoin::Transaction =
        bitcoin::consensus::deserialize(&hex::decode(&pair.reveal).map_err(error)?)
            .map_err(error)?;
    let envelope = envelope::verify_reveal(&reveal, &commit).map_err(error)?;
    let (text, target, wire, hashtags) =
        match PublicRecord::decode(&envelope.record).map_err(error)? {
            PublicRecord::Post(text) => (text, None, String::new(), Vec::new()),
            PublicRecord::Reply { text, target } => {
                (text, Some(target.to_string()), String::new(), Vec::new())
            }
            PublicRecord::WirePost { text, topics } => (text, None, topics.wire, topics.hashtags),
            PublicRecord::WireReply {
                text,
                target,
                topics,
            } => (text, Some(target.to_string()), topics.wire, topics.hashtags),
            _ => return Err("Unsupported Wire publication".into()),
        };
    let mut confirmed = 0;
    let mut mempool = 0;
    for txid in [commit.compute_txid(), reveal.compute_txid()] {
        match node.presence(txid).map_err(error)? {
            Presence::Confirmed { .. } => confirmed += 1,
            Presence::Mempool => mempool += 1,
            Presence::Missing => (),
        }
    }
    if node.block_hash(anchor.0).map_err(error)?.to_string() != anchor.1 {
        return Err(
            "Chain changed during observation; refresh or resume for fresh evidence".into(),
        );
    }
    let approved = directory.join("approved").exists();
    let message = if confirmed == 2 {
        "Confirmed on Litecoin Testnet"
    } else if !approved {
        "Prepared. Review the exact text and fee before approving."
    } else {
        "Waiting for blockchain confirmation; publication continues while following."
    };
    let funding = pair.funding.prevout().map_err(error)?.1.value.to_sat();
    Ok(Publication {
        id: plan.id().map_err(error)?,
        txid: plan.root_txid.clone(),
        text,
        target,
        wire,
        hashtags,
        fee: plan.total_fee,
        selected_funding: funding,
        returned: funding
            .checked_sub(plan.total_fee)
            .ok_or("Invalid plan fee")?,
        approved,
        confirmed,
        mempool,
        complete: confirmed == 2,
        message: message.into(),
    })
}
