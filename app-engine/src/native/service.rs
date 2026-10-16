use crate::config::{Settings, EVIDENCE_UNAVAILABLE, LIGHT_CLIENT_COLD, NETWORK};
use crate::errors::NativeError;
use crate::native::archive::OperatorState;
use crate::native::local_node::{LocalNode, LocalSource};
use crate::native::mempool::Pool;
use crate::native::wallet::{Balance, WireWallet};
use fs2::FileExt;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use urma_identity::identity::IdentityKey;
use urma_identity::vault::{EncryptedVault, UnlockCredential};
use urma_runtime::error::Error as RuntimeError;
use urma_runtime::light::LightSync;
use urma_runtime::node::Node;
use urma_wallet::account::Account;
use urma_wire::index::Index;
use wire_app_core::connection::{ConnectionHealth, NodeHealth, SourceChoice, Trouble};
use wire_app_core::feed::SyncStatus;
use wire_app_core::model::{AuthorId, Chain, WalletSummary};
use wire_app_core::session::{Credential, IdentityStatus, Session, VaultSource};
use zeroize::Zeroizing;

enum Vault {
    At(PathBuf),
    Unconfigured,
}

pub(super) enum Link {
    Closed,
    Open(Arc<Node>),
}

pub(super) enum Identity {
    Locked,
    Unlocked(WireWallet),
}

pub struct Service {
    pub(super) local_node: LocalSource,
    pub(super) directory: PathBuf,
    settings: RwLock<Settings>,
    identity: Mutex<Identity>,
    pub(super) link: Arc<Mutex<Link>>,
    pub(super) archive_feed: Arc<Mutex<OperatorState>>,
    pub(super) sync: Arc<Mutex<SyncStatus>>,
    pub(super) reading: Arc<AtomicBool>,
    pub(super) scanning: Arc<AtomicBool>,
    pub(super) node_seen: Mutex<NodeHealth>,
    pub(super) mempool: Pool,
    _lock: File,
}

impl Service {
    pub(super) const IDENTITY_FILE: &'static str = "identity-v2.urma";
    pub(super) const INDEX_FILE: &'static str = "ltc-testnet-index-v3.json";
    pub(super) const START_FILE: &'static str = "ltc-testnet-start.json";
    pub(super) const ACTIVE_FILE: &'static str = "active.json";
    pub(super) const PLANS_DIR: &'static str = "plans";
    pub(super) const PLAN_FILE: &'static str = "plan.json";
    pub(super) const APPROVAL_FILE: &'static str = "approved";
    pub(super) const PROGRESS_FILE: &'static str = "progress.json";
    const LOCK_FILE: &'static str = "instance.lock";

    pub fn open(directory: &Path, settings: Settings) -> Result<Self, NativeError> {
        std::fs::create_dir_all(directory)?;
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(directory.join(Self::LOCK_FILE))?;
        lock.try_lock_exclusive()
            .map_err(NativeError::AlreadyRunning)?;
        Ok(Self {
            directory: directory.into(),
            local_node: LocalNode::configured(&settings)?,
            settings: RwLock::new(settings),
            identity: Mutex::new(Identity::Locked),
            link: Arc::new(Mutex::new(Link::Closed)),
            archive_feed: Arc::new(Mutex::new(OperatorState::Absent)),
            sync: Arc::new(Mutex::new(SyncStatus::idle())),
            reading: Arc::new(AtomicBool::new(false)),
            scanning: Arc::new(AtomicBool::new(false)),
            node_seen: Mutex::new(NodeHealth::Unconfigured),
            mempool: Pool::empty(),
            _lock: lock,
        })
    }

