use leptos::prelude::{GetUntracked, Owner, Set, Update};
use wire_app_core::composer::Photo;
use wire_app_core::model::Chain;
use wire_app_core::publication::{Addressed, Content, Kind, Publication};
use wire_app_ui::components::publication_panel::{staged, Stage};
use wire_app_ui::state::{AppState, Reviewed};

fn publication(approved: bool, confirmed: usize, mempool: usize) -> Publication {
    Publication {
        content: Content::Post {
            text: "hello".into(),
            wire: String::new(),
            hashtags: Vec::new(),
            addressed: Addressed::Standalone,
        },
        id: "id".into(),
        txid: String::new(),
        fee: 279,
        selected_funding: 10_000,
        returned: 9_721,
        approved,
        confirmed,
        mempool,
        message: String::new(),
        complete: confirmed == 2,
        abandonable: false,
    }
}

fn approved(id: &str) -> Publication {
    Publication {
        content: Content::Post {
            text: "Reviewed text".into(),
            wire: "original".into(),
            hashtags: vec![],
            addressed: Addressed::Standalone,
        },
        id: id.into(),
        txid: String::new(),
        fee: 0,
        selected_funding: 0,
        returned: 0,
        approved: true,
        confirmed: 0,
        mempool: 0,
        message: String::new(),
        complete: false,
        abandonable: false,
    }
}

#[test]
fn the_cost_is_quoted_in_whole_coins() {
    assert_eq!(Chain::Litecoin.amount(279), "0.00000279 LTC");
    assert_eq!(Chain::Litecoin.amount(123_456_789), "1.23456789 LTC");
}

#[test]
fn the_line_follows_the_publication_from_quote_to_block() {
    assert!(matches!(
        staged(publication(false, 0, 0), false),
        Stage::Quote(_)
    ));
    assert!(matches!(
        staged(publication(true, 0, 0), true),
        Stage::Sending
    ));
    assert!(matches!(
        staged(publication(true, 0, 1), true),
        Stage::Sending
    ));
    assert!(matches!(
        staged(publication(true, 0, 2), true),
        Stage::Confirming
    ));
    assert!(matches!(
        staged(publication(true, 1, 1), true),
        Stage::Confirming
    ));
    assert!(matches!(
        staged(publication(true, 2, 0), true),
        Stage::Published
    ));
    assert!(matches!(
        staged(publication(true, 2, 0), false),
        Stage::Published
    ));
    assert!(matches!(
        staged(publication(true, 0, 1), false),
        Stage::Stalled(_)
    ));
    assert!(matches!(
        staged(publication(true, 0, 2), false),
        Stage::Stalled(_)
    ));
}

#[test]
fn each_panel_answers_to_its_own_kind() {
    assert_eq!(Content::Avatar(vec![0; 128]).kind(), Kind::Avatar);
    assert_eq!(Content::Nickname(String::new()).kind(), Kind::Nickname);
    assert_eq!(publication(false, 0, 0).content.kind(), Kind::Post);
}

#[test]
fn delayed_approval_keeps_new_edits_and_clears_only_the_unchanged_draft() {
    Owner::new().with(|| {
        let state = AppState::blank();
        state.set_draft("", "Reviewed text".into());
        state.set_photos(
            "",
            vec![Photo::new("https://cdn.example/old.png", "Old").unwrap()],
        );
        state.set_draft_labels("", "original".into(), "first".into());
        let reviewed = Reviewed::Draft {
            key: String::new(),
            version: state.draft_version(""),
        };

        state.set_draft("", "New text while approval was pending".into());
        let new_photo = Photo::new("https://cdn.example/new.png", "New").unwrap();
        state.set_photos("", vec![new_photo.clone()]);
        state.set_draft_labels("", "new-wire".into(), "second".into());
        state.invalidated_reviews.update(|ids| {
            ids.insert("old-plan".into());
        });
        state.publication.set(Some(approved("old-plan")));
        assert!(state.apply_advance_result(approved("old-plan"), true, &reviewed));
        assert_eq!(state.draft(""), "New text while approval was pending");
        assert_eq!(state.photos(""), vec![new_photo]);
        assert_eq!(state.draft_labels(""), ("new-wire".into(), "second".into()));
        assert!(state.publication.get_untracked().unwrap().approved);
        assert!(!state
            .invalidated_reviews
            .get_untracked()
            .contains("old-plan"));

        let clean = AppState::blank();
        clean.set_draft("", "Reviewed text".into());
        clean.set_photos(
            "",
            vec![Photo::new("https://cdn.example/old.png", "Old").unwrap()],
        );
        clean.set_draft_labels("", "original".into(), "first".into());
        let reviewed = Reviewed::Draft {
            key: String::new(),
            version: clean.draft_version(""),
        };
        clean.publication.set(Some(approved("clean-plan")));
        assert!(clean.apply_advance_result(approved("clean-plan"), true, &reviewed));
        assert_eq!(clean.draft(""), "");
        assert!(clean.photos("").is_empty());
        assert_eq!(clean.draft_labels(""), (String::new(), String::new()));
        assert_eq!(clean.draft_clear_revision(""), 1);

        let stranger = AppState::blank();
        stranger.publication.set(Some(approved("held-plan")));
        assert!(!stranger.apply_advance_result(approved("other-plan"), false, &Reviewed::Nothing));
        assert_eq!(
            stranger.publication.get_untracked().unwrap().id,
            "held-plan"
        );
    });
}
