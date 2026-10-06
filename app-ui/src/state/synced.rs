use crate::state::said::Said;
use wire_app_core::feed::{SyncPhase, SyncStatus};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Synced {
    Unchecked,
    Connecting,
    Verifying {
        done: u64,
        total: u64,
    },
    Retrying {
        done: u64,
        total: u64,
        attempt: u32,
        seconds: u64,
        reason: String,
    },
    Building,
    Indexed {
        complete: bool,
    },
    Failed(String),
}

impl Synced {
    pub fn shown(status: &SyncStatus) -> Self {
        match &status.phase {
            SyncPhase::Idle => Synced::Unchecked,
            SyncPhase::Connecting => Synced::Connecting,
            SyncPhase::Verifying => Synced::Verifying {
                done: status.done,
                total: status.total,
            },
            SyncPhase::Retrying {
                attempt,
                seconds,
                reason,
            } => Synced::Retrying {
                done: status.done,
                total: status.total,
                attempt: *attempt,
                seconds: *seconds,
                reason: reason.clone(),
            },
            SyncPhase::Complete => Synced::Indexed {
                complete: status.complete_to_tip,
            },
            SyncPhase::Failed(reason) => Synced::Failed(reason.clone()),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Synced::Unchecked => String::new(),
            Synced::Connecting => "Connecting…".to_owned(),
            Synced::Verifying { .. } => String::new(),
            Synced::Retrying { seconds, .. } => format!("Retrying in {seconds} s"),
            Synced::Building => "Building feed…".to_owned(),
            Synced::Indexed { complete: true } => "Up to date".to_owned(),
            Synced::Indexed { complete: false } => "History incomplete".to_owned(),
            Synced::Failed(_reason) => "Sync failed".to_owned(),
        }
    }

    pub fn trouble(&self) -> Said {
        match self {
            Synced::Failed(reason) => Said::Something(format!(
                "Sync failed; displayed records are cached: {reason}"
            )),
            Synced::Retrying {
                attempt, reason, ..
            } => Said::Something(format!(
                "Chain sources are struggling; retry {attempt} is scheduled. Last error: {reason}"
            )),
            Synced::Unchecked => Said::Nothing,
            Synced::Connecting => Said::Nothing,
            Synced::Verifying { .. } => Said::Nothing,
            Synced::Building => Said::Nothing,
            Synced::Indexed { .. } => Said::Nothing,
        }
    }
}
