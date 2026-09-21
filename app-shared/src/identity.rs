use serde::{Deserialize, Serialize};

/// URMA 0.2 encrypted vault limit. No secret material belongs in session status.
pub const MAX_IDENTITY_VAULT_BYTES: usize = 4532;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityStatus {
    pub unlocked: bool,
    pub vault_available: bool,
    pub source: String,
    pub author_pubkey: Option<String>,
    pub author_fingerprint: Option<String>,
    pub configured_unlock_available: bool,
}
