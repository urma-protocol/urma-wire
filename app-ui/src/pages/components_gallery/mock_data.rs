use wire_app_core::avatar::Urma;
use wire_app_core::model::{AuthorId, Chain, ConfirmedStatus, Placement, Post, Txid};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub txid: Txid,
    pub author: AuthorId,
    pub chain: Chain,
    pub target_txid: Txid,
    pub text: String,
    pub status: ConfirmedStatus,
}

pub fn mock_author_investigator() -> (AuthorId, &'static str, &'static str) {
    let mut bytes = [0x3bu8; 32];
    bytes[0] = 0x3b;
    bytes[1] = 0x8d;
    bytes[2] = 0x4f;
    bytes[3] = 0x1a;
    bytes[31] = 0x9f;
    (AuthorId::from_bytes(bytes), "Alex Rivera", "@alex_intel")
}

pub fn mock_author_auditor() -> (AuthorId, &'static str, &'static str) {
    let mut bytes = [0x7cu8; 32];
    bytes[0] = 0x7c;
    bytes[1] = 0x1a;
    bytes[2] = 0x9e;
    bytes[3] = 0x5b;
    bytes[31] = 0x44;
    (AuthorId::from_bytes(bytes), "Elena Vance", "@chain_auditor")
}

pub fn mock_author_source() -> (AuthorId, &'static str, &'static str) {
    let mut bytes = [0xe5u8; 32];
    bytes[0] = 0xe5;
    bytes[1] = 0x2c;
    bytes[2] = 0x88;
    bytes[3] = 0xd0;
    bytes[31] = 0x11;
    (AuthorId::from_bytes(bytes), "Source Zero", "@whistle_anon")
}

pub fn mock_txid_investigation() -> Txid {
    let mut bytes = [0x12u8; 32];
    bytes[0] = 0x9f;
    bytes[1] = 0x8a;
    bytes[2] = 0x77;
    bytes[3] = 0x4c;
    bytes[30] = 0x33;
    bytes[31] = 0xd1;
    Txid::from_raw_bytes(bytes)
}

pub fn mock_txid_alert() -> Txid {
    let mut bytes = [0xabu8; 32];
    bytes[0] = 0x44;
    bytes[1] = 0xee;
    bytes[2] = 0x12;
    bytes[3] = 0x90;
    bytes[30] = 0xfa;
    bytes[31] = 0x08;
    Txid::from_raw_bytes(bytes)
}

pub fn mock_txid_editorial() -> Txid {
    let mut bytes = [0x66u8; 32];
    bytes[0] = 0x71;
    bytes[1] = 0x28;
    bytes[2] = 0x93;
    bytes[3] = 0x01;
    bytes[30] = 0xbb;
    bytes[31] = 0x52;
    Txid::from_raw_bytes(bytes)
}

pub fn mock_post_investigative() -> Post {
    let (author, _, _) = mock_author_investigator();
    Post {
        txid: mock_txid_investigation(),
        author,
        chain: Chain::Litecoin,
        text: "Initial satellite imagery corroborates naval repositioning off the Constanța corridor (44.128° N, 28.654° E). Merkle commitment verified at block #4892707.\n\nFull sensor logs and AIS cross-references anchored permanently on-chain. Witness proof status: 100% verified.".to_string(),
        wire: "rosint".to_string(),
        hashtags: vec!["investigation".into(), "satellites".into(), "maritime".into()],
        placement: Placement::Root,
        status: ConfirmedStatus {
            block_height: 4_892_707,
            block_hash: "3a8f10bc92d8471e55c0299aa14bc8fe9210c4d8123019847182746190abcf12".to_string(),
            confirmations: 42,
            tx_index: 8,
        },
        reply_count: 5,
    }
}

