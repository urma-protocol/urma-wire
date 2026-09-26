use crate::config::{DEFAULT_SOURCE, MAX_STATE_BYTES, SOURCE_KEY};
use crate::errors::BrowserError;
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

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
export async function app_delete(key) {
    const db = await app_database();
    try { await new Promise((resolve,reject) => {
        const tx = db.transaction('private','readwrite');
        tx.objectStore('private').delete(key);
        tx.oncomplete = resolve;
        tx.onabort = tx.onerror = () => reject(new Error('Discarding browser quote failed'));
    }); } finally { db.close(); }
}
let release, held;
export async function acquire_app_lock() {
    if (!navigator.locks) throw new Error('This browser lacks Web Locks required for safe publication');
    while (held) { const previous = held; try { await previous; } catch (_) {} if (held === previous) held = undefined; }
    return new Promise((resolve, reject) => {
        held = navigator.locks.request('urma-wire-chain-operation', {ifAvailable:true}, async lock => {
            if (!lock) { reject(new Error('Another tab or operation is using Wire. Retry when it finishes.')); return; }
            await new Promise(done => { release = done; resolve(); });
        });
        held.catch(reject);
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
    async fn app_delete(key: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(catch)]
    async fn acquire_app_lock() -> Result<(), JsValue>;
    fn release_app_lock();
}

pub enum Stored<T> {
    Found(T),
    Absent,
}

pub async fn read<T: DeserializeOwned>(key: &str) -> Result<Stored<T>, BrowserError> {
    let held = app_read(key).await?;
    let Some(text) = held.as_string() else {
        return Ok(Stored::Absent);
    };
    Ok(Stored::Found(serde_json::from_str(&text)?))
}

pub async fn write(key: &str, value: &impl Serialize) -> Result<(), BrowserError> {
    let text = serde_json::to_string(value)?;
    if text.len() > MAX_STATE_BYTES {
        return Err(BrowserError::StateCapacity);
    }
    Ok(app_write(key, &text).await?)
}

pub async fn delete(key: &str) -> Result<(), BrowserError> {
    Ok(app_delete(key).await?)
}

pub async fn source() -> Result<String, BrowserError> {
    match read::<String>(SOURCE_KEY).await? {
        Stored::Found(url) => Ok(url),
        Stored::Absent => Ok(DEFAULT_SOURCE.to_owned()),
    }
}

pub struct OperationLock;

impl OperationLock {
    pub async fn acquire() -> Result<Self, BrowserError> {
        acquire_app_lock().await?;
        Ok(Self)
    }
}

impl Drop for OperationLock {
    fn drop(&mut self) {
        release_app_lock();
    }
}
