use crate::{library::hash::HashCache, model::CounterModel, node::FileSizeModel};
use anyhow::Context;
use base64::{Engine, prelude::BASE64_STANDARD};
use dashmap::DashMap;
use image::{ImageReader, codecs::jpeg::JpegEncoder, imageops::FilterType};
use priority_queue::PriorityQueue;
use rubato::{FftFixedIn, Resampler};
use std::{
    borrow::Borrow,
    collections::HashSet,
    fs::File,
    hash::{Hash, Hasher},
    io::{Cursor, Seek, SeekFrom},
    ops::Deref,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use symphonia::core::{
    formats::{TrackType, probe::Hint},
    io::MediaSourceStream,
    meta::{StandardTag, StandardVisualKey},
};
use tokio::sync::mpsc;

/// The transcode status of a file.
///
/// If a file is not Ready or Failed (it's status is not in the cache), it
/// might be queued for transcoding, in progress, or waiting to be requested.
#[derive(Debug)]
pub enum TranscodeStatus {
    /// The file is transcoded and available at `transcode_path`.
    Ready {
        transcode_path: PathBuf,
        file_size: u64,
    },

    /// Transcoding the file failed.
    Failed { error: anyhow::Error },
}

/// Helper trait for creating a borrowed hash key.
///
/// This is required because we can't use a tuple of borrowed parts, we need a
/// borrowed tuple of parts. The trait object adds indirection but avoids
/// needing to clone.
///
/// See https://stackoverflow.com/a/45795699
trait HashKey {
    fn hash_kind(&self) -> &str;
    fn hash(&self) -> [u8; 16];
}

impl<'a> Borrow<dyn HashKey + 'a> for (String, [u8; 16]) {
    fn borrow(&self) -> &(dyn HashKey + 'a) {
        self
    }
}

impl Hash for dyn HashKey + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_kind().hash(state);
        self.hash().hash(state);
    }
}

impl PartialEq for dyn HashKey + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.hash_kind() == other.hash_kind() && self.hash() == other.hash()
    }
}

impl Eq for dyn HashKey + '_ {}

impl HashKey for (String, [u8; 16]) {
    fn hash_kind(&self) -> &str {
        &self.0
    }

    fn hash(&self) -> [u8; 16] {
        self.1
    }
}

impl HashKey for (&str, [u8; 16]) {
    fn hash_kind(&self) -> &str {
        self.0
    }

    fn hash(&self) -> [u8; 16] {
        self.1
    }
}

/// A borrowed entry in the transcoding status cache.
///
/// This wraps a RwLockReadGuard for the DashMap entry.
pub struct TranscodeStatusCacheEntry<'a>(
    dashmap::mapref::one::Ref<'a, (String, [u8; 16]), TranscodeStatus>,
);

impl Deref for TranscodeStatusCacheEntry<'_> {
    type Target = TranscodeStatus;

    fn deref(&self) -> &Self::Target {
        self.0.value()
    }
}

/// An in-memory cache of the transcoding status of files.
///
/// This is populated on startup by reading the transcode cache directory and
/// updated as files are transcoded. It's initialized in Core and passed down
/// to TranscodePool because it needs to be shared with Node as well.
///
/// We key by hash because rescanning causes file IDs to change and can happen
/// at any time, and source files can be renamed or moved. This also accounts
/// for multiple copies of the same file existing in the library.
///
/// Also keeps counts of the number of items with each status.
#[derive(Debug, Clone)]
pub struct TranscodeStatusCache {
    cache: Arc<DashMap<(String, [u8; 16]), TranscodeStatus>>,

    ready_counter: Arc<AtomicU64>,
    failed_counter: Arc<AtomicU64>,
}

impl TranscodeStatusCache {
    /// Creates a new TranscodeStatusCache.
    pub fn new() -> Self {
        TranscodeStatusCache {
            cache: Arc::new(DashMap::new()),

            ready_counter: Arc::new(AtomicU64::new(0)),
            failed_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Gets a reference to an entry in the cache.
    pub fn get(&self, hash_kind: &str, hash: [u8; 16]) -> Option<TranscodeStatusCacheEntry<'_>> {
        self.cache
            .get(&(hash_kind, hash) as &dyn HashKey)
            .map(TranscodeStatusCacheEntry)
    }

    /// Inserts a key and a value into the cache, replacing the old value.
    pub fn insert(&self, hash_kind: String, hash: [u8; 16], status: TranscodeStatus) {
        match status {
            TranscodeStatus::Ready { .. } => {
                self.ready_counter.fetch_add(1, Ordering::Relaxed);
            }
            TranscodeStatus::Failed { .. } => {
                self.failed_counter.fetch_add(1, Ordering::Relaxed);
            }
        }

        let prev = self.cache.insert((hash_kind, hash), status);

        match prev {
            Some(TranscodeStatus::Ready { .. }) => {
                self.ready_counter.fetch_sub(1, Ordering::Relaxed);
            }
            Some(TranscodeStatus::Failed { .. }) => {
                self.failed_counter.fetch_sub(1, Ordering::Relaxed);
            }
            None => {}
        }
    }

    /// Retain elements according to the predicate, updating counters as needed.
    fn retain(&self, mut f: impl FnMut(&(String, [u8; 16]), &TranscodeStatus) -> bool) {
        self.cache.retain(|key, status| {
            let keep = f(key, status);
            if !keep {
                match status {
                    TranscodeStatus::Ready { .. } => {
                        self.ready_counter.fetch_sub(1, Ordering::Relaxed);
                    }
                    TranscodeStatus::Failed { .. } => {
                        self.failed_counter.fetch_sub(1, Ordering::Relaxed);
                    }
                }
            }
            keep
        });
    }

    pub fn ready_counter(&self) -> &Arc<AtomicU64> {
        &self.ready_counter
    }

    pub fn failed_counter(&self) -> &Arc<AtomicU64> {
        &self.failed_counter
    }
}

impl Default for TranscodeStatusCache {
    fn default() -> Self {
        Self::new()
    }
}

/// When to transcode files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TranscodePolicy {
    /// Only transcode files if they are requested.
    IfRequested,
    /// Transcode all files ahead of time.
    Always,
}

/// The queue of items to be transcoded.
#[derive(Debug)]
struct TranscodeQueue {
    policy: Mutex<TranscodePolicy>,
    queue: Mutex<PriorityQueue<PathBuf, u64>>,
    ready: Condvar,
    ready_counter: Arc<AtomicU64>,
}

