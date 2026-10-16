#[cfg(target_arch = "wasm32")]
use crate::config::UNREADABLE_VAULT;
use crate::data::engine::ask;
use wire_app_core::contract::{Request, Response};
use wire_app_core::errors::{Fault, FaultKind};
use wire_app_core::session::{Credential, IdentityStatus};

pub async fn status() -> Result<IdentityStatus, Fault> {
    match ask(Request::IdentityStatus).await? {
        Response::Identity(status) => Ok(status),
        other => Err(other.unexpected("identity status")),
    }
}

pub async fn import(vault: Vec<u8>, password: String) -> Result<IdentityStatus, Fault> {
    match ask(Request::ImportIdentity { vault, password }).await? {
        Response::Identity(status) => Ok(status),
        other => Err(other.unexpected("identity import")),
    }
}

pub struct Created {
    pub status: IdentityStatus,
    pub phrase: String,
}

pub async fn create(phrase: String, password: String) -> Result<Created, Fault> {
    match ask(Request::CreateIdentity { phrase, password }).await? {
        Response::Created { status, phrase } => Ok(Created { status, phrase }),
        other => Err(other.unexpected("identity creation")),
    }
}

pub async fn unlock(credential: Credential) -> Result<(), Fault> {
    match ask(Request::Unlock(credential)).await? {
        Response::Unlocked => Ok(()),
        other => Err(other.unexpected("unlock")),
    }
}

pub async fn lock() -> Result<(), Fault> {
    match ask(Request::Lock).await? {
        Response::Locked => Ok(()),
        other => Err(other.unexpected("lock")),
    }
}

#[cfg(target_arch = "wasm32")]
fn unreadable(reason: String) -> Fault {
    Fault::new(FaultKind::Identity, reason)
}

#[cfg(target_arch = "wasm32")]
pub async fn selected_vault() -> Result<Vec<u8>, Fault> {
    match read_identity_file(wire_app_core::config::MAX_VAULT_BYTES).await {
        Ok(value) => decode_vault_bytes(value),
        Err(rejection) => Err(vault_rejection(rejection)),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn selected_vault() -> Result<Vec<u8>, Fault> {
    Err(Fault::new(
        FaultKind::Unsupported,
        "File selection requires the app frontend",
    ))
}

#[cfg(target_arch = "wasm32")]
fn decode_vault_bytes(value: wasm_bindgen::JsValue) -> Result<Vec<u8>, Fault> {
    let Some(text) = value.as_string() else {
        return Err(unreadable(UNREADABLE_VAULT.to_owned()));
    };
    match serde_json::from_str(&text) {
        Ok(bytes) => Ok(bytes),
        Err(error) => Err(unreadable(error.to_string())),
    }
}

#[cfg(target_arch = "wasm32")]
fn vault_rejection(rejection: wasm_bindgen::JsValue) -> Fault {
    let Some(reason) = rejection.as_string() else {
        return unreadable(UNREADABLE_VAULT.to_owned());
    };
    unreadable(reason)
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
