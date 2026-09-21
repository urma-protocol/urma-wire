//! Browser identity adapter. Only encrypted URMA bytes cross the IDB boundary.
use super::{
    browser_chain as chain, browser_publish as publish, browser_source::Source,
    browser_store as store,
};
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

pub fn session_epoch() -> u64 {
    EPOCH.with(Cell::get)
}
pub fn with_signer<T>(
    epoch: Option<u64>,
    f: impl FnOnce(&urma_identity::identity::IdentityKey) -> Result<T, String>,
) -> Result<T, String> {
    if epoch.is_some_and(|epoch| epoch != session_epoch()) {
        return Err("Identity changed or locked while preparing; review again".into());
    }
    SESSION.with(|session| {
        let session = session.borrow();
        let key = session
            .as_ref()
            .ok_or("Unlock your identity in Settings first")?
            .keyring()
            .active()
            .map_err(|e| e.to_string())?;
        f(&key)
    })
}

#[wasm_bindgen(
    inline_js = "export function chain_notice(message) { window.dispatchEvent(new CustomEvent('urma:chain-status', {detail: message})); }"
)]
extern "C" {
    fn chain_notice(message: &str);
}

async fn chain_command(cmd: &str, args: &Value) -> Result<Value, String> {
    if args
        .get("chain")
        .and_then(Value::as_str)
        .is_some_and(|chain| chain != "ltc")
    {
        return Err("Only Litecoin Testnet is enabled".into());
    }
    if cmd == "get_chain_source" {
        return Ok(json!(store::source().await?));
    }
    if cmd == "publication_status" && publish::active().await?.is_none() {
        return Ok(Value::Null);
    }
    let _guard = store::Guard::acquire().await?;
    if cmd == "set_chain_source" {
        let url = args["url"]
            .as_str()
            .ok_or("Missing source URL")?
            .trim()
            .trim_end_matches('/')
            .to_owned();
        Source::connect(url.clone()).await?;
        store::write("chain-source", &url).await?;
        return Ok(Value::Null);
    }
    if cmd == "get_post" || cmd == "get_replies" {
        let txid = args["txid"].as_str().ok_or("Missing post transaction")?;
        let Some(index) = chain::cached().await? else {
            return Ok(if cmd == "get_post" {
                Value::Null
            } else {
                json!([])
            });
        };
        let feed = chain::project(
            &index,
            false,
            if cmd == "get_replies" {
                Some(txid)
            } else {
                None
            },
        )?;
        return if cmd == "get_post" {
            serde_json::to_value(
                feed.posts
                    .into_iter()
                    .find(|p| p.txid.to_display_hex() == txid),
            )
        } else {
            serde_json::to_value(feed.posts)
        }
        .map_err(|e| e.to_string());
    }
    let connection = Source::connect(store::source().await?).await;
    if cmd == "get_feed" {
        let refreshed = match connection {
            Ok(source) => chain::sync(&source).await,
            Err(error) => Err(error),
        };
        let feed = match refreshed {
            Ok(feed) => {
                chain_notice("Chain source connected. Confirmed records verified; recent-history catch-up is shown in the feed.");
                feed
            }
            Err(error) => {
                chain_notice(&format!(
                    "Showing cached verified history; chain refresh failed: {error}"
                ));
                let index = chain::cached().await?.ok_or(error)?;
                chain::project(&index, false, None)?
            }
        };
        return serde_json::to_value(feed).map_err(|e| e.to_string());
    }
    if cmd == "publication_status" {
        let active = publish::active().await?.ok_or("No publication")?;
        let observed = match connection {
            Ok(source) => publish::observe(&source, &active).await,
            Err(error) => Err(error),
        };
        let publication = match observed {
            Ok(publication) => publication,
            Err(error) => {
                chain_notice(&format!(
                    "Saved publication; current chain status unavailable: {error}"
                ));
                publish::offline_status(&active)?
            }
        };
        return serde_json::to_value(publication).map_err(|e| e.to_string());
    }
    let source = connection?;
    let value = match cmd {
        "get_wallet_summary" => serde_json::to_value(publish::wallet(&source).await?),
        "quote_publication" => serde_json::to_value(publish::prepare(&source, args).await?),
        "advance_publication" => serde_json::to_value(
            publish::advance(
                &source,
                args["id"].as_str().ok_or("Missing exact plan ID")?,
                args["approve"].as_bool().unwrap_or(false),
            )
            .await?,
        ),
        _ => return Err(format!("Unsupported browser command: {cmd}")),
    };
    value.map_err(|e| e.to_string())
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
            let epoch = EPOCH.with(|e| {
                e.set(e.get().wrapping_add(1));
                e.get()
            });
            if args.get("useUnlockFile").and_then(Value::as_bool) == Some(true) {
                return Err("Browser identities require a password each session".into());
            }
            let password = Zeroizing::new(
                args.get("password")
                    .and_then(Value::as_str)
                    .ok_or("Enter the vault password")?
                    .to_owned(),
            );
            let importing = cmd == "import_identity";
            let bytes: Vec<u8> = if importing {
                serde_json::from_value(
                    args.get("vaultBytes")
                        .cloned()
                        .ok_or("Select an encrypted URMA vault")?,
                )
                .map_err(|_| "Invalid vault bytes")?
            } else {
                stored()
                    .await?
                    .ok_or("Import an encrypted URMA vault first")?
            };
            let unlocked = EncryptedVault::from_bytes(&bytes)
                .and_then(|v| v.unlock(UnlockCredential::Password(&password)))
                .map_err(|_| "Vault unlock failed; check the password and URMA vault file")?;
            if importing {
                vault_insert(&serde_json::to_string(&bytes).map_err(|e| e.to_string())?)
                    .await
                    .map_err(storage_error)?;
            }
            if EPOCH.with(|e| e.get()) != epoch {
                return Err("Identity operation cancelled by a newer lock or unlock".into());
            }
            SESSION.with(|s| *s.borrow_mut() = Some(unlocked));
            if importing {
                status(true)
            } else {
                Ok(Value::Null)
            }
        }
        _ => chain_command(cmd, args).await,
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
    if cmd == "fixture-source-probe" {
        let args: Value = args;
        let source = Source::connect(args["url"].as_str().ok_or("Source URL required")?.into())
            .await
            .map_err(|e| JsValue::from_str(&e))?;
        let (height, _) = source.tip().await.map_err(|e| JsValue::from_str(&e))?;
        let block = source
            .block(height)
            .await
            .map_err(|e| JsValue::from_str(&e))?;
        return Ok(json!({"height": height, "hash": block.block_hash().to_string(), "transactions": block.txdata.len()}).to_string());
    }
    if cmd.starts_with("fixture-") {
        return super::browser_fixture::command(&cmd, &args)
            .map(|value| value.to_string())
            .map_err(|e| JsValue::from_str(&e));
    }
    dispatch(&cmd, &args)
        .await
        .map(|v| v.to_string())
        .map_err(|e| JsValue::from_str(&e))
}
