use crate::limits::topics_when_unset;
use crate::limits::MAX_REMEMBERED_LABELS;
use crate::limits::MAX_SUGGESTIONS;
use crate::limits::TOPICS_STORAGE_KEY;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeSet;
use tracing::warn;
use wire_app_shared::topic_labels::Kind;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Suggestion {
    pub label: String,
    pub kind: Kind,
    pub followed: bool,
}

impl Suggestion {
    pub fn spelled(&self) -> String {
        format!("{}{}", self.kind.mark(), self.label)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Remembered {
    #[serde(default)]
    followed: Vec<String>,
    #[serde(default)]
    recent_wires: Vec<String>,
    #[serde(default)]
    recent_hashtags: Vec<String>,
}

impl Remembered {
    pub fn followed_labels(&self) -> Vec<String> {
        self.followed.clone()
    }

    pub fn follows(&self, spelled: &str) -> bool {
        self.followed.iter().any(|held| held == spelled)
    }

    pub fn toggle(&mut self, spelled: String) {
        let mut kept: Vec<String> = Vec::new();
        let mut removed = false;
        for held in self.followed.drain(..) {
            if held == spelled {
                removed = true;
                continue;
            }
            kept.push(held);
        }
        if !removed {
            kept.push(spelled);
        }
        kept.sort();
        self.followed = kept;
    }

    pub fn used(&mut self, wire: &str, hashtags: &[String]) {
        if !wire.is_empty() {
            promote(&mut self.recent_wires, wire);
        }
        for tag in hashtags {
            promote(&mut self.recent_hashtags, tag);
        }
    }

    pub fn suggestions(&self, kind: Kind, typed: &str, seen: &BTreeSet<String>) -> Vec<Suggestion> {
        let typed = typed
            .trim()
            .trim_start_matches(['/', '#'])
            .to_ascii_lowercase();
        let mut ranked: Vec<Suggestion> = Vec::new();
        for label in self.ordered(kind, seen) {
            if !label.starts_with(&typed) || label == typed {
                continue;
            }
            if ranked.iter().any(|held| held.label == label) {
                continue;
            }
            let followed = self.follows(&format!("{}{label}", kind.mark()));
            ranked.push(Suggestion {
                label,
                kind,
                followed,
            });
            if ranked.len() >= MAX_SUGGESTIONS {
                break;
            }
        }
        ranked
    }

    fn ordered(&self, kind: Kind, seen: &BTreeSet<String>) -> Vec<String> {
        let mark = kind.mark();
        let mut ordered: Vec<String> = Vec::new();
        for spelled in &self.followed {
            let Some(label) = spelled.strip_prefix(mark) else {
                continue;
            };
            ordered.push(label.to_owned());
        }
        let recent = match kind {
            Kind::Wire => &self.recent_wires,
            Kind::Hashtag => &self.recent_hashtags,
        };
        for label in recent {
            ordered.push(label.clone());
        }
        for spelled in seen {
            let Some(label) = spelled.strip_prefix(mark) else {
                continue;
            };
            ordered.push(label.to_owned());
        }
        ordered
    }
}

fn promote(held: &mut Vec<String>, label: &str) {
    let mut kept: Vec<String> = vec![label.to_owned()];
    for previous in held.drain(..) {
        if previous == label {
            continue;
        }
        kept.push(previous);
    }
    kept.truncate(MAX_REMEMBERED_LABELS);
    *held = kept;
}

pub enum Recalled {
    Topics(Remembered),
    Empty,
    Unreadable(String),
}

impl Recalled {
    pub fn remembered(self) -> Remembered {
        match self {
            Recalled::Topics(remembered) => remembered,
            Recalled::Empty => topics_when_unset(),
            Recalled::Unreadable(_) => topics_when_unset(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load() -> Recalled {
    let Kept::Storage(storage) = topic_storage() else {
        return Recalled::Empty;
    };
    let held = match storage.get_item(TOPICS_STORAGE_KEY) {
        Ok(held) => held,
        Err(error) => {
            warn!(error = ?error, "topic memory could not be read");
            return Recalled::Unreadable("Topic memory could not be read".to_owned());
        }
    };
    let Some(text) = held else {
        return Recalled::Empty;
    };
    match serde_json::from_str(&text) {
        Ok(remembered) => Recalled::Topics(remembered),
        Err(error) => {
            warn!(error = %error, "topic memory is unreadable");
            Recalled::Unreadable("Saved topics were unreadable and start empty".to_owned())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load() -> Recalled {
    Recalled::Empty
}

#[cfg(target_arch = "wasm32")]
pub fn save(remembered: &Remembered) {
    let Kept::Storage(storage) = topic_storage() else {
        return;
    };
    let text = match serde_json::to_string(remembered) {
        Ok(text) => text,
        Err(error) => {
            warn!(error = %error, "topic memory could not be encoded");
            return;
        }
    };
    match storage.set_item(TOPICS_STORAGE_KEY, &text) {
        Ok(()) => (),
        Err(error) => warn!(error = ?error, "topic memory could not be saved"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save(_remembered: &Remembered) {}

#[cfg(target_arch = "wasm32")]
enum Kept {
    Storage(web_sys::Storage),
    Unavailable,
}

#[cfg(target_arch = "wasm32")]
fn topic_storage() -> Kept {
    let Some(window) = web_sys::window() else {
        return Kept::Unavailable;
    };
    let stored = match window.local_storage() {
        Ok(stored) => stored,
        Err(error) => {
            warn!(error = ?error, "topic memory storage unavailable");
            return Kept::Unavailable;
        }
    };
    let Some(storage) = stored else {
        return Kept::Unavailable;
    };
    Kept::Storage(storage)
}
