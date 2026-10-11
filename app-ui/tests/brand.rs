use wire_app_core::feed::{SyncPhase, SyncStatus};
use wire_app_ui::components::brand::{distance, pulse_at, Mark, Trace};
use wire_app_ui::state::Synced;

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    distance(a, b) < 0.01
}

fn verifying(done: u64, total: u64) -> Mark {
    Mark::of(&Synced::Verifying { done, total })
}

#[test]
fn pulse_runs_from_entry_to_exit_and_clamps() {
    assert!(close(pulse_at(0.0), Trace::ENTRY));
    assert!(close(pulse_at(1.0), Trace::EXIT));
    assert!(close(pulse_at(-1.0), Trace::ENTRY));
    assert!(close(pulse_at(2.0), Trace::EXIT));
}

#[test]
fn each_block_of_a_hundred_advances_one_hundredth_of_the_trace() {
    let steps: Vec<(f64, f64)> = (0..=100).map(|n| verifying(n, 100).pulse()).collect();
    let lengths: Vec<f64> = steps.windows(2).map(|w| distance(w[0], w[1])).collect();
    let on_left_rail = &lengths[8..=28];
    let on_right_rail = &lengths[72..=91];
    let step = on_left_rail[0];
    for length in on_left_rail.iter().chain(on_right_rail) {
        assert!((length - step).abs() < 1e-6, "{length} vs {step}");
    }
    assert!(steps[8..=29].iter().all(|(x, _y)| (x - 120.0).abs() < 1e-6));
    assert!(steps[72..=92].iter().all(|(x, _y)| (x - 392.0).abs() < 1e-6));
    assert!(steps.windows(2).all(|w| w[1].0 >= w[0].0 - 1e-9));
}

#[test]
fn the_pulse_sweeps_the_trace_at_a_bounded_pace_and_never_walks_back() {
    use wire_app_ui::components::brand::swept;
    use wire_app_ui::config::{PULSE_FRAME_MS, PULSE_SWEEP_MS};
    let mut shown = 0.0;
    let mut frames = 0;
    while (1.0 - shown) > f64::EPSILON {
        let next = swept(shown, 1.0);
        assert!(next > shown);
        let (from, to) = (pulse_at(shown), pulse_at(next));
        assert!(distance(from, to) < 30.0, "one frame moves a short way along the trace");
        shown = next;
        frames += 1;
    }
    assert_eq!(frames, (PULSE_SWEEP_MS / PULSE_FRAME_MS) + 1);
    assert_eq!(swept(0.3, 0.31), 0.31);
    assert_eq!(swept(0.8, 0.0), 0.0);
}

#[test]
fn halfway_sits_at_the_bottom_of_the_u() {
    let (x, y) = pulse_at(0.5);
    assert!((x - Trace::BOTTOM.0).abs() < 1.0, "{x}");
    assert!((y - Trace::BOTTOM.1).abs() < 1.0, "{y}");
}

#[test]
fn cuts_seal_as_the_pulse_passes_and_stay_open_on_failure() {
    assert!(Trace::CUTS.iter().all(|(x, _d)| verifying(0, 100).cut_open(*x)));
    assert!(!verifying(100, 100).cut_open(Trace::CUTS[0].0));
    assert!(!verifying(48, 100).cut_open(Trace::CUTS[0].0));
    assert!(verifying(48, 100).cut_open(Trace::CUTS[1].0));
    assert!(!verifying(52, 100).cut_open(Trace::CUTS[2].0));
    assert!(verifying(52, 100).cut_open(Trace::CUTS[3].0));
    let failed = Mark::of(&Synced::Failed("source down".into()));
    assert!(Trace::CUTS.iter().all(|(x, _d)| failed.cut_open(*x)));
    assert!(!failed.pulse_shown);
}

#[test]
fn completion_heals_the_mark_and_hides_the_pulse() {
    let healed = Mark::of(&Synced::Indexed { complete: true });
    assert!(healed.healed);
    assert!(!healed.pulse_shown);
    assert_eq!(healed.fraction, 1.0);
    assert_eq!(verifying(0, 0).fraction, 1.0);
    assert_eq!(verifying(7, 100).fraction, 0.07);
    assert_eq!(verifying(200, 100).fraction, 1.0);
}

