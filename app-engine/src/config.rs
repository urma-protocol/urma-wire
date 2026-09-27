use std::num::NonZeroU64;
use urma_chain::observation::Chain;

pub const NETWORK: Chain = Chain::LitecoinTestnet;

pub const SYNC_BLOCKS: u64 = 100;

pub const HISTORY_BLOCKS: u64 = 100;

pub const FEE_RATE: NonZeroU64 = NonZeroU64::MIN;

#[cfg(not(target_arch = "wasm32"))]
pub const CONFIG_FILE_BYTES: usize = 16384;

#[cfg(not(target_arch = "wasm32"))]
pub const START_FILE_BYTES: usize = 64;

#[cfg(not(target_arch = "wasm32"))]
pub const PLAN_ID_FILE_BYTES: usize = 128;

#[cfg(not(target_arch = "wasm32"))]
pub const APPROVAL_FILE_BYTES: usize = 64;

#[cfg(not(target_arch = "wasm32"))]
pub const MAX_IMAGE_DOWNLOAD_BYTES: u64 = 4 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
pub const MAX_IMAGE_PIXELS: u64 = 4 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
pub const MAX_IMAGE_DIMENSION: u32 = 4096;

#[cfg(not(target_arch = "wasm32"))]
pub const MAX_IMAGE_RENDER_BYTES: usize = 20 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
pub const MAX_IMAGE_ALLOC_BYTES: u64 = 32 * 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
pub const IMAGE_TIMEOUT_SECS: u64 = 15;

#[cfg(not(target_arch = "wasm32"))]
pub const IMAGE_CONNECT_TIMEOUT_SECS: u64 = 5;

#[cfg(not(target_arch = "wasm32"))]
pub const SUPPORTED_IMAGE_FORMATS: &[image::ImageFormat] = &[
    image::ImageFormat::Png,
    image::ImageFormat::Jpeg,
    image::ImageFormat::WebP,
    image::ImageFormat::Gif,
    image::ImageFormat::Bmp,
    image::ImageFormat::Ico,
    image::ImageFormat::Tiff,
    #[cfg(not(target_os = "android"))]
    image::ImageFormat::Avif,
];

#[cfg(target_arch = "wasm32")]
pub const DEFAULT_SOURCE: &str = "https://litecoinspace.org/testnet/api";

#[cfg(target_arch = "wasm32")]
pub const SOURCE_KEY: &str = "chain-source";

#[cfg(target_arch = "wasm32")]
pub const INDEX_KEY: &str = "ltc-testnet-index-v2";

#[cfg(target_arch = "wasm32")]
pub const ACTIVE_KEY: &str = "ltc-testnet-active-plan";

#[cfg(target_arch = "wasm32")]
pub const INDEX_FORMAT: &str = "URMA-WIRE-INDEX-2";

#[cfg(target_arch = "wasm32")]
pub const MAX_CHECKPOINTS: usize = 500_000;

#[cfg(target_arch = "wasm32")]
pub const MAX_ENTRIES: usize = 50_000;

#[cfg(target_arch = "wasm32")]
pub const MAX_STATE_BYTES: usize = 64 * 1024 * 1024;

#[cfg(target_arch = "wasm32")]
pub const MAX_UTXOS: usize = 1000;

#[cfg(target_arch = "wasm32")]
pub const COINBASE_MATURITY: u64 = 100;

#[cfg(not(target_arch = "wasm32"))]
pub use settings::Settings;

#[cfg(not(target_arch = "wasm32"))]
mod settings {
    use super::{CONFIG_FILE_BYTES, NETWORK};
    use crate::errors::NativeError;
    use serde::Deserialize;
    use std::path::{Path, PathBuf};
    use urma_runtime::node::{Node, NodeConfig};

    #[derive(Default, Deserialize)]
    #[serde(default)]
    pub struct Settings {
        pub rpc_url: Option<String>,
        pub node_auth_file: Option<PathBuf>,
        pub vault: Option<PathBuf>,
        pub unlock_file: Option<PathBuf>,
    }

    impl Settings {
        pub fn mobile(app_data_dir: &Path) -> Self {
            Self {
                vault: Some(app_data_dir.join("identity.urma")),
                ..Self::default()
            }
        }

        pub fn load() -> Result<Self, NativeError> {
            let Some(home) = std::env::var_os("HOME") else {
                return Err(NativeError::NoHome);
            };
            let home = PathBuf::from(home);
            let path = match std::env::var_os("URMA_CONFIG") {
                Some(path) => PathBuf::from(path),
                None => home.join(".config/urma/config.json"),
            };
            let mut settings: Self = match path.try_exists()? {
                true => serde_json::from_slice(&urma_io::read_bounded(&path, CONFIG_FILE_BYTES)?)?,
                false => Self::default(),
            };
            match std::env::var_os("URMA_RPC_URL") {
                Some(url) => match url.into_string() {
                    Ok(url) => settings.rpc_url = Some(url),
                    Err(raw) => return Err(NativeError::RpcUrlEncoding(raw)),
                },
                None => (),
            }
            match std::env::var_os("URMA_NODE_AUTH_FILE") {
                Some(file) => settings.node_auth_file = Some(file.into()),
                None => (),
            }
            settings.vault = Some(match settings.vault {
                Some(vault) => vault,
                None => home.join(".local/share/urma/vault.urma"),
            });
            settings.unlock_file = Some(match settings.unlock_file {
                Some(file) => file,
                None => home.join(".config/urma/unlock"),
            });
            Ok(settings)
        }

        pub fn connect(&self) -> Result<Node, NativeError> {
            match (&self.rpc_url, &self.node_auth_file) {
                (Some(url), Some(cookie)) => Ok(Node::connect(NodeConfig {
                    chain: NETWORK,
                    rpc_url: url.clone(),
                    cookie_file: cookie.clone(),
                })?),
                (None, None) => Ok(Node::public(NETWORK)?),
                (Some(_url), None) => Err(NativeError::HalfConfiguredNode),
                (None, Some(_cookie)) => Err(NativeError::HalfConfiguredNode),
            }
        }
    }
}
