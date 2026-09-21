use wire_app_shared::{
    decode_avatar_pixels, render_avatar_svg, render_identicon_svg, AuthorId, AvatarPalette, Chain,
    ConfirmedStatus, MockWireStore, Post, ProtocolAdapter, ProtocolCodec, RecordKind, Txid,
    WireError, WireStore,
};

#[test]
fn test_author_id_fingerprint_and_hex() {
    let raw = [0xabu8; 32];
    let author = AuthorId::from_bytes(raw);
    assert_eq!(author.fingerprint(), "abababab");
    assert_eq!(author.to_hex().len(), 64);
    assert!(author.to_hex().starts_with("abab"));

    let parsed = AuthorId::from_hex(&author.to_hex()).unwrap();
    assert_eq!(author, parsed);
}

#[test]
fn test_txid_wire_and_display_hex() {
    let mut raw = [0u8; 32];
    raw[0] = 0x01;
    raw[31] = 0xfe;
    let txid = Txid::from_raw_bytes(raw);

    let wire_hex = txid.to_wire_hex();
    assert!(wire_hex.starts_with("01"));
    assert!(wire_hex.ends_with("fe"));

    let display_hex = txid.to_display_hex();
    assert!(display_hex.starts_with("fe"));
    assert!(display_hex.ends_with("01"));

    let from_disp = Txid::from_display_hex(&display_hex).unwrap();
    assert_eq!(txid, from_disp);
}

#[test]
fn test_codec_post_roundtrip() {
    let codec = ProtocolCodec;
    let text = "Free press investigation URMA V0 — without intermediaries.";
    let encoded = codec.encode_post(text).unwrap();

    assert_eq!(&encoded[..4], b"URMA");
    assert_eq!(encoded[4], 0); // version 0
    assert_eq!(encoded[5], RecordKind::Post.to_u8());
    assert_eq!(&encoded[6..8], &[0, 0]); // flags

    let decoded = codec.decode_post(&encoded).unwrap();
    assert_eq!(text, decoded);
}

#[test]
fn test_codec_reply_roundtrip() {
    let codec = ProtocolCodec;
    let target = Txid::from_raw_bytes([0x42; 32]);
    let text = "Receipt confirmation for investigation records.";
    let encoded = codec.encode_reply(&target, text).unwrap();

    assert_eq!(encoded[5], RecordKind::Reply.to_u8());
    let (decoded_target, decoded_text) = codec.decode_reply(&encoded).unwrap();
    assert_eq!(target, decoded_target);
    assert_eq!(text, decoded_text);
}

#[test]
fn test_codec_avatar_roundtrip_and_exact_size() {
    let codec = ProtocolCodec;
    let pixels = [0x5au8; 512]; // 512 bytes = 1024 4-bit nibbles
    let encoded = codec.encode_avatar(&pixels).unwrap();

    assert_eq!(encoded.len(), 520); // 8 header + 512 bytes = 520 bytes
    assert_eq!(encoded[5], RecordKind::Avatar.to_u8());

    let decoded = codec.decode_avatar(&encoded).unwrap();
    assert_eq!(pixels, decoded);

    // Invalid length rejection
    let short = vec![0u8; 519];
    assert!(codec.decode_avatar(&short).is_err());
}

#[test]
fn test_codec_rejections() {
    let codec = ProtocolCodec;

    // Bad magic
    let mut bad_magic = codec.encode_post("hello").unwrap();
    bad_magic[0] = b'X';
    let err = codec.decode_post(&bad_magic).unwrap_err();
    assert!(matches!(err, WireError::Protocol(msg) if msg.contains("marker")));

    // Non-zero flags
    let mut bad_flags = codec.encode_post("hello").unwrap();
    bad_flags[6] = 1;
    let err = codec.decode_post(&bad_flags).unwrap_err();
    assert!(matches!(err, WireError::Protocol(msg) if msg.contains("flags")));

    // Truncated prefix
    let err = codec.decode_post(b"UR").unwrap_err();
    assert!(matches!(err, WireError::Protocol(msg) if msg.contains("truncated")));

    // Exceeds 32 KiB cap
    let large_text = "A".repeat(32_761); // 8 prefix + 32,761 = 32,769 > 32,768
    let err = codec.encode_post(&large_text).unwrap_err();
    assert!(matches!(err, WireError::Protocol(msg) if msg.contains("32 KiB")));
}

