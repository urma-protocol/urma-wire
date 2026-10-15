//! Pure projection fixtures: encoded records and synthetic placement metadata.
//! These tests do not establish signatures, block inclusion, or scan coverage.

use urma_core::{
    format::{PublicRecord, RecordKind},
    topics::Topics,
};
use urma_wire::index::{Checkpoint, Entry, Index};
use wire_app_core::{
    feed::Lookup,
    model::{Placement, Txid},
};
use wire_app_engine::{
    errors::ProjectionError,
    projection::{self, View},
};

fn id(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

fn raw(height: u64, position: u32, identifier: u8, bytes: &[u8]) -> Entry {
    Entry {
        height,
        position,
        txid: id(identifier),
        author: id(3),
        record: hex::encode(bytes),
        return_script: None,
    }
}

fn entry(height: u64, position: u32, identifier: u8, record: PublicRecord) -> Entry {
    raw(height, position, identifier, &record.encode().unwrap())
}

fn checkpoint(height: u64) -> Checkpoint {
    Checkpoint {
        height,
        hash: format!("{height:064x}"),
    }
}

fn mixed() -> Index {
    let legacy_profile = [RecordKind::Profile.prefix().as_slice(), &[b'l'; 129]].concat();
    let legacy_avatar = [RecordKind::Avatar.prefix().as_slice(), &[0; 512]].concat();
    Index {
        format: Index::FORMAT.into(),
        genesis: id(0),
        start: 100,
        blocks: (100..=105).map(checkpoint).collect(),
        entries: vec![
            // Identity updates deliberately arrive out of placement order.
            entry(105, 4, 5, PublicRecord::Profile("latest".into())),
            entry(
                100,
                0,
                1,
                PublicRecord::Post("plain #literal /literal\n".into()),
            ),
            entry(100, 1, 6, PublicRecord::Profile("earlier".into())),
            entry(100, 2, 7, PublicRecord::Avatar(Box::new([0x12; 128]))),
            entry(
                104,
                1,
                2,
                PublicRecord::WirePost {
                    topics: Topics {
                        wire: "rosint".into(),
                        hashtags: vec!["investigation".into()],
                    },
                    text: "structured".into(),
                },
            ),
            entry(
                105,
                0,
                3,
                PublicRecord::Reply {
                    target: id(1).parse().unwrap(),
                    text: "plain reply".into(),
                },
            ),
            entry(
                105,
                1,
                4,
                PublicRecord::WireReply {
                    target: id(2).parse().unwrap(),
                    topics: Topics {
                        wire: "other".into(),
                        hashtags: vec!["investigation".into()],
                    },
                    text: "structured reply".into(),
                },
            ),
            entry(105, 3, 8, PublicRecord::Avatar(Box::new([0x34; 128]))),
            raw(105, 5, 9, &legacy_profile),
            raw(105, 6, 10, &legacy_avatar),
        ],
    }
}

fn view(index: &Index, tip: u64) -> View<'_> {
    View {
        start: index.start,
        tip,
        blocks: &index.blocks,
        entries: &index.entries,
    }
}

#[test]
fn index_wrapper_and_view_preserve_mixed_records_identities_topics_and_lookups() {
    let index = mixed();
    let original = serde_json::to_value(&index).unwrap();
    for complete in [false, true] {
        let wrapped = projection::project(&index, complete).unwrap();
        let borrowed = projection::project_view(&view(&index, 105), complete).unwrap();
        assert_eq!(
            serde_json::to_value(&borrowed).unwrap(),
            serde_json::to_value(&wrapped).unwrap()
        );
        assert_eq!(borrowed.start, 100);
        assert_eq!(borrowed.tip, 105);
        assert_eq!(borrowed.complete_to_tip, complete);
        assert_eq!(
            borrowed
                .posts
                .iter()
                .map(|post| post.txid.to_display_hex())
                .collect::<Vec<_>>(),
            vec![id(4), id(3), id(2), id(1)]
        );
        let identity = &borrowed.identities[&id(3)];
        assert_eq!(identity.nickname.as_deref(), Some("latest"));
        assert_eq!(identity.avatar.as_deref(), Some([0x34; 128].as_slice()));
        let plain = &borrowed.posts[3];
        assert_eq!(plain.text, "plain #literal /literal\n");
        assert!(plain.wire.is_empty());
        assert!(plain.hashtags.is_empty());
        assert_eq!(plain.placement, Placement::Root);
        assert_eq!(plain.reply_count, 1);
        let structured = &borrowed.posts[2];
        assert_eq!(structured.wire, "rosint");
        assert_eq!(structured.hashtags, vec!["investigation".to_owned()]);
        assert_eq!(structured.reply_count, 1);
        assert_eq!(borrowed.posts[0].wire, "other");
        assert_eq!(
            borrowed.posts[0].placement,
            Placement::Reply(structured.txid)
        );
        assert_eq!(borrowed.posts[1].placement, Placement::Reply(plain.txid));
        assert_eq!(plain.status.confirmations, 6);
        assert_eq!(structured.status.confirmations, 2);
        assert_eq!(borrowed.posts[0].status.confirmations, 1);
    }
    let parent = Txid::from_display_hex(&id(1)).unwrap();
    let child = Txid::from_display_hex(&id(3)).unwrap();
    assert!(
        matches!(projection::lookup(&index, child).unwrap(), Lookup::Found(post) if post.text == "plain reply")
    );
    let replies = projection::replies(&index, parent).unwrap();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].txid, child);
    let unknown = Txid::from_display_hex(&id(255)).unwrap();
    assert!(matches!(
        projection::lookup(&index, unknown).unwrap(),
        Lookup::Absent
    ));
    assert!(projection::replies(&index, unknown).unwrap().is_empty());
    assert_eq!(serde_json::to_value(&index).unwrap(), original);
}

