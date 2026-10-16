use wire_app_core::connection::{
    NodeHealth, SourceKind, SourceStanding, SourceState, TransportStatus,
};
use wire_app_core::feed::FeedSource;
use wire_app_ui::components::brand::local_view;
use wire_app_ui::state::session::shows;

fn serving(indexed_through: u64, generation: u64) -> NodeHealth {
    NodeHealth::Serving {
        endpoint: "127.0.0.1:28331".into(),
        indexed_through,
        tip: indexed_through,
        generation,
        chain_peers: 8,
    }
}

fn operator(indexed_through: u64, generation: u64) -> FeedSource {
    FeedSource::LocalNodeOperator {
        indexed_through: Some(indexed_through),
        peer_synced_through: None,
        generation,
        trust: FeedSource::OPERATOR_TRUST.into(),
    }
}

fn gone() -> NodeHealth {
    NodeHealth::Unavailable {
        endpoint: "127.0.0.1:28331".into(),
        reason: "not running".into(),
    }
}

fn transport(node: NodeHealth, core: SourceStanding) -> TransportStatus {
    TransportStatus {
        peers: 1,
        sources: vec![SourceState {
            label: "http://127.0.0.1:19332".into(),
            kind: SourceKind::LocalNode,
            standing: core,
        }],
        node,
    }
}

#[test]
fn the_feed_follows_the_node_view_it_shows() {
    assert!(shows(&operator(10, 1), &serving(10, 1)));
    assert!(!shows(&operator(10, 1), &serving(11, 1)));
    assert!(!shows(&operator(10, 1), &serving(10, 2)));
    assert!(!shows(&operator(10, 1), &gone()));
    assert!(!shows(&FeedSource::AutonomousLocal, &serving(10, 1)));
    assert!(shows(&FeedSource::AutonomousLocal, &gone()));
    assert!(shows(
        &FeedSource::AutonomousLocal,
        &NodeHealth::Unconfigured
    ));
}

#[test]
fn the_local_indicator_names_the_active_source() {
    let node = local_view(&transport(serving(10, 1), SourceStanding::Ready));
    assert_eq!(node.label, "Node");
    assert!(node.tone.ends_with("live"));
    assert!(node.hint.contains("serves the feed"));
    let behind = NodeHealth::Behind {
        endpoint: "127.0.0.1:28331".into(),
        reason: "operator archive has nothing indexed".into(),
    };
    let syncing = local_view(&transport(behind, SourceStanding::Cold));
    assert_eq!(syncing.label, "Node");
    assert!(syncing.tone.ends_with("partial"));
    let core = local_view(&transport(gone(), SourceStanding::Ready));
    assert_eq!(core.label, "Core");
    assert!(core.tone.ends_with("live"));
    assert!(core.hint.contains("URMA Node: not running."));
    let none = local_view(&transport(gone(), SourceStanding::Cold));
    assert_eq!(none.label, "Local");
    assert!(none.tone.ends_with("down"));
    assert!(none.hint.starts_with("No local source."));
}
