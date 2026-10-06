use wire_app_core::feed::{SyncPhase, SyncStatus};
use wire_app_ui::components::brand::{distance, pulse_at, Mark, Trace};
use wire_app_ui::state::session::Synced;

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
}
