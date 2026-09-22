use crate::data::engine::ask;
use crate::data::failure::Failure;
#[cfg(target_arch = "wasm32")]
use crate::limits::MAX_VAULT_BYTES;
#[cfg(target_arch = "wasm32")]
use crate::limits::UNREADABLE_VAULT;
use serde_json::json;
use wire_app_shared::IdentityStatus;

pub enum Unlock {
    Password(String),
    ConfiguredFile,
}

pub async fn status() -> Result<IdentityStatus, Failure> {
    ask("identity_status", &json!({})).await
}

pub async fn import(vault_bytes: Vec<u8>, password: String) -> Result<IdentityStatus, Failure> {
    ask(
        "import_identity",
        &json!({ "vaultBytes": vault_bytes, "password": password }),
    )
    .await
}

pub async fn unlock(credential: Unlock) -> Result<(), Failure> {
    let (password, use_unlock_file) = match credential {
        Unlock::Password(password) => (password, false),
        Unlock::ConfiguredFile => (String::new(), true),
    };
    ask(
        "unlock_wallet",
        &json!({ "password": password, "useUnlockFile": use_unlock_file }),
    )
    .await
}

pub async fn lock() -> Result<(), Failure> {
    ask("lock_wallet", &json!({})).await
}

#[cfg(target_arch = "wasm32")]
pub async fn selected_vault() -> Result<Vec<u8>, Failure> {
    match read_identity_file(MAX_VAULT_BYTES).await {
        Ok(value) => decode_vault_bytes(value),
        Err(rejection) => Err(vault_rejection(rejection)),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn selected_vault() -> Result<Vec<u8>, Failure> {
    Err(Failure::BrowserOnly)
}

#[cfg(target_arch = "wasm32")]
fn decode_vault_bytes(value: wasm_bindgen::JsValue) -> Result<Vec<u8>, Failure> {
    let Some(text) = value.as_string() else {
        return Err(Failure::VaultUnreadable(UNREADABLE_VAULT.to_owned()));
    };
    match serde_json::from_str(&text) {
        Ok(bytes) => Ok(bytes),
        Err(error) => Err(Failure::Decode(error.to_string())),
    }
}

#[cfg(target_arch = "wasm32")]
fn vault_rejection(rejection: wasm_bindgen::JsValue) -> Failure {
    let Some(reason) = rejection.as_string() else {
        return Failure::VaultUnreadable(UNREADABLE_VAULT.to_owned());
    };
    Failure::VaultUnreadable(reason)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export async function read_identity_file(limit) {
    const input = document.getElementById('identity-vault-file');
    const file = input.files && input.files[0];
    if (!file) throw 'Choose an encrypted URMA vault first';
    if (file.size > limit) throw 'Encrypted URMA vault exceeds the size limit';
    const bytes = new Uint8Array(await file.arrayBuffer());
    input.value = '';
    return JSON.stringify(Array.from(bytes));
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn read_identity_file(
        limit: usize,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}
