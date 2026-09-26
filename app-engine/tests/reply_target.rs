use wire_app_core::model::Txid;
use wire_app_core::publication::{Addressed, Content};
use wire_app_engine::record::{build, review};

const TARGET: &str = "1f8e0c3a5b7d9e2f4a6c8e0b1d3f5a7c9e2b4d6f8a0c2e4b6d8f0a2c4e6b8d0f";

fn target() -> Txid {
    Txid::from_display_hex(TARGET).expect("fixture target is a txid")
}

fn roundtrip(content: &Content) -> Content {
    let bytes = build(content)
        .expect("record builds")
        .encode()
        .expect("record encodes");
    review(&bytes).expect("signed bytes decode")
}

fn post(wire: &str, hashtags: &[&str], addressed: Addressed) -> Content {
    Content::Post {
        text: "the text".into(),
        wire: wire.into(),
        hashtags: hashtags.iter().map(|tag| (*tag).to_owned()).collect(),
        addressed,
    }
}

#[test]
fn a_plain_reply_carries_the_target_it_was_given() {
    let reply = post("", &[], Addressed::Thread(target()));
    assert_eq!(roundtrip(&reply), reply);
    let Content::Post { addressed, .. } = roundtrip(&reply) else {
        panic!("a reply reviews as a post");
    };
    assert_eq!(addressed, Addressed::Thread(target()));
    assert_eq!(target().to_display_hex(), TARGET);
}

#[test]
fn a_reply_with_topics_keeps_both_target_and_topics() {
    let reply = post("rosint", &["documents"], Addressed::Thread(target()));
    assert_eq!(roundtrip(&reply), reply);
}

#[test]
fn a_post_without_a_target_stays_targetless() {
    let plain = post("", &[], Addressed::Standalone);
    assert_eq!(roundtrip(&plain), plain);
    let with_topics = post("rosint", &[], Addressed::Standalone);
    assert_eq!(roundtrip(&with_topics), with_topics);
}

#[test]
fn a_target_that_is_not_a_txid_never_becomes_one() {
    assert!(Txid::from_display_hex("not-a-txid").is_err());
    assert!(Txid::from_display_hex(&TARGET[..62]).is_err());
}

#[test]
fn nickname_records_review_without_a_target() {
    let nickname = Content::Nickname("nickname".into());
    assert_eq!(roundtrip(&nickname), nickname);
}

#[test]
fn empty_posts_are_refused_before_signing() {
    let blank = Content::Post {
        text: "  \n".into(),
        wire: String::new(),
        hashtags: vec![],
        addressed: Addressed::Standalone,
    };
    assert!(build(&blank).is_err());
}