impl TranscodeQueue {
    /// Creates a new TranscodeQueue.
    pub fn new(policy: TranscodePolicy) -> Self {
        TranscodeQueue {
            policy: Mutex::new(policy),
            queue: Mutex::new(PriorityQueue::new()),
            ready: Condvar::new(),
            ready_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Sets the transcoding policy.
    pub fn set_policy(&self, policy: TranscodePolicy) {
        // update policy
        {
            let mut policy_guard = self.policy.lock().unwrap();
            *policy_guard = policy;
        }

        {
            let queue = self.queue.lock().unwrap();

            // update ready counter by re-counting queue
            let ready_count = match policy {
                TranscodePolicy::IfRequested => queue.iter().filter(|entry| *entry.1 > 0).count(),
                TranscodePolicy::Always => queue.len(),
            };
            self.ready_counter
                .store(ready_count as u64, Ordering::Relaxed);
        }

        // notify waiting consumers
        self.ready.notify_all();
    }

    /// Adds items to the queue.
    pub fn extend(&self, items: impl IntoIterator<Item = PathBuf>) {
        // read policy before locking queue
        let policy = {
            let policy = self.policy.lock().unwrap();
            *policy
        };

        {
            // add items to the queue if they aren't already present
            let mut queue = self.queue.lock().unwrap();
            for item in items {
                queue.push_increase(item, 0);
            }

            // update ready counter by re-counting queue
            let ready_count = match policy {
                TranscodePolicy::IfRequested => queue.iter().filter(|entry| *entry.1 > 0).count(),
                TranscodePolicy::Always => queue.len(),
            };
            self.ready_counter
                .store(ready_count as u64, Ordering::Relaxed);
        }

        // notify waiting consumers
        self.ready.notify_all();
    }

    /// Increases the priority of items in the queue.
    pub fn prioritize(&self, items: HashSet<PathBuf>) {
        // read policy before locking queue
        let policy = {
            let policy = self.policy.lock().unwrap();
            *policy
        };

        {
            let mut queue = self.queue.lock().unwrap();

            // increase priority
            for (item, priority) in queue.iter_mut() {
                if items.contains(item) {
                    *priority += 1;
                }
            }

            // update ready counter by re-counting queue
            let ready_count = match policy {
                TranscodePolicy::IfRequested => queue.iter().filter(|entry| *entry.1 > 0).count(),
                TranscodePolicy::Always => queue.len(),
            };
            self.ready_counter
                .store(ready_count as u64, Ordering::Relaxed);
        }

        // notify waiting consumers
        self.ready.notify_all();
    }

    /// Removes items from the queue if they aren't in the given HashSet.
    pub fn remove_missing(&self, items: &HashSet<PathBuf>) {
        // read policy before locking queue
        let policy = {
            let policy = self.policy.lock().unwrap();
            *policy
        };

        {
            // remove items from queue
            let mut queue = self.queue.lock().unwrap();
            queue.retain(|item, _priority| items.contains(item));

            // update ready counter by re-counting queue
            let ready_count = match policy {
                TranscodePolicy::IfRequested => queue.iter().filter(|entry| *entry.1 > 0).count(),
                TranscodePolicy::Always => queue.len(),
            };
            self.ready_counter
                .store(ready_count as u64, Ordering::Relaxed);
        }
    }

    /// Waits for a job and takes it from the queue.
    pub fn wait(&self) -> PathBuf {
        let mut queue = self.queue.lock().unwrap();
        loop {
            // check for a job
            let next = queue.pop_if(|_item, priority| {
                let policy = self.policy.lock().unwrap();
                match *policy {
                    TranscodePolicy::IfRequested => *priority > 0,
                    TranscodePolicy::Always => true,
                }
            });

            match next {
                Some((item, _priority)) => {
                    // decrease ready counter
                    self.ready_counter.fetch_sub(1, Ordering::Relaxed);

                    return item;
                }
                None => {
                    // no job, wait for notification
                    queue = self.ready.wait(queue).unwrap();
                }
            }
        }
    }
}

/// A command sent to the transcoding pool.
pub enum TranscodeCommand {
    /// Sent on startup and when the library is scanned. Files are enqueued if
    /// they aren't already transcoded or in the queue. Files are dequeued if
    /// they aren't in the library anymore.
    Load(HashSet<PathBuf>),

    /// Increase the priority of some files. Sent when files are requested.
    /// This is useful for partial downloads when the library isn't fully
    /// transcoded yet.
    Prioritize(HashSet<PathBuf>),

    /// Delete transcodes of files that aren't in the library anymore.
    DeleteMissing(Vec<PathBuf>),

    /// Delete all transcodes.
    DeleteAll,

    /// Set the transcode policy.
    SetPolicy(TranscodePolicy),
}

/// A handle to a pool of worker threads for transcoding files.
pub struct TranscodePool {
    transcodes_dir: PathBuf,
    status_cache: TranscodeStatusCache,

    queue: Arc<TranscodeQueue>,
    inprogress_counter: RegionCounter,

    command_tx: mpsc::UnboundedSender<TranscodeCommand>,
}

impl TranscodePool {
    /// Spawns the transcode worker pool and returns its handle.
    ///
    /// The transcode status cache is guaranteed to be populated after this
    /// returns.
    pub fn spawn(
        transcodes_dir: PathBuf,
        initial_policy: TranscodePolicy,
        status_cache: TranscodeStatusCache,
        hash_cache: HashCache,
    ) -> Self {
        // initialize status cache
        Self::read_transcodes_dir(&transcodes_dir, &status_cache);

        let queue = Arc::new(TranscodeQueue::new(initial_policy));
        let inprogress_counter = RegionCounter::new();

        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn({
            let transcodes_dir = transcodes_dir.clone();
            let status_cache = status_cache.clone();
            let queue = queue.clone();
            let inprogress_counter = inprogress_counter.clone();
            async move {
                if let Err(e) = Self::run(
                    transcodes_dir,
                    status_cache,
                    hash_cache,
                    queue,
                    inprogress_counter,
                    command_rx,
                )
                .await
                {
                    log::error!("error running transcode pool: {e:#}");
                }
            }
        });

        TranscodePool {
            transcodes_dir,
            status_cache,

            queue,
            inprogress_counter,

            command_tx,
        }
    }

    // initialize the transcode status cache by reading the transcode cache directory
    fn read_transcodes_dir(transcodes_dir: &Path, status_cache: &TranscodeStatusCache) {
        // create transcode cache directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(transcodes_dir) {
            log::error!(
                "failed to create transcode cache directory at {}: {}",
                transcodes_dir.display(),
                e
            );
        }

        // read the transcode cache directory
        let items = match std::fs::read_dir(transcodes_dir) {
            Ok(entries) => entries,
            Err(e) => {
                log::error!(
                    "failed to read transcode cache directory at {}: {}",
                    transcodes_dir.display(),
                    e
                );
                return;
            }
        };

        // parse transcode cache directory entries
        let items = items
            .filter_map(|entry| match entry {
                Ok(entry) => Some(entry),
                Err(e) => {
                    log::error!("failed to read entry in transcode cache directory: {e:#}");
                    None
                }
            })
            .filter_map(|entry| match Self::parse_transcodes_dir_entry(&entry) {
                Ok(res) => res,
                Err(e) => {
                    log::error!(
                        "failed to parse transcode cache directory entry at {}: {}",
                        entry.path().display(),
                        e
                    );
                    None
                }
            })
            .collect::<Vec<_>>();

        // update status cache
        for (transcode_path, hash_kind, hash, file_size) in items {
            status_cache.insert(
                hash_kind,
                hash,
                TranscodeStatus::Ready {
                    transcode_path,
                    file_size,
                },
            );
        }
    }

    fn parse_transcodes_dir_entry(
        entry: &std::fs::DirEntry,
    ) -> anyhow::Result<Option<(PathBuf, String, [u8; 16], u64)>> {
        // get entry file type
        let file_type = entry.file_type().context("failed to get file type")?;

        // skip non-files
        if !file_type.is_file() {
            anyhow::bail!("entry is not a file");
        }

        let path = entry.path();

        // check if the file has a valid extension
        match path.extension() {
            Some(ext) if ext == "ogg" => {}
            Some(ext) if ext == "tmp" => {
                // remove temp files from previous runs
                log::info!("removing old temp file: {}", path.display());

                let _ = std::fs::remove_file(path);

                return Ok(None);
            }
            _ => {
                log::warn!("unexpected file in transcodes dir: {}", path.display());

                return Ok(None);
            }
        }

        // parse file name as <hash kind>-<hash hex>.ext
        let file_stem = path
            .file_stem()
            .context("file missing name")?
            .to_string_lossy();
        let (hash_kind, hash) = file_stem
            .split_once("-")
            .context("failed to parse file name")?;
        let hash_kind = hash_kind.to_string();
        let hash = hex::decode(hash)
            .context("failed to decode hash bytes")?
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid hash length"))?;

        // get file size
        let file_size = path
            .metadata()
            .context("failed to get file metadata")?
            .len();

        Ok(Some((path, hash_kind, hash, file_size)))
    }

    async fn run(
        transcodes_dir: PathBuf,
        status_cache: TranscodeStatusCache,
        hash_cache: HashCache,
        queue: Arc<TranscodeQueue>,
        inprogress_counter: RegionCounter,
        mut rx: mpsc::UnboundedReceiver<TranscodeCommand>,
    ) -> anyhow::Result<()> {
        // spawn transcode workers
        // TODO
        for _ in 0..8 {
            TranscodeWorker::new(
                transcodes_dir.clone(),
                status_cache.clone(),
                hash_cache.clone(),
                queue.clone(),
                inprogress_counter.clone(),
            );
        }

        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        TranscodeCommand::Load(mut items) => {
                            // remove items that are no longer in the library
                            queue.remove_missing(&items);

                            // filter out items that are already transcoded
                            items.retain(|item| {
                                // get the cached hash without computing it. we need to be conservative here
                                // since every scan could send all files again. if the hash is not cached, it's
                                // definitely not transcoded, so it's safe to queue it
                                let (hash_kind, hash) = match hash_cache.get_cached_hash(item) {
                                    Ok(Some((hash_kind, hash))) => (hash_kind, hash),

                                    Ok(None) => {
                                        // add to queue
                                        return true;
                                    }

                                    Err(e) => {
                                        log::warn!("TranscodePool: failed to get cached hash for {}: {e:#}", item.display());

                                        // add to queue
                                        return true;
                                    }
                                };

                                // if we have a cached hash, check if it's already waiting/transcoded/failed
                                let status = status_cache.get(&hash_kind, hash);
                                match status {
                                    Some(status) => {
                                        log::trace!("TranscodePool: skipping file {} (status: {:?})", item.display(), *status);

                                        // don't add to queue
                                        false
                                    },
                                    None => {
                                        // add to queue
                                        true
                                    },
                                }
                            });

                            if !items.is_empty() {
                                // spawn task to estimate file sizes in parallel using rayon
                                // this seems fast enough to do without indicating progress.
                                // it requires opening each file and reading metadata, but doesn't need to
                                // decode the file, so it's fast ish. spawn it as a background task though
                                tokio::spawn({
                                    let hash_cache = hash_cache.clone();
                                    let items = items.iter().cloned().collect::<Vec<_>>();
                                    async move {
                                        let start = std::time::Instant::now();
                                        log::info!("TranscodePool: estimating sizes for {} files", items.len());

                                        let Ok(res) = tokio::task::spawn_blocking(move || {
                                            hash_cache.batch_get_estimated_size(items)
                                        }).await else {
                                            log::error!("TranscodePool: failed to join file size estimation task");
                                            return;
                                        };

                                        match res {
                                            Ok(_) => {
                                                let elapsed = (start.elapsed().as_millis() as f64) / 1000.0;
                                                log::info!("TranscodePool: finished estimating file sizes in {elapsed:?}s");
                                            },
                                            Err(e) => {
                                                log::error!("TranscodePool: failed to estimate file sizes: {e:#}");
                                            }
                                        }
                                    }
                                });

                                // add items to queue
                                queue.extend(items);
                            }
                        },

                        TranscodeCommand::Prioritize(items) => {
                            queue.prioritize(items);
                        },

                        TranscodeCommand::DeleteMissing(items) => {
                            Self::delete_missing(&status_cache, &hash_cache, items);
                        },

                        TranscodeCommand::DeleteAll => {
                            Self::delete_all(&status_cache);
                        },

                        TranscodeCommand::SetPolicy(policy) => {
                            queue.set_policy(policy);
                        }
                    }
                }
            }
        }
    }

    pub fn send(&self, command: TranscodeCommand) -> anyhow::Result<()> {
        self.command_tx
            .send(command)
            .map_err(|e| anyhow::anyhow!("failed to send TranscodeCommand: {e:#}"))
    }

    pub fn transcodes_dir(&self) -> String {
        self.transcodes_dir.to_string_lossy().to_string()
    }

    pub fn transcodes_dir_size(&self) -> FileSizeModel {
        let size = self
            .status_cache
            .cache
            .iter()
            .fold(0, |acc_size, e| match e.value() {
                TranscodeStatus::Ready { file_size, .. } => acc_size + file_size,
                TranscodeStatus::Failed { .. } => acc_size,
            });
        FileSizeModel::Actual(size)

        // TODO: expose separately current used size and total size if everything was transcoded?
        // TODO: old code that dealt with estimated sizes
        // let (size, estimated) = self.status_cache.cache.iter().fold(
        //     (0, false),
        //     |(acc_size, acc_estimated), e| match &*e {
        //         TranscodeStatus::Waiting { estimated_size } => {
        //             (acc_size + estimated_size.unwrap_or(0), true)
        //         }
        //         TranscodeStatus::Ready { file_size, .. } => (acc_size + file_size, acc_estimated),
        //         TranscodeStatus::Failed { .. } => (acc_size, acc_estimated),
        //     },
        // );

        // if estimated {
        //     FileSizeModel::Estimated(size)
        // } else {
        //     FileSizeModel::Actual(size)
        // }
    }

    pub fn queued_count_model(&self) -> CounterModel {
        CounterModel::from(&self.queue.ready_counter)
    }

    pub fn inprogress_count_model(&self) -> CounterModel {
        CounterModel::from(&self.inprogress_counter.0)
    }

    pub fn ready_count_model(&self) -> CounterModel {
        CounterModel::from(&self.status_cache.ready_counter)
    }

    pub fn failed_count_model(&self) -> CounterModel {
        CounterModel::from(&self.status_cache.failed_counter)
    }

    fn delete_missing(
        status_cache: &TranscodeStatusCache,
        hash_cache: &HashCache,
        items: Vec<PathBuf>,
    ) {
        let start = std::time::Instant::now();
        log::debug!(
            "TranscodePool::delete_missing: hashing library of {} items",
            items.len()
        );

        let hashes = match hash_cache.batch_get_hash(items) {
            Ok(hashes) => hashes,
            Err(e) => {
                log::error!("TranscodePool::delete_missing: failed to get file hashes: {e:#}");
                return;
            }
        };

        let elapsed = start.elapsed().as_secs_f64();
        log::debug!("TranscodePool::delete_missing: hashed library in {elapsed:.2}s",);

        let mut count_deleted = 0;
        let mut bytes_deleted = 0;

        status_cache.retain(|(hash_kind, hash), status| {
            // ignore if not Ready
            let TranscodeStatus::Ready { transcode_path, file_size } = status else {
                return true;
            };

            // check if missing from set
            if !hashes.contains(&(hash_kind.into(), *hash)) {
                // try to delete transcode file
                if let Err(e) = std::fs::remove_file(transcode_path) {
                    log::error!(
                        "TranscodePool::delete_missing: failed to delete transcode file at {}: {e:#}",
                        transcode_path.display()
                    );
                }

                count_deleted += 1;
                bytes_deleted += *file_size;

                // remove from cache
                false
            } else {
                // ignore
                true
            }
        });

        log::info!(
            "TranscodePool::delete_missing: deleted {count_deleted} transcode files, {bytes_deleted} bytes total"
        );
    }

    fn delete_all(status_cache: &TranscodeStatusCache) {
        let mut count_deleted = 0;
        let mut bytes_deleted = 0;

        status_cache.retain(|_key, status| {
            // ignore if not Ready
            let TranscodeStatus::Ready {
                transcode_path,
                file_size,
            } = status
            else {
                return true;
            };

            // try to delete transcode file
            if let Err(e) = std::fs::remove_file(transcode_path) {
                log::error!(
                    "TranscodePool::delete_all: failed to delete transcode file at {}: {e:#}",
                    transcode_path.display()
                );
            }

            count_deleted += 1;
            bytes_deleted += *file_size;

            // remove from cache
            false
        });

        log::info!(
            "TranscodePool::delete_all: deleted {count_deleted} transcode files, {bytes_deleted} bytes total"
        );
    }
}