#[test]
fn engine_status_maps_to_the_shown_state() {
    let mut status = SyncStatus::idle();
    assert_eq!(Synced::shown(&status), Synced::Unchecked);
    status.phase = SyncPhase::Verifying;
    status.done = 42;
    status.total = 100;
    assert_eq!(
        Synced::shown(&status),
        Synced::Verifying {
            done: 42,
            total: 100
        }
    );
    status.phase = SyncPhase::Complete;
    status.complete_to_tip = true;
    assert_eq!(Synced::shown(&status), Synced::Indexed { complete: true });
    assert_eq!(Synced::shown(&status).label(), "Up to date");
    status.phase = SyncPhase::Failed("x".into());
    assert_eq!(Synced::shown(&status).label(), "Sync failed");
    assert_eq!(Synced::Verifying { done: 1, total: 2 }.label(), "");
    status.phase = SyncPhase::Retrying {
        attempt: 2,
        seconds: 7,
        reason: "peer gone".into(),
    };
    let retrying = Synced::shown(&status);
    assert_eq!(retrying.label(), "Retrying in 7 s");
    let wire_app_ui::state::Said::Something(trouble) = retrying.trouble() else {
        panic!("a retry carries its last error");
    };
    assert!(trouble.contains("peer gone"));
    let mark = Mark::of(&retrying);
    assert!(mark.pulse_shown && !mark.healed);
    assert_eq!(mark.fraction, 0.42);
}

#[test]
fn transport_hints_describe_peers_and_sources() {
    use wire_app_core::connection::{SourceKind, SourceStanding, SourceState, TransportStatus};
    use wire_app_ui::components::brand::{peers_hint, sources_hint};
    let status = TransportStatus {
        peers: 3,
        sources: vec![
            SourceState {
                label: "p2p light client".into(),
                kind: SourceKind::Peers,
                standing: SourceStanding::Ready,
            },
            SourceState {
                label: "ssl://electrum.example:51002".into(),
                kind: SourceKind::Electrum,
                standing: SourceStanding::Ready,
            },
            SourceState {
                label: "https://api.example".into(),
                kind: SourceKind::Http,
                standing: SourceStanding::Blocked { seconds: 23 },
            },
        ],
    };
    assert_eq!(status.remote_sources().len(), 2);
    assert_eq!(status.ready_sources(), 1);
    assert!(peers_hint(&status).starts_with("3 peers"));
    let hint = sources_hint(&status);
    assert!(hint.contains("ssl://electrum.example:51002 · Electrum · ready"));
    assert!(hint.contains("https://api.example · HTTP API · cooling down 23 s"));
    let cold = TransportStatus {
        peers: 0,
        sources: vec![SourceState {
            label: "p2p light client".into(),
            kind: SourceKind::Peers,
            standing: SourceStanding::Cold,
        }],
    };
    assert!(peers_hint(&cold).contains("warming up"));
}

#[test]
fn local_daemon_has_its_own_status_and_is_not_counted_as_public_rpc() {
    use wire_app_core::connection::{SourceKind, SourceStanding, SourceState, TransportStatus};
    use wire_app_ui::components::brand::{local_hint, sources_hint};
    let mut status = TransportStatus {
        peers: 2,
        sources: vec![
            SourceState {
                label: "http://127.0.0.1:19332".into(),
                kind: SourceKind::LocalNode,
                standing: SourceStanding::Cold,
            },
            SourceState {
                label: "https://public.example".into(),
                kind: SourceKind::Http,
                standing: SourceStanding::Ready,
            },
        ],
    };
    assert!(local_hint(&status).contains("checked automatically"));
    assert_eq!(status.remote_sources().len(), 1);
    assert_eq!(status.ready_sources(), 1);
    status.sources[0].standing = SourceStanding::Ready;
    assert!(local_hint(&status).contains("preferred for block reads"));
    assert!(!sources_hint(&status).contains("127.0.0.1"));
    status.sources[0].standing = SourceStanding::Blocked { seconds: 3 };
    assert!(local_hint(&status).contains("retrying in 3 s"));
    status.sources.remove(0);
    assert!(local_hint(&status).contains("not configured"));
}
