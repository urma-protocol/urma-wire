use urma_core::format::PublicRecord;
use urma_wire::index::{Checkpoint, Entry, Index};
use wire_app_shared::{
    avatar::{encode_avatar_pixels, AvatarPalette},
    decode_avatar_pixels, live_index,
    publication_record::{build, review},
};

#[test]
fn profile_and_avatar_bytes_and_signed_review_are_exact() {
    let name = "  e\u{301}\0昵称\r\n";
    let record = build("profile", name.into(), None, String::new(), vec![], None)
        .unwrap()
        .encode()
        .unwrap();
    assert_eq!(&record[8..], name.as_bytes());
    assert_eq!(review(&record).unwrap().text, name);
    assert!(build(
        "profile",
        "x".repeat(32760),
        None,
        String::new(),
        vec![],
        None
    )
    .is_ok());
    assert!(build(
        "profile",
        "x".repeat(32761),
        None,
        String::new(),
        vec![],
        None
    )
    .is_err());
    assert_eq!(
        build("profile", String::new(), None, String::new(), vec![], None)
            .unwrap()
            .encode()
            .unwrap()
            .len(),
        8
    );
    assert!(build("profile", name.into(), None, "wire".into(), vec![], None).is_err());
    let pixels = std::array::from_fn(|index| (index % 16) as u8);
    let bytes = encode_avatar_pixels(&pixels);
    assert_eq!(
        &bytes[..8],
        &[0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]
    );
    assert_eq!(decode_avatar_pixels(&bytes), pixels);
    assert_eq!(
        AvatarPalette::COLORS,
        [
            "#000000", "#0000AA", "#00AA00", "#00AAAA", "#AA0000", "#AA00AA", "#AA5500", "#AAAAAA",
            "#555555", "#5555FF", "#55FF55", "#55FFFF", "#FF5555", "#FF55FF", "#FFFF55", "#FFFFFF"
        ]
    );
    let record = build(
        "avatar",
        String::new(),
        None,
        String::new(),
        vec![],
        Some(bytes.to_vec()),
    )
    .unwrap()
    .encode()
    .unwrap();
    assert_eq!(record.len(), 520);
    assert_eq!(&record[8..], &bytes);
    assert_eq!(review(&record).unwrap().avatar.unwrap(), bytes);
    for length in [0, 511, 513] {
        assert!(build(
            "avatar",
            String::new(),
            None,
            String::new(),
            vec![],
            Some(vec![0; length])
        )
        .is_err());
    }
}

#[test]
fn colliding_names_latest_order_and_rollback_rebuild_separate_identity_fields() {
    let a = "03".repeat(32);
    let b = "04".repeat(32);
    let entry = |author: &str, height, position, record: PublicRecord| Entry {
        height,
        position,
        author: author.into(),
        txid: format!("{height:064x}"),
        record: hex::encode(record.encode().unwrap()),
    };
    let mut index = Index {
        format: "URMA-WIRE-INDEX-2".into(),
        genesis: "00".repeat(32),
        start: 100,
        blocks: vec![
            Checkpoint {
                height: 100,
                hash: "01".repeat(32),
            },
            Checkpoint {
                height: 101,
                hash: "02".repeat(32),
            },
        ],
        entries: vec![
            entry(&a, 101, 2, PublicRecord::Profile("last".into())),
            entry(&a, 100, 1, PublicRecord::Profile("collision".into())),
            entry(&b, 100, 2, PublicRecord::Profile("collision".into())),
            entry(&a, 100, 3, PublicRecord::Avatar(Box::new([0x12; 512]))),
            entry(&a, 101, 1, PublicRecord::Profile("earlier position".into())),
            entry(&a, 101, 3, PublicRecord::Avatar(Box::new([0x34; 512]))),
        ],
    };
    let latest = live_index::project(&index, true).unwrap();
    assert!(latest.posts.is_empty());
    assert_eq!(latest.identities[&a].nickname.as_deref(), Some("last"));
    assert_eq!(
        latest.identities[&a].avatar.as_deref(),
        Some([0x34; 512].as_slice())
    );
    // The indexer's existing rollback removes orphan entries; projection must
    // discard every orphan-derived name and avatar, not preserve a UI cache.
    index.entries.retain(|entry| entry.height <= 100);
    index.blocks.pop();
    let restored = live_index::project(&index, true).unwrap();
    assert_eq!(
        restored.identities[&a].nickname,
        restored.identities[&b].nickname
    );
    assert_eq!(
        restored.identities[&a].avatar.as_deref(),
        Some([0x12; 512].as_slice())
    );
    index
        .entries
        .push(entry(&a, 100, 4, PublicRecord::Profile(String::new())));
    let empty = live_index::project(&index, true).unwrap();
    assert_eq!(empty.identities[&a].nickname.as_deref(), Some(""));
    assert!(empty.identities[&a].avatar.is_some());
    index.entries.clear();
    assert!(live_index::project(&index, true)
        .unwrap()
        .identities
        .is_empty());
}
