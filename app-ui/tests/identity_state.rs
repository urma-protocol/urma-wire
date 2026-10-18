use leptos::prelude::{GetUntracked, Owner};
use wire_app_core::errors::{Fault, FaultKind};
use wire_app_core::model::AuthorId;
use wire_app_core::session::{IdentityStatus, Session, Vault};
use wire_app_ui::state::onboarding::{replacing, screen, verb, Chosen, Screen, Step};
use wire_app_ui::state::{AppState, Identity, IdentityAnswer, Said};

fn executor() {
    match any_spawner::Executor::init_futures_executor() {
        Ok(()) => {}
        Err(error) => tracing::warn!(error = %error, "executor already set"),
    }
}

fn status(session: Session, vault: Vault) -> IdentityStatus {
    IdentityStatus { session, vault }
}

fn unlocked(byte: u8) -> Session {
    Session::Unlocked(AuthorId([byte; 32]))
}

#[test]
fn one_writer_moves_the_identity_and_drops_stale_status_answers() {
    executor();
    Owner::new().with(|| {
        let state = AppState::blank();
        assert_eq!(state.identity.get_untracked(), Identity::Reading);
        assert_eq!(state.session(), Session::Locked);
        let startup = state.identity_epoch.get_untracked();

        state.identity_is(status(unlocked(1), Vault::Local));
        assert_eq!(state.session(), unlocked(1));
        assert!(state.wallet_summary.get_untracked().is_none());

        assert_eq!(
            state.settle_identity(startup, Ok(status(Session::Locked, Vault::Missing))),
            IdentityAnswer::Stale
        );
        assert_eq!(state.session(), unlocked(1));

        let current = state.identity_epoch.get_untracked();
        assert_eq!(
            state.settle_identity(current, Ok(status(Session::Locked, Vault::Local))),
            IdentityAnswer::Shown
        );
        assert_eq!(state.session(), Session::Locked);
        assert_eq!(
            state.identity.get_untracked(),
            Identity::Known(status(Session::Locked, Vault::Local))
        );

        let later = state.identity_epoch.get_untracked();
        state.settle_identity(later, Err(Fault::new(FaultKind::Transport, "IPC down")));
        assert_eq!(
            state.identity.get_untracked(),
            Identity::Unavailable("IPC down".into())
        );
        assert_eq!(state.session(), Session::Locked);
    });
}

#[test]
fn the_screen_follows_the_status_until_the_user_picks_a_step() {
    let local = status(Session::Locked, Vault::Local);
    let missing = status(Session::Locked, Vault::Missing);
    let configured = status(
        Session::Locked,
        Vault::Configured {
            path: "/home/u/.local/share/urma/vault.urma".into(),
            unlock_file: Some("/home/u/.config/urma/unlock".into()),
        },
    );
    let active = status(unlocked(7), Vault::Local);
    assert_eq!(screen(&Identity::Reading, Chosen::ByStatus), Screen::Reading);
    assert_eq!(
        screen(&Identity::Unavailable("gone".into()), Chosen::Step(Step::Create)),
        Screen::Unavailable("gone".into())
    );
    assert_eq!(
        screen(&Identity::Known(missing.clone()), Chosen::ByStatus),
        Screen::Entry(Step::Welcome, missing.clone())
    );
    assert_eq!(
        screen(&Identity::Known(local.clone()), Chosen::ByStatus),
        Screen::Entry(Step::Unlock, local.clone())
    );
    assert_eq!(
        screen(&Identity::Known(configured.clone()), Chosen::ByStatus),
        Screen::Entry(Step::Unlock, configured)
    );
    assert_eq!(
        screen(&Identity::Known(local.clone()), Chosen::Step(Step::Recover)),
        Screen::Entry(Step::Recover, local)
    );
    assert_eq!(
        screen(&Identity::Known(active.clone()), Chosen::Step(Step::Import)),
        Screen::Active(active)
    );
}

#[test]
fn entry_steps_say_when_they_replace_the_stored_identity() {
    let local = status(Session::Locked, Vault::Local);
    let configured = status(
        Session::Locked,
        Vault::Configured {
            path: "/v/vault.urma".into(),
            unlock_file: Some("/v/unlock".into()),
        },
    );
    let missing = status(Session::Locked, Vault::Missing);
    assert_eq!(verb("Recover", &local), "Replace and recover");
    assert_eq!(verb("Create identity", &local), "Replace and create identity");
    assert_eq!(verb("Import", &configured), "Import");
    assert_eq!(verb("Recover", &missing), "Recover");
    assert!(matches!(
        replacing(&local),
        Said::Something(text) if text.contains("replaces the identity stored on this device")
    ));
    assert!(matches!(replacing(&configured), Said::Nothing));
    assert!(matches!(replacing(&missing), Said::Nothing));
}