struct TranscodeWorker {}

impl TranscodeWorker {
    /// Start a new transcode worker thread and return a handle to it.
    pub fn new(
        transcodes_dir: PathBuf,
        status_cache: TranscodeStatusCache,
        hash_cache: HashCache,
        queue: Arc<TranscodeQueue>,
        inprogress_counter: RegionCounter,
    ) -> Self {
        std::thread::spawn(move || {
            if let Err(e) = Self::run(
                transcodes_dir,
                status_cache,
                hash_cache,
                queue,
                inprogress_counter,
            ) {
                log::error!("transcode worker failed: {e:#}");
            }
        });

        Self {}
    }

    /// Implementation of the transcode worker thread.
    fn run(
        transcodes_dir: PathBuf,
        status_cache: TranscodeStatusCache,
        hash_cache: HashCache,
        queue: Arc<TranscodeQueue>,
        inprogress_counter: RegionCounter,
    ) -> anyhow::Result<()> {
        loop {
            // wait for a job
            let job = queue.wait();

            // mark thread as in-progress
            let _counter_guard = inprogress_counter.entered();

            // get file hash
            let (hash_kind, hash) = match hash_cache.get_hash(&job) {
                Ok((hash_kind, hash)) => (hash_kind, hash),

                Err(e) => {
                    log::error!(
                        "failed to compute file hash for transcoding: {}: {e:#}",
                        job.display()
                    );

                    // TODO: can't set status to Failed because we don't have the hash
                    // maybe store failed paths somewhere?

                    // next job
                    continue;
                }
            };

            // check if already transcoded
            if let Some(TranscodeStatus::Ready { .. }) =
                status_cache.get(&hash_kind, hash).as_deref()
            {
                log::info!("skipping already transcoded file: {}", job.display());

                // next job
                continue;
            }

            // write to temp filename
            let temp_path = transcodes_dir.join(format!("{}-{}.tmp", hash_kind, hex::encode(hash)));

            log::info!("transcoding file: {}", job.display());
            let file_size = match transcode(&job, &temp_path) {
                Ok(file_size) => file_size,

                Err(e) => {
                    log::error!(
                        "failed to transcode file: {} -> {}: {e:#}",
                        job.display(),
                        temp_path.display()
                    );

                    // try to remove the temp file
                    let _ = std::fs::remove_file(&temp_path);

                    // set status to Failed
                    status_cache.insert(
                        hash_kind.to_string(),
                        hash,
                        TranscodeStatus::Failed { error: e },
                    );

                    // next job
                    continue;
                }
            };

            // rename the temp file
            let final_path = temp_path.with_extension("ogg");
            if let Err(e) = std::fs::rename(&temp_path, &final_path) {
                log::error!(
                    "failed to rename temp file: {} -> {}: {e:#}",
                    temp_path.display(),
                    final_path.display()
                );

                // set status to Failed
                status_cache.insert(
                    hash_kind.to_string(),
                    hash,
                    TranscodeStatus::Failed {
                        error: anyhow::anyhow!("failed to rename temp file: {e:#}"),
                    },
                );

                // next job
                continue;
            };

            log::info!(
                "finished transcoding file: {} -> {}",
                job.display(),
                final_path.display()
            );

            // set status to Ready
            status_cache.insert(
                hash_kind.to_string(),
                hash,
                TranscodeStatus::Ready {
                    transcode_path: final_path,
                    file_size,
                },
            );
        }

        // worker shut down
        Ok(())
    }
}

