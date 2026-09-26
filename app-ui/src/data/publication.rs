use crate::data::engine::ask;
use tracing::warn;
use wire_app_core::composed::scan;
use wire_app_core::contract::{Request, Response};
use wire_app_core::errors::{ComposeError, Fault};
use wire_app_core::model::Chain;
use wire_app_core::publication::{Addressed, Content, Pending, Publication};
use wire_app_core::topics::{read_hashtags, read_label, Label};

#[derive(Clone, PartialEq, Eq)]
pub enum Overridden {
    Field(String),
    Nothing,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Meant {
    pub wire: String,
    pub hashtags: Vec<String>,
    pub overridden_wire: Overridden,
    pub extra_wires: Vec<String>,
    pub dropped_hashtags: Vec<String>,
}

enum Written {
    Wire(String),
    Nothing,
}

fn typed_wire(wire: &str) -> Written {
    let typed = wire.trim();
    if typed.is_empty() {
        return Written::Nothing;
    }
    match read_label(typed) {
        Label::Valid(label) => Written::Wire(label),
        Label::Rejected(reason) => {
            warn!(reason = %reason, "typed wire is not a protocol label");
            Written::Nothing
        }
    }
}

impl Meant {
    pub fn read(text: &str, wire: &str, hashtags: &str) -> Result<Self, ComposeError> {
        let composed = scan(text)?;
        let written = composed.wire();
        let (wire, overridden_wire) = match (written.is_empty(), typed_wire(wire)) {
            (true, Written::Wire(typed)) => (typed, Overridden::Nothing),
            (true, Written::Nothing) => (String::new(), Overridden::Nothing),
            (false, Written::Wire(typed)) if typed != written => {
                (written, Overridden::Field(typed))
            }
            (false, _kept) => (written, Overridden::Nothing),
        };
        let mut labels = composed.hashtag_labels();
        for label in read_hashtags(hashtags) {
            if labels.contains(&label) {
                continue;
            }
            labels.push(label);
        }
        labels.sort();
        let dropped = labels.split_off(Label::MAX_HASHTAGS.min(labels.len()));
        let mut dropped_hashtags = composed.dropped_hashtags();
        for label in dropped {
            if dropped_hashtags.contains(&label) {
                continue;
            }
            dropped_hashtags.push(label);
        }
        Ok(Self {
            wire,
            hashtags: labels,
            overridden_wire,
            extra_wires: composed.extra_wires(),
            dropped_hashtags,
        })
    }

    pub fn conflicted(&self) -> bool {
        self.overridden_wire != Overridden::Nothing
            || !self.extra_wires.is_empty()
            || !self.dropped_hashtags.is_empty()
    }

    pub fn spoken(self, text: String, addressed: Addressed) -> Content {
        Content::Post {
            text,
            wire: self.wire,
            hashtags: self.hashtags,
            addressed,
        }
    }
}

pub async fn quote(chain: Chain, content: Content) -> Result<Publication, Fault> {
    match ask(Request::Quote { chain, content }).await? {
        Response::Publication(publication) => Ok(publication),
        other => Err(other.unexpected("publication quote")),
    }
}

pub async fn pending() -> Result<Pending, Fault> {
    match ask(Request::PublicationStatus).await? {
        Response::Pending(pending) => Ok(pending),
        other => Err(other.unexpected("publication status")),
    }
}

pub async fn advance(id: String, approve: bool) -> Result<Publication, Fault> {
    match ask(Request::Advance { id, approve }).await? {
        Response::Publication(publication) => Ok(publication),
        other => Err(other.unexpected("publication advance")),
    }
}
