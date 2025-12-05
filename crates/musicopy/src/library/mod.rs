pub mod hash;
pub mod transcode;

use crate::{
    EventHandler,
    database::{Database, InsertFile},
    library::{
        hash::HashCache,
        transcode::{TranscodeCommand, TranscodePolicy, TranscodePool, TranscodeStatusCache},
    },
    model::CounterModel,
    node::FileSizeModel,
};
use anyhow::Context;
use iroh::NodeId;
use itertools::Itertools;
use log::warn;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{Notify, mpsc};

#[derive(Debug, Clone, uniffi::Record)]
pub struct LibraryRootModel {
    pub name: String,
    pub path: String,
    pub num_files: u64,
}

/// Library state sent to the UI.
///
/// Needs to be Clone to send snapshots to the UI.
#[derive(Debug, Clone, uniffi::Record)]
pub struct LibraryModel {
    pub local_roots: Vec<LibraryRootModel>,

    pub transcodes_dir: String,
    pub transcodes_dir_size: FileSizeModel,

    pub transcode_count_queued: Arc<CounterModel>,
    pub transcode_count_inprogress: Arc<CounterModel>,
    pub transcode_count_ready: Arc<CounterModel>,
    pub transcode_count_failed: Arc<CounterModel>,

    pub transcode_policy: TranscodePolicy,
}

#[derive(Debug)]
pub enum LibraryCommand {
    AddRoot { name: String, path: String },
    RemoveRoot { name: String },
    Rescan,

    PrioritizeTranscodes(HashSet<PathBuf>),
    SetTranscodePolicy(TranscodePolicy),

    DeleteUnusedTranscodes,
    DeleteAllTranscodes,

    Stop,
}

/// An update to the library model.
enum LibraryModelUpdate {
    UpdateLocalRoots,
    UpdateTranscodesDirSize,
    SetTranscodePolicy(TranscodePolicy),
}

pub struct Library {
    event_handler: Arc<dyn EventHandler>,
    db: Arc<Mutex<Database>>,
    local_node_id: NodeId,

    transcode_pool: TranscodePool,

    command_tx: mpsc::UnboundedSender<LibraryCommand>,

    scan_notify: Arc<Notify>,

    model: Mutex<LibraryModel>,
}

// stub debug implementation
impl std::fmt::Debug for Library {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Library").finish()
    }
}

/// The resources needed to run the Library run loop.
///
/// This is created by Library::new() and passed linearly to Library::run().
/// This pattern allows the run loop to own and mutate these resources while
/// hiding the details from the public API.
#[derive(Debug)]
pub struct LibraryRun {
    command_rx: mpsc::UnboundedReceiver<LibraryCommand>,
}

