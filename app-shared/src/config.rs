use serde::Deserialize;
use std::path::PathBuf;
use urma_chain::observation::Chain;
use urma_runtime::{
    error::Error,
    node::{Node, NodeConfig},
};

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub rpc_url: Option<String>,
    pub node_auth_file: Option<PathBuf>,
    pub vault: Option<PathBuf>,
    pub unlock_file: Option<PathBuf>,
}

impl Settings {
    pub fn load() -> Result<Self, Error> {
        let home = home()?;
        let path = std::env::var_os("URMA_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config/urma/config.json"));
        let mut settings: Self = if path.try_exists()? {
            serde_json::from_slice(&urma_io::read_bounded(&path, 16384)?)?
        } else {
            Self::default()
        };
        if let Some(url) = std::env::var_os("URMA_RPC_URL") {
            settings.rpc_url = Some(
                url.into_string()
                    .map_err(|_| Error::Invalid("RPC URL must be UTF-8".into()))?,
            );
        }
        if let Some(path) = std::env::var_os("URMA_NODE_AUTH_FILE") {
            settings.node_auth_file = Some(path.into());
        }
        settings.vault = Some(
            settings
                .vault
                .unwrap_or_else(|| home.join(".local/share/urma/vault.urma")),
        );
        settings.unlock_file = Some(
            settings
                .unlock_file
                .unwrap_or_else(|| home.join(".config/urma/unlock")),
        );
        Ok(settings)
    }

    pub fn connect(&self) -> Result<Node, Error> {
        match (&self.rpc_url, &self.node_auth_file) {
            (Some(url), Some(cookie)) => Node::connect(NodeConfig {
                chain: Chain::LitecoinTestnet,
                rpc_url: url.clone(),
                cookie_file: cookie.clone(),
            }),
            (None, None) => Node::public(Chain::LitecoinTestnet),
            _ => Err(Error::Invalid(
                "Configure both rpc_url and node_auth_file, or neither".into(),
            )),
        }
    }
}

fn home() -> Result<PathBuf, Error> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| Error::Invalid("HOME is not set".into()))
}
