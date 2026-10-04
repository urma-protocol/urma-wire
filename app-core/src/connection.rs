use crate::config::HIDDEN_ENDPOINT;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Trouble {
    Clear,
    IndexUnreadable(String),
    SourceUnverified(String),
}

impl Trouble {
    pub fn message(&self) -> &str {
        match self {
            Trouble::Clear => "",
            Trouble::IndexUnreadable(message) => message,
            Trouble::SourceUnverified(message) => message,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceChoice {
    Public,
    LocalNode,
    Esplora(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceStatus {
    pub choice: SourceChoice,
    pub endpoints: Vec<String>,
    pub local_node_configured: bool,
    pub local_endpoint: Option<String>,
    pub cookie_available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionHealth {
    pub network: String,
    pub source_type: String,
    pub endpoint: String,
    pub checked_at_ms: u64,
    pub remote_height: Option<u64>,
    pub local_start: Option<u64>,
    pub local_height: Option<u64>,
    pub trouble: Trouble,
    pub evidence: String,
    pub light_client: String,
}

impl ConnectionHealth {
    pub fn status(&self) -> &'static str {
        match self.trouble {
            Trouble::IndexUnreadable(_) => return "Local index unavailable",
            Trouble::SourceUnverified(_) => return "Source unavailable",
            Trouble::Clear => (),
        }
        let Some(remote) = self.remote_height else {
            return "No locally indexed blocks";
        };
        let Some(local) = self.local_height else {
            return "No locally indexed blocks";
        };
        match local.cmp(&remote) {
            std::cmp::Ordering::Equal => "Up to date at last check",
            std::cmp::Ordering::Greater => "Local index ahead of checked source",
            std::cmp::Ordering::Less => "Catch-up needed",
        }
    }
}

pub fn safe_endpoint(raw: &str) -> String {
    let Some((scheme, rest)) = raw.split_once("://") else {
        return HIDDEN_ENDPOINT.into();
    };
    if !matches!(scheme, "http" | "https" | "ssl" | "tcp") {
        return HIDDEN_ENDPOINT.into();
    }
    let Some(authority) = rest.split(['/', '?', '#']).next() else {
        return HIDDEN_ENDPOINT.into();
    };
    let valid = !authority.is_empty()
        && !authority.contains('@')
        && authority
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'))
        && numeric_port(authority);
    match valid {
        true => format!("{scheme}://{authority}"),
        false => HIDDEN_ENDPOINT.into(),
    }
}

fn numeric_port(authority: &str) -> bool {
    let Some((_host, port)) = authority.rsplit_once(':') else {
        return true;
    };
    !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())
}