/// Counts the number of threads of execution that are in a region.
///
/// This is used to track how many worker threads are currently working.
#[derive(Debug, Clone)]
struct RegionCounter(Arc<AtomicU64>);

impl RegionCounter {
    /// Creates a new RegionCounter.
    pub fn new() -> Self {
        Self(Arc::new(AtomicU64::new(0)))
    }

    /// Gets the current count.
    pub fn count(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    /// Enters the region and increments the count, returning a guard that
    /// decrements the count when dropped at the end of the region.
    pub fn entered(&self) -> RegionCounterGuard<'_> {
        RegionCounterGuard::new(self)
    }
}

struct RegionCounterGuard<'a>(&'a RegionCounter);

impl<'a> RegionCounterGuard<'a> {
    fn new(counter: &'a RegionCounter) -> Self {
        counter.0.fetch_add(1, Ordering::Relaxed);
        Self(counter)
    }
}

impl Drop for RegionCounterGuard<'_> {
    fn drop(&mut self) {
        self.0.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Transcode a file.
///
/// Returns the file size of the output file.
fn transcode(input_path: &Path, output_path: &Path) -> anyhow::Result<u64> {
    let input_file = File::open(input_path).context("failed to open input file")?;

    let mss = MediaSourceStream::new(Box::new(input_file), Default::default());

    let mut hint = Hint::new();
    if let Some(extension) = input_path.extension() {
        hint.with_extension(extension.to_str().context("invalid file extension")?);
    }

    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, Default::default(), Default::default())
        .context("failed to probe file format")?;

    // get the default audio track
    let audio_track = format
        .default_track(TrackType::Audio)
        .context("failed to get default audio track")?;
    let audio_track_id = audio_track.id;

    // get codec parameters for the audio track
    let codec_params = audio_track
        .codec_params
        .as_ref()
        .context("failed to get codec parameters")?;
    let audio_codec_params = codec_params
        .audio()
        .context("codec parameters are not audio")?;

    // get channel count and sample rate from codec parameters
    let channel_count = audio_codec_params
        .channels
        .as_ref()
        .context("failed to get channel count from codec params")?
        .count();
    let sample_rate = audio_codec_params
        .sample_rate
        .context("failed to get sample rate from codec params")? as usize;

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(audio_codec_params, &Default::default())
        .context("failed to create decoder")?;

    // decode the audio track into planar samples
    let mut original_samples: Vec<Vec<f32>> = vec![Vec::new(); channel_count];
    loop {
        // read next packet
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,

            // end of track
            Ok(None) => break,

            Err(e) => {
                return Err(e).context("failed to read packet");
            }
        };

        // skip packets from other tracks
        if packet.track_id() != audio_track_id {
            continue;
        }

        // decode packet
        let audio_buf = decoder.decode(&packet).context("failed to decode packet")?;

        // copy to output buffer
        // symphonia only lets us copy to vecs/slices, which replaces instead of extending
        // we need to manually resize each channel and then copy to mut slices of the new extended areas
        let mut output_slices = Vec::with_capacity(channel_count);
        for channel in &mut original_samples {
            let curr_len = channel.len();
            let new_len = curr_len + audio_buf.frames();
            channel.resize(new_len, 0.0);
            output_slices.push(&mut channel[curr_len..new_len]);
        }
        audio_buf.copy_to_slice_planar(&mut output_slices);
    }

    // construct the encoder before resampling to determine the lookahead
    let mut encoder = opus::Encoder::new(
        48000,
        match channel_count {
            1 => opus::Channels::Mono,
            2 => opus::Channels::Stereo,
            _ => anyhow::bail!("unsupported channel count: {}", channel_count),
        },
        opus::Application::Audio,
    )
    .context("failed to create opus encoder")?;
    encoder
        .set_bitrate(opus::Bitrate::Bits(128000))
        .context("failed to set opus bitrate")?;

    let lookahead_frames = encoder
        .get_lookahead()
        .context("failed to get opus encoder lookahead")? as usize;

    // resample to 48k if needed
    // also pad the start with zeros to account for encoder lookahead. doing
    // this now allows the encoding logic to be simpler and more efficient.
    let mut resampled_samples = if sample_rate != 48000 {
        let mut resampler = FftFixedIn::<f32>::new(
            sample_rate,
            48000,
            1024, // arbitrary
            4,    // arbitrary
            channel_count,
        )
        .context("failed to create resampler")?;

        let delay = resampler.output_delay();

        let original_frames = original_samples[0].len();

        // number of frames after resampling, including zero-padding for encoder lookahead
        let new_frames = (original_frames * 48000 / sample_rate) + lookahead_frames;

        // pre-allocate output buffer with enough capacity
        // TODO: we might need a little more than this, should check its final capacity to see if it gets resized usually
        let mut resampled_samples: Vec<Vec<f32>> =
            vec![Vec::with_capacity(new_frames + delay); channel_count];

        // pad start with zeros
        for channel in resampled_samples.iter_mut() {
            channel.resize(lookahead_frames, 0.0);
        }

        // allocate chunk input slices vec and chunk output buffer
        let mut input_slices: Vec<&[f32]> = vec![&[]; channel_count];
        let mut output_buf = resampler.output_buffer_allocate(true);

        // resample in chunks
        let mut pos = 0;
        loop {
            // get number of frames needed for next chunk
            let frames_needed = resampler.input_frames_next();

            // check if we have enough frames for a full chunk
            if pos + frames_needed > original_frames {
                break;
            }

            // copy reference to slice of original buffer to input slices vec
            for i in 0..channel_count {
                input_slices[i] = &original_samples[i][pos..(pos + frames_needed)];
            }

            // call resampler with chunk input slices vec and chunk output buffer
            let (input_frames, output_frames) = resampler
                .process_into_buffer(&input_slices, &mut output_buf, None)
                .expect("bad inputs to resampler");

            // copy chunk output buffer to resampled samples
            for i in 0..channel_count {
                resampled_samples[i].extend_from_slice(&output_buf[i][0..output_frames]);
            }

            // increment position by number of input frames consumed
            pos += input_frames;
        }

        // resample final chunk with remaining frames
        if pos < original_frames {
            // copy reference to remaining frames in original samples to input buffer
            for i in 0..channel_count {
                input_slices[i] = &original_samples[i][pos..original_frames];
            }

            let (_input_frames, output_frames) = resampler
                .process_partial_into_buffer(Some(&input_slices), &mut output_buf, None)
                .expect("bad inputs to resampler");

            // copy chunk output buffer to resampled samples
            for i in 0..channel_count {
                resampled_samples[i].extend_from_slice(&output_buf[i][0..output_frames]);
            }
        }

        // continue feeding zeros to the resampler until we have enough frames
        // this ensures we account for resample delay and push everything through its internal buffer
        while resampled_samples[0].len() < new_frames + delay {
            let (_input_frames, output_frames) = resampler
                .process_partial_into_buffer(None::<&[&[f32]]>, &mut output_buf, None)
                .expect("bad inputs to resampler");

            // copy chunk output buffer to resampled samples
            for i in 0..channel_count {
                resampled_samples[i].extend_from_slice(&output_buf[i][0..output_frames]);
            }
        }

        // remove resample delay frames from the start and truncate to new frame count
        // TODO: can we do this without a copy from .drain()?
        for channel in resampled_samples.iter_mut() {
            channel.drain(0..delay);
            channel.truncate(new_frames);
        }

        resampled_samples
    } else {
        // we don't need to resample, but we still need to pad the start with zeros

        let original_frames = original_samples[0].len();

        let mut resampled_samples = vec![Vec::new(); channel_count];
        for i in 0..channel_count {
            resampled_samples[i].resize(lookahead_frames + original_frames, 0.0);
            resampled_samples[i][lookahead_frames..].copy_from_slice(&original_samples[i][..]);
        }

        resampled_samples
    };

    // interleave samples since opus needs interleaved input
    // TODO: profile + explore SIMD for this
    let interleaved_samples = if channel_count == 2 {
        let mut interleaved_samples = vec![0.0; resampled_samples[0].len() * channel_count];

        for i in 0..resampled_samples[0].len() {
            for j in 0..channel_count {
                interleaved_samples[i * channel_count + j] = resampled_samples[j][i];
            }
        }

        interleaved_samples
    } else if channel_count == 1 {
        resampled_samples.swap_remove(0)
    } else {
        unreachable!();
    };

    let mut output_file = File::create(output_path).context("failed to create output file")?;

    let mut packet_writer = ogg::PacketWriter::new(&mut output_file);

    // we write the number of lookahead frames as pre-skip in the opus header
    // we added this many zeros to the start of the resampled samples to account for encoder lookahead
    // players should skip these frames when decoding
    let preskip_bytes = lookahead_frames.to_le_bytes();

    // input sample rate is always 48000 since we resample to it
    let rate_bytes = 48000u32.to_le_bytes();

    #[rustfmt::skip]
	let opus_head: [u8; 19] = [
        b'O', b'p', b'u', b's', b'H', b'e', b'a', b'd', // magic signature
        1, // version, always 1
        channel_count as u8, // channel count
        preskip_bytes[0], preskip_bytes[1], // pre-skip
        rate_bytes[0], rate_bytes[1], rate_bytes[2], rate_bytes[3], // input sample rate
        0, 0, // output gain
        0, // channel mapping family
    ];

    let (user_comments_len, user_comments_buf) = {
        let mut len = 0u32;
        let mut buf = Vec::new();

        if let Some(metadata) = format.metadata().skip_to_latest() {
            for tag in metadata.tags().iter().flat_map(|t| &t.std) {
                // TODO: replace =
                let comment = match tag {
                    StandardTag::TrackTitle(tag) => Some(format!("TITLE={tag}")),
                    StandardTag::Album(tag) => Some(format!("ALBUM={tag}")),
                    StandardTag::TrackNumber(tag) => Some(format!("TRACKNUMBER={tag}")),
                    StandardTag::Artist(tag) => Some(format!("ARTIST={tag}")),
                    _ => None,
                };

                if let Some(s) = comment {
                    len += 1;
                    buf.extend((s.len() as u32).to_le_bytes());
                    buf.extend(s.bytes());
                }
            }

            // find front cover visual or first available
            let mut best_visual = metadata.visuals().first();
            for visual in metadata.visuals() {
                if visual.usage == Some(StandardVisualKey::FrontCover) {
                    best_visual = Some(visual);
                }
            }

            if let Some(visual) = best_visual {
                let rdr = ImageReader::new(Cursor::new(&visual.data))
                    .with_guessed_format()
                    .expect("cursor io never fails");

                // convert to jpeg 500x500 90% quality
                let image_buf = {
                    let original_image = rdr.decode().context("failed to decode image")?;

                    let resized_image = original_image.resize(500, 500, FilterType::Lanczos3);

                    let mut image_buf = vec![];
                    let mut encoder = JpegEncoder::new_with_quality(&mut image_buf, 90);
                    encoder
                        .encode_image(&resized_image)
                        .context("failed to encode image")?;

                    image_buf
                };

                // construct flac picture structure
                // note that flac uses big endian while vorbis comments use little endian
                let mut picture = Vec::<u8>::new();
                picture.extend(&3u32.to_be_bytes()); // picture type (3, front cover)

                let media_type = "image/jpeg";
                picture.extend(&(media_type.len() as u32).to_be_bytes());
                picture.extend(media_type.as_bytes());

                picture.extend(&[0, 0, 0, 0]); // description length
                picture.extend(&500u32.to_be_bytes()); // width (500px)
                picture.extend(&500u32.to_be_bytes()); // height (500px)
                picture.extend(&[0, 0, 0, 0]); // color depth (0, unknown)
                picture.extend(&[0, 0, 0, 0]); // indexed color count (0, non-indexed)

                picture.extend(&(image_buf.len() as u32).to_be_bytes()); // picture data length
                picture.extend(&image_buf); // picture data

                // encode picture with base64 for comment
                let comment = format!(
                    "METADATA_BLOCK_PICTURE={}",
                    BASE64_STANDARD.encode(&picture)
                );

                log::debug!(
                    "adding visual to opus tags, image size = {}, comment size = {}",
                    image_buf.len(),
                    comment.len(),
                );

                len += 1;
                buf.extend((comment.len() as u32).to_le_bytes());
                buf.extend(comment.as_bytes());
            }
        }

        (len, buf)
    };

    #[rustfmt::skip]
    let opus_tags = {
        let mut buf = vec![
            b'O', b'p', b'u', b's', b'T', b'a', b'g', b's', // magic signature
            0x08, 0x00, 0x00, 0x00, // vendor string length (8u32 in little-endian)
            b'm', b'u', b's', b'i', b'c', b'o', b'p', b'y', // vendor string
        ];
        buf.extend(user_comments_len.to_le_bytes());
        buf.extend(user_comments_buf);
        buf
    };

    // stream unique serial identifier
    let serial = 0;

    // write opus head and opus tags packets
    packet_writer
        .write_packet(&opus_head, serial, ogg::PacketWriteEndInfo::EndPage, 0)
        .context("failed to write packet")?;
    packet_writer
        .write_packet(&opus_tags, serial, ogg::PacketWriteEndInfo::EndPage, 0)
        .context("failed to write packet")?;

    // number of frames per chunk (48khz / 1000 * 20ms = 960 frames)
    // NB: we are calling opus frames 'chunks' to differentiate from sample frames (one sample per channel)
    let chunk_frames = 48000 / 1000 * 20;
    let chunk_samples = chunk_frames * channel_count;

    let interleaved_len = interleaved_samples.len();

    // encode in chunks
    let mut pos = 0;
    loop {
        // check if we have enough samples for a full chunk
        if pos + chunk_samples > interleaved_len {
            break;
        }

        // allocate chunk output buffer
        // encode_float uses the length (not capacity) as max_data_size
        // length comes from recommended max_data_size in opus documentation
        let mut output_buf = vec![0; 4000];

        // call encoder with input slice and chunk output buffer
        let output_len = encoder
            .encode_float(
                &interleaved_samples[pos..(pos + chunk_samples)],
                &mut output_buf,
            )
            .context("failed to encode chunk")?;
        output_buf.truncate(output_len);

        let end_info = if pos + chunk_samples == interleaved_len {
            // if this chunk ended exactly at the end of input
            ogg::PacketWriteEndInfo::EndStream
        } else {
            ogg::PacketWriteEndInfo::NormalPacket
        };

        // the number of frames up to and including the last frame in this packet
        // this is measured in frames, so mono and stereo increase at the same rate
        let granule_position = ((pos + chunk_samples) / channel_count) as u64;

        // write packet
        packet_writer
            .write_packet(output_buf, serial, end_info, granule_position)
            .context("failed to write packet")?;

        // increment position by number of samples consumed
        pos += chunk_samples;
    }

    // encode final chunk with remaining samples
    if pos < interleaved_len {
        // allocate chunk output buffer
        let mut output_buf = vec![0; 4000];

        // opus always requires a full chunk of input but we don't have enough remaining samples,
        // so allocate a zero-padded input buffer for the final chunk
        let mut input_buf = vec![0.0; chunk_samples];
        input_buf[0..(interleaved_len - pos)]
            .copy_from_slice(&interleaved_samples[pos..interleaved_len]);

        // call encoder with chunk input buffer and chunk output buffer
        let output_len = encoder
            .encode_float(&input_buf, &mut output_buf)
            .context("failed to encode final chunk")?;
        output_buf.truncate(output_len);

        // for end-trimming, the granule position of the final packet is the total number of input frames
        // this may be less than the position of the final frame in the final packet
        // this allows the player to trim the padding samples from the final chunk
        let granule_position = (interleaved_len / channel_count) as u64;

        // write packet
        packet_writer
            .write_packet(
                output_buf,
                serial,
                ogg::PacketWriteEndInfo::EndStream,
                granule_position,
            )
            .context("failed to write packet")?;
    }

    let file = packet_writer.into_inner();
    let file_size = file
        .seek(SeekFrom::End(0))
        .context("failed to seek to end of file")?;

    // we did it
    Ok(file_size)
}

