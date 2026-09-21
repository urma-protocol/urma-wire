use serde::{Deserialize, Serialize};

/// A single read-only check. A failed check may still report an older local index.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionHealth {
    pub network: String,
    pub source_type: String,
    pub endpoint: String,
    pub checked_at_ms: u64,
    pub remote_height: Option<u64>,
    pub local_start: Option<u64>,
    pub local_height: Option<u64>,
    pub error: Option<String>,
}

impl ConnectionHealth {
    pub fn status(&self) -> &'static str {
        if self.error.as_deref() == Some("Local index unavailable") {
            "Local index unavailable"
        } else if self.error.is_some() {
            "Source unavailable"
        } else if let (Some(remote), Some(local)) = (self.remote_height, self.local_height) {
            if local == remote {
                "Up to date at last check"
            } else if local > remote {
                "Local index ahead of checked source"
            } else {
                "Catch-up needed"
            }
        } else {
            "No locally indexed blocks"
        }
    }
}

/// Only an origin with an ordinary host and optional numeric port is displayed.
/// Paths, user info, query parameters and fragments never enter the health response.
pub fn safe_endpoint(raw: &str) -> String {
    let Some((scheme, rest)) = raw.split_once("://") else {
        return "Endpoint hidden".into();
    };
    if !matches!(scheme, "http" | "https") {
        return "Endpoint hidden".into();
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let valid = !authority.is_empty()
        && !authority.contains('@')
        && authority
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'))
        && authority
            .rsplit_once(':')
            .is_none_or(|(_, port)| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()));
    if valid {
        format!("{scheme}://{authority}")
    } else {
        "Endpoint hidden".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_classifies_current_catchup_and_failure() {
        let mut health = ConnectionHealth {
            network: "Litecoin Testnet".into(),
            source_type: "Public source".into(),
            endpoint: "https://example.org".into(),
            checked_at_ms: 1,
            remote_height: Some(100),
            local_start: Some(90),
            local_height: Some(100),
            error: None,
        };
        assert_eq!(health.status(), "Up to date at last check");
        health.local_height = Some(99);
        assert_eq!(health.status(), "Catch-up needed");
        health.local_height = Some(101);
        assert_eq!(health.status(), "Local index ahead of checked source");
        health.error = Some("Source unavailable".into());
        assert_eq!(health.status(), "Source unavailable");
    }

    #[test]
    fn endpoint_never_displays_credentials_or_path() {
        assert_eq!(
            safe_endpoint("https://example.org/secret?key=x"),
            "https://example.org"
        );
        assert_eq!(
            safe_endpoint("https://user:pass@example.org/path"),
            "Endpoint hidden"
        );
    }
}
