use wire_app_core::model::AuthorId;
use wire_app_core::model::Chain;
use wire_app_core::model::ConfirmedStatus;
use wire_app_core::model::Placement;
use wire_app_core::model::Post;
use wire_app_core::model::Txid;
use wire_app_core::topics::read_hashtags;
use wire_app_core::topics::read_label;
use wire_app_core::topics::seen_in;
use wire_app_core::topics::Label;

fn spoken(typed: &str) -> String {
    match read_label(typed) {
        Label::Valid(label) => label,
        Label::Rejected(reason) => panic!("{typed} was rejected: {reason}"),
    }
}

fn rejected(typed: &str) -> String {
    match read_label(typed) {
        Label::Valid(label) => panic!("{typed} was accepted as {label}"),
        Label::Rejected(reason) => reason,
    }
}

#[test]
fn typed_labels_normalize_to_protocol_labels() {
    assert_eq!(spoken("/ROSINT"), "rosint");
    assert_eq!(spoken("#Investigation"), "investigation");
    assert_eq!(spoken("  leaks_2026  "), "leaks_2026");
    assert_eq!(spoken("open-source"), "open-source");
}

#[test]
fn labels_the_protocol_refuses_are_refused_here() {
    for typed in ["", "/", "#", "-leading", "spaced out", "ünicode", "UPPER!"] {
        let reason = rejected(typed);
        assert!(!reason.is_empty(), "{typed} produced an empty reason");
    }
    let long = "a".repeat(33);
    assert!(!rejected(&long).is_empty());
    let at_limit = "a".repeat(32);
    assert_eq!(spoken(&at_limit), at_limit);
}

#[test]
fn hashtags_are_deduplicated_sorted_and_capped() {
    let read = read_hashtags("#zulu #alpha #ALPHA #mike");
    assert_eq!(read, vec!["alpha", "mike", "zulu"]);

    let many = read_hashtags("a1 b2 c3 d4 e5 f6 g7 h8 i9 j10");
    assert_eq!(many.len(), 8);
    assert!(many.windows(2).all(|pair| pair[0] < pair[1]));

    assert!(read_hashtags("!! ?? ,,").is_empty());
    assert_eq!(read_hashtags("#valid !!bad"), vec!["valid"]);
}

fn post(wire: &str, hashtags: &[&str]) -> Post {
    Post {
        txid: Txid::from_raw_bytes([7; 32]),
        author: AuthorId::from_bytes([3; 32]),
        text: "text".into(),
        wire: wire.to_owned(),
        hashtags: hashtags.iter().map(|tag| (*tag).to_owned()).collect(),
        chain: Chain::Litecoin,
        placement: Placement::Root,
        status: ConfirmedStatus {
            block_height: 1,
            block_hash: "00".repeat(32),
            confirmations: 1,
            tx_index: 0,
        },
        reply_count: 0,
    }
}

#[test]
fn every_topic_in_loaded_history_is_offered() {
    let seen = seen_in(&[
        post("rosint", &["investigation", "documents"]),
        post("rosint", &["investigation"]),
        post("", &["leaks"]),
    ]);
    let held: Vec<&str> = seen.iter().map(String::as_str).collect();
    assert_eq!(
        held,
        vec!["#documents", "#investigation", "#leaks", "/rosint"]
    );
}
