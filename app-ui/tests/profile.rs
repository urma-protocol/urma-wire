use leptos::prelude::{GetUntracked, Owner, Set};
use std::collections::BTreeMap;
use wire_app_core::{avatar::Urma, feed::PublicIdentity, model::AuthorId};
use wire_app_ui::{
    components::profile_editor::confirmed_nickname, config::MAX_NICKNAME_BYTES, state::AppState,
};

#[test]
fn confirmed_editor_load_preserves_empty_nul_whitespace_nfc_nfd_and_legacy_text() {
    assert_eq!(MAX_NICKNAME_BYTES, Urma::MAX_PROFILE_BYTES);
    assert_eq!(MAX_NICKNAME_BYTES, 128);
    for text in [
        String::new(),
        " \0\r\n".into(),
        "é".repeat(64),
        format!("{}xx", "e\u{301}".repeat(42)),
        "x".repeat(32760),
    ] {
        assert_eq!(
            confirmed_nickname(&PublicIdentity {
                nickname: Some(text.clone()),
                avatar: None
            }),
            text
        );
    }
}

#[test]
fn directly_stored_oversized_identity_uses_fingerprint_without_mutating_text_or_avatar() {
    Owner::new().with(|| {
        let state = AppState::blank();
        let author = AuthorId::from_bytes([0x37; 32]);
        for nickname in ["x".repeat(129), "界".repeat(43), "x".repeat(32760)] {
            let identity = PublicIdentity {
                nickname: Some(nickname.clone()),
                avatar: Some(vec![0x12; 128]),
            };
            let stored = BTreeMap::from([(author.to_hex(), identity)]);
            state.public_identities.set(stored.clone());
            assert_eq!(state.display_name(author), author.fingerprint());
            assert_eq!(state.public_identities.get_untracked(), stored);
            let loaded = confirmed_nickname(&PublicIdentity {
                nickname: Some(nickname.clone()),
                avatar: None,
            });
            assert_eq!(loaded, nickname);
        }
        state.public_identities.set(BTreeMap::from([(
            author.to_hex(),
            PublicIdentity {
                nickname: Some("é".repeat(64)),
                avatar: None,
            },
        )]));
        assert_eq!(state.display_name(author), "é".repeat(64));
    });
}
