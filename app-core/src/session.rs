use crate::model::AuthorId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vault {
    Missing,
    Local,
    Configured { path: String, unlock_file: Option<String> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Session {
    Locked,
    Unlocked(AuthorId),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityStatus {
    pub session: Session,
    pub vault: Vault,
}

impl IdentityStatus {
    pub fn unlocked(&self) -> bool {
        match self.session {
            Session::Locked => false,
            Session::Unlocked(_author) => true,
        }
    }

    pub fn has_local_copy(&self) -> bool {
        self.vault == Vault::Local
    }
}

#[derive(Serialize, Deserialize)]
pub enum Credential {
    Password(String),
    ConfiguredFile,
}
