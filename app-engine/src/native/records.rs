use crate::config::{
    retry_delay_secs, ArchiveSettings, FeedRoute, Settings, HISTORY_BLOCKS, NETWORK,
    START_FILE_BYTES, SYNC_BLOCKS, SYNC_RETRY_ATTEMPTS,
};
use crate::errors::NativeError;
use crate::native::archive::{self, OperatorState, OperatorView};
use crate::native::reader::NodeReader;
use crate::native::service::{locked_link, locked_sync, opened, Link, Service};
use crate::projection::{lookup, project, replies};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use urma_runtime::node::Node;
use urma_runtime::transport::Provider;
use urma_wire::index::{self, Index};
use wire_app_core::connection::{NodeHealth, SourceChoice};
use wire_app_core::feed::{Feed, FeedSource, Lookup, SyncPhase, SyncStatus};
use wire_app_core::model::{Placement, Post, Txid};

enum Held {
    Operator(Feed),
    Autonomous,
}

enum Scan {
    Started(SyncWorker),
    Running,
}

fn locked_view(view: &Mutex<OperatorState>) -> MutexGuard<'_, OperatorState> {
    match view.lock() {
        Ok(view) => view,
        Err(poisoned) => {
            tracing::error!("operator view lock poisoned; reusing the held view");
            poisoned.into_inner()
        }
    }
}

fn held(view: &Mutex<OperatorState>) -> bool {
    matches!(*locked_view(view), OperatorState::Held(_))
}

fn same_state(seen: &NodeHealth, now: &NodeHealth) -> bool {
    match (seen, now) {
        (NodeHealth::Serving { .. }, NodeHealth::Serving { .. }) => true,
        (NodeHealth::Behind { .. }, NodeHealth::Behind { .. }) => true,
        (NodeHealth::Unconfigured, NodeHealth::Unconfigured) => true,
        (
            NodeHealth::Unavailable { reason: was, .. },
            NodeHealth::Unavailable { reason: is, .. },
        ) => was == is,
        _changed => false,
    }
}

struct ArchiveWorker {
    configured: ArchiveSettings,
    settings: Settings,
    status: Arc<Mutex<SyncStatus>>,
    view: Arc<Mutex<OperatorState>>,
    link: Arc<Mutex<Link>>,
    directory: PathBuf,
    providers: Vec<Box<dyn Provider>>,
    reading: Arc<AtomicBool>,
    scanning: Arc<AtomicBool>,
}

impl ArchiveWorker {
    fn run(self) {
        match archive::read(&self.configured) {
            Ok(view) => {
                self.publish(view);
                self.reading.store(false, Ordering::Release);
            }
            Err(error) => {
                tracing::warn!(error = %error, "operator archive view refused; using autonomous history sync");
                self.refused();
            }
        }
    }

    fn refused(self) {
        let status = self.status.clone();
        let reading = self.reading.clone();
        {
            let mut refused = locked_sync(&status);
            refused.complete_to_tip = false;
            refused.source = FeedSource::AutonomousLocal;
            *locked_view(&self.view) = OperatorState::Absent;
        }
        let started = self.fallback();
        reading.store(false, Ordering::Release);
        match started {
            Ok(Scan::Started(worker)) => worker.run(),
            Ok(Scan::Running) => tracing::info!("autonomous history sync already running"),
            Err(error) => {
                tracing::warn!(error = %error, "autonomous history sync could not start");
                locked_sync(&status).phase = SyncPhase::Failed(error.to_string());
            }
        }
    }

    fn publish(&self, view: OperatorView) {
        let mut status = locked_sync(&self.status);
        status.start = view.feed.start;
        status.from = view.feed.start;
        status.tip = view.feed.tip;
        status.indexed_to = view.archive.indexed_through.map(|point| point.height);
        status.complete_to_tip = view.feed.complete_to_tip;
        status.done = view.archive.scanned_blocks;
        status.total = view.feed.tip - view.feed.start + 1;
        status.source = view.feed.source.clone();
        *locked_view(&self.view) = OperatorState::Held(view);
        status.phase = SyncPhase::Complete;
    }

    fn fallback(self) -> Result<Scan, NativeError> {
        if self.scanning.swap(true, Ordering::AcqRel) {
            return Ok(Scan::Running);
        }
        let scanning = self.scanning.clone();
        match self.scanner() {
            Ok(worker) => Ok(Scan::Started(worker)),
            Err(error) => {
                scanning.store(false, Ordering::Release);
                Err(error)
            }
        }
    }

