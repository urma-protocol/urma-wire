use crate::config::{NETWORK, SOURCE_FILE};
use crate::errors::NativeError;
use crate::native::service::Service;
use urma_runtime::endpoints;
use urma_runtime::light::LightSync;
use urma_runtime::transport::{Evidence, Standing};
use wire_app_core::connection::{
    safe_endpoint, SourceChoice, SourceKind, SourceStanding, SourceState, SourceStatus,
    TransportStatus,
};

impl Service {
    pub fn transport(&self) -> Result<TransportStatus, NativeError> {
        let node = self.node()?;
        let light = node.light_client_progress();
        let peers = match light {
            LightSync::Running(progress) => u64::try_from(progress.peers_connected)?,
            LightSync::Cold(_progress) => 0,
            LightSync::Absent => 0,
        };
        let mut sources: Vec<SourceState> = node
            .standings()
            .into_iter()
            .map(|standing| {
                let kind = kind_of(&standing);
                SourceState {
                    label: shown(&standing.label),
                    standing: standing_of(kind, &standing, light),
                    kind,
                }
            })
            .collect();
        if self.settings().source == SourceChoice::Public {
            for source in &mut sources {
                if source.kind == SourceKind::LocalNode {
                    source.standing = match self.local_node.ready() {
                        true => source.standing.clone(),
                        false => SourceStanding::Cold,
                    };
                }
            }
        }
        Ok(TransportStatus { peers, sources })
    }

    pub fn source_status(&self) -> SourceStatus {
        let settings = self.settings().clone();
        let endpoints = match &settings.source {
            SourceChoice::Public => {
                let open = self.open_provider_labels();
                match open.is_empty() {
                    true => public_endpoints(),
                    false => open.iter().map(|label| shown(label)).collect(),
                }
            }
            SourceChoice::LocalNode => settings
                .rpc_url
                .iter()
                .map(|url| safe_endpoint(url))
                .collect(),
            SourceChoice::Esplora(url) => vec![safe_endpoint(url)],
        };
        SourceStatus {
            choice: settings.source.clone(),
            endpoints,
            local_node_configured: settings.local_node_configured(),
            local_endpoint: settings.rpc_url.as_deref().map(safe_endpoint),
            cookie_available: settings.cookie_available(),
        }
    }

    pub fn adopt_source(&self, choice: SourceChoice) -> Result<SourceStatus, NativeError> {
        let settings = self.settings().with_source(choice);
        let node = self.connect_node(&settings)?;
        urma_io::write_replace(
            &self.directory.join(SOURCE_FILE),
            &serde_json::to_vec(&settings.source)?,
        )?;
        self.local_node
            .set_enabled(settings.source == SourceChoice::Public);
        *self.settings_mut() = settings;
        self.replace_link(node);
        Ok(self.source_status())
    }

    pub(super) fn described_source(&self) -> (&'static str, String) {
        let label = match &self.settings().source {
            SourceChoice::Public => "Public source pool",
            SourceChoice::LocalNode => "Configured RPC",
            SourceChoice::Esplora(_url) => "Browser Esplora API",
        };
        (label, self.source_status().endpoints.join(", "))
    }
}

fn public_endpoints() -> Vec<String> {
    endpoints::defaults(NETWORK)
        .iter()
        .map(|endpoint| safe_endpoint(endpoint.url()))
        .collect()
}

fn shown(label: &str) -> String {
    match label.contains("://") {
        true => safe_endpoint(label),
        false => label.to_owned(),
    }
}

fn kind_of(standing: &Standing) -> SourceKind {
    match standing.evidence {
        Evidence::LocalValidatingNode => SourceKind::LocalNode,
        Evidence::LightClientInclusion => SourceKind::Peers,
        Evidence::PublicProviderObservation => {
            match standing.label.starts_with("ssl://") || standing.label.starts_with("tcp://") {
                true => SourceKind::Electrum,
                false => SourceKind::Http,
            }
        }
    }
}

fn standing_of(kind: SourceKind, standing: &Standing, light: LightSync) -> SourceStanding {
    match kind {
        SourceKind::Peers => match light {
            LightSync::Running(_progress) => SourceStanding::Ready,
            LightSync::Cold(_progress) => SourceStanding::Cold,
            LightSync::Absent => SourceStanding::Cold,
        },
        SourceKind::LocalNode | SourceKind::Electrum | SourceKind::Http => {
            match standing.blocked_for.is_zero() {
                true => SourceStanding::Ready,
                false => SourceStanding::Blocked {
                    seconds: standing.blocked_for.as_secs() + 1,
                },
            }
        }
    }
}
