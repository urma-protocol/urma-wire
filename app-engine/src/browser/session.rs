use crate::config::{NETWORK, VAULT_KEY};
use crate::errors::BrowserError;
use crate::vault::{minted, Minted, Words};
use std::cell::{Cell, RefCell};
use std::num::Wrapping;
use urma_identity::identity::IdentityKey;
use urma_identity::vault::{EncryptedVault, UnlockCredential, UnlockedVault};
use urma_wallet::account::Account;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wire_app_core::model::AuthorId;
use wire_app_core::session::{Credential, IdentityStatus, Session, Vault};
use zeroize::Zeroizing;

thread_local! { static SESSION: RefCell<Option<UnlockedVault>> = const { RefCell::new(None) }; }
thread_local! { static EPOCH: Cell<Wrapping<u64>> = const { Cell::new(Wrapping(0)) }; }

#[wasm_bindgen(inline_js = r#"
function database() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open('urma-wire', 1);
        request.onupgradeneeded = () => request.result.createObjectStore('private');
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(new Error('Browser storage unavailable'));
    });
}
export async function vault_read(key) {
    const db = await database();
    try {
        return await new Promise((resolve, reject) => {
            const tx = db.transaction('private', 'readonly');
            const request = tx.objectStore('private').get(key);
            tx.oncomplete = () => resolve(request.result || null);
            tx.onabort = tx.onerror = () => reject(new Error('Cannot read encrypted vault'));
        });
    } finally { db.close(); }
}
export async function vault_store(key, bytes, replace) {
    const db = await database();
    try {
        await new Promise((resolve, reject) => {
            const tx = db.transaction('private', 'readwrite');
            const store = tx.objectStore('private');
            if (replace) store.put(bytes, key); else store.add(bytes, key);
            tx.oncomplete = resolve;
            tx.onabort = tx.onerror = () => reject(new Error('Storing the vault failed; the previous vault stands'));
        });
    } finally { db.close(); }
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn vault_read(key: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn vault_store(key: &str, bytes: &str, replace: bool) -> Result<(), JsValue>;
}

enum Copy {
    Stored(Vec<u8>),
    Absent,
}

pub enum Epoch {
    Any,
    Exactly(u64),
}

pub struct Created {
    pub status: IdentityStatus,
    pub words: Zeroizing<String>,
}

fn vault_storage(thrown: JsValue) -> BrowserError {
    BrowserError::VaultStorage(BrowserError::from(thrown).to_string())
}

async fn stored() -> Result<Copy, BrowserError> {
    let held = vault_read(VAULT_KEY).await.map_err(vault_storage)?;
    let Some(text) = held.as_string() else {
        return Ok(Copy::Absent);
    };
    let bytes = serde_json::from_str(&text).map_err(BrowserError::StoredVault)?;
    Ok(Copy::Stored(bytes))
}

pub async fn status() -> Result<IdentityStatus, BrowserError> {
    match stored().await? {
        Copy::Stored(_bytes) => identity(Vault::Local),
        Copy::Absent => identity(Vault::Missing),
    }
}

fn identity(vault: Vault) -> Result<IdentityStatus, BrowserError> {
    Ok(IdentityStatus {
        session: current_session()?,
        vault,
    })
}

fn current_session() -> Result<Session, BrowserError> {
    SESSION.with(|session| {
        let session = session.borrow();
        let Some(vault) = session.as_ref() else {
            return Ok(Session::Locked);
        };
        let key = vault.keyring().active()?;
        Ok(Session::Unlocked(AuthorId(key.author().0.serialize())))
    })
}

pub fn session_epoch() -> u64 {
    EPOCH.with(|epoch| epoch.get().0)
}

fn bump_epoch() -> u64 {
    EPOCH.with(|epoch| epoch.set(epoch.get() + Wrapping(1)));
    session_epoch()
}

fn with_vault<T>(
    epoch: Epoch,
    act: impl FnOnce(&UnlockedVault) -> Result<T, BrowserError>,
) -> Result<T, BrowserError> {
    match epoch {
        Epoch::Exactly(expected) if expected != session_epoch() => {
            return Err(BrowserError::SignerChanged)
        }
        _current => (),
    }
    SESSION.with(|session| {
        let session = session.borrow();
        let Some(vault) = session.as_ref() else {
            return Err(BrowserError::Locked);
        };
        act(vault)
    })
}

pub fn with_signer<T>(
    epoch: Epoch,
    act: impl FnOnce(&IdentityKey) -> Result<T, BrowserError>,
) -> Result<T, BrowserError> {
    with_vault(epoch, |vault| act(&vault.keyring().active()?))
}

pub fn funder(epoch: Epoch) -> Result<Account, BrowserError> {
    with_vault(epoch, |vault| {
        Ok(Account::from_root(NETWORK, vault.keyring().root())?)
    })
}

fn unlocked(bytes: &[u8], password: &str) -> Result<UnlockedVault, BrowserError> {
    let vault = EncryptedVault::from_bytes(bytes).map_err(BrowserError::Unlock)?;
    vault
        .unlock(UnlockCredential::Password(password))
        .map_err(BrowserError::Unlock)
}

fn adopt(epoch: u64, vault: UnlockedVault) -> Result<IdentityStatus, BrowserError> {
    if session_epoch() != epoch {
        return Err(BrowserError::Cancelled);
    }
    SESSION.with(|session| *session.borrow_mut() = Some(vault));
    identity(Vault::Local)
}

async fn replaceable(replace: bool) -> Result<(), BrowserError> {
    match stored().await? {
        Copy::Stored(_bytes) if !replace => Err(BrowserError::LocalVaultExists),
        Copy::Stored(_bytes) => Ok(()),
        Copy::Absent => Ok(()),
    }
}

async fn store(sealed: &[u8], replace: bool) -> Result<(), BrowserError> {
    vault_store(VAULT_KEY, &serde_json::to_string(sealed)?, replace)
        .await
        .map_err(vault_storage)
}

pub async fn import(
    bytes: Vec<u8>,
    password: String,
    replace: bool,
) -> Result<IdentityStatus, BrowserError> {
    let epoch = bump_epoch();
    let password = Zeroizing::new(password);
    replaceable(replace).await?;
    let opened = unlocked(&bytes, &password)?;
    store(&bytes, replace).await?;
    adopt(epoch, opened)
}

pub async fn create(password: String, replace: bool) -> Result<Created, BrowserError> {
    let epoch = bump_epoch();
    let password = Zeroizing::new(password);
    replaceable(replace).await?;
    let Minted {
        vault,
        sealed,
        words,
    } = minted(Words::Fresh, &password)?;
    store(&sealed, replace).await?;
    Ok(Created {
        status: adopt(epoch, vault)?,
        words,
    })
}

pub async fn recover(
    phrase: String,
    password: String,
    replace: bool,
) -> Result<IdentityStatus, BrowserError> {
    let epoch = bump_epoch();
    let password = Zeroizing::new(password);
    let phrase = Zeroizing::new(phrase);
    replaceable(replace).await?;
    let Minted { vault, sealed, .. } = minted(Words::Given(phrase), &password)?;
    store(&sealed, replace).await?;
    adopt(epoch, vault)
}

pub async fn unlock(credential: Credential) -> Result<IdentityStatus, BrowserError> {
    let epoch = bump_epoch();
    let Credential::Password(password) = credential else {
        return Err(BrowserError::PasswordRequired);
    };
    let password = Zeroizing::new(password);
    let Copy::Stored(bytes) = stored().await? else {
        return Err(BrowserError::NoVault);
    };
    adopt(epoch, unlocked(&bytes, &password)?)
}

pub async fn lock() -> Result<IdentityStatus, BrowserError> {
    bump_epoch();
    SESSION.with(|session| *session.borrow_mut() = None);
    status().await
}
