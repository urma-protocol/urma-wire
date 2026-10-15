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
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use urma_runtime::node::Node;
use urma_runtime::transport::Provider;
use urma_wire::index::{self, Index};
use wire_app_core::connection::SourceChoice;
use wire_app_core::feed::{Feed, FeedSource, Lookup, SyncPhase, SyncStatus};
use wire_app_core::model::{Placement, Post, Txid};

enum Held {
    Operator(Feed),
    Autonomous,
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

struct ArchiveWorker {
    configured: ArchiveSettings,
    settings: Settings,
    status: Arc<Mutex<SyncStatus>>,
    view: Arc<Mutex<OperatorState>>,
    link: Arc<Mutex<Link>>,
    directory: PathBuf,
    providers: Vec<Box<dyn Provider>>,
}

impl ArchiveWorker {
    fn run(self) {
        match archive::read(&self.configured) {
            Ok(view) => self.publish(view),
            Err(error) => {
                tracing::warn!(error = %error, "operator archive view refused; using autonomous history sync");
                let status = self.status.clone();
                {
                    let mut refused = locked_sync(&status);
                    refused.complete_to_tip = false;
                    refused.source = FeedSource::AutonomousLocal;
                    *locked_view(&self.view) = OperatorState::Absent;
                }
                match self.fallback() {
                    Ok(worker) => worker.run(),
                    Err(error) => {
                        tracing::warn!(error = %error, "autonomous history sync could not start");
                        locked_sync(&status).phase = SyncPhase::Failed(error.to_string());
                    }
                }
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

    fn fallback(self) -> Result<SyncWorker, NativeError> {
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
        })
    }
}

impl Service {
    pub(super) fn start(&self, node: &Node) -> Result<u64, NativeError> {
        start_at(&self.directory.join(Self::START_FILE), node)
    }

    pub fn start_sync(&self) -> Result<SyncStatus, NativeError> {
        let status = self.sync_status();
        if !status.settled() {
            return Ok(status.clone());
        }
        let settings = self.settings().clone();
        match settings.route() {
            FeedRoute::Operator(configured) => self.start_archive(status, configured, settings),
            FeedRoute::Autonomous => self.start_autonomous(status),
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
        let worker = ArchiveWorker {
            configured,
            settings,
            status: self.sync.clone(),
            view: self.archive_feed.clone(),
            link: self.link.clone(),
            directory: self.directory.clone(),
            providers: self.local_node.providers(),
        };
        let spawned = std::thread::Builder::new()
            .name("wire-archive".into())
            .spawn(move || worker.run());
        match spawned {
            Ok(_handle) => Ok(snapshot),
            Err(error) => {
                tracing::warn!(error = %error, "operator archive worker did not start");
                self.sync_status().phase = SyncPhase::Failed(error.to_string());
                Err(error.into())
            }
        }
    }

    fn start_autonomous(
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
        };
        std::thread::Builder::new()
            .name("wire-sync".into())
            .spawn(move || worker.run())?;
        Ok(snapshot)
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
}

enum Batch {
    Done,
    More,
}

impl SyncWorker {
    fn run(self) {
        let mut attempt: u32 = 0;
        loop {
            match self.batch() {
                Ok(Batch::Done) => return,
                Ok(Batch::More) => attempt = 0,
                Err(error) => {
                    attempt += 1;
                    if attempt > SYNC_RETRY_ATTEMPTS {
                        tracing::warn!(error = %error, attempt, "history sync gave up");
                        locked_sync(&self.status).phase = SyncPhase::Failed(format!(
                            "Sync gave up after {SYNC_RETRY_ATTEMPTS} retries: {error}"
                        ));
                        return;
                    }
                    tracing::warn!(error = %error, attempt, "history sync batch failed; retrying");
                    self.wait(attempt, error.to_string());
                }
            }
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
        status.tip = report.tip;
        status.indexed_to = index.blocks.last().map(|block| block.height);
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
            locked_sync(&self.status).phase = SyncPhase::Retrying {
                attempt,
                seconds,
                reason: reason.clone(),
            };
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}