    pub(super) fn settings(&self) -> RwLockReadGuard<'_, Settings> {
        match self.settings.read() {
            Ok(settings) => settings,
            Err(poisoned) => {
                tracing::error!("settings lock poisoned; reading the held settings");
                poisoned.into_inner()
            }
        }
    }

    pub(super) fn settings_mut(&self) -> RwLockWriteGuard<'_, Settings> {
        match self.settings.write() {
            Ok(settings) => settings,
            Err(poisoned) => {
                tracing::error!("settings lock poisoned; writing the held settings");
                poisoned.into_inner()
            }
        }
    }

    fn identity(&self) -> MutexGuard<'_, Identity> {
        match self.identity.lock() {
            Ok(identity) => identity,
            Err(poisoned) => {
                tracing::error!("identity lock poisoned; reusing the held identity");
                poisoned.into_inner()
            }
        }
    }

    fn link(&self) -> MutexGuard<'_, Link> {
        locked_link(&self.link)
    }

    pub(super) fn sync_status(&self) -> MutexGuard<'_, SyncStatus> {
        locked_sync(&self.sync)
    }

    pub(super) fn connect_node(&self, settings: &Settings) -> Result<Node, NativeError> {
        if settings.source == SourceChoice::Public {
            let providers = self.local_node.providers();
            return Ok(Node::public_in_with_providers(
                NETWORK,
                &settings.cache_dir,
                providers,
            )?);
        }
        settings.connect()
    }

    pub(super) fn node(&self) -> Result<Arc<Node>, NativeError> {
        let mut link = self.link();
        match &*link {
            Link::Open(node) => Ok(node.clone()),
            Link::Closed => {
                let settings = self.settings().clone();
                let node = Arc::new(self.connect_node(&settings)?);
                opened(&node);
                *link = Link::Open(node.clone());
                Ok(node)
            }
        }
    }

    pub(super) fn with_node<T>(
        &self,
        work: impl FnOnce(&Node) -> Result<T, NativeError>,
    ) -> Result<T, NativeError> {
        let node = self.node()?;
        match work(&node) {
            Ok(value) => Ok(value),
            Err(error) => {
                if transport_failed(&error) {
                    drop_link(&self.link, &node);
                }
                Err(error)
            }
        }
    }

    pub(super) fn replace_link(&self, node: Node) {
        opened(&node);
        *self.link() = Link::Open(Arc::new(node));
    }

    pub(super) fn open_provider_labels(&self) -> Vec<String> {
        match &*self.link() {
            Link::Open(node) => node.provider_labels(),
            Link::Closed => Vec::new(),
        }
    }

    fn open_light_client(&self) -> String {
        match &*self.link() {
            Link::Open(node) => light_client_label(node.light_client_progress()),
            Link::Closed => String::new(),
        }
    }

    pub(super) fn index_path(&self) -> PathBuf {
        self.directory.join(Self::INDEX_FILE)
    }

    fn vault(&self) -> Vault {
        let imported = self.directory.join(Self::IDENTITY_FILE);
        if imported.exists() {
            return Vault::At(imported);
        }
        let Some(configured) = self.settings().vault.clone() else {
            return Vault::Unconfigured;
        };
        Vault::At(configured)
    }

    pub(super) fn signer(&self) -> Result<IdentityKey, NativeError> {
        match &*self.identity() {
            Identity::Unlocked(wallet) => Ok(wallet.signer()?),
            Identity::Locked => Err(NativeError::SignerLocked),
        }
    }

    pub(super) fn funder(&self) -> Result<Account, NativeError> {
        match &*self.identity() {
            Identity::Unlocked(wallet) => wallet.funder(Chain::Litecoin),
            Identity::Locked => Err(NativeError::SignerLocked),
        }
    }

    fn author(&self) -> Result<AuthorId, NativeError> {
        match &*self.identity() {
            Identity::Unlocked(wallet) => Ok(wallet.author_id()?),
            Identity::Locked => Err(NativeError::Locked),
        }
    }

    pub fn unlock(&self, credential: Credential) -> Result<(), NativeError> {
        let password = match credential {
            Credential::Password(password) => Zeroizing::new(password),
            Credential::ConfiguredFile => {
                let Some(file) = self.settings().unlock_file.clone() else {
                    return Err(NativeError::NoUnlockFile);
                };
                urma_workflows::vault::read_secret(&file)?
            }
        };
        let Vault::At(path) = self.vault() else {
            return Err(NativeError::NoVault);
        };
        let vault = urma_workflows::vault::load(&path, UnlockCredential::Password(&password))?;
        let wallet = WireWallet::from_vault(vault);
        wallet.author_id()?;
        *self.identity() = Identity::Unlocked(wallet);
        Ok(())
    }

    pub fn identity_status(&self) -> Result<IdentityStatus, NativeError> {
        let imported = self.directory.join(Self::IDENTITY_FILE).exists();
        let vault_available = self.vault_available();
        let source = match (imported, vault_available) {
            (true, _available) => VaultSource::Imported,
            (false, true) => VaultSource::Configured,
            (false, false) => VaultSource::Missing,
        };
        let configured_unlock_available = self.configured_unlock_available();
        let configured_unlock_path = match configured_unlock_available {
            true => self.configured_unlock_path(),
            false => String::new(),
        };
        Ok(IdentityStatus {
            session: self.session()?,
            vault_available,
            source,
            configured_unlock_available,
            configured_unlock_path,
        })
    }

    fn configured_unlock_path(&self) -> String {
        let Some(file) = self.settings().unlock_file.clone() else {
            return String::new();
        };
        file.display().to_string()
    }

    fn session(&self) -> Result<Session, NativeError> {
        match &*self.identity() {
            Identity::Unlocked(wallet) => Ok(Session::Unlocked(wallet.author_id()?)),
            Identity::Locked => Ok(Session::Locked),
        }
    }

    fn vault_available(&self) -> bool {
        match self.vault() {
            Vault::At(path) => path.is_file(),
            Vault::Unconfigured => false,
        }
    }

    fn configured_unlock_available(&self) -> bool {
        let Some(file) = self.settings().unlock_file.clone() else {
            return false;
        };
        file.is_file()
    }

    pub fn import_identity(
        &self,
        bytes: Vec<u8>,
        password: String,
    ) -> Result<IdentityStatus, NativeError> {
        let password = Zeroizing::new(password);
        if bytes.len() > EncryptedVault::MAX_BYTES {
            return Err(NativeError::VaultTooLarge);
        }
        let path = self.directory.join(Self::IDENTITY_FILE);
        if path.try_exists()? {
            return Err(NativeError::AlreadyImported);
        }
        let wallet = WireWallet::unlock(&bytes, UnlockCredential::Password(&password))
            .map_err(NativeError::WrongPassword)?;
        wallet.author_id()?;
        urma_io::write_new(&path, &bytes)?;
        *self.identity() = Identity::Unlocked(wallet);
        self.identity_status()
    }

    pub fn lock(&self) {
        *self.identity() = Identity::Locked;
    }

    pub fn wallet(&self) -> Result<WalletSummary, NativeError> {
        let author = self.author()?;
        let funder = self.funder()?;
        let (view, tip_height) =
            self.with_node(|node| Ok((node.wallet(&funder)?, node.tip_height()?)))?;
        let mut confirmed: u64 = 0;
        for coin in &view.coins {
            confirmed = confirmed
                .checked_add(coin.value)
                .ok_or(NativeError::BalanceOverflow)?;
        }
        let identity = self.identity();
        let Identity::Unlocked(wallet) = &*identity else {
            tracing::warn!("identity locked during the wallet query; discarding its result");
            return Err(NativeError::Locked);
        };
        if wallet.author_id()? != author {
            tracing::warn!("identity changed during the wallet query; discarding its result");
            return Err(NativeError::Locked);
        }
        wallet.summary(
            Chain::Litecoin,
            Balance {
                confirmed,
                unconfirmed: 0,
                outputs: view.coins.len(),
                tip_height,
            },
        )
    }

    pub fn connection_health(&self) -> Result<ConnectionHealth, NativeError> {
        let (source_type, endpoint) = self.described_source();
        let mut health = ConnectionHealth {
            network: Chain::Litecoin.network_name().into(),
            source_type: source_type.into(),
            endpoint,
            checked_at_ms: now_ms()?,
            remote_height: None,
            local_start: None,
            local_height: None,
            trouble: Trouble::Clear,
            evidence: EVIDENCE_UNAVAILABLE.into(),
            light_client: String::new(),
        };
        if self.index_path().exists() {
            match Index::load(&self.index_path()) {
                Ok(index) => {
                    health.local_start = Some(index.start);
                    health.local_height = index.blocks.last().map(|block| block.height);
                }
                Err(error) => {
                    tracing::warn!(error = %error, "local index could not be read");
                    health.trouble = Trouble::IndexUnreadable("Local index unavailable".into());
                }
            }
        }
        match self.remote_height() {
            Ok((height, evidence)) => {
                health.remote_height = Some(height);
                health.evidence = evidence;
            }
            Err(error) => {
                tracing::warn!(error = %error, "chain source check failed");
                health.trouble = Trouble::SourceUnverified(
                    "Chain source unavailable or configuration invalid".into(),
                );
            }
        }
        health.light_client = self.open_light_client();
        Ok(health)
    }

    fn remote_height(&self) -> Result<(u64, String), NativeError> {
        self.with_node(|node| {
            let observed = node.observe("getblockchaininfo", &[])?;
            let Some(height) = observed.value["blocks"].as_u64() else {
                return Err(NativeError::Runtime(RuntimeError::Missing(
                    "tip height missing from chain source".into(),
                )));
            };
            tracing::debug!(provider = %observed.provider, evidence = observed.evidence.label(), height, "tip observed");
            Ok((height, observed.evidence.label().to_owned()))
        })
    }
}

