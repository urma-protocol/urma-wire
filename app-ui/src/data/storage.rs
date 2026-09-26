#[cfg(target_arch = "wasm32")]
use crate::config::IMAGE_PREFERENCE_KEY;
#[cfg(target_arch = "wasm32")]
use crate::config::REPLIES_SHOWN_KEY;
#[cfg(target_arch = "wasm32")]
use crate::config::TOPICS_STORAGE_KEY;
#[cfg(target_arch = "wasm32")]
use tracing::warn;
use wire_app_core::errors::Fault;
#[cfg(target_arch = "wasm32")]
use wire_app_core::errors::FaultKind;

#[derive(Clone, Copy)]
pub enum Slot {
    Images,
    Replies,
    Topics,
}

#[cfg(target_arch = "wasm32")]
impl Slot {
    fn key(self) -> &'static str {
        match self {
            Slot::Images => IMAGE_PREFERENCE_KEY,
            Slot::Replies => REPLIES_SHOWN_KEY,
            Slot::Topics => TOPICS_STORAGE_KEY,
        }
    }

    fn subject(self) -> &'static str {
        match self {
            Slot::Images => "image preference",
            Slot::Replies => "reply preview size",
            Slot::Topics => "followed topics",
        }
    }
}

pub enum Held {
    Value(String),
    Unset,
    Unreadable(String),
}

pub enum Complaint {
    Unreadable(String),
    Nothing,
}

pub struct Recalled<T> {
    pub value: T,
    pub complaint: Complaint,
}

impl<T> Recalled<T> {
    pub fn kept(value: T) -> Self {
        Self {
            value,
            complaint: Complaint::Nothing,
        }
    }

    pub fn doubted(value: T, reason: String) -> Self {
        Self {
            value,
            complaint: Complaint::Unreadable(reason),
        }
    }
}

#[cfg(target_arch = "wasm32")]
enum Kept {
    Storage(web_sys::Storage),
    Unavailable(String),
}

#[cfg(target_arch = "wasm32")]
fn local(slot: Slot) -> Kept {
    let unavailable = format!("Storage for the {} is unavailable", slot.subject());
    let Some(window) = web_sys::window() else {
        return Kept::Unavailable(unavailable);
    };
    let stored = match window.local_storage() {
        Ok(stored) => stored,
        Err(error) => {
            warn!(error = ?error, key = slot.key(), "local storage unavailable");
            return Kept::Unavailable(unavailable);
        }
    };
    let Some(storage) = stored else {
        return Kept::Unavailable(unavailable);
    };
    Kept::Storage(storage)
}

#[cfg(target_arch = "wasm32")]
pub fn read(slot: Slot) -> Held {
    let storage = match local(slot) {
        Kept::Storage(storage) => storage,
        Kept::Unavailable(reason) => return Held::Unreadable(reason),
    };
    match storage.get_item(slot.key()) {
        Ok(Some(value)) => Held::Value(value),
        Ok(None) => Held::Unset,
        Err(error) => {
            warn!(error = ?error, key = slot.key(), "saved value could not be read");
            Held::Unreadable(format!("Cannot read the saved {}", slot.subject()))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read(_slot: Slot) -> Held {
    Held::Unset
}

#[cfg(target_arch = "wasm32")]
pub fn write(slot: Slot, value: &str) -> Result<(), Fault> {
    let storage = match local(slot) {
        Kept::Storage(storage) => storage,
        Kept::Unavailable(reason) => return Err(Fault::new(FaultKind::Storage, reason)),
    };
    match storage.set_item(slot.key(), value) {
        Ok(()) => Ok(()),
        Err(error) => {
            warn!(error = ?error, key = slot.key(), "value could not be saved");
            Err(Fault::new(
                FaultKind::Storage,
                format!("Cannot save the {}", slot.subject()),
            ))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write(_slot: Slot, _value: &str) -> Result<(), Fault> {
    Ok(())
}
