use wire_app_shared::live_model::matches_topic;
use wire_app_shared::model::AuthorId;
use wire_app_shared::model::ConfirmedStatus;
use wire_app_shared::model::Post;
use wire_app_shared::model::Txid;
use wire_app_shared::Chain;

fn post(wire: &str, hashtags: &[&str]) -> Post {
    Post {
        txid: Txid::from_raw_bytes([7; 32]),
        author: AuthorId::from_bytes([3; 32]),
        text: "text".into(),
        wire: wire.to_owned(),
        hashtags: hashtags.iter().map(|tag| (*tag).to_owned()).collect(),
        chain: Chain::Litecoin,
        status: ConfirmedStatus {
            block_height: 1,
            block_hash: "00".repeat(32),
            confirmations: 1,
            tx_index: 0,
        },
        reply_count: 0,
    }
}

fn shown(posts: Vec<Post>, followed: &[&str]) -> Vec<Post> {
    let mut kept = Vec::new();
    for held in posts {
        if followed.iter().any(|label| matches_topic(&held, label)) {
            kept.push(held);
        }
    }
    kept
}

#[test]
fn following_a_wire_keeps_only_that_wire() {
    let posts = vec![
        post("rosint", &["documents"]),
        post("other", &["documents"]),
        post("", &["leaks"]),
    ];
    let kept = shown(posts, &["/rosint"]);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].wire, "rosint");
}

#[test]
fn following_a_hashtag_crosses_wires() {
    let posts = vec![
        post("rosint", &["documents"]),
        post("other", &["documents"]),
        post("", &["leaks"]),
    ];
    assert_eq!(shown(posts, &["#documents"]).len(), 2);
}

#[test]
fn following_several_topics_keeps_any_match() {
    let posts = vec![
        post("rosint", &[]),
        post("", &["leaks"]),
        post("unrelated", &["noise"]),
    ];
    assert_eq!(shown(posts, &["/rosint", "#leaks"]).len(), 2);
}

#[test]
fn a_bare_label_without_its_mark_matches_nothing() {
    let posts = vec![post("rosint", &["documents"])];
    assert!(shown(posts, &["rosint"]).is_empty());
}