    fn scanner(self) -> Result<SyncWorker, NativeError> {
        let node = {
            let mut link = locked_link(&self.link);
            match &*link {
                Link::Open(node) => node.clone(),
                Link::Closed => {
                    let node = Arc::new(if self.settings.source == SourceChoice::Public {
                        Node::public_in_with_providers(
                            NETWORK,
                            &self.settings.cache_dir,
                            self.providers,
                        )?
                    } else {
                        self.settings.connect()?
                    });
                    opened(&node);
                    *link = Link::Open(node.clone());
                    node
                }
            }
        };
        let start = start_at(&self.directory.join(Service::START_FILE), &node)?;
        let mut status = locked_sync(&self.status);
        status.start = start;
        status.from = start;
        Ok(SyncWorker {
            node,
            path: self.directory.join(Service::INDEX_FILE),
            start,
            status: self.status.clone(),
            view: self.view.clone(),
            scanning: self.scanning.clone(),
        })
    }
}

impl Service {
    pub(super) fn start(&self, node: &Node) -> Result<u64, NativeError> {
        start_at(&self.directory.join(Self::START_FILE), node)
    }

    pub fn start_sync(&self) -> Result<SyncStatus, NativeError> {
        let status = self.sync_status();
        let settings = self.settings().clone();
        match settings.route() {
            FeedRoute::Operator(configured) => {
                if self.reading.swap(true, Ordering::AcqRel) {
                    return Ok(status.clone());
                }
                self.start_archive(status, configured, settings)
            }
            FeedRoute::Autonomous => {
                if !status.settled() {
                    return Ok(status.clone());
                }
                self.start_autonomous(status)
            }
        }
    }

    fn start_archive(
        &self,
        mut status: MutexGuard<'_, SyncStatus>,
        configured: ArchiveSettings,
        settings: Settings,
    ) -> Result<SyncStatus, NativeError> {
        status.run += 1;
        status.phase = SyncPhase::Connecting;
        status.complete_to_tip = false;
        status.done = 0;
        status.total = 0;
        let snapshot = status.clone();
        drop(status);
        match self.spawn_archive(configured, settings) {
            Ok(()) => Ok(snapshot),
            Err(error) => {
                tracing::warn!(error = %error, "operator archive worker did not start");
                self.sync_status().phase = SyncPhase::Failed(error.to_string());
                Err(error)
            }
        }
    }

    fn spawn_archive(
        &self,
        configured: ArchiveSettings,
        settings: Settings,
    ) -> Result<(), NativeError> {
        let worker = ArchiveWorker {
            configured,
            settings,
            status: self.sync.clone(),
            view: self.archive_feed.clone(),
            link: self.link.clone(),
            directory: self.directory.clone(),
            providers: self.local_node.providers(),
            reading: self.reading.clone(),
            scanning: self.scanning.clone(),
        };
        let spawned = std::thread::Builder::new()
            .name("wire-archive".into())
            .spawn(move || worker.run());
        match spawned {
            Ok(_handle) => Ok(()),
            Err(error) => {
                self.reading.store(false, Ordering::Release);
                Err(error.into())
            }
        }
    }

    fn start_autonomous(
        &self,
        status: MutexGuard<'_, SyncStatus>,
    ) -> Result<SyncStatus, NativeError> {
        if self.scanning.swap(true, Ordering::AcqRel) {
            return Ok(status.clone());
        }
        match self.spawn_autonomous(status) {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => {
                self.scanning.store(false, Ordering::Release);
                Err(error)
            }
        }
    }

    fn spawn_autonomous(
        &self,
        mut status: MutexGuard<'_, SyncStatus>,
    ) -> Result<SyncStatus, NativeError> {
        *locked_view(&self.archive_feed) = OperatorState::Absent;
        let node = self.node()?;
        let start = self.start(&node)?;
        *status = SyncStatus {
            run: status.run + 1,
            phase: SyncPhase::Connecting,
            start,
            from: start,
            indexed_to: status.indexed_to,
            stored_to: status.stored_to,
            tip: status.tip,
            done: 0,
            total: 0,
            complete_to_tip: false,
            source: FeedSource::AutonomousLocal,
        };
        let snapshot = status.clone();
        drop(status);
        let worker = SyncWorker {
            node,
            path: self.index_path(),
            start,
            status: self.sync.clone(),
            view: self.archive_feed.clone(),
            scanning: self.scanning.clone(),
        };
        std::thread::Builder::new()
            .name("wire-sync".into())
            .spawn(move || worker.run())?;
        Ok(snapshot)
    }

