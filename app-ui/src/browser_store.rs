//! Durable app data uses one IDB store and one origin-wide Web Lock per operation.
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
function app_database() {
    return new Promise((resolve, reject) => {
        const r = indexedDB.open('urma-wire', 1);
        r.onupgradeneeded = () => r.result.createObjectStore('private');
        r.onsuccess = () => resolve(r.result);
        r.onerror = () => reject(new Error('Browser storage unavailable'));
    });
}
export async function app_read(key) {
    const db = await app_database();
    try { return await new Promise((resolve,reject) => {
        const tx = db.transaction('private','readonly');
        const r = tx.objectStore('private').get(key);
        tx.oncomplete = () => resolve(r.result || null);
        tx.onabort = tx.onerror = () => reject(new Error('Reading browser state failed'));
    }); } finally { db.close(); }
}
export async function app_write(key, value) {
    const db = await app_database();
    try { await new Promise((resolve,reject) => {
        const tx = db.transaction('private','readwrite');
        tx.objectStore('private').put(value,key);
        tx.oncomplete = resolve;
        tx.onabort = tx.onerror = () => reject(new Error('Saving browser state failed; no publication authorized'));
    }); } finally { db.close(); }
}
let release;
export async function acquire_app_lock() {
    if (!navigator.locks) throw new Error('This browser lacks Web Locks required for safe publication');
    return new Promise((resolve, reject) => {
        navigator.locks.request('urma-wire-chain-operation', {ifAvailable:true}, async lock => {
            if (!lock) { reject(new Error('Another tab or operation is using Wire. Retry when it finishes.')); return; }
            await new Promise(done => { release = done; resolve(); });
        }).catch(reject);
    });
}
export function release_app_lock() { if (release) { const done = release; release = undefined; done(); } }
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn app_read(key: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn app_write(key: &str, value: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(catch)]
    async fn acquire_app_lock() -> Result<(), JsValue>;
    fn release_app_lock();
}
fn error(e: JsValue) -> String {
    super::browser_source::js_error(e)
}
pub async fn read<T: DeserializeOwned>(key: &str) -> Result<Option<T>, String> {
    app_read(key)
        .await
        .map_err(error)?
        .as_string()
        .map(|value| serde_json::from_str(&value).map_err(|e| e.to_string()))
        .transpose()
}
pub async fn write(key: &str, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_string(value).map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("Browser state exceeds capacity".into());
    }
    app_write(key, &bytes).await.map_err(error)
}
pub struct Guard;
impl Guard {
    pub async fn acquire() -> Result<Self, String> {
        acquire_app_lock().await.map_err(error)?;
        Ok(Self)
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        release_app_lock();
    }
}

pub const DEFAULT_SOURCE: &str = "https://litecoinspace.org/testnet/api";
pub async fn source() -> Result<String, String> {
    Ok(read("chain-source")
        .await?
        .unwrap_or_else(|| DEFAULT_SOURCE.into()))
}
