use bitcoin::secp256k1::{Keypair, Secp256k1, SecretKey};
use urma_core::format::{PublicRecord, RecordKind};
use urma_wire::index::{Checkpoint, Entry, Index};
use wire_app_core::feed::Lookup;
use wire_app_core::model::Txid;
use wire_app_core::publication::Content;
use wire_app_engine::projection;
use wire_app_engine::record::{build, review};

fn author() -> String {
    Keypair::from_secret_key(&Secp256k1::new(), &SecretKey::from_slice(&[7; 32]).unwrap())
        .x_only_public_key()
        .0
        .to_string()
}

fn entry(position: u32, raw: &[u8]) -> Entry {
    Entry {
        height: 100,
        position,
        author: author(),
        txid: format!("{position:064x}"),
        record: hex::encode(raw),
        return_script: None,
    }
}

fn legacy() -> Vec<u8> {
    let mut bytes = RecordKind::Avatar.prefix().to_vec();
    bytes.extend_from_slice(&[0x12; 512]);
    bytes
}

fn mixed() -> Index {
    Index {
        format: Index::FORMAT.into(),
        genesis: "00".repeat(32),
        start: 100,
        blocks: vec![Checkpoint {
            height: 100,
            hash: "01".repeat(32),
        }],
        entries: vec![
            entry(
                0,
                &PublicRecord::Post("retained post".into()).encode().unwrap(),
            ),
            entry(
                1,
                &PublicRecord::Profile("retained name".into())
                    .encode()
                    .unwrap(),
            ),
            entry(2, &legacy()),
            entry(
                3,
                &build(&Content::Avatar(vec![0x34; 128]))
                    .unwrap()
                    .encode()
                    .unwrap(),
            ),
            entry(4, &legacy()),
        ],
    }
}

#[test]
fn native_cache_restart_retains_mixed_bytes_and_keeps_feed_readable() {
    let index = mixed();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.json");
    index.persist(&path).unwrap();
    let saved = std::fs::read(&path).unwrap();
    let restarted = Index::load(&path).unwrap();
    let feed = projection::project(&restarted, false).unwrap();
    assert_eq!(feed.posts.len(), 1);
    assert_eq!(feed.posts[0].text, "retained post");
    assert_eq!(
        feed.identities[&author()].nickname.as_deref(),
        Some("retained name")
    );
    assert_eq!(
        feed.identities[&author()].avatar.as_deref(),
        Some([0x34; 128].as_slice())
    );
    assert_eq!(hex::decode(&restarted.entries[2].record).unwrap(), legacy());
    assert_eq!(
        serde_json::to_value(&restarted).unwrap(),
        serde_json::to_value(&index).unwrap()
    );
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    restarted.persist(&path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    assert!(matches!(
        projection::lookup(
            &restarted,
            Txid::from_display_hex(&restarted.entries[0].txid).unwrap()
        )
        .unwrap(),
        Lookup::Found(_)
    ));
    assert!(
        projection::replies(&restarted, Txid::from_raw_bytes([0; 32]))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn browser_serialized_cache_retains_legacy_entry_without_rendering_it() {
    let mut index = mixed();
    index.entries.retain(|entry| entry.position != 3);
    let saved = serde_json::to_vec(&index).unwrap();
    let restarted: Index = serde_json::from_slice(&saved).unwrap();
    let feed = projection::project(&restarted, false).unwrap();
    assert_eq!(feed.posts.len(), 1);
    assert_eq!(
        feed.identities[&author()].nickname.as_deref(),
        Some("retained name")
    );
    assert!(feed.identities[&author()].avatar.is_none());
    assert_eq!(serde_json::to_vec(&restarted).unwrap(), saved);
    assert!(review(&legacy()).is_err());
    assert!(build(&Content::Avatar(vec![0; 512])).is_err());
}

#[test]
fn other_bad_lengths_headers_flags_and_hex_remain_errors() {
    let mut invalid = Vec::new();
    for length in [0, 127, 129, 136, 511, 513] {
        let mut raw = RecordKind::Avatar.prefix().to_vec();
        raw.extend(vec![0; length]);
        invalid.push(raw);
    }
    for offset in [0, 4, 6, 7] {
        let mut raw = legacy();
        raw[offset] ^= 1;
        invalid.push(raw);
    }
    for raw in invalid {
        let mut index = mixed();
        index.entries[2].record = hex::encode(raw);
        assert!(projection::project(&index, false).is_err());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.json");
        index.persist(&path).unwrap();
        assert!(Index::load(&path).is_err());
    }
    let mut index = mixed();
    index.entries[2].record = "zz".into();
    assert!(projection::project(&index, false).is_err());
}

#[test]
fn image_magic_is_not_sniffed_and_raw_record_lengths_cannot_be_confused() {
    for magic in [b"\x89PNG\r\n\x1a\n".as_slice(), b"GIF89a", b"<svg onload="] {
        let mut pixels = vec![0; 128];
        pixels[..magic.len()].copy_from_slice(magic);
        let record = build(&Content::Avatar(pixels.clone()))
            .unwrap()
            .encode()
            .unwrap();
        assert_eq!(record.len(), 136);
        assert_eq!(&record[..8], &RecordKind::Avatar.prefix());
        assert_eq!(review(&record).unwrap(), Content::Avatar(pixels));
        assert!(build(&Content::Avatar(record)).is_err());
    }
}