    pub fn watch_node(&self) -> NodeHealth {
        let Some(target) = self.settings().archive.clone() else {
            return NodeHealth::Unconfigured;
        };
        let health = match archive::probe(&target) {
            Ok(health) => health,
            Err(error) => {
                let health = NodeHealth::Unavailable {
                    endpoint: target.endpoint.to_string(),
                    reason: error.to_string(),
                };
                if self.changed(&health) {
                    tracing::warn!(error = %error, "URMA Node unavailable; the feed scans the chain");
                }
                health
            }
        };
        match &health {
            NodeHealth::Serving {
                generation,
                indexed_through,
                ..
            } => self.follow_node(target, *generation, *indexed_through),
            NodeHealth::Behind { .. }
            | NodeHealth::Unavailable { .. }
            | NodeHealth::Unconfigured => self.leave_node(),
        }
        health
    }

    fn changed(&self, health: &NodeHealth) -> bool {
        let mut seen = match self.node_seen.lock() {
            Ok(seen) => seen,
            Err(poisoned) => {
                tracing::error!("node health lock poisoned; reusing the held health");
                poisoned.into_inner()
            }
        };
        let changed = !same_state(&seen, health);
        *seen = health.clone();
        changed
    }

    fn follow_node(&self, target: ArchiveSettings, generation: u64, indexed: u64) {
        if self.changed(&NodeHealth::Serving {
            endpoint: String::new(),
            indexed_through: indexed,
            tip: indexed,
            generation,
            chain_peers: 0,
        }) {
            tracing::info!(indexed, generation, "URMA Node serves the feed");
        }
        let current = match &*locked_view(&self.archive_feed) {
            OperatorState::Held(view) => view.current(generation, indexed),
            OperatorState::Absent => false,
        };
        if current || self.reading.swap(true, Ordering::AcqRel) {
            return;
        }
        let settings = self.settings().clone();
        match self.spawn_archive(target, settings) {
            Ok(()) => (),
            Err(error) => tracing::warn!(error = %error, "operator archive refresh did not start"),
        }
    }

    fn leave_node(&self) {
        let mut status = self.sync_status();
        let mut held = locked_view(&self.archive_feed);
        let OperatorState::Held(_view) = &*held else {
            return;
        };
        tracing::warn!(
            "URMA Node stopped serving; its view is dropped and the feed scans the chain"
        );
        *held = OperatorState::Absent;
        status.complete_to_tip = false;
        status.source = FeedSource::AutonomousLocal;
        if status.phase == SyncPhase::Complete {
            status.phase = SyncPhase::Idle;
        }
    }

    fn expire_operator(&self, status: &mut SyncStatus) {
        let mut held = locked_view(&self.archive_feed);
        let stale = match &*held {
            OperatorState::Held(view) => !view.fresh(),
            OperatorState::Absent => return,
        };
        if stale {
            tracing::warn!("operator archive view expired; serving the autonomous index");
            *held = OperatorState::Absent;
            status.complete_to_tip = false;
            status.source = FeedSource::AutonomousLocal;
            if status.phase == SyncPhase::Complete {
                status.phase = SyncPhase::Idle;
            }
        }
    }

    fn operator_feed(&self, status: &mut SyncStatus) -> Held {
        self.expire_operator(status);
        match &*locked_view(&self.archive_feed) {
            OperatorState::Held(view) => Held::Operator(view.feed.clone()),
            OperatorState::Absent => Held::Autonomous,
        }
    }

    pub fn sync_snapshot(&self) -> SyncStatus {
        let mut status = self.sync_status();
        self.expire_operator(&mut status);
        status.clone()
    }

    pub fn feed(&self) -> Result<Feed, NativeError> {
        let status = {
            let mut status = self.sync_status();
            match self.operator_feed(&mut status) {
                Held::Operator(mut feed) => {
                    feed.complete_to_tip &= status.complete_to_tip;
                    return Ok(feed);
                }
                Held::Autonomous => status.clone(),
            }
        };
        let index = match self.index_path().exists() {
            true => Index::load(&self.index_path())?,
            false => Index {
                format: Index::FORMAT.into(),
                genesis: NETWORK.genesis()?.0.to_string(),
                start: status.start,
                blocks: Vec::new(),
                entries: Vec::new(),
            },
        };
        Ok(project(&index, status.complete_to_tip)?)
    }

    fn held_feed(&self) -> Held {
        let mut status = self.sync_status();
        self.operator_feed(&mut status)
    }

    pub fn post(&self, txid: Txid) -> Result<Lookup, NativeError> {
        let feed = match self.held_feed() {
            Held::Operator(feed) => feed,
            Held::Autonomous => return self.indexed_post(txid),
        };
        let Some(post) = feed.posts.into_iter().find(|post| post.txid == txid) else {
            return Ok(Lookup::Absent);
        };
        Ok(Lookup::Found(post))
    }

