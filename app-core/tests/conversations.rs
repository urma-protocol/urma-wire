use wire_app_core::feed::conversations;
use wire_app_core::feed::with_pending;
use wire_app_core::feed::Conversation;
use wire_app_core::model::AuthorId;
use wire_app_core::model::Chain;
use wire_app_core::model::ConfirmedStatus;
use wire_app_core::model::Inclusion;
use wire_app_core::model::Placement;
use wire_app_core::model::Post;
use wire_app_core::model::Txid;

fn id(seed: u8) -> Txid {
    Txid::from_raw_bytes([seed; 32])
}

fn said(seed: u8, placement: Placement) -> Post {
    Post {
        txid: id(seed),
        author: AuthorId::from_bytes([3; 32]),
        text: format!("record {seed}"),
        wire: String::new(),
        hashtags: Vec::new(),
        chain: Chain::Litecoin,
        placement,
        inclusion: Inclusion::Confirmed(ConfirmedStatus {
            block_height: u64::from(seed),
            block_hash: "00".repeat(32),
            confirmations: 1,
            tx_index: 0,
        }),
        reply_count: 0,
    }
}

fn root(seed: u8) -> Post {
    said(seed, Placement::Root)
}

#[test]
fn pending_records_join_the_feed_and_confirmed_copies_win() {
    let confirmed = vec![root(1), root(2)];
    let mut reply = said(3, Placement::Reply(id(1)));
    reply.inclusion = Inclusion::Mempool;
    let mut twin = root(2);
    twin.inclusion = Inclusion::Mempool;
    let merged = with_pending(&confirmed, &[reply.clone(), twin]);
    let order: Vec<Txid> = merged.iter().map(|post| post.txid).collect();
    assert_eq!(order, vec![id(3), id(1), id(2)]);
    assert_eq!(merged[2], confirmed[1]);
    let threaded = conversations(merged.clone(), &merged, 5);
    assert_eq!(threaded.len(), 2);
    assert_eq!(threaded[0].post.txid, id(1));
    assert_eq!(threaded[0].replies, vec![reply]);
    assert_eq!(with_pending(&confirmed, &[]), confirmed);
}

fn reply(seed: u8, parent: u8) -> Post {
    said(seed, Placement::Reply(id(parent)))
}

fn leading(threaded: &[Conversation]) -> Vec<Txid> {
    threaded
        .iter()
        .map(|conversation| conversation.post.txid)
        .collect()
}

fn txids(posts: &[Post]) -> Vec<Txid> {
    posts.iter().map(|post| post.txid).collect()
}

#[test]
fn replies_nest_under_their_parent_newest_first() {
    let feed = vec![reply(4, 1), reply(3, 1), root(2), root(1)];
    let threaded = conversations(feed.clone(), &feed, 5);
    assert_eq!(leading(&threaded), vec![id(2), id(1)]);
    assert!(threaded[0].replies.is_empty());
    assert_eq!(txids(&threaded[1].replies), vec![id(4), id(3)]);
    assert_eq!(threaded[1].unseen, 0);
}

#[test]
fn the_preview_stops_at_its_size_and_counts_the_rest() {
    let feed = vec![reply(5, 1), reply(4, 1), reply(3, 1), root(1)];
    let threaded = conversations(feed.clone(), &feed, 2);
    assert_eq!(leading(&threaded), vec![id(1)]);
    assert_eq!(txids(&threaded[0].replies), vec![id(5), id(4)]);
    assert_eq!(threaded[0].unseen, 1);
}

#[test]
fn an_empty_preview_keeps_every_reply_behind_the_thread() {
    let feed = vec![reply(3, 1), reply(2, 1), root(1)];
    let threaded = conversations(feed.clone(), &feed, 0);
    assert_eq!(leading(&threaded), vec![id(1)]);
    assert!(threaded[0].replies.is_empty());
    assert_eq!(threaded[0].unseen, 2);
}

#[test]
fn a_reply_to_a_record_outside_the_index_stands_alone() {
    let feed = vec![reply(3, 9), root(1)];
    let threaded = conversations(feed.clone(), &feed, 5);
    assert_eq!(leading(&threaded), vec![id(3), id(1)]);
}

#[test]
fn a_reply_whose_parent_is_filtered_out_stands_alone() {
    let feed = vec![reply(3, 1), root(2), root(1)];
    let shown = vec![reply(3, 1), root(2)];
    let threaded = conversations(shown, &feed, 5);
    assert_eq!(leading(&threaded), vec![id(3), id(2)]);
}

#[test]
fn a_shown_parent_carries_replies_the_filter_left_out() {
    let feed = vec![reply(2, 1), root(1)];
    let shown = vec![root(1)];
    let threaded = conversations(shown, &feed, 5);
    assert_eq!(txids(&threaded[0].replies), vec![id(2)]);
}

#[test]
fn a_reply_to_a_reply_stays_inside_that_reply_thread() {
    let feed = vec![reply(3, 2), reply(2, 1), root(1)];
    let threaded = conversations(feed.clone(), &feed, 5);
    assert_eq!(leading(&threaded), vec![id(1)]);
    assert_eq!(txids(&threaded[0].replies), vec![id(2)]);
}
