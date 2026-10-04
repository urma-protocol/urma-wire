use crate::config::{NETWORK, SOURCE_FILE};
use crate::errors::NativeError;
use crate::native::service::Service;
use urma_runtime::endpoints;
use wire_app_core::connection::{safe_endpoint, SourceChoice, SourceStatus};

impl Service {
    pub fn source_status(&self) -> SourceStatus {
        let endpoints = match &self.settings.source {
            SourceChoice::Public => {
                let open = self.open_provider_labels();
                match open.is_empty() {
                    true => public_endpoints(),
                    false => open.iter().map(|label| shown(label)).collect(),
                }
            }
            SourceChoice::LocalNode => self
                .settings
                .rpc_url
                .iter()
                .map(|url| safe_endpoint(url))
                .collect(),
            SourceChoice::Esplora(url) => vec![safe_endpoint(url)],
        };
        SourceStatus {
            choice: self.settings.source.clone(),
            endpoints,
            local_node_configured: self.settings.local_node_configured(),
            local_endpoint: self.settings.rpc_url.as_deref().map(safe_endpoint),
            cookie_available: self.settings.cookie_available(),
        }
    }

    pub fn adopt_source(&mut self, choice: SourceChoice) -> Result<SourceStatus, NativeError> {
        let settings = self.settings.with_source(choice);
        let node = settings.connect()?;
        urma_io::write_replace(
            &self.directory.join(SOURCE_FILE),
            &serde_json::to_vec(&settings.source)?,
        )?;
        self.settings = settings;
        self.replace_link(node);
        Ok(self.source_status())
    }

    pub(super) fn described_source(&self) -> (&'static str, String) {
        let label = match &self.settings.source {
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