#[test]
fn sparse_placements_use_explicit_tip_without_adding_checkpoints_or_coverage() {
    let index = mixed();
    // No intermediate heights or tip checkpoint are manufactured by projection.
    let blocks = [checkpoint(100), checkpoint(104), checkpoint(105)];
    let original_blocks = serde_json::to_value(&blocks).unwrap();
    let sparse = View {
        start: 100,
        tip: 120,
        blocks: &blocks,
        entries: &index.entries,
    };
    let feed = projection::project_view(&sparse, false).unwrap();
    assert_eq!(feed.tip, 120);
    assert_eq!(feed.start, 100);
    assert!(!feed.complete_to_tip);
    assert_eq!(
        feed.posts
            .iter()
            .map(|post| post.status.confirmations)
            .collect::<Vec<_>>(),
        vec![16, 16, 17, 21]
    );
    for post in &feed.posts {
        let supplied = blocks
            .iter()
            .find(|block| block.height == post.status.block_height)
            .unwrap();
        assert_eq!(post.status.block_hash, supplied.hash);
    }
    let dense = projection::project(&index, false).unwrap();
    assert_eq!(feed.identities, dense.identities);
    for (actual, old) in feed.posts.iter().zip(&dense.posts) {
        let mut expected = old.clone();
        expected.status.confirmations += 15;
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
    }
    assert_eq!(serde_json::to_value(&blocks).unwrap(), original_blocks);
    assert_eq!(blocks.len(), 3);
}

#[test]
fn missing_checkpoint_and_above_tip_keep_the_existing_error_precedence() {
    let mut index = Index {
        format: Index::FORMAT.into(),
        genesis: id(0),
        start: 99,
        blocks: vec![],
        entries: vec![entry(100, 0, 1, PublicRecord::Post("post".into()))],
    };
    assert!(matches!(
        projection::project(&index, false),
        Err(ProjectionError::NoCheckpoint)
    ));
    assert!(matches!(
        projection::project_view(&view(&index, 99), false),
        Err(ProjectionError::NoCheckpoint)
    ));
    // Inconsistent metadata is passed directly to the pure legacy projection,
    // without Index::load or any claim that this is a validated chain/index.
    index.blocks = vec![checkpoint(100), checkpoint(99)];
    assert!(matches!(
        projection::project(&index, false),
        Err(ProjectionError::HeightAboveTip)
    ));
    assert!(matches!(
        projection::project_view(&view(&index, 99), false),
        Err(ProjectionError::HeightAboveTip)
    ));
    index.blocks = vec![checkpoint(99)];
    assert!(matches!(
        projection::project_view(&view(&index, 99), false),
        Err(ProjectionError::NoCheckpoint)
    ));
}

#[test]
fn empty_wrapper_keeps_start_as_tip_and_view_preserves_explicit_false_coverage() {
    let index = Index {
        format: Index::FORMAT.into(),
        genesis: id(0),
        start: 42,
        blocks: vec![],
        entries: vec![],
    };
    let wrapped = projection::project(&index, false).unwrap();
    let borrowed = projection::project_view(&view(&index, 42), false).unwrap();
    assert_eq!(
        serde_json::to_value(&wrapped).unwrap(),
        serde_json::to_value(&borrowed).unwrap()
    );
    assert_eq!(wrapped.tip, 42);
    let explicit = projection::project_view(&view(&index, 120), false).unwrap();
    assert_eq!(explicit.start, 42);
    assert_eq!(explicit.tip, 120);
    assert!(explicit.posts.is_empty());
    assert!(explicit.identities.is_empty());
    assert!(!explicit.complete_to_tip);
}

#[test]
fn both_paths_retain_strict_cached_decoding_failures() {
    let mut index = mixed();
    for bytes in [
        [RecordKind::Profile.prefix().as_slice(), &[0xff; 129]].concat(),
        [RecordKind::Avatar.prefix().as_slice(), &[0; 127]].concat(),
    ] {
        index.entries[0].record = hex::encode(bytes);
        assert!(matches!(
            projection::project(&index, false),
            Err(ProjectionError::Record(_))
        ));
        assert!(matches!(
            projection::project_view(&view(&index, 105), false),
            Err(ProjectionError::Record(_))
        ));
    }
    index.entries[0].record = "z".into();
    assert!(matches!(
        projection::project(&index, false),
        Err(ProjectionError::Hex(_))
    ));
    assert!(matches!(
        projection::project_view(&view(&index, 105), false),
        Err(ProjectionError::Hex(_))
    ));
}