impl Library {
    pub async fn new(
        event_handler: Arc<dyn EventHandler>,
        db: Arc<Mutex<Database>>,
        local_node_id: NodeId,
        transcodes_dir: PathBuf,
        transcode_policy: TranscodePolicy,
        transcode_status_cache: TranscodeStatusCache,
        hash_cache: HashCache,
    ) -> anyhow::Result<(Arc<Self>, LibraryRun)> {
        // spawn transcode pool task
        let transcode_pool = TranscodePool::spawn(
            transcodes_dir.clone(),
            transcode_policy,
            transcode_status_cache,
            hash_cache,
        );

        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let model = LibraryModel {
            local_roots: Vec::new(),

            transcodes_dir: transcode_pool.transcodes_dir(),
            transcodes_dir_size: transcode_pool.transcodes_dir_size(),

            transcode_count_queued: Arc::new(transcode_pool.queued_count_model()),
            transcode_count_inprogress: Arc::new(transcode_pool.inprogress_count_model()),
            transcode_count_ready: Arc::new(transcode_pool.ready_count_model()),
            transcode_count_failed: Arc::new(transcode_pool.failed_count_model()),

            transcode_policy,
        };

        let library = Arc::new(Self {
            event_handler,
            db,
            local_node_id,

            transcode_pool,

            command_tx,

            scan_notify: Arc::new(Notify::new()),

            model: Mutex::new(model),
        });

        // initialize model
        // TODO: don't push updates during init
        library.update_model(LibraryModelUpdate::UpdateLocalRoots);

        // send all local files to the transcode pool to be transcoded if needed
        library
            .check_transcodes()
            .context("failed to check transcodes")?;

        // spawn scan task
        tokio::spawn({
            let library = library.clone();
            async move {
                loop {
                    library.scan_notify.notified().await;

                    let start = std::time::Instant::now();
                    log::debug!("Library: starting scan");

                    if let Err(e) = library.scan().await {
                        log::error!("Library: error during scan: {e:#}");
                    }

                    let elapsed = start.elapsed().as_secs_f64();
                    log::debug!("Library: finished library scan in {elapsed:.2}s");

                    // update root file counts in model
                    library.update_model(LibraryModelUpdate::UpdateLocalRoots);
                }
            }
        });

        // spawn transcodes dir size polling task
        tokio::spawn({
            let library = library.clone();
            async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    library.update_model(LibraryModelUpdate::UpdateTranscodesDirSize);
                }
            }
        });

        let library_run = LibraryRun { command_rx };

        Ok((library, library_run))
    }

    pub async fn run(self: &Arc<Self>, run_token: LibraryRun) -> anyhow::Result<()> {
        let LibraryRun { mut command_rx } = run_token;

        loop {
            tokio::select! {
                Some(command) = command_rx.recv() => {
                    match command {
                        LibraryCommand::AddRoot { name, path } => {
                            {
                                let db = self.db.lock().unwrap();
                                let path = PathBuf::from(path);
                                let path = path.canonicalize().context("failed to canonicalize path")?;
                                db.add_root(self.local_node_id, &name, &path.to_string_lossy()).context("failed to add root")?;
                            }

                            // update model
                            self.update_model(LibraryModelUpdate::UpdateLocalRoots);

                            // rescan the library
                            self.scan_notify.notify_one();
                        }

                        LibraryCommand::RemoveRoot { name } => {
                            {
                                let db = self.db.lock().unwrap();
                                db.delete_root_by_name(self.local_node_id, &name).context("failed to delete root")?;
                            }

                            // TODO: remove files from root

                            // update model
                            self.update_model(LibraryModelUpdate::UpdateLocalRoots);

                            // rescan the library
                            self.scan_notify.notify_one();
                        }

                        LibraryCommand::Rescan => {
                            self.scan_notify.notify_one();
                        }

                        LibraryCommand::PrioritizeTranscodes(paths) => {
                            if let Err(e) = self.transcode_pool.send(TranscodeCommand::Prioritize(paths)) {
                                warn!("LibraryCommand::PrioritizeTranscodes: failed to send to transcode pool: {e:#}");
                            }
                        }

                        LibraryCommand::SetTranscodePolicy(transcode_policy) => {
                            if let Err(e) = self.transcode_pool.send(TranscodeCommand::SetPolicy(transcode_policy)) {
                                warn!("LibraryCommand::SetTranscodePolicy: failed to send to transcode pool: {e:#}");
                            }

                            // update model
                            self.update_model(LibraryModelUpdate::SetTranscodePolicy(transcode_policy));
                        }

                        LibraryCommand::DeleteUnusedTranscodes => {
                            // get local file paths
                            let local_files = {
                                let db = self.db.lock().expect("failed to lock database");
                                db.get_files_by_node_id(self.local_node_id)
                                    .context("failed to get local files")?
                                    .into_iter()
                                    .map(|f| PathBuf::from(f.local_path))
                                    .collect()
                            };

                            if let Err(e) = self.transcode_pool.send(TranscodeCommand::DeleteMissing(local_files)) {
                                warn!("LibraryCommand::DeleteUnusedTranscodes: failed to send to transcode pool: {e:#}");
                            }
                        }

                        LibraryCommand::DeleteAllTranscodes => {
                            if let Err(e) = self.transcode_pool.send(TranscodeCommand::DeleteAll) {
                                warn!("LibraryCommand::DeleteAllTranscodes: failed to send to transcode pool: {e:#}");
                            }
                        }

                        LibraryCommand::Stop => {
                            break;
                        }
                    }
                }

                else => {
                    log::warn!("all senders dropped in Library::run, shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn scan(self: &Arc<Self>) -> anyhow::Result<()> {
        let mut errors = Vec::new();

        let roots = {
            let db = self.db.lock().unwrap();
            db.get_roots_by_node_id(self.local_node_id)
                .context("failed to get local roots")?
        };

        log::info!("scan: scanning {} roots", roots.len());

        // remove roots that don't exist
        let roots = roots
            .into_iter()
            .filter(|root| {
                let path = PathBuf::from(&root.path);
                if path.exists() {
                    true
                } else {
                    errors.push(anyhow::anyhow!(
                        "root path `{}` does not exist",
                        path.display()
                    ));
                    false
                }
            })
            .collect::<Vec<_>>();

        // walk roots and collect entries
        let (entries, walk_errors): (Vec<_>, Vec<_>) = roots
            .iter()
            .flat_map(|root| {
                let walker = globwalk::GlobWalkerBuilder::new(
                    &root.path,
                    "*.{mp3,flac,ogg,m4a,wav,aif,aiff}",
                )
                .file_type(globwalk::FileType::FILE)
                .build()
                .expect("glob shouldn't fail");

                walker.into_iter().map_ok(move |entry| (root, entry))
            })
            .partition_result();

        log::info!("scan: found {} files", entries.len());

        // extend errors
        errors.extend(
            walk_errors
                .into_iter()
                .map(|e| anyhow::anyhow!("failed to scan file {:?}: {}", e.path(), e)),
        );

        struct ScanItem {
            root: String,
            path: String,
            local_path: String,
        }

        let (items, scan_errors): (Vec<_>, Vec<_>) = entries
            .into_iter()
            .map(|(root, entry)| {
                let local_path = entry.into_path();

                // get path without root
                let path = local_path
                    .strip_prefix(&root.path)
                    .context("failed to strip root path prefix")?;

                // strip leading separator if present
                //
                // this happens when the root is a verbatim UNC path like \\?\UNC\\server\share,
                // which is parsed as Component::Prefix instead of Component::Prefix + Component::RootDir,
                // so the leading separator isn't stripped above and needs to be stripped here.
                //
                // even if some other case is possible, the path is always supposed to be
                // relative to the root, so it should be fine to strip it here.
                let path = if path.starts_with(std::path::MAIN_SEPARATOR_STR) {
                    path.strip_prefix(std::path::MAIN_SEPARATOR_STR)
                        .context("failed to strip leading separator")?
                } else {
                    path
                };

                // convert to slash path (replace backslashes on windows)
                use path_slash::PathExt;
                let path = path.to_slash_lossy().to_string();

                anyhow::Result::Ok(ScanItem {
                    root: root.name.clone(),
                    path,
                    local_path: local_path.to_string_lossy().to_string(),
                })
            })
            .partition_result();

        // extend errors
        errors.extend(
            scan_errors
                .into_iter()
                .map(|e: anyhow::Error| e.context("failed to scan file")),
        );

        for error in errors {
            log::error!("error scanning library: {error:#}");
        }

        {
            let mut db = self.db.lock().unwrap();
            db.replace_local_files(
                self.local_node_id,
                items.iter().map(|item| InsertFile {
                    root: &item.root,
                    path: &item.path,
                    local_tree: "", // local_tree is only used for remote files
                    local_path: &item.local_path,
                }),
            )
            .context("failed to insert files into database")?;
        }

        log::info!("scan: inserted {} files into database", items.len());

        // send local files to transcode pool
        let items = items
            .into_iter()
            .map(|item| PathBuf::from(item.local_path))
            .collect::<HashSet<_>>();

        self.transcode_pool.send(TranscodeCommand::Load(items))?;

        Ok(())
    }

    /// Send all local files to the transcode pool to be transcoded if needed.
    fn check_transcodes(&self) -> anyhow::Result<()> {
        let local_files = {
            let db = self.db.lock().expect("failed to lock database");
            db.get_files_by_node_id(self.local_node_id)
                .context("failed to get local files")?
        };

        let items = local_files
            .into_iter()
            .map(|file| PathBuf::from(file.local_path))
            .collect::<HashSet<_>>();

        self.transcode_pool.send(TranscodeCommand::Load(items))?;

        Ok(())
    }

    pub fn send(self: &Arc<Self>, command: LibraryCommand) -> anyhow::Result<()> {
        self.command_tx
            .send(command)
            .map_err(|e| anyhow::anyhow!("failed to send command: {e:?}"))
    }

    pub fn get_model(self: &Arc<Self>) -> LibraryModel {
        let model = self.model.lock().unwrap();
        model.clone()
    }

    // TODO: throttle pushing updates?
    fn update_model(self: &Arc<Self>, update: LibraryModelUpdate) {
        match update {
            LibraryModelUpdate::UpdateLocalRoots => {
                let local_roots = {
                    let db = self.db.lock().unwrap();
                    db.get_roots_by_node_id(self.local_node_id)
                        .expect("failed to get local roots")
                        .into_iter()
                        .map(|root| {
                            let count = db
                                .count_files_by_root(self.local_node_id, &root.name)
                                .expect("failed to count files"); // TODO

                            // de-UNC paths on windows (\\?\C:\foo -> C:\foo)
                            let path = PathBuf::from(root.path);
                            let path = dunce::simplified(&path).to_string_lossy().to_string();

                            LibraryRootModel {
                                name: root.name,
                                path,
                                num_files: count,
                            }
                        })
                        .collect()
                };

                let mut model = self.model.lock().unwrap();
                model.local_roots = local_roots;

                self.event_handler.on_library_model_snapshot(model.clone());
            }

            LibraryModelUpdate::UpdateTranscodesDirSize => {
                let mut model = self.model.lock().unwrap();
                model.transcodes_dir_size = self.transcode_pool.transcodes_dir_size();

                self.event_handler.on_library_model_snapshot(model.clone());
            }

            LibraryModelUpdate::SetTranscodePolicy(transcode_policy) => {
                let mut model = self.model.lock().unwrap();
                model.transcode_policy = transcode_policy;

                self.event_handler.on_library_model_snapshot(model.clone());
            }
        }
    }
}
