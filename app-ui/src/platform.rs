use leptos::prelude::expect_context;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Browser,
    Desktop,
}

impl Platform {
    pub fn detected() -> Self {
        match native() {
            true => Platform::Desktop,
            false => Platform::Browser,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Platform::Browser => "Browser · local encrypted identity",
            Platform::Desktop => "Tauri 2 Native",
        }
    }

    pub fn storage_name(self) -> &'static str {
        match self {
            Platform::Browser => "Browser persistent verified index · confirmed records",
            Platform::Desktop => "Persistent Disk Index (>= 1 Confirmed Only)",
        }
    }

    pub fn can_autoload_images(self) -> bool {
        match self {
            Platform::Browser => false,
            Platform::Desktop => true,
        }
    }
}

pub fn use_platform() -> Platform {
    expect_context::<Platform>()
}

#[cfg(target_arch = "wasm32")]
fn native() -> bool {
    wire_app_engine::client::is_native()
}

#[cfg(not(target_arch = "wasm32"))]
fn native() -> bool {
    true
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
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
pub fn copy_to_clipboard(text: &str) {
    copy_text(text);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn copy_to_clipboard(_text: &str) {}
