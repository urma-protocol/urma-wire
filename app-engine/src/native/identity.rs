use crate::errors::NativeError;
use crate::native::service::{Identity, Service};
use crate::native::wallet::WireWallet;
use crate::vault::{minted, Minted, Words};
use std::path::PathBuf;
use urma_identity::vault::{EncryptedVault, UnlockCredential};
use wire_app_core::session::{Credential, IdentityStatus, Session, Vault};
use zeroize::Zeroizing;

enum Copy {
    Local(PathBuf),
    Configured(PathBuf),
    Nowhere,
}

pub struct Created {
    pub status: IdentityStatus,
    pub words: Zeroizing<String>,
}

impl Service {
    fn local_copy(&self) -> PathBuf {
        self.directory.join(Self::IDENTITY_FILE)
    }

    fn copy(&self) -> Copy {
        let local = self.local_copy();
        if local.is_file() {
            return Copy::Local(local);
        }
        let Some(configured) = self.settings().vault.clone() else {
            return Copy::Nowhere;
        };
        match configured.is_file() {
            true => Copy::Configured(configured),
            false => Copy::Nowhere,
        }
    }

    fn unlock_file(&self) -> Option<String> {
        let file = self.settings().unlock_file.clone()?;
        match file.is_file() {
            true => Some(file.display().to_string()),
            false => None,
        }
    }

    pub fn identity_status(&self) -> Result<IdentityStatus, NativeError> {
        let vault = match self.copy() {
            Copy::Local(_path) => Vault::Local,
            Copy::Configured(path) => Vault::Configured {
                path: path.display().to_string(),
                unlock_file: self.unlock_file(),
            },
            Copy::Nowhere => Vault::Missing,
        };
        let session = match &*self.identity() {
            Identity::Unlocked(wallet) => Session::Unlocked(wallet.author_id()?),
            Identity::Locked => Session::Locked,
        };
        Ok(IdentityStatus { session, vault })
    }

    pub fn unlock(&self, credential: Credential) -> Result<IdentityStatus, NativeError> {
        let password = match credential {
            Credential::Password(password) => Zeroizing::new(password),
            Credential::ConfiguredFile => {
                let Some(file) = self.settings().unlock_file.clone() else {
                    return Err(NativeError::NoUnlockFile);
                };
                urma_workflows::vault::read_secret(&file)?
            }
        };
        let path = match self.copy() {
            Copy::Local(path) | Copy::Configured(path) => path,
            Copy::Nowhere => return Err(NativeError::NoVault),
        };
        let vault = urma_workflows::vault::load(&path, UnlockCredential::Password(&password))
            .map_err(NativeError::Runtime)?;
        self.adopt(WireWallet::from_vault(vault))
    }

    pub fn lock(&self) -> Result<IdentityStatus, NativeError> {
        *self.identity() = Identity::Locked;
        self.identity_status()
    }

    pub fn create_identity(&self, password: String, replace: bool) -> Result<Created, NativeError> {
        let password = Zeroizing::new(password);
        self.replaceable(replace)?;
        let Minted {
            vault,
            sealed,
            words,
        } = minted(Words::Fresh, &password)?;
        urma_io::write_replace(&self.local_copy(), &sealed)?;
        Ok(Created {
            status: self.adopt(WireWallet::from_vault(vault))?,
            words,
        })
    }

    pub fn recover_identity(
        &self,
        phrase: String,
        password: String,
        replace: bool,
    ) -> Result<IdentityStatus, NativeError> {
        let password = Zeroizing::new(password);
        let phrase = Zeroizing::new(phrase);
        self.replaceable(replace)?;
        let Minted { vault, sealed, .. } = minted(Words::Given(phrase), &password)?;
        urma_io::write_replace(&self.local_copy(), &sealed)?;
        self.adopt(WireWallet::from_vault(vault))
    }

    pub fn import_identity(
        &self,
        bytes: Vec<u8>,
        password: String,
        replace: bool,
    ) -> Result<IdentityStatus, NativeError> {
        let password = Zeroizing::new(password);
        if bytes.len() > EncryptedVault::MAX_BYTES {
            return Err(NativeError::VaultTooLarge);
        }
        self.replaceable(replace)?;
        let wallet = WireWallet::unlock(&bytes, UnlockCredential::Password(&password))
            .map_err(NativeError::WrongPassword)?;
        urma_io::write_replace(&self.local_copy(), &bytes)?;
        self.adopt(wallet)
    }

    fn replaceable(&self, replace: bool) -> Result<(), NativeError> {
        match self.local_copy().try_exists()? {
            true if !replace => Err(NativeError::LocalVaultExists),
            true => Ok(()),
            false => Ok(()),
        }
    }

    fn adopt(&self, wallet: WireWallet) -> Result<IdentityStatus, NativeError> {
        wallet.author_id()?;
        *self.identity() = Identity::Unlocked(wallet);
        self.identity_status()
    }
}
