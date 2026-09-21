use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
#[path = "browser.rs"]
mod browser;
#[cfg(target_arch = "wasm32")]
#[path = "browser_chain.rs"]
mod browser_chain;
#[cfg(all(target_arch = "wasm32", feature = "browser-fixture-test"))]
#[path = "browser_fixture.rs"]
mod browser_fixture;
#[cfg(target_arch = "wasm32")]
#[path = "browser_publish.rs"]
mod browser_publish;
#[cfg(target_arch = "wasm32")]
#[path = "browser_source.rs"]
mod browser_source;
#[cfg(target_arch = "wasm32")]
#[path = "browser_store.rs"]
mod browser_store;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformType {
    Web,
    Desktop,
    Mobile,
}

pub trait PlatformBridge: Send + Sync {
    fn platform_type(&self) -> PlatformType;
    fn platform_name(&self) -> &'static str;
    fn storage_name(&self) -> &'static str;
    fn can_autoload_images(&self) -> bool;
    fn open_external_url(&self, url: &str) -> Result<(), String>;
}

#[derive(Clone, Debug, Default)]
pub struct TauriPlatformBridge;

impl PlatformBridge for TauriPlatformBridge {
    fn platform_type(&self) -> PlatformType {
        #[cfg(target_arch = "wasm32")]
        if !is_native() {
            return PlatformType::Web;
        }
        #[cfg(target_os = "android")]
        {
            PlatformType::Mobile
        }
        #[cfg(not(target_os = "android"))]
        {
            PlatformType::Desktop
        }
    }

    fn platform_name(&self) -> &'static str {
        #[cfg(target_arch = "wasm32")]
        if !is_native() {
            return "Browser · local encrypted identity";
        }
        #[cfg(target_os = "android")]
        {
            "Tauri 2 Android"
        }
        #[cfg(not(target_os = "android"))]
        {
            "Tauri 2 Native"
        }
    }

    fn storage_name(&self) -> &'static str {
        #[cfg(target_arch = "wasm32")]
        if !is_native() {
            return "Browser persistent verified index · confirmed records";
        }
        "Persistent Disk Index (>= 1 Confirmed Only)"
    }

    fn can_autoload_images(&self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            return is_native();
        }
        #[cfg(not(target_arch = "wasm32"))]
        true
    }

    fn open_external_url(&self, url: &str) -> Result<(), String> {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                let _ = window.open_with_url_and_target(url, "_blank");
                return Ok(());
            }
        }
        let _ = url;
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = r#"
export function copy_text(text) {
    if (navigator && navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text);
    }
}
"#)]
extern "C" {
    fn copy_text(text: &str);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(
    inline_js = "export function is_native() { return !!(window.__TAURI__ || window.__TAURI_INTERNALS__); }"
)]
extern "C" {
    fn is_native() -> bool;
}

pub fn copy_to_clipboard(text: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        copy_text(text);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = text;
    }
}

pub fn default_bridge() -> Arc<dyn PlatformBridge> {
    Arc::new(TauriPlatformBridge)
}

pub fn use_platform_bridge() -> Arc<dyn PlatformBridge> {
    leptos::prelude::expect_context::<Arc<dyn PlatformBridge>>()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = r#"
export async function invoke_tauri_raw(cmd, args_json) {
    const args = args_json ? JSON.parse(args_json) : {};
    if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
        const res = await window.__TAURI__.core.invoke(cmd, args);
        return JSON.stringify(res);
    } else if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
        const res = await window.__TAURI_INTERNALS__.invoke(cmd, args);
        return JSON.stringify(res);
    }
    throw new Error("Tauri IPC bridge not available");
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn invoke_tauri_raw(cmd: &str, args_json: &str) -> Result<JsValue, JsValue>;
}

/// Dispatches a strongly-typed IPC command through Tauri 2.
pub async fn tauri_call<T: serde::de::DeserializeOwned>(
    cmd: &str,
    args: &serde_json::Value,
) -> Result<T, String> {
    #[cfg(target_arch = "wasm32")]
    {
        if !is_native() {
            return serde_json::from_value(browser::dispatch(cmd, args).await?)
                .map_err(|e| format!("Browser response: {e}"));
        }
        let args_str = args.to_string();
        match invoke_tauri_raw(cmd, &args_str).await {
            Ok(js_val) => {
                if let Some(json_str) = js_val.as_string() {
                    serde_json::from_str::<T>(&json_str)
                        .map_err(|e| format!("JSON decode error: {e}"))
                } else {
                    Err("IPC call did not return a string result".into())
                }
            }
            Err(e) => {
                let err_msg = e
                    .as_string()
                    .unwrap_or_else(|| "Unknown Tauri IPC error".into());
                Err(format!("Tauri IPC error: {err_msg}"))
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (cmd, args);
        Err("Tauri IPC is not supported on non-WASM targets".into())
    }
}
