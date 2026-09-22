use crate::model::Post;
use std::collections::BTreeSet;
use tracing::warn;
use urma_core::topics::Topics;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Kind {
    Wire,
    Hashtag,
}

impl Kind {
    pub fn mark(self) -> char {
        match self {
            Kind::Wire => '/',
            Kind::Hashtag => '#',
        }
    }
}

pub enum Label {
    Valid(String),
    Rejected(String),
}

impl Label {
    pub const MAX_HASHTAGS: usize = Topics::MAX_HASHTAGS;
}

pub fn read_label(typed: &str) -> Label {
    let bare = typed
        .trim()
        .trim_start_matches(['/', '#'])
        .to_ascii_lowercase();
    if bare.is_empty() {
        return Label::Rejected("A label cannot be empty".to_owned());
    }
    match Topics::validate_label(&bare) {
        Ok(()) => Label::Valid(bare),
        Err(rejection) => {
            warn!(rejection = %rejection, "typed label is not a protocol label");
            Label::Rejected(rejection.to_string())
        }
    }
}

pub fn read_hashtags(typed: &str) -> Vec<String> {
    let mut labels: Vec<String> = Vec::new();
    for word in typed.split_whitespace() {
        let Label::Valid(label) = read_label(word) else {
            continue;
        };
        if labels.contains(&label) {
            continue;
        }
        labels.push(label);
    }
    labels.sort();
    labels.truncate(Label::MAX_HASHTAGS);
    labels
}

pub fn seen_in(posts: &[Post]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    for post in posts {
        if !post.wire.is_empty() {
            seen.insert(format!("/{}", post.wire));
        }
        for tag in &post.hashtags {
            seen.insert(format!("#{tag}"));
        }
    }
    seen
}
