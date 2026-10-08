#[path = "support/node.rs"]
mod node;
use urma_identity::{keyring::Keyring, phrase::IdentityPhrase, vault::UnlockedVault};
use wire_app_core::connection::SourceChoice;
use wire_app_core::feed::Lookup;
use wire_app_core::model::{Placement, Txid};
use wire_app_core::publication::{Addressed, Content, Pending, Publication};
use wire_app_core::session::Credential;
use wire_app_engine::config::Settings;
use wire_app_engine::native::service::Service;
use wire_app_engine::projection;

const PASSWORD: &str = "public-fixture-password";

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

fn post(text: &str, structured: bool, wire: &str, addressed: Addressed) -> Content {
    Content::Post {
        text: text.into(),
        wire: if structured { wire } else { "" }.into(),
        hashtags: match structured {
            true => vec!["investigation".into()],
            false => vec![],
        },
        addressed,
    }
}

fn content(kind: &str, structured: bool) -> Content {
    match kind {
        "avatar" => Content::Avatar(vec![0x12; 128]),
        "profile" => Content::Nickname("  昵称\0e\u{301}\r\n".into()),
        _ => post("hello", structured, "rosint", Addressed::Standalone),
    }
}

fn awaiting(pending: Pending) -> Publication {
    match pending {
        Pending::Awaiting(publication) => publication,
        Pending::Settled => panic!("expected an awaiting publication"),
    }
}

fn publication_roundtrip(kind: &str, structured: bool) {
    let dir = tempfile::tempdir().unwrap();
    let phrase = IdentityPhrase::parse(&format!("{}art", "abandon ".repeat(23))).unwrap();
    let keys = Keyring::create(&phrase, "public test fixture").unwrap();
    let mock = node::Mock::new(dir.path(), keys.active().unwrap());
    let vault = UnlockedVault::create(keys, PASSWORD, &mut rand::rngs::OsRng).unwrap();
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
        source: SourceChoice::LocalNode,
        cache_dir: dir.path().join("app/runtime"),
    };
    let app = dir.path().join("app");
    let mut service = Service::open(&app, settings()).unwrap();
    assert!(Service::open(&app, settings()).is_err());
    let hello = post("hello", false, "", Addressed::Standalone);
    assert!(service
        .quote(&hello)
        .unwrap_err()
        .to_string()
        .contains("Unlock"));
    service
        .unlock(Credential::Password(PASSWORD.into()))
        .unwrap();
    let expected = content(kind, structured);
    let mut quote = service.quote(&expected).unwrap();
    if kind == "post" && !structured {
        service.discard_quote(&quote.id).unwrap();
        assert!(matches!(service.status().unwrap(), Pending::Settled));
        assert!(
            service.advance(&quote.id, true).is_err(),
            "an edited draft cannot approve its discarded review"
        );
        quote = service.quote(&expected).unwrap();
    }
    assert_eq!(quote.content, expected);
    assert_eq!(quote.selected_funding - quote.returned, quote.fee);
    assert!(!quote.approved);
    assert!(mock.state.lock().unwrap().submissions.is_empty());
    assert!(service.advance("wrong plan", true).is_err());
    assert!(service.advance(&quote.id, false).is_err());
    assert!(mock.state.lock().unwrap().submissions.is_empty());
    let first = service.advance(&quote.id, true).unwrap();
    if kind == "post" && !structured {
        assert!(
            service.discard_quote(&quote.id).is_err(),
            "an approved plan cannot be discarded by a draft edit"
        );
    }
    assert_eq!((first.confirmed, first.mempool), (0, 2));
    assert!(!first.complete);
    assert!(service
        .quote(&post("another", false, "", Addressed::Standalone))
        .is_err());
    drop(service);
    let mut service = Service::open(&app, settings()).unwrap();
    assert!(awaiting(service.status().unwrap()).approved);
    let resumed = awaiting(service.status().unwrap());
    assert_eq!(resumed.content, quote.content);
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
    assert!(!awaiting(service.status().unwrap()).complete);
    assert_eq!(mock.state.lock().unwrap().submissions.len(), 2);
    mock.state.lock().unwrap().preflight = Some("mempool full".into());
    assert!(service
        .advance(&quote.id, false)
        .unwrap_err()
        .to_string()
        .contains("paused"));
    mock.state.lock().unwrap().preflight = None;
    service.advance(&quote.id, false).unwrap();
    assert_eq!(mock.state.lock().unwrap().submissions.last(), raws.last());
    mock.confirm_all();
    service
        .unlock(Credential::Password(PASSWORD.into()))
        .unwrap();
    let target = Txid::from_display_hex(&quote.txid).unwrap();
    let answer = post(
        "reply",
        structured,
        "another-wire",
        Addressed::Thread(target),
    );
    let reply = service.quote(&answer).unwrap();
    assert_eq!(reply.content, answer);
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
    let parent_id = Txid::from_display_hex(&parent).unwrap();
    let child_id = Txid::from_display_hex(&child).unwrap();
    let feed = projection::project(&index, true).unwrap();
    assert_eq!(feed.posts.len(), 2);
    assert_eq!(feed.posts[1].text, "exact text\n");
    assert_eq!(feed.posts[1].reply_count, 1);
    assert_eq!(feed.posts[1].placement, Placement::Root);
    assert_eq!(feed.posts[0].placement, Placement::Reply(parent_id));
    assert_eq!(
        projection::replies(&index, parent_id).unwrap()[0]
            .txid
            .to_display_hex(),
        child
    );
    let stranger = Txid::from_display_hex(&"ff".repeat(32)).unwrap();
    assert!(projection::replies(&index, stranger).unwrap().is_empty());
    assert!(matches!(
        projection::lookup(&index, child_id).unwrap(),
        Lookup::Found(found) if found.text == "reply"
    ));
    assert!(matches!(
        projection::lookup(&index, stranger).unwrap(),
        Lookup::Absent
    ));
}
