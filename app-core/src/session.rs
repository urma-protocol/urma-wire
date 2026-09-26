use crate::model::AuthorId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultSource {
    Imported,
    Configured,
    Missing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Session {
    Locked,
    Unlocked(AuthorId),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityStatus {
    pub session: Session,
    pub vault_available: bool,
    pub source: VaultSource,
    pub configured_unlock_available: bool,
}

impl IdentityStatus {
    pub fn unlocked(&self) -> bool {
        match self.session {
            Session::Locked => false,
            Session::Unlocked(_author) => true,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub enum Credential {
    Password(String),
    ConfiguredFile,
}