pub(super) fn locked_link(link: &Mutex<Link>) -> MutexGuard<'_, Link> {
    match link.lock() {
        Ok(link) => link,
        Err(poisoned) => {
            tracing::error!("chain link lock poisoned; reusing the held link");
            poisoned.into_inner()
        }
    }
}

pub(super) fn locked_sync(sync: &Mutex<SyncStatus>) -> MutexGuard<'_, SyncStatus> {
    match sync.lock() {
        Ok(status) => status,
        Err(poisoned) => {
            tracing::error!("sync status lock poisoned; reusing the held status");
            poisoned.into_inner()
        }
    }
}

pub(super) fn drop_link(link: &Mutex<Link>, failed: &Arc<Node>) {
    let mut link = locked_link(link);
    let Link::Open(current) = &*link else {
        return;
    };
    if Arc::ptr_eq(current, failed) {
        tracing::warn!("chain link dropped after a transport failure; reconnecting on next use");
        *link = Link::Closed;
    }
}

pub(super) fn opened(node: &Node) {
    tracing::info!(providers = ?node.provider_labels(), evidence = node.inclusion_evidence(), "chain link opened");
    match node.warm_light_client() {
        Ok(()) => {
            tracing::debug!(light_client = %light_client_label(node.light_client_progress()), "light client warming")
        }
        Err(error) => {
            tracing::warn!(error = %error, "light client did not start; reads continue on public providers")
        }
    }
}

fn light_client_label(sync: LightSync) -> String {
    match sync {
        LightSync::Absent => String::new(),
        LightSync::Cold(_progress) => LIGHT_CLIENT_COLD.to_owned(),
        LightSync::Running(progress) => format!(
            "Headers {}/{} · peers {}",
            progress.headers_synced, progress.headers_target, progress.peers_connected
        ),
    }
}

fn transport_failed(error: &NativeError) -> bool {
    match error {
        NativeError::Runtime(cause) => matches!(
            cause,
            RuntimeError::Io(_)
                | RuntimeError::Http(_)
                | RuntimeError::Tls(_)
                | RuntimeError::Rpc(_)
        ),
        NativeError::Io(_) => true,
        _ => false,
    }
}

fn now_ms() -> Result<u64, NativeError> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH)?;
    Ok(u64::try_from(elapsed.as_millis())?)
}