pub fn mock_post_breaking() -> Post {
    let (author, _, _) = mock_author_auditor();
    Post {
        txid: mock_txid_alert(),
        author,
        chain: Chain::Litecoin,
        text: "URMA Protocol Alert: Transient 2-block reorg detected at height #4892715. Local transparent index rewound checkpoint cleanly. Verified feed resumed with zero data loss.".to_string(),
        wire: "alerts".to_string(),
        hashtags: vec!["reorg".into(), "network_health".into()],
        placement: Placement::Root,
        status: ConfirmedStatus {
            block_height: 4_892_716,
            block_hash: "9102cba841029471928374619028374619208374619028374619028374619283".to_string(),
            confirmations: 6,
            tx_index: 3,
        },
        reply_count: 1,
    }
}

pub fn mock_post_editorial() -> Post {
    let (author, _, _) = mock_author_source();
    Post {
        txid: mock_txid_editorial(),
        author,
        chain: Chain::Litecoin,
        text: "### Sovereign Journalism Without Middlemen\n\nEvery character published here is signed by an ephemeral BIP-340 key held in user session memory. No cookies, no trackers, no centralized feed algorithm.\n\n> When news is anchored in the ledger, censorship becomes mathematically intractable.\n\nArchival reference: `ipfs://bafybeic...` cross-indexed.".to_string(),
        wire: "deepdive".to_string(),
        hashtags: vec!["sovereignty".into(), "journalism".into(), "cryptography".into()],
        placement: Placement::Root,
        status: ConfirmedStatus {
            block_height: 4_892_680,
            block_hash: "beef102938475610293847561029384756102938475610293847561029384756".to_string(),
            confirmations: 134,
            tx_index: 14,
        },
        reply_count: 12,
    }
}

pub fn mock_reply_corroboration() -> Reply {
    let (author, _, _) = mock_author_auditor();
    Reply {
        txid: mock_txid_alert(),
        author,
        chain: Chain::Litecoin,
        target_txid: mock_txid_investigation(),
        text: "Cross-checked AIS coastal radar logs. Transponder MMSI 271000123 switched to passive broadcast 14 nmi prior to the waypoint. Data matches satellite pass time exactly.".to_string(),
        status: ConfirmedStatus {
            block_height: 4_892_709,
            block_hash: "4918237461928374619283746192837461928374619283746192837461928374".to_string(),
            confirmations: 39,
            tx_index: 12,
        },
    }
}

pub fn mock_reply_quick() -> Reply {
    let (author, _, _) = mock_author_source();
    Reply {
        txid: mock_txid_editorial(),
        author,
        chain: Chain::Litecoin,
        target_txid: mock_txid_alert(),
        text: "Confirmed on node `node-ro-01.urma.is`. Checkpoint re-anchored at block 4,892,716 with zero unspent output collisions.".to_string(),
        status: ConfirmedStatus {
            block_height: 4_892_718,
            block_hash: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
            confirmations: 3,
            tx_index: 4,
        },
    }
}

/// A crisp 16x16 pixel art avatar depicting an investigative falcon/hawk shield emblem
/// encoded into 128 bytes with the URMA 16-color palette.
pub fn mock_hawk_avatar_bytes() -> [u8; Urma::AVATAR_BYTES] {
    [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x88, 0x88, 0x88, 0x88, 0x88, 0x88, 0x00, 0x00, 0x88, 0x88, 0x22, 0x22, 0x22,
        0x88, 0x00, 0x00, 0x88, 0x22, 0x22, 0xaa, 0x22, 0x22, 0x00, 0x00, 0x88, 0x22, 0xaa, 0xaa,
        0xaa, 0x22, 0x00, 0x00, 0x88, 0x22, 0xee, 0xee, 0xee, 0x22, 0x00, 0x00, 0x88, 0xaa, 0xee,
        0xee, 0xee, 0xaa, 0x00, 0x00, 0x88, 0xaa, 0xee, 0xee, 0xee, 0xaa, 0x00, 0x00, 0x88, 0xaa,
        0xee, 0xee, 0xee, 0xaa, 0x00, 0x00, 0x88, 0x22, 0xee, 0xee, 0xee, 0x22, 0x00, 0x00, 0x88,
        0x22, 0xaa, 0xaa, 0xaa, 0x22, 0x00, 0x00, 0x88, 0x22, 0x22, 0xaa, 0x22, 0x22, 0x00, 0x00,
        0x88, 0x88, 0x22, 0x22, 0x22, 0x88, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]
}
