#![cfg(not(target_arch = "wasm32"))]

//! Public app-boundary checks in supervised test children. No daemon, RPC,
//! chain source, user state, DNS or external endpoint is used.
use std::{
    fs::File,
    io::{self, Read, Write},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tracing::{field::Visit, span, Event, Metadata, Subscriber};
use wire_app_core::{connection::SourceChoice, feed::SyncPhase};
use wire_app_engine::{
    config::{ArchiveSettings, Settings},
    native::service::Service,
};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const CHILD_MARKER: &str = "URMA_ARCHIVE_TOKEN_TEST_CHILD";

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

// Service exposes no worker join/cancel handle. A child contains its detached
// worker, including assertion unwind; the parent always kills/reaps on timeout.
fn supervised(name: &str, body: impl FnOnce()) {
    if std::env::var(CHILD_MARKER).as_deref() == Ok(name) {
        body();
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let log_path = directory.path().join("child.log");
    let output = File::create(&log_path).unwrap();
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env_clear()
        .env(CHILD_MARKER, name)
        .env("HOME", directory.path())
        .env("TMPDIR", directory.path())
        .stdin(Stdio::null())
        .stdout(output.try_clone().unwrap())
        .stderr(output)
        .spawn()
        .unwrap();
    let mut child = ChildGuard(child);
    let deadline = Instant::now() + Duration::from_secs(12);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "test child exceeded 12 seconds");
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "{}",
        std::fs::read_to_string(log_path).unwrap()
    );
}

fn private_file(path: &Path, bytes: &[u8]) {
    let mut options = File::options();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).unwrap().write_all(bytes).unwrap();
}

struct Mock {
    address: SocketAddr,
    stop: Option<mpsc::Sender<()>>,
    active: Arc<Mutex<Option<TcpStream>>>,
    worker: Option<JoinHandle<Vec<Vec<u8>>>>,
}

impl Mock {
    fn new(reflect_bearer: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (stop, stopped) = mpsc::channel();
        let active = Arc::new(Mutex::new(None));
        let socket = active.clone();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(8);
            let mut requests = Vec::new();
            loop {
                // Drain queued connections before acknowledging cancellation.
                while let Ok((mut stream, peer)) = listener.accept() {
                    assert!(peer.ip().is_loopback());
                    *socket.lock().unwrap() = Some(stream.try_clone().unwrap());
                    let mut request = Vec::new();
                    if reflect_bearer && requests.is_empty() {
                        let read_deadline = Instant::now() + Duration::from_secs(2);
                        while !request.ends_with(b"\r\n\r\n") && request.len() < 8192 {
                            if Instant::now() >= read_deadline || stopped.try_recv().is_ok() {
                                break;
                            }
                            stream
                                .set_read_timeout(Some(Duration::from_millis(100)))
                                .unwrap();
                            let mut byte = [0];
                            match stream.read(&mut byte) {
                                Ok(1) => request.push(byte[0]),
                                Err(error)
                                    if matches!(
                                        error.kind(),
                                        io::ErrorKind::TimedOut
                                            | io::ErrorKind::WouldBlock
                                            | io::ErrorKind::Interrupted
                                    ) =>
                                {
                                    ()
                                }
                                _ => break,
                            }
                        }
                        // Valid HTTP, invalid typed page: a naïvely propagated
                        // serde error would quote this reflected bearer string.
                        let body = serde_json::to_vec(&format!("Bearer {TOKEN}")).unwrap();
                        let mut response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes();
                        response.extend_from_slice(&body);
                        let write_deadline = Instant::now() + Duration::from_secs(1);
                        let mut pending = response.as_slice();
                        while !pending.is_empty() {
                            let Some(remaining) = write_deadline
                                .checked_duration_since(Instant::now())
                                .filter(|time| !time.is_zero())
                            else {
                                break;
                            };
                            stream.set_write_timeout(Some(remaining)).unwrap();
                            match stream.write(pending) {
                                Ok(0) => break,
                                Ok(count) => pending = &pending[count..],
                                Err(error) if error.kind() == io::ErrorKind::Interrupted => (),
                                Err(_) => break,
                            }
                        }
                    }
                    // Negative tests close unexpected connections immediately;
                    // they cannot strand the Service waiting for a reply.
                    let _ = stream.shutdown(Shutdown::Both);
                    socket.lock().unwrap().take();
                    requests.push(request);
                    if Instant::now() >= deadline {
                        return requests;
                    }
                }
                if Instant::now() >= deadline
                    || !matches!(
                        stopped.recv_timeout(Duration::from_millis(10)),
                        Err(mpsc::RecvTimeoutError::Timeout)
                    )
                {
                    // The caller observes worker completion before sending stop.
                    while let Ok((stream, _)) = listener.accept() {
                        let _ = stream.shutdown(Shutdown::Both);
                        requests.push(Vec::new());
                        if Instant::now() >= deadline {
                            break;
                        }
                    }
                    return requests;
                }
            }
        });
        Self {
            address,
            stop: Some(stop),
            active,
            worker: Some(worker),
        }
    }

    fn cleanup(&mut self) -> Option<Vec<Vec<u8>>> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(stream) = self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            let _ = stream.shutdown(Shutdown::Both);
        }
        if let Some(worker) = self.worker.take() {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !worker.is_finished() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
            if worker.is_finished() {
                return worker.join().ok();
            }
            return None;
        }
        Some(Vec::new())
    }

    fn finish(mut self) -> Vec<Vec<u8>> {
        self.cleanup()
            .expect("mock must stop and be reaped within two seconds")
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        if self.cleanup().is_none() {
            eprintln!("mock cleanup exceeded deadline");
        }
    }
}

