use crate::browser::dispatch::handle;
use crate::errors::BrowserError;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wire_app_core::contract::{Request, Response};
use wire_app_core::errors::{Fault, FaultKind};

#[wasm_bindgen(inline_js = r#"
export function is_native() { return !!(window.__TAURI__ || window.__TAURI_INTERNALS__); }
export async function invoke_engine(request_json) {
    const request = JSON.parse(request_json);
    const core = window.__TAURI__ && window.__TAURI__.core;
    const internals = window.__TAURI_INTERNALS__;
    const invoke = (core && core.invoke) || (internals && internals.invoke);
    if (!invoke) return JSON.stringify({Err: {kind: 'Transport', message: 'Tauri IPC bridge not available'}});
    try {
        return JSON.stringify({Ok: await invoke('engine', {request})});
    } catch (fault) {
        const typed = fault && typeof fault === 'object' && 'kind' in fault && 'message' in fault;
        return JSON.stringify({Err: typed ? fault : {kind: 'Transport', message: 'Tauri IPC error: ' + String(fault)}});
    }
}
"#)]
extern "C" {
    pub fn is_native() -> bool;
    #[wasm_bindgen(catch)]
    async fn invoke_engine(request_json: &str) -> Result<JsValue, JsValue>;
}

pub async fn ask(request: Request) -> Result<Response, Fault> {
    match is_native() {
        true => over_ipc(request).await,
        false => Ok(handle(request).await?),
    }
}

async fn over_ipc(request: Request) -> Result<Response, Fault> {
    let asked = match serde_json::to_string(&request) {
        Ok(asked) => asked,
        Err(error) => return Err(transport(format!("Request encoding failed: {error}"))),
    };
    let answered = match invoke_engine(&asked).await {
        Ok(answered) => answered,
        Err(thrown) => {
            return Err(transport(format!(
                "Tauri IPC error: {}",
                BrowserError::from(thrown)
            )))
        }
    };
    let Some(text) = answered.as_string() else {
        return Err(transport("IPC call did not return a string result".into()));
    };
    match serde_json::from_str::<Result<Response, Fault>>(&text) {
        Ok(answer) => answer,
        Err(error) => Err(transport(format!("JSON decode error: {error}"))),
    }
}

fn transport(message: String) -> Fault {
    Fault::new(FaultKind::Transport, message)
}
