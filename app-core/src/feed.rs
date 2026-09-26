use crate::model::{Placement, Post, Txid};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicIdentity {
    pub nickname: Option<String>,
    pub avatar: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Feed {
    pub identities: BTreeMap<String, PublicIdentity>,
    pub posts: Vec<Post>,
    pub start: u64,
    pub tip: u64,
    pub complete_to_tip: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Lookup {
    Found(Post),
    Absent,
}

enum Labelled<'a> {
    Wire(&'a str),
    Hashtag(&'a str),
    Unmarked,
}

impl<'a> Labelled<'a> {
    fn read(label: &'a str) -> Self {
        let mut spelling = label.chars();
        let Some(mark) = spelling.next() else {
            return Labelled::Unmarked;
        };
        let rest = spelling.as_str();
        match mark {
            '/' => Labelled::Wire(rest),
            '#' => Labelled::Hashtag(rest),
            _other => Labelled::Unmarked,
        }
    }
}

pub fn matches_topic(post: &Post, label: &str) -> bool {
    match Labelled::read(label) {
        Labelled::Wire(wire) => !wire.is_empty() && post.wire == wire,
        Labelled::Hashtag(tag) => post.hashtags.iter().any(|value| value == tag),
        Labelled::Unmarked => false,
    }
}

pub fn following(posts: &[Post], labels: &[String]) -> Vec<Post> {
    posts
        .iter()
        .filter(|post| labels.iter().any(|label| matches_topic(post, label)))
        .cloned()
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversation {
    pub post: Post,
    pub replies: Vec<Post>,
    pub unseen: usize,
}

pub fn conversations(shown: Vec<Post>, everything: &[Post], preview: usize) -> Vec<Conversation> {
    let present: BTreeSet<Txid> = shown.iter().map(|post| post.txid).collect();
    let mut answers: Vec<(Txid, &Post)> = Vec::new();
    for post in everything {
        let Placement::Reply(parent) = post.placement else {
            continue;
        };
        answers.push((parent, post));
    }
    answers.sort_by_key(|(parent, _answer)| *parent);
    let mut threaded = Vec::new();
    for post in shown {
        if answers_one_of(&post, &present) {
            continue;
        }
        let first = answers.partition_point(|(parent, _answer)| *parent < post.txid);
        let past = answers.partition_point(|(parent, _answer)| *parent <= post.txid);
        let replies: Vec<Post> = answers[first..past]
            .iter()
            .take(preview)
            .map(|(_parent, answer)| (*answer).clone())
            .collect();
        let unseen = past - first - replies.len();
        threaded.push(Conversation {
            post,
            replies,
            unseen,
        });
    }
    threaded
}

fn answers_one_of(post: &Post, present: &BTreeSet<Txid>) -> bool {
    match post.placement {
        Placement::Reply(parent) => present.contains(&parent),
        Placement::Root => false,
    }
}