#[cfg(test)]
mod tests {
    use std::{sync::atomic::AtomicBool, time::Duration};

    use super::*;

    fn join_timeout<T>(timeout: std::time::Duration, thread: std::thread::JoinHandle<T>) -> T {
        let now = std::time::Instant::now();

        while now.elapsed() < timeout {
            if thread.is_finished() {
                return thread.join().unwrap();
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        panic!("thread timed out");
    }

    /// Asserts that a condition is true for a given duration.
    fn assert_duration(timeout: std::time::Duration, condition: impl Fn() -> bool) {
        let now = std::time::Instant::now();

        while now.elapsed() < timeout {
            if !condition() {
                panic!("condition failed before timeout");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn test_assert_duration_success() {
        let flag = Arc::new(AtomicBool::new(true));

        // set flag to false after 200ms
        std::thread::spawn({
            let flag = flag.clone();
            move || {
                std::thread::sleep(std::time::Duration::from_millis(200));
                flag.store(false, Ordering::SeqCst);
            }
        });

        // should remain true for 100ms
        assert_duration(std::time::Duration::from_millis(100), || {
            flag.load(Ordering::SeqCst)
        });
    }

    #[test]
    #[should_panic]
    fn test_assert_duration_panic() {
        let flag = Arc::new(AtomicBool::new(true));

        // set flag to false after 50ms
        std::thread::spawn({
            let flag = flag.clone();
            move || {
                std::thread::sleep(std::time::Duration::from_millis(50));
                flag.store(false, Ordering::SeqCst);
            }
        });

        // should become false before 100ms and panic
        assert_duration(std::time::Duration::from_millis(100), || {
            flag.load(Ordering::SeqCst)
        });
    }

    #[test]
    fn test_region_counter() {
        let counter = RegionCounter::new();
        assert_eq!(counter.count(), 0);

        let guard_1 = counter.entered();
        assert_eq!(counter.count(), 1);

        let guard_2 = counter.entered();
        assert_eq!(counter.count(), 2);

        drop(guard_2);
        assert_eq!(counter.count(), 1);

        drop(guard_1);
        assert_eq!(counter.count(), 0);
    }

    #[test]
    fn test_queue_wait_after() {
        let queue = Arc::new(TranscodeQueue::new(TranscodePolicy::Always));

        // add to queue
        let item_1 = PathBuf::from("item_1");
        let item_2 = PathBuf::from("item_2");
        queue.extend(vec![item_1, item_2]);

        std::thread::sleep(std::time::Duration::from_millis(100));

        // wait after adding item
        let thread = std::thread::spawn(move || {
            let item = queue.wait();
            assert_eq!(item, PathBuf::from("item_1"));
            let item = queue.wait();
            assert_eq!(item, PathBuf::from("item_2"));
        });

        join_timeout(std::time::Duration::from_secs(1), thread);
    }

    #[test]
    fn test_queue_wait_before() {
        let queue = Arc::new(TranscodeQueue::new(TranscodePolicy::Always));

        // wait before before item
        let thread = std::thread::spawn({
            let queue = queue.clone();
            move || {
                let item = queue.wait();
                assert_eq!(item, PathBuf::from("item_1"));
                let item = queue.wait();
                assert_eq!(item, PathBuf::from("item_2"));
            }
        });

        std::thread::sleep(std::time::Duration::from_millis(100));

        // add to queue
        let item_1 = PathBuf::from("item_1");
        let item_2 = PathBuf::from("item_2");
        queue.extend(vec![item_1, item_2]);

        join_timeout(std::time::Duration::from_secs(1), thread);
    }

    #[test]
    fn test_queue_wait_parallel() {
        let queue = Arc::new(TranscodeQueue::new(TranscodePolicy::Always));

        // spawn consumer threads
        let thread_1 = std::thread::spawn({
            let queue = queue.clone();
            move || {
                queue.wait();
            }
        });
        let thread_2 = std::thread::spawn({
            let queue = queue.clone();
            move || {
                queue.wait();
            }
        });

        std::thread::sleep(std::time::Duration::from_millis(100));

        // add to queue
        let item_1 = PathBuf::from("item_1");
        let item_2 = PathBuf::from("item_2");
        queue.extend(vec![item_1, item_2]);

        join_timeout(std::time::Duration::from_secs(1), thread_1);
        join_timeout(std::time::Duration::from_secs(1), thread_2);
    }

    #[test]
    fn test_queue_remove_missing() {
        let queue = Arc::new(TranscodeQueue::new(TranscodePolicy::Always));

        // add to queue
        let item_1 = PathBuf::from("item_1");
        let item_2 = PathBuf::from("item_2");
        let item_3 = PathBuf::from("item_3");
        queue.extend(vec![item_1.clone(), item_2.clone(), item_3.clone()]);

        // wait for next
        let item = queue.wait();
        assert_eq!(item, PathBuf::from("item_1"));

        // remove #2 from queue
        queue.remove_missing(&HashSet::from([item_3]));

        // wait for next
        let item = queue.wait();
        assert_eq!(item, PathBuf::from("item_3"));
    }

    #[test]
    fn test_queue_if_requested() {
        let queue = Arc::new(TranscodeQueue::new(TranscodePolicy::IfRequested));

        // add to queue
        let item_1 = PathBuf::from("item_1");
        let item_2 = PathBuf::from("item_2");
        let item_3 = PathBuf::from("item_3");
        queue.extend(vec![item_1.clone(), item_2.clone(), item_3.clone()]);

        // spawn consumer thread
        let thread_1 = std::thread::spawn({
            let queue = queue.clone();
            move || queue.wait()
        });

        // should wait and not receive item
        assert_duration(Duration::from_millis(100), || !thread_1.is_finished());

        // request #2
        queue.prioritize(HashSet::from([item_2]));

        // should receive #2
        let item = thread_1.join().unwrap();
        assert_eq!(item, PathBuf::from("item_2"));

        // spawn another consumer thread
        let thread_2 = std::thread::spawn({
            let queue = queue.clone();
            move || queue.wait()
        });

        // should wait and not receive item
        assert_duration(Duration::from_millis(100), || !thread_2.is_finished());

        // request #3
        queue.prioritize(HashSet::from([item_3]));

        // should receive #3
        let item = thread_2.join().unwrap();
        assert_eq!(item, PathBuf::from("item_3"));
    }

    #[test]
    fn test_queue_change_policy_to_always() {
        let queue = Arc::new(TranscodeQueue::new(TranscodePolicy::IfRequested));

        // add to queue
        let item_1 = PathBuf::from("item_1");
        let item_2 = PathBuf::from("item_2");
        let item_3 = PathBuf::from("item_3");
        queue.extend(vec![item_1.clone(), item_2.clone(), item_3.clone()]);

        // spawn consumer thread to wait for 3 items
        let thread_1 = std::thread::spawn({
            let queue = queue.clone();
            move || {
                queue.wait();
                queue.wait();
                queue.wait();
            }
        });

        // should wait and not receive item
        assert_duration(Duration::from_millis(100), || !thread_1.is_finished());

        // change policy to Always
        queue.set_policy(TranscodePolicy::Always);

        // should receive items and exit
        join_timeout(Duration::from_millis(100), thread_1);
    }

    #[test]
    fn test_queue_change_policy_to_if_requested() {
        let queue = Arc::new(TranscodeQueue::new(TranscodePolicy::Always));

        // add to queue
        let item_1 = PathBuf::from("item_1");
        let item_2 = PathBuf::from("item_2");
        let item_3 = PathBuf::from("item_3");
        queue.extend(vec![item_1.clone(), item_2.clone(), item_3.clone()]);

        // should receive some item
        queue.wait();

        // change policy to IfRequested
        queue.set_policy(TranscodePolicy::IfRequested);

        // spawn consumer thread to wait for 2 more items
        let thread_1 = std::thread::spawn({
            let queue = queue.clone();
            move || {
                queue.wait();
                queue.wait();
            }
        });

        // should wait and not receive item
        assert_duration(Duration::from_millis(100), || !thread_1.is_finished());

        // request all
        queue.prioritize(HashSet::from([item_1, item_2, item_3]));

        // should receive items and exit
        join_timeout(Duration::from_millis(100), thread_1);
    }

    #[test]
    fn test_queue_ready_count() {
        let queue = Arc::new(TranscodeQueue::new(TranscodePolicy::Always));

        // should have 0 ready
        assert_eq!(queue.ready_counter.load(Ordering::SeqCst), 0);

        // add to queue
        let item_1 = PathBuf::from("item_1");
        let item_2 = PathBuf::from("item_2");
        let item_3 = PathBuf::from("item_3");
        queue.extend(vec![item_1.clone(), item_2.clone(), item_3.clone()]);

        // should have 3 ready
        assert_eq!(queue.ready_counter.load(Ordering::SeqCst), 3);

        // should receive some item
        queue.wait();

        // should have 2 ready
        assert_eq!(queue.ready_counter.load(Ordering::SeqCst), 2);

        // change policy to IfRequested
        queue.set_policy(TranscodePolicy::IfRequested);

        // should have 0 ready
        assert_eq!(queue.ready_counter.load(Ordering::SeqCst), 0);

        // request #2
        queue.prioritize(HashSet::from([item_2]));

        // should have 1 ready
        assert_eq!(queue.ready_counter.load(Ordering::SeqCst), 1);

        // should receive some item
        queue.wait();

        // should have 0 ready
        assert_eq!(queue.ready_counter.load(Ordering::SeqCst), 0);
    }
}
