use crate::errors::BrowserError;
use std::cell::{Cell, RefCell};
use std::num::Wrapping;
use urma_identity::identity::IdentityKey;
use urma_identity::vault::{EncryptedVault, UnlockCredential, UnlockedVault};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wire_app_core::model::AuthorId;
use wire_app_core::session::{Credential, IdentityStatus, Session, VaultSource};
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
export async function vault_read() {
    const db = await database();
    try {
        return await new Promise((resolve, reject) => {
            const tx = db.transaction('private', 'readonly');
            const request = tx.objectStore('private').get('identity-v1');
            tx.oncomplete = () => resolve(request.result || null);
            tx.onabort = tx.onerror = () => reject(new Error('Cannot read encrypted vault'));
        });
    } finally { db.close(); }
}
export async function vault_insert(bytes) {
    const db = await database();
    try {
        await new Promise((resolve, reject) => {
            const tx = db.transaction('private', 'readwrite');
            tx.objectStore('private').add(bytes, 'identity-v1');
            tx.oncomplete = resolve;
            tx.onabort = tx.onerror = () => reject(new Error('Vault already exists or storage failed; existing vault was preserved'));
        });
    } finally { db.close(); }
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn vault_read() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn vault_insert(bytes: &str) -> Result<(), JsValue>;
}

enum Vault {
    Stored(Vec<u8>),
    Absent,
}

pub enum Epoch {
    Any,
    Exactly(u64),
}

fn vault_storage(thrown: JsValue) -> BrowserError {
    BrowserError::VaultStorage(BrowserError::from(thrown).to_string())
}

async fn stored() -> Result<Vault, BrowserError> {
    let held = vault_read().await.map_err(vault_storage)?;
    let Some(text) = held.as_string() else {
        return Ok(Vault::Absent);
    };
    let bytes = serde_json::from_str(&text).map_err(BrowserError::StoredVault)?;
    Ok(Vault::Stored(bytes))
}

pub async fn status() -> Result<IdentityStatus, BrowserError> {
    let available = match stored().await? {
        Vault::Stored(_bytes) => true,
        Vault::Absent => false,
    };
    identity(available)
}

fn identity(available: bool) -> Result<IdentityStatus, BrowserError> {
    Ok(IdentityStatus {
        session: current_session()?,
        vault_available: available,
        source: match available {
            true => VaultSource::Imported,
            false => VaultSource::Missing,
        },
        configured_unlock_available: false,
        configured_unlock_path: String::new(),
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

fn bump_epoch() {
    EPOCH.with(|epoch| epoch.set(epoch.get() + Wrapping(1)));
}

pub fn with_signer<T>(
    epoch: Epoch,
    act: impl FnOnce(&IdentityKey) -> Result<T, BrowserError>,
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
        act(&vault.keyring().active()?)
    })
}

fn unlocked(bytes: &[u8], password: &str) -> Result<UnlockedVault, BrowserError> {
    let vault = EncryptedVault::from_bytes(bytes).map_err(BrowserError::Unlock)?;
    vault
        .unlock(UnlockCredential::Password(password))
        .map_err(BrowserError::Unlock)
}

fn adopt(epoch: u64, vault: UnlockedVault) -> Result<(), BrowserError> {
    if session_epoch() != epoch {
        return Err(BrowserError::Cancelled);
    }
    SESSION.with(|session| *session.borrow_mut() = Some(vault));
    Ok(())
}

pub async fn import(vault: Vec<u8>, password: String) -> Result<IdentityStatus, BrowserError> {
    bump_epoch();
    let epoch = session_epoch();
    let password = Zeroizing::new(password);
    let opened = unlocked(&vault, &password)?;
    vault_insert(&serde_json::to_string(&vault)?)
        .await
        .map_err(vault_storage)?;
    adopt(epoch, opened)?;
    identity(true)
}

pub async fn unlock(credential: Credential) -> Result<(), BrowserError> {
    bump_epoch();
    let epoch = session_epoch();
    let Credential::Password(password) = credential else {
        return Err(BrowserError::PasswordRequired);
    };
    let password = Zeroizing::new(password);
    let Vault::Stored(bytes) = stored().await? else {
        return Err(BrowserError::NoVault);
    };
    adopt(epoch, unlocked(&bytes, &password)?)
}

pub fn lock() {
    bump_epoch();
    SESSION.with(|session| *session.borrow_mut() = None);
}
