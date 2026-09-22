use wire_app_shared::publication_record::build;
use wire_app_shared::publication_record::review;

const TARGET: &str = "1f8e0c3a5b7d9e2f4a6c8e0b1d3f5a7c9e2b4d6f8a0c2e4b6d8f0a2c4e6b8d0f";

fn encoded(target: Option<String>, wire: &str, hashtags: Vec<String>) -> Vec<u8> {
    let record = build(
        "post",
        "the text".to_owned(),
        target,
        wire.to_owned(),
        hashtags,
        None,
    )
    .expect("record builds");
    record.encode().expect("record encodes")
}

#[test]
fn a_plain_reply_carries_the_target_it_was_given() {
    let bytes = encoded(Some(TARGET.to_owned()), "", Vec::new());
    let read = review(&bytes).expect("signed bytes decode");
    assert_eq!(read.kind, "post");
    assert_eq!(read.text, "the text");
    assert_eq!(read.target.as_deref(), Some(TARGET));
    assert!(read.wire.is_empty());
    assert!(read.hashtags.is_empty());
}

#[test]
fn a_reply_with_topics_keeps_both_target_and_topics() {
    let bytes = encoded(
        Some(TARGET.to_owned()),
        "rosint",
        vec!["documents".to_owned()],
    );
    let read = review(&bytes).expect("signed bytes decode");
    assert_eq!(read.target.as_deref(), Some(TARGET));
    assert_eq!(read.wire, "rosint");
    assert_eq!(read.hashtags, vec!["documents"]);
}

#[test]
fn a_post_without_a_target_stays_targetless() {
    let plain = review(&encoded(None, "", Vec::new())).expect("decodes");
    assert_eq!(plain.target, None);
    let with_topics = review(&encoded(None, "rosint", Vec::new())).expect("decodes");
    assert_eq!(with_topics.target, None);
    assert_eq!(with_topics.wire, "rosint");
}

#[test]
fn a_target_that_is_not_a_txid_is_refused() {
    let refused = build(
        "post",
        "the text".to_owned(),
        Some("not-a-txid".to_owned()),
        String::new(),
        Vec::new(),
        None,
    );
    assert!(refused.is_err());
}

#[test]
fn profile_records_cannot_carry_a_reply_target() {
    let refused = build(
        "profile",
        "nickname".to_owned(),
        Some(TARGET.to_owned()),
        String::new(),
        Vec::new(),
        None,
    );
    assert!(refused.is_err());
}