fn rejected_by_public_service(directory: &Path, token_file: &Path, endpoint: SocketAddr) -> String {
    let cache_dir = directory.join("runtime");
    let app_dir = directory.join("app");
    let service = Service::open(
        &app_dir,
        Settings {
            archive: Some(ArchiveSettings {
                endpoint,
                token_file: token_file.into(),
            }),
            source: SourceChoice::LocalNode,
            cache_dir: cache_dir.clone(),
            ..Settings::default()
        },
    )
    .unwrap();
    assert_eq!(service.start_sync().unwrap().phase, SyncPhase::Connecting);
    let deadline = Instant::now() + Duration::from_secs(7);
    let reason = loop {
        match service.sync_snapshot().phase {
            SyncPhase::Failed(reason) => break reason,
            phase => {
                assert_ne!(phase, SyncPhase::Complete, "invalid archive was published");
                assert!(
                    Instant::now() < deadline,
                    "Service did not finish: {phase:?}"
                );
                thread::sleep(Duration::from_millis(10));
            }
        }
    };
    assert!(reason.contains("rpc_url and node_auth_file"), "{reason}");
    let feed = service.feed().unwrap();
    assert!(feed.posts.is_empty() && feed.identities.is_empty() && !feed.complete_to_tip);
    assert!(!cache_dir.exists());
    assert!(!app_dir.join("ltc-testnet-start.json").exists());
    assert!(!app_dir.join("ltc-testnet-index-v3.json").exists());
    reason
}

#[test]
fn token_file_refusals_happen_before_http() {
    supervised("token_file_refusals_happen_before_http", || {
        for case in ["missing", "directory", "oversize"] {
            let directory = tempfile::tempdir().unwrap();
            let token_file = directory.path().join("token");
            match case {
                "missing" => (),
                "directory" => std::fs::create_dir(&token_file).unwrap(),
                "oversize" => private_file(
                    &token_file,
                    format!("{TOKEN}{}", " ".repeat(193)).as_bytes(),
                ),
                _ => unreachable!(),
            }
            let mock = Mock::new(false);
            rejected_by_public_service(directory.path(), &token_file, mock.address);
            assert!(mock.finish().is_empty(), "{case} triggered HTTP");
        }
        #[cfg(unix)]
        for case in ["symlink", "permissions"] {
            use std::os::unix::fs::{symlink, PermissionsExt};
            let directory = tempfile::tempdir().unwrap();
            let token_file = directory.path().join("token");
            if case == "symlink" {
                let target = directory.path().join("target");
                private_file(&target, TOKEN.as_bytes());
                symlink(target, &token_file).unwrap();
            } else {
                private_file(&token_file, TOKEN.as_bytes());
                std::fs::set_permissions(&token_file, std::fs::Permissions::from_mode(0o644))
                    .unwrap();
            }
            let mock = Mock::new(false);
            rejected_by_public_service(directory.path(), &token_file, mock.address);
            assert!(mock.finish().is_empty(), "{case} triggered HTTP");
        }
    });
}