#[test]
fn test_avatar_palette_and_rendering() {
    assert_eq!(AvatarPalette::COLORS.len(), 16);
    assert_eq!(AvatarPalette::hex(0), "#000000");
    assert_eq!(AvatarPalette::hex(15), "#FFFFFF");

    let mut bytes = [0u8; 512];
    bytes[0] = 0x1F; // pixel 0 = 1, pixel 1 = 15
    let pixels = decode_avatar_pixels(&bytes);
    assert_eq!(pixels[0], 1);
    assert_eq!(pixels[1], 15);

    let svg = render_avatar_svg(&bytes, 64);
    assert!(svg.contains("<svg"));
    assert!(svg.contains(r#"viewBox="0 0 32 32""#));
    assert!(svg.contains(r#"width="64""#));

    let author = AuthorId::from_bytes([0x77; 32]);
    let identicon = render_identicon_svg(&author, 64);
    assert!(identicon.contains("<svg"));
    assert!(identicon.contains(r#"viewBox="0 0 32 32""#));
}

#[test]
fn test_store_chain_isolation_and_mempool_rejection() {
    let mut store = MockWireStore::new();

    let post_btc = Post {
        txid: Txid::from_raw_bytes([0x11; 32]),
        author: AuthorId::from_bytes([0xaa; 32]),
        chain: Chain::Bitcoin,
        text: "Bitcoin testnet post".into(),
        wire: String::new(),
        hashtags: Vec::new(),
        status: ConfirmedStatus {
            block_height: 150_000,
            block_hash: "hash_btc_1".into(),
            confirmations: 5,
            tx_index: 1,
        },
        reply_count: 0,
    };
    let post_ltc = Post {
        txid: Txid::from_raw_bytes([0x22; 32]),
        author: AuthorId::from_bytes([0xbb; 32]),
        chain: Chain::Litecoin,
        text: "Litecoin testnet post".into(),
        wire: String::new(),
        hashtags: Vec::new(),
        status: ConfirmedStatus {
            block_height: 4_800_000,
            block_hash: "hash_ltc_1".into(),
            confirmations: 10,
            tx_index: 2,
        },
        reply_count: 0,
    };

    store.insert_post(post_btc).unwrap();
    store.insert_post(post_ltc).unwrap();

    let btc_feed = store.get_feed(Chain::Bitcoin, 50).unwrap();
    let ltc_feed = store.get_feed(Chain::Litecoin, 50).unwrap();

    assert_eq!(btc_feed.len(), 1);
    assert_eq!(ltc_feed.len(), 1);
    assert_eq!(btc_feed[0].chain, Chain::Bitcoin);
    assert_eq!(ltc_feed[0].chain, Chain::Litecoin);

    // Mempool (0 confirmations) must be rejected per D-017
    let unconfirmed_post = Post {
        txid: Txid::from_raw_bytes([0x99; 32]),
        author: AuthorId::from_bytes([0x88; 32]),
        chain: Chain::Bitcoin,
        text: "Mempool post".into(),
        wire: String::new(),
        hashtags: Vec::new(),
        status: ConfirmedStatus {
            block_height: 150_100,
            block_hash: "mempool".into(),
            confirmations: 0,
            tx_index: 0,
        },
        reply_count: 0,
    };
    assert!(matches!(
        store.insert_post(unconfirmed_post),
        Err(WireError::UnconfirmedMempool)
    ));
}

#[test]
fn test_store_reorg_rollback() {
    let mut store = MockWireStore::new();
    let author = AuthorId::from_bytes([0x12; 32]);

    let post_base = Post {
        txid: Txid::from_raw_bytes([0x10; 32]),
        author,
        chain: Chain::Bitcoin,
        text: "Base post".into(),
        wire: String::new(),
        hashtags: Vec::new(),
        status: ConfirmedStatus {
            block_height: 150_000,
            block_hash: "hash_base".into(),
            confirmations: 10,
            tx_index: 0,
        },
        reply_count: 0,
    };
    store.insert_post(post_base).unwrap();

    // Insert a post on an orphaned branch at height 150_010
    let post_high = Post {
        txid: Txid::from_raw_bytes([0xaa; 32]),
        author,
        chain: Chain::Bitcoin,
        text: "Post on reorg fork".into(),
        wire: String::new(),
        hashtags: Vec::new(),
        status: ConfirmedStatus {
            block_height: 150_010,
            block_hash: "hash_fork".into(),
            confirmations: 2,
            tx_index: 1,
        },
        reply_count: 0,
    };
    store.insert_post(post_high).unwrap();

    let btc_feed_before = store.get_feed(Chain::Bitcoin, 50).unwrap();
    assert_eq!(btc_feed_before.len(), 2);

    // Perform reorg rollback to 150_000 (common ancestor)
    let removed = store.rollback_to_height(Chain::Bitcoin, 150_000).unwrap();
    assert_eq!(removed, 1);

    let btc_feed_after = store.get_feed(Chain::Bitcoin, 50).unwrap();
    assert_eq!(btc_feed_after.len(), 1);
    assert_eq!(btc_feed_after[0].txid, Txid::from_raw_bytes([0x10; 32]));
}

#[test]
fn test_persistent_wire_store_disk_roundtrip_and_reorg() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db_path = tmp_dir.path().join("test_store.json");

    let author = AuthorId::from_bytes([0x44; 32]);
    let post_txid = Txid::from_raw_bytes([0x55; 32]);

    {
        let mut store = wire_app_shared::PersistentWireStore::open_or_create(&db_path).unwrap();

        // Check mempool rejection (0 conf)
        let unconfirmed = Post {
            txid: post_txid,
            author,
            chain: Chain::Litecoin,
            text: "test unconfirmed".into(),
            wire: String::new(),
            hashtags: Vec::new(),
            status: ConfirmedStatus {
                block_height: 1000,
                block_hash: "hash_0".into(),
                confirmations: 0,
                tx_index: 0,
            },
            reply_count: 0,
        };
        assert_eq!(
            store.insert_post(unconfirmed),
            Err(WireError::UnconfirmedMempool)
        );

        // Insert confirmed post
        let confirmed = Post {
            txid: post_txid,
            author,
            chain: Chain::Litecoin,
            text: "test".into(),
            wire: String::new(),
            hashtags: Vec::new(),
            status: ConfirmedStatus {
                block_height: 1000,
                block_hash: "hash_1000".into(),
                confirmations: 1,
                tx_index: 0,
            },
            reply_count: 0,
        };
        store.insert_post(confirmed).unwrap();

        let post_high = Post {
            txid: Txid::from_raw_bytes([0x66; 32]),
            author,
            chain: Chain::Litecoin,
            text: "orphan branch".into(),
            wire: String::new(),
            hashtags: Vec::new(),
            status: ConfirmedStatus {
                block_height: 1005,
                block_hash: "hash_1005".into(),
                confirmations: 1,
                tx_index: 0,
            },
            reply_count: 0,
        };
        store.insert_post(post_high).unwrap();
    }

    // Reopen from disk (verifying persistence)
    {
        let mut reopened = wire_app_shared::PersistentWireStore::open_or_create(&db_path).unwrap();
        let feed = reopened.get_feed(Chain::Litecoin, 10).unwrap();
        assert_eq!(feed.len(), 2);
        assert_eq!(feed[1].text, "test");
        assert_eq!(feed[1].txid, post_txid);

        // Roll back to 1000
        let removed = reopened.rollback_to_height(Chain::Litecoin, 1000).unwrap();
        assert_eq!(removed, 1);

        let feed_after = reopened.get_feed(Chain::Litecoin, 10).unwrap();
        assert_eq!(feed_after.len(), 1);
        assert_eq!(feed_after[0].txid, post_txid);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn shared_wallet() -> wire_app_shared::WireWallet {
    use urma_identity::{phrase::IdentityPhrase, vault::UnlockCredential};
    let words = format!("{}art", "abandon ".repeat(23));
    let phrase = IdentityPhrase::parse(&words).unwrap();
    wire_app_shared::WireWallet::unlock(
        include_bytes!("fixtures/valid.vault"),
        UnlockCredential::RecoveryPhrase(&phrase),
    )
    .unwrap()
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_shared_wallet_author_and_funding_are_the_same_active_identity() {
    use urma_identity::identity::{IdentitySigner, IdentitySlot};
    let mut wallet = shared_wallet();
    let initial = wallet.author_id().unwrap();
    let key = wallet.signer().unwrap();
    assert_eq!(
        initial.0,
        key.public_key().inner.x_only_public_key().0.serialize()
    );
    assert_eq!(
        wallet.get_receive_address(Chain::Bitcoin).unwrap(),
        urma_wallet::address::receive_address(
            &key,
            urma_chain::observation::Chain::BitcoinTestnet4
        )
        .unwrap()
    );
    assert!(wallet
        .get_receive_address(Chain::Litecoin)
        .unwrap()
        .starts_with("tltc1q"));
    wallet.select(IdentitySlot::new(0).unwrap()).unwrap();
    assert_ne!(initial, wallet.author_id().unwrap());
    let summary = wallet.summary(Chain::Bitcoin, 1000, 0, 1, 150000).unwrap();
    assert_eq!(summary.author_pubkey, wallet.author_id().unwrap().to_hex());
    assert_eq!(summary.confirmed_balance_sats, 1000);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_shared_wallet_exports_only_encrypted_vault() {
    use urma_identity::{phrase::IdentityPhrase, vault::UnlockCredential};
    let wallet = shared_wallet();
    let backup = wallet.encrypted_backup(&mut rand::rngs::OsRng).unwrap();
    let words = format!("{}art", "abandon ".repeat(23));
    assert!(!backup
        .as_bytes()
        .windows(words.len())
        .any(|x| x == words.as_bytes()));
    let phrase = IdentityPhrase::parse(&words).unwrap();
    let reopened = wire_app_shared::WireWallet::unlock(
        backup.as_bytes(),
        UnlockCredential::RecoveryPhrase(&phrase),
    )
    .unwrap();
    assert_eq!(reopened.author_id().unwrap(), wallet.author_id().unwrap());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_publication_quote() {
    let text = "Public interest post test";
    let signer = shared_wallet().signer().unwrap();
    let quote = wire_app_shared::LedgerClient::quote_publication(
        Chain::Litecoin,
        text,
        1,
        100_000,
        &signer,
    )
    .unwrap();

    assert_eq!(quote.network_name, "Litecoin Testnet");
    assert!(quote.sufficient_balance);
    assert!(quote.estimated_fee_sats > 0);

    // Insufficient balance test
    let poor_quote =
        wire_app_shared::LedgerClient::quote_publication(Chain::Litecoin, text, 1, 100, &signer)
            .unwrap();
    assert!(!poor_quote.sufficient_balance);
    assert!(poor_quote.warning.is_some());
    let required = quote.estimated_fee_sats
        + 2 * urma_runtime::config::publication_return(
            urma_chain::observation::Chain::LitecoinTestnet,
        );
    assert!(
        wire_app_shared::LedgerClient::quote_publication(
            Chain::Litecoin,
            text,
            1,
            required,
            &signer
        )
        .unwrap()
        .sufficient_balance
    );
    assert!(
        !wire_app_shared::LedgerClient::quote_publication(
            Chain::Litecoin,
            text,
            1,
            required - 1,
            &signer
        )
        .unwrap()
        .sufficient_balance
    );
    for rate in [0, 101, u64::MAX] {
        assert!(wire_app_shared::LedgerClient::quote_publication(
            Chain::Litecoin,
            text,
            rate,
            u64::MAX,
            &signer
        )
        .is_err());
    }
    let oversized = "a".repeat(32_761);
    assert!(wire_app_shared::LedgerClient::quote_publication(
        Chain::Litecoin,
        &oversized,
        1,
        u64::MAX,
        &signer
    )
    .is_err());
}
