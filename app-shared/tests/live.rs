#[path = "support/node.rs"]
mod node;
use urma_identity::{keyring::Keyring, phrase::IdentityPhrase, vault::UnlockedVault};
use wire_app_shared::{config::Settings, live::Service, live_index};

#[test]
fn exact_approval_restart_and_confirmation_use_saved_bytes() {
    publication_roundtrip("post", false);
}

#[test]
fn structured_metadata_survives_exact_approval_restart_and_confirmation() {
    publication_roundtrip("post", true);
}

#[test]
fn profile_and_avatar_use_exact_approval_restart_and_confirmation() {
    publication_roundtrip("profile", false);
    publication_roundtrip("avatar", false);
}

fn publication_roundtrip(kind: &str, structured: bool) {
    let dir = tempfile::tempdir().unwrap();
    let phrase = IdentityPhrase::parse(&format!("{}art", "abandon ".repeat(23))).unwrap();
    let keys = Keyring::create(&phrase, "public test fixture").unwrap();
    let mock = node::Mock::new(dir.path(), keys.active().unwrap());
    let vault =
        UnlockedVault::create(keys, "public-fixture-password", &mut rand::rngs::OsRng).unwrap();
    let vault_path = dir.path().join("fixture.vault");
    std::fs::write(
        &vault_path,
        vault.seal(&mut rand::rngs::OsRng).unwrap().as_bytes(),
    )
    .unwrap();
    let settings = || Settings {
        rpc_url: Some(mock.config.rpc_url.clone()),
        node_auth_file: Some(mock.config.cookie_file.clone()),
        vault: Some(vault_path.clone()),
        unlock_file: None,
    };
    let app = dir.path().join("app");
    let mut service = Service::open(&app, settings()).unwrap();
    assert!(Service::open(&app, settings()).is_err());
    assert!(service
        .prepare("hello".into(), None)
        .unwrap_err()
        .contains("Unlock"));
    service
        .unlock("public-fixture-password".into(), false)
        .unwrap();
    let content = if kind == "avatar" {
        ""
    } else if kind == "profile" {
        "  昵称\0e\u{301}\r\n"
    } else {
        "hello"
    };
    let quote = if kind != "post" {
        service
            .prepare_record(
                kind,
                content.into(),
                None,
                String::new(),
                vec![],
                (kind == "avatar").then(|| vec![0x12; 512]),
            )
            .unwrap()
    } else if structured {
        service
            .prepare_topics(
                "hello".into(),
                None,
                "rosint".into(),
                vec!["investigation".into()],
            )
            .unwrap()
    } else {
        service.prepare("hello".into(), None).unwrap()
    };
    assert_eq!(quote.wire, if structured { "rosint" } else { "" });
    assert_eq!(quote.hashtags.len(), usize::from(structured));
    assert_eq!(quote.text, content);
    assert_eq!(quote.kind, kind);
    assert_eq!(quote.avatar, (kind == "avatar").then(|| vec![0x12; 512]));
    assert_eq!(quote.selected_funding - quote.returned, quote.fee);
    assert!(!quote.approved);
    assert!(mock.state.lock().unwrap().submissions.is_empty());
    assert!(service.advance("wrong plan", true).is_err());
    assert!(service.advance(&quote.id, false).is_err());
    assert!(mock.state.lock().unwrap().submissions.is_empty());
    let first = service.advance(&quote.id, true).unwrap();
    assert_eq!((first.confirmed, first.mempool), (0, 2));
    assert!(!first.complete);
    assert!(service.prepare("another".into(), None).is_err());
    drop(service);
    let mut service = Service::open(&app, settings()).unwrap();
    assert!(service.status().unwrap().unwrap().approved);
    let resumed = service.status().unwrap().unwrap();
    assert_eq!(resumed.kind, quote.kind);
    assert_eq!(resumed.text, quote.text);
    assert_eq!(resumed.avatar, quote.avatar);
    assert_eq!(
        (resumed.wire, resumed.hashtags),
        (quote.wire.clone(), quote.hashtags.clone())
    );
    service.advance(&quote.id, false).unwrap();
    assert_eq!(mock.state.lock().unwrap().submissions.len(), 2);
    mock.confirm_all();
    let second = service.advance(&quote.id, false).unwrap();
    assert_eq!((second.confirmed, second.mempool), (2, 0));
    assert!(second.complete);
    let raws = mock.state.lock().unwrap().submissions.clone();
    mock.state.lock().unwrap().unavailable = true;
    assert!(service.status().is_err());
    assert_eq!(mock.state.lock().unwrap().submissions.len(), 2);
    mock.state.lock().unwrap().unavailable = false;
    mock.state.lock().unwrap().transactions.remove(&quote.txid);
    assert!(!service.status().unwrap().unwrap().complete);
    assert_eq!(mock.state.lock().unwrap().submissions.len(), 2);
    mock.state.lock().unwrap().preflight = Some("mempool full".into());
    assert!(service
        .advance(&quote.id, false)
        .unwrap_err()
        .contains("paused"));
    mock.state.lock().unwrap().preflight = None;
    service.advance(&quote.id, false).unwrap();
    assert_eq!(mock.state.lock().unwrap().submissions.last(), raws.last());
    mock.confirm_all();
    service
        .unlock("public-fixture-password".into(), false)
        .unwrap();
    let reply = if structured {
        service
            .prepare_topics(
                "reply".into(),
                Some(quote.txid.clone()),
                "another-wire".into(),
                vec!["investigation".into()],
            )
            .unwrap()
    } else {
        service
            .prepare("reply".into(), Some(quote.txid.clone()))
            .unwrap()
    };
    assert_eq!(reply.wire, if structured { "another-wire" } else { "" });
    assert_eq!(reply.target, Some(quote.txid));
    assert_eq!(reply.text, "reply");
}

#[test]
fn confirmed_index_projects_posts_and_actual_replies() {
    use urma_core::format::PublicRecord;
    use urma_wire::index::{Checkpoint, Entry, Index};
    let parent = "01".repeat(32);
    let child = "02".repeat(32);
    let author = "03".repeat(32);
    let index = Index {
        format: "URMA-WIRE-INDEX-1".into(),
        genesis: "04".repeat(32),
        start: 100,
        blocks: vec![Checkpoint {
            height: 100,
            hash: "05".repeat(32),
        }],
        entries: vec![
            Entry {
                height: 100,
                position: 0,
                txid: parent.clone(),
                author: author.clone(),
                record: hex::encode(PublicRecord::Post("exact text\n".into()).encode().unwrap()),
            },
            Entry {
                height: 100,
                position: 1,
                txid: child.clone(),
                author,
                record: hex::encode(
                    PublicRecord::Reply {
                        target: parent.parse().unwrap(),
                        text: "reply".into(),
                    }
                    .encode()
                    .unwrap(),
                ),
            },
        ],
    };
    let feed = live_index::project(&index, true).unwrap();
    assert_eq!(feed.posts.len(), 2);
    assert_eq!(feed.posts[1].text, "exact text\n");
    assert_eq!(feed.posts[1].reply_count, 1);
    assert_eq!(
        live_index::replies(&index, &parent).unwrap()[0]
            .txid
            .to_display_hex(),
        child
    );
    assert!(live_index::replies(&index, &"ff".repeat(32))
        .unwrap()
        .is_empty());
}
