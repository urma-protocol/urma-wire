use wire_app_core::model::Chain;
use wire_app_core::publication::{Addressed, Content, Publication};
use wire_app_ui::components::publication_panel::{staged, Stage};

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

#[test]
fn the_cost_is_quoted_in_whole_coins() {
    assert_eq!(Chain::Litecoin.amount(279), "0.00000279 LTC");
    assert_eq!(Chain::Litecoin.amount(123_456_789), "1.23456789 LTC");
}

#[test]
fn the_dialog_shows_only_until_both_transactions_are_sent() {
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
        Stage::Hidden
    ));
    assert!(matches!(
        staged(publication(true, 1, 1), false),
        Stage::Hidden
    ));
    assert!(matches!(
        staged(publication(true, 2, 0), false),
        Stage::Hidden
    ));
    assert!(matches!(
        staged(publication(true, 0, 1), false),
        Stage::Stalled(_)
    ));
}