    fn indexed_post(&self, txid: Txid) -> Result<Lookup, NativeError> {
        if !self.index_path().exists() {
            return Ok(Lookup::Absent);
        }
        Ok(lookup(&Index::load(&self.index_path())?, txid)?)
    }

    pub fn replies(&self, txid: Txid) -> Result<Vec<Post>, NativeError> {
        let feed = match self.held_feed() {
            Held::Operator(feed) => feed,
            Held::Autonomous => return self.indexed_replies(txid),
        };
        Ok(feed
            .posts
            .into_iter()
            .filter(|post| post.placement == Placement::Reply(txid))
            .collect())
    }

    fn indexed_replies(&self, txid: Txid) -> Result<Vec<Post>, NativeError> {
        if !self.index_path().exists() {
            return Ok(Vec::new());
        }
        Ok(replies(&Index::load(&self.index_path())?, txid)?)
    }
}

fn start_at(path: &Path, node: &Node) -> Result<u64, NativeError> {
    if path.try_exists()? {
        return Ok(serde_json::from_slice(&urma_io::read_bounded(
            path,
            START_FILE_BYTES,
        )?)?);
    }
    let tip = node.tip_height()?;
    let start = tip.max(HISTORY_BLOCKS) - HISTORY_BLOCKS;
    urma_io::write_new(path, &serde_json::to_vec(&start)?)?;
    Ok(start)
}

struct SyncWorker {
    node: Arc<Node>,
    path: PathBuf,
    start: u64,
    status: Arc<Mutex<SyncStatus>>,
    view: Arc<Mutex<OperatorState>>,
    scanning: Arc<AtomicBool>,
}

enum Batch {
    Done,
    More,
}

impl SyncWorker {
    fn run(self) {
        self.scan();
        self.scanning.store(false, Ordering::Release);
    }

    fn scan(&self) {
        let mut attempt: u32 = 0;
        loop {
            if held(&self.view) {
                tracing::info!("URMA Node view adopted; autonomous history sync stops");
                return;
            }
            match self.batch() {
                Ok(Batch::Done) => return,
                Ok(Batch::More) => attempt = 0,
                Err(error) => {
                    attempt += 1;
                    if attempt > SYNC_RETRY_ATTEMPTS {
                        tracing::warn!(error = %error, attempt, "history sync gave up");
                        self.settle(SyncPhase::Failed(format!(
                            "Sync gave up after {SYNC_RETRY_ATTEMPTS} retries: {error}"
                        )));
                        return;
                    }
                    tracing::warn!(error = %error, attempt, "history sync batch failed; retrying");
                    self.wait(attempt, error.to_string());
                }
            }
        }
    }

    fn settle(&self, phase: SyncPhase) {
        let mut status = locked_sync(&self.status);
        if !held(&self.view) {
            status.phase = phase;
        }
    }

    fn batch(&self) -> Result<Batch, NativeError> {
        let reader = NodeReader(&self.node);
        let (report, index) = index::sync(
            &reader,
            &self.path,
            self.start,
            SYNC_BLOCKS,
            |height, tip| {
                let mut status = locked_sync(&self.status);
                if held(&self.view) {
                    return;
                }
                status.from = match status.phase {
                    SyncPhase::Verifying => status.from,
                    SyncPhase::Idle
                    | SyncPhase::Connecting
                    | SyncPhase::Retrying { .. }
                    | SyncPhase::Complete
                    | SyncPhase::Failed(_) => height,
                };
                status.phase = SyncPhase::Verifying;
                status.tip = tip;
                status.indexed_to = Some(height);
                status.done = height - status.from + 1;
                status.total = tip.max(height) - status.from + 1;
            },
        )?;
        let mut status = locked_sync(&self.status);
        if held(&self.view) {
            return Ok(Batch::Done);
        }
        status.tip = report.tip;
        status.indexed_to = index.blocks.last().map(|block| block.height);
        status.stored_to = status.indexed_to;
        status.complete_to_tip = report.complete_to_tip;
        if report.complete_to_tip || report.scanned == 0 {
            status.done = status.total;
            status.phase = SyncPhase::Complete;
            tracing::info!(
                tip = report.tip,
                records = report.records,
                "history sync complete"
            );
            return Ok(Batch::Done);
        }
        Ok(Batch::More)
    }

    fn wait(&self, attempt: u32, reason: String) {
        for seconds in (1..=retry_delay_secs(attempt)).rev() {
            if held(&self.view) {
                return;
            }
            self.settle(SyncPhase::Retrying {
                attempt,
                seconds,
                reason: reason.clone(),
            });
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}
