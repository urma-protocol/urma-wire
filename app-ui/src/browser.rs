//! Browser identity adapter. Only encrypted URMA bytes cross the IDB boundary.
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use urma_identity::vault::{EncryptedVault, UnlockCredential, UnlockedVault};
use wasm_bindgen::prelude::*;
use zeroize::Zeroizing;

thread_local! { static SESSION: RefCell<Option<UnlockedVault>> = const { RefCell::new(None) }; }
thread_local! { static EPOCH: Cell<u64> = const { Cell::new(0) }; }

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

fn storage_error(_: JsValue) -> String {
    "Encrypted browser storage failed (an existing vault is never overwritten)".into()
}

async fn stored() -> Result<Option<Vec<u8>>, String> {
    let value = vault_read().await.map_err(storage_error)?;
    value
        .as_string()
        .map(|s| serde_json::from_str(&s).map_err(|_| "Invalid stored vault".into()))
        .transpose()
}

fn status(available: bool) -> Result<Value, String> {
    SESSION.with(|session| {
        let session = session.borrow();
        let author = session
            .as_ref()
            .map(|vault| {
                vault
                    .keyring()
                    .active()
                    .map(|key| key.author().0.to_string())
                    .map_err(|e| e.to_string())
            })
            .transpose()?;
        Ok(
            json!({"unlocked": session.is_some(), "vault_available": available,
            "source": if available { "imported" } else { "none" },
            "author_fingerprint": author.as_ref().map(|s| &s[..8]),
            "author_pubkey": author, "configured_unlock_available": false}),
        )
    })
}

pub async fn dispatch(cmd: &str, args: &Value) -> Result<Value, String> {
    match cmd {
        "identity_status" => status(stored().await?.is_some()),
        "lock_wallet" => {
            EPOCH.with(|e| e.set(e.get().wrapping_add(1)));
            SESSION.with(|s| *s.borrow_mut() = None);
            Ok(Value::Null)
        }
        "import_identity" | "unlock_wallet" => {
            let epoch = EPOCH.with(|e| { e.set(e.get().wrapping_add(1)); e.get() });
            if args.get("useUnlockFile").and_then(Value::as_bool) == Some(true) {
                return Err("Browser identities require a password each session".into());
            }
            let password = Zeroizing::new(args.get("password").and_then(Value::as_str)
                .ok_or("Enter the vault password")?.to_owned());
            let importing = cmd == "import_identity";
            let bytes: Vec<u8> = if importing {
                serde_json::from_value(args.get("vaultBytes").cloned().ok_or("Select an encrypted URMA vault")?)
                    .map_err(|_| "Invalid vault bytes")?
            } else { stored().await?.ok_or("Import an encrypted URMA vault first")? };
            let unlocked = EncryptedVault::from_bytes(&bytes).and_then(|v| v.unlock(UnlockCredential::Password(&password)))
                .map_err(|_| "Vault unlock failed; check the password and URMA vault file")?;
            if importing { vault_insert(&serde_json::to_string(&bytes).map_err(|e| e.to_string())?).await.map_err(storage_error)?; }
            if EPOCH.with(|e| e.get()) != epoch { return Err("Identity operation cancelled by a newer lock or unlock".into()); }
            SESSION.with(|s| *s.borrow_mut() = Some(unlocked));
            if importing { status(true) } else { Ok(Value::Null) }
        }
        "publication_status" => Ok(Value::Null),
        _ => Err("Browser chain sync and publication are not available in this build. Native Wire is required for verified chain operations; unlocking here does not fetch balances or submit transactions.".into()),
    }
}

// Only explicit local fixture builds export this harness. No fixture API ships by default.
#[cfg(feature = "browser-fixture-test")]
#[wasm_bindgen]
pub async fn browser_fixture_command(cmd: String, args: String) -> Result<String, JsValue> {
    if cmd == "fixture-sign" {
        use urma_identity::identity::IdentitySigner;
        return SESSION
            .with(|session| {
                let session = session.borrow();
                let key = session
                    .as_ref()
                    .ok_or("locked")?
                    .keyring()
                    .active()
                    .map_err(|e| e.to_string())?;
                let message = bitcoin::secp256k1::Message::from_digest([42; 32]);
                let signature = key.sign_author(message).map_err(|e| e.to_string())?;
                bitcoin::secp256k1::Secp256k1::new()
                    .verify_schnorr(&signature, &message, &key.author().0)
                    .map_err(|e| e.to_string())?;
                Ok(signature.to_string())
            })
            .map_err(|e: String| JsValue::from_str(&e));
    }
    let args = serde_json::from_str(&args).map_err(|e| JsValue::from_str(&e.to_string()))?;
    dispatch(&cmd, &args)
        .await
        .map(|v| v.to_string())
        .map_err(|e| JsValue::from_str(&e))
}