#[test]
fn invalid_token_and_nonloopback_endpoint_do_not_connect() {
    supervised(
        "invalid_token_and_nonloopback_endpoint_do_not_connect",
        || {
            for bytes in [
                vec![0xff; 64],
                b"short".to_vec(),
                vec![b'g'; 64],
                format!("\"{TOKEN}\"").into_bytes(),
            ] {
                let directory = tempfile::tempdir().unwrap();
                let token_file = directory.path().join("token");
                private_file(&token_file, &bytes);
                let mock = Mock::new(false);
                rejected_by_public_service(directory.path(), &token_file, mock.address);
                assert!(mock.finish().is_empty(), "malformed token triggered HTTP");
            }
            let directory = tempfile::tempdir().unwrap();
            let token_file = directory.path().join("token");
            private_file(&token_file, TOKEN.as_bytes());
            let mock = Mock::new(false);
            // Unspecified is nonloopback; no external address is targeted even if
            // validation regresses. The only bound listener is owned loopback.
            let endpoint = SocketAddr::from((Ipv4Addr::UNSPECIFIED, mock.address.port()));
            rejected_by_public_service(directory.path(), &token_file, endpoint);
            assert!(
                mock.finish().is_empty(),
                "nonloopback endpoint triggered HTTP"
            );
        },
    );
}

#[derive(Clone)]
struct Capture(Arc<Mutex<Vec<String>>>);

struct Fields(String);
impl Visit for Fields {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        use std::fmt::Write as _;
        write!(&mut self.0, "{}={value:?};", field.name()).unwrap();
    }
}

impl Subscriber for Capture {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
        span::Id::from_u64(1)
    }
    fn record(&self, _: &span::Id, _: &span::Record<'_>) {}
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, event: &Event<'_>) {
        let mut fields = Fields(String::new());
        event.record(&mut fields);
        self.0.lock().unwrap().push(fields.0);
    }
    fn enter(&self, _: &span::Id) {}
    fn exit(&self, _: &span::Id) {}
}

#[test]
fn reflected_bearer_is_not_published_in_app_errors_or_worker_logs() {
    supervised(
        "reflected_bearer_is_not_published_in_app_errors_or_worker_logs",
        || {
            let events = Arc::new(Mutex::new(Vec::new()));
            // Scoped thread-local dispatch would miss Service's spawned worker.
            // This global collector exists only in this isolated test process.
            tracing::subscriber::set_global_default(Capture(events.clone())).unwrap();
            let directory = tempfile::tempdir().unwrap();
            let token_file = directory.path().join("token");
            let token = format!("{TOKEN}{}", " ".repeat(192));
            assert_eq!(token.len(), 256);
            private_file(&token_file, token.as_bytes());
            let mock = Mock::new(true);
            let reason = rejected_by_public_service(directory.path(), &token_file, mock.address);
            let requests = mock.finish();
            assert_eq!(
                requests.len(),
                1,
                "app must make one request, then fall back"
            );
            let request = std::str::from_utf8(&requests[0]).unwrap();
            assert!(request.starts_with("GET /v1/wire?limit=128 HTTP/1.1\r\n"));
            assert!(request.contains(&format!("\r\nAuthorization: Bearer {TOKEN}\r\n")));
            assert!(request.ends_with("\r\n\r\n"));
            let events = events.lock().unwrap();
            assert!(
                events
                    .iter()
                    .any(|event| event.contains("operator archive view refused")
                        && event.contains("Optional URMA archive: feed")),
                "{events:?}"
            );
            assert!(!reason.contains(TOKEN));
            assert!(
                events
                    .iter()
                    .all(|event| !event.contains(TOKEN) && !event.contains("Authorization")),
                "{events:?}"
            );
        },
    );
}
