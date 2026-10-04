use crate::config::{Settings, EVIDENCE_UNAVAILABLE};
use crate::errors::NativeError;
use crate::native::wallet::{Balance, WireWallet};
use fs2::FileExt;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use urma_identity::vault::{EncryptedVault, UnlockCredential};
use urma_wire::index::Index;
use wire_app_core::connection::{ConnectionHealth, Trouble};
use wire_app_core::model::{Chain, WalletSummary};
use wire_app_core::session::{Credential, IdentityStatus, Session, VaultSource};
use zeroize::Zeroizing;

enum Vault {
    At(PathBuf),
    Unconfigured,
}

pub struct Service {
    pub(super) directory: PathBuf,
    pub(super) settings: Settings,
    pub(super) wallet: Option<WireWallet>,
    _lock: File,
}

impl Service {
    pub(super) const IDENTITY_FILE: &'static str = "identity.urma";
    pub(super) const INDEX_FILE: &'static str = "ltc-testnet-index.json";
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
            settings,
            wallet: None,
            _lock: lock,
        })
    }

    pub(super) fn index_path(&self) -> PathBuf {
        self.directory.join(Self::INDEX_FILE)
    }

    fn vault(&self) -> Vault {
        let imported = self.directory.join(Self::IDENTITY_FILE);
        if imported.exists() {
            return Vault::At(imported);
        }
        let Some(configured) = self.settings.vault.clone() else {
            return Vault::Unconfigured;
        };
        Vault::At(configured)
    }

    pub fn unlock(&mut self, credential: Credential) -> Result<(), NativeError> {
        let password = match credential {
            Credential::Password(password) => Zeroizing::new(password),
            Credential::ConfiguredFile => {
                let Some(file) = self.settings.unlock_file.as_ref() else {
                    return Err(NativeError::NoUnlockFile);
                };
                urma_workflows::vault::read_secret(file)?
            }
        };
        let Vault::At(path) = self.vault() else {
            return Err(NativeError::NoVault);
        };
        let vault = urma_workflows::vault::load(&path, UnlockCredential::Password(&password))?;
        let wallet = WireWallet::from_vault(vault);
        wallet.author_id()?;
        self.wallet = Some(wallet);
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
        Ok(IdentityStatus {
            session: self.session()?,
            vault_available,
            source,
            configured_unlock_available: self.configured_unlock_available(),
        })
    }

    fn session(&self) -> Result<Session, NativeError> {
        let Some(wallet) = &self.wallet else {
            return Ok(Session::Locked);
        };
        Ok(Session::Unlocked(wallet.author_id()?))
    }

    fn vault_available(&self) -> bool {
        match self.vault() {
            Vault::At(path) => path.is_file(),
            Vault::Unconfigured => false,
        }
    }

    fn configured_unlock_available(&self) -> bool {
        let Some(file) = self.settings.unlock_file.as_ref() else {
            return false;
        };
        file.is_file()
    }

    pub fn import_identity(
        &mut self,
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
        self.wallet = Some(wallet);
        self.identity_status()
    }

    pub fn lock(&mut self) {
        self.wallet = None;
    }

    pub fn wallet(&self) -> Result<WalletSummary, NativeError> {
        let Some(wallet) = &self.wallet else {
            return Err(NativeError::Locked);
        };
        let node = self.settings.connect()?;
        let outputs = node.available_utxos(&wallet.signer()?)?;
        let mut confirmed: u64 = 0;
        for output in &outputs {
            confirmed = confirmed
                .checked_add(output.value)
                .ok_or(NativeError::BalanceOverflow)?;
        }
        wallet.summary(
            Chain::Litecoin,
            Balance {
                confirmed,
                unconfirmed: 0,
                outputs: outputs.len(),
                tip_height: node.tip_height()?,
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
        Ok(health)
    }

    fn remote_height(&self) -> Result<(u64, String), NativeError> {
        let node = self.settings.connect()?;
        Ok((node.tip_height()?, node.inclusion_evidence().to_owned()))
    }
}

fn now_ms() -> Result<u64, NativeError> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH)?;
    Ok(u64::try_from(elapsed.as_millis())?)
}
