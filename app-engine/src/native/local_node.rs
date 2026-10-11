use crate::config::{Settings, LOCAL_NODE_PROBE_SECONDS, NETWORK};
use serde::Deserialize;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use urma_runtime::error::Error;
use urma_runtime::node::{Node, NodeConfig};
use urma_runtime::transport::{BlockEncoding, Evidence, Provider};
use wire_app_core::connection::{safe_endpoint, SourceChoice};

#[derive(Clone)]
enum Availability {
    Unavailable,
    Ready(Arc<Node>),
}

impl Availability {
    fn ready(&self) -> bool {
        matches!(self, Self::Ready(_node))
    }
}

pub enum LocalSource {
    Unconfigured,
    Configured(Arc<LocalNode>),
}

impl LocalSource {
    pub fn ready(&self) -> bool {
        match self {
            Self::Unconfigured => false,
            Self::Configured(local) => local.ready(),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        match self {
            Self::Unconfigured => (),
            Self::Configured(local) => local.set_enabled(enabled),
        }
    }

    pub fn providers(&self) -> Vec<Box<dyn Provider>> {
        match self {
            Self::Unconfigured => Vec::new(),
            Self::Configured(local) => vec![Box::new(LocalProvider(local.clone()))],
        }
    }
}

#[derive(Deserialize)]
struct ChainState {
    initialblockdownload: bool,
    blocks: u64,
    headers: u64,
}

pub struct LocalNode {
    config: NodeConfig,
    enabled: AtomicBool,
    node: Mutex<Availability>,
}

impl LocalNode {
    pub fn configured(settings: &Settings) -> Result<LocalSource, std::io::Error> {
        let (Some(url), Some(cookie)) = (&settings.rpc_url, &settings.node_auth_file) else {
            return Ok(LocalSource::Unconfigured);
        };
        let local = Arc::new(Self {
            config: NodeConfig {
                chain: NETWORK,
                rpc_url: url.clone(),
                cookie_file: cookie.clone(),
            },
            node: Mutex::new(Availability::Unavailable),
            enabled: AtomicBool::new(settings.source == SourceChoice::Public),
        });
        let weak = Arc::downgrade(&local);
        std::thread::Builder::new()
            .name("wire-local-node".into())
            .spawn(move || loop {
                let Some(local) = weak.upgrade() else {
                    break;
                };
                if local.enabled.load(Ordering::Acquire) {
                    local.probe();
                }
                drop(local);
                std::thread::sleep(Duration::from_secs(LOCAL_NODE_PROBE_SECONDS));
            })?;
        Ok(LocalSource::Configured(local))
    }

    fn current(&self) -> Availability {
        match self.node.lock() {
            Ok(node) => node.clone(),
            Err(poisoned) => {
                tracing::error!("local source lock poisoned; reading held state");
                poisoned.into_inner().clone()
            }
        }
    }

    pub fn ready(&self) -> bool {
        self.enabled.load(Ordering::Acquire) && self.current().ready()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Release);
        if !enabled {
            self.replace(Availability::Unavailable);
        }
    }

    fn replace(&self, next: Availability) {
        let mut held = match self.node.lock() {
            Ok(held) => held,
            Err(poisoned) => {
                tracing::error!("local source lock poisoned; updating held state");
                poisoned.into_inner()
            }
        };
        if held.ready() != next.ready() {
            tracing::info!(available = next.ready(), endpoint = %safe_endpoint(&self.config.rpc_url),
                "automatic local block source changed");
        }
        *held = next;
    }

    fn probe(&self) {
        let result = Node::connect(self.config.clone()).and_then(|node| {
            let info: ChainState = serde_json::from_value(node.call("getblockchaininfo", &[])?)?;
            if info.initialblockdownload || info.blocks < info.headers {
                return Err(Error::Unsupported(
                    "local node is still synchronizing".into(),
                ));
            }
            Ok(Arc::new(node))
        });
        match result {
            Ok(node) => {
                if self.enabled.load(Ordering::Acquire) {
                    self.replace(Availability::Ready(node));
                }
            }
            Err(error) => {
                if self.ready() {
                    tracing::warn!(%error, "local block source unavailable; public fallback remains");
                }
                tracing::debug!(%error, "local block source probe failed");
                self.replace(Availability::Unavailable);
            }
        }
    }
}

pub struct LocalProvider(pub Arc<LocalNode>);

impl Provider for LocalProvider {
    fn label(&self) -> String {
        safe_endpoint(&self.0.config.rpc_url)
    }
    fn evidence(&self) -> Evidence {
        Evidence::LocalValidatingNode
    }
    fn block_encoding(&self) -> BlockEncoding {
        BlockEncoding::Core
    }
    fn supports(&self, method: &str) -> bool {
        matches!(method, "getblock" | "getblockhash" | "getblockheader") && self.0.ready()
    }
    fn call(
        &self,
        chain: urma_chain::observation::Chain,
        method: &str,
        args: &[Value],
    ) -> Result<Value, Error> {
        if chain.genesis()? != self.0.config.chain.genesis()?
            || !matches!(method, "getblock" | "getblockhash" | "getblockheader")
        {
            return Err(Error::Unsupported(
                "automatic local source serves configured-chain block reads only".into(),
            ));
        }
        let Availability::Ready(node) = self.0.current() else {
            return Err(Error::Unsupported("local daemon unavailable".into()));
        };
        match node.call(method, args) {
            Ok(value) => Ok(value),
            Err(error) => {
                self.0.replace(Availability::Unavailable);
                Err(error)
            }
        }
    }
}
