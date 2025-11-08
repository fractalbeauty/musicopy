pub mod database;
pub mod error;
pub mod file_dialog;
pub mod fs;
pub mod library;
pub mod model;
pub mod node;

use crate::{
    database::Database,
    error::{CoreError, core_error},
    library::{
        Library, LibraryCommand, LibraryModel,
        hash::HashCache,
        transcode::{TranscodePolicy, TranscodeStatusCache},
    },
    node::{DownloadPartialItemModel, Node, NodeCommand, NodeModel},
};
use anyhow::Context;
use iroh::{NodeAddr, NodeId, SecretKey};
use log::{debug, error};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

uniffi::setup_scaffolding!();

/// Foreign trait implemented in Compose for receiving events from the Rust core.
#[uniffi::export(with_foreign)]
pub trait EventHandler: Send + Sync {
    fn on_library_model_snapshot(&self, model: LibraryModel);
    fn on_node_model_snapshot(&self, model: NodeModel);
}

#[derive(Debug, uniffi::Record)]
pub struct ProjectDirsOptions {
    pub data_dir: String,
    pub cache_dir: String,
}

#[derive(Debug, uniffi::Record)]
pub struct CoreOptions {
    pub init_logging: bool,
    pub in_memory: bool,
    pub project_dirs: Option<ProjectDirsOptions>,
    pub transcode_policy: TranscodePolicy,
}

/// Long-lived object created by Compose as the entry point to the Rust core.
///
/// The core is split into separate logical components. Components may require
/// async initialization, and the core needs handles to them to route commands
/// and queries from the UI.
#[derive(uniffi::Object)]
pub struct Core {
    db: Arc<Mutex<Database>>,

    node: Arc<Node>,
    library: Arc<Library>,
}

// stub debug implementation
impl std::fmt::Debug for Core {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Core").finish()
    }
}

#[uniffi::export]
impl Core {
    /// Starts the app core.
    ///
    /// This is async, since components may require async initialization before
    /// their handles are ready. The constructor itself runs on the UI's async
    /// runtime, but the core spawns its own thread with a Tokio runtime for
    /// actual app logic. The constructor only uses async to wait for component
    /// handles to be ready, since they are spawned on the Tokio runtime and
    /// sent back to the constructor using channels. The UI should wait for the
    /// core before the initial render, so that it can have initial data ready
    /// immediately.
    #[uniffi::constructor]
    pub async fn start(
        event_handler: Arc<dyn EventHandler>,
        options: CoreOptions,
    ) -> Result<Arc<Self>, CoreError> {
        if options.init_logging {
            #[cfg(target_os = "android")]
            {
                android_logger::init_once(
                    android_logger::Config::default()
                        .with_max_level(log::LevelFilter::Trace) // limit log level
                        .with_tag("musicopy")
                        .with_filter(
                            android_logger::FilterBuilder::new()
                                .try_parse("musicopy=debug")
                                .expect("failed to parse log filter")
                                .build(),
                        ),
                );
            }

            #[cfg(target_os = "ios")]
            {
                let os_logger =
                    oslog::OsLogger::new("app.musicopy").level_filter(log::LevelFilter::Debug);
                let logger = env_filter::FilteredLog::new(
                    os_logger,
                    env_filter::Builder::new().parse("musicopy=debug").build(),
                );
                log::set_boxed_logger(Box::new(logger)).expect("failed to initialize logger");
            }

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                env_logger::Builder::from_env(
                    env_logger::Env::default().default_filter_or("musicopy=debug"),
                )
                .init();
            }

            log_panics::init();
        }

        debug!("core: starting core");

        let (db, secret_key, transcodes_dir) = if options.in_memory {
            let db = Database::open_in_memory().context("failed to open database")?;

            let secret_key = SecretKey::generate(rand::rngs::OsRng);

            // TODO: maybe clean contents or name uniquely each run
            let transcodes_dir = {
                // TODO: this doesn't work on android, but we only use it for transcoding for now
                let mut p = std::env::temp_dir();
                p.push("musicopy/transcodes");
                p
            };

            (db, secret_key, transcodes_dir)
        } else {
            let (data_dir, cache_dir) = match options.project_dirs {
                Some(project_dirs) => (
                    PathBuf::from(project_dirs.data_dir),
                    PathBuf::from(project_dirs.cache_dir),
                ),
                None => {
                    let project_dirs = directories_next::ProjectDirs::from("", "", "musicopy")
                        .context("failed to get project directories")?;
                    let data_dir = project_dirs.data_local_dir().to_owned();
                    let cache_dir = project_dirs.cache_dir().to_owned();

                    std::fs::create_dir_all(&data_dir)
                        .context("failed to create data directory")?;
                    std::fs::create_dir_all(&cache_dir)
                        .context("failed to create cache directory")?;

                    (data_dir, cache_dir)
                }
            };

            let db_path = data_dir.join("musicopy_v1.db");
            let db = Database::open_file(&db_path).context("failed to open database")?;

            let key_path = data_dir.join("secret_key");
            let secret_key = if key_path.exists() {
                let key_bytes =
                    std::fs::read(&key_path).context("failed to read secret key file")?;
                SecretKey::from_bytes(
                    key_bytes[..32]
                        .try_into()
                        .context("failed to parse secret key file")?,
                )
            } else {
                let new_key = SecretKey::generate(rand::rngs::OsRng);
                std::fs::write(&key_path, new_key.to_bytes())
                    .context("failed to write secret key file")?;
                new_key
            };

            let transcodes_dir = cache_dir.join("transcodes");

            (db, secret_key, transcodes_dir)
        };
        let db = Arc::new(Mutex::new(db));

        let transcode_status_cache = TranscodeStatusCache::new();
        let hash_cache = HashCache::new(db.clone());

        let node_id = NodeId::from(secret_key.public());

        let (res_tx, res_rx) = tokio::sync::oneshot::channel();

        // spawn node thread
        std::thread::spawn({
            let db = db.clone();
            move || {
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    // TODO: tune number of threads on mobile?
                    let builder = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .expect("should build runtime");

                    builder.block_on(async move {
                        debug!("core: inside async runtime");

                        // initialize components concurrently
                        let (library_res, node_res) = tokio::join!(
                            Library::new(
                                event_handler.clone(),
                                db.clone(),
                                node_id,
                                transcodes_dir.clone(),
                                options.transcode_policy,
                                transcode_status_cache.clone(),
                                hash_cache.clone(),
                            ),
                            Node::new(
                                event_handler,
                                secret_key,
                                db,
                                transcode_status_cache,
                                hash_cache
                            ),
                        );

                        let (library, library_run) = match library_res {
                            Ok(x) => x,
                            Err(e) => {
                                error!("core: error creating library: {e:#}");
                                res_tx
                                    .send(Err(core_error!("failed to create library")))
                                    .expect("failed to send result to core constructor");
                                return;
                            }
                        };

                        let (node, node_run) = match node_res {
                            Ok(x) => x,
                            Err(e) => {
                                error!("core: error creating node: {e:#}");
                                res_tx
                                    .send(Err(core_error!("failed to create node")))
                                    .expect("failed to send result to core constructor");
                                return;
                            }
                        };

                        res_tx
                            .send(Ok((library.clone(), node.clone())))
                            .expect("failed to send result to core constructor");

                        let mut library_task = tokio::spawn({
                            let library = library.clone();
                            async move {
                                if let Err(e) = library.run(library_run).await {
                                    error!("core: error running library: {e:#}");
                                }
                            }
                        });

                        let mut node_task = tokio::spawn({
                            let node = node.clone();
                            let library = library.clone();
                            async move {
                                if let Err(e) = node.run(node_run, library).await {
                                    error!("core: error running node: {e:#}");
                                }
                            }
                        });

                        let res = tokio::try_join!(&mut library_task, &mut node_task);
                        if let Some(e) = res.err() {
                            error!("core: error in main tasks: {e:#}");
                        }

                        debug!("core: async runtime exiting");
                    });
                }));

                if let Err(e) = &res {
                    error!("core: panic in core thread: {e:?}");

                    if let Some(s) = e.downcast_ref::<std::string::String>() {
                        error!("core: panic info: {s}");
                    }
                    if let Some(s) = e.downcast_ref::<&'static str>() {
                        error!("core: panic info: {s}");
                    }
                }
            }
        });

        let (library, node) = async_std::future::timeout(Duration::from_secs(10), res_rx)
            .await
            .map_err(|_elapsed| core_error!("timed out waiting for core components to initialize"))?
            .map_err(|_dropped| {
                core_error!("core components failed to initialize, sender dropped")
            })?
            .context("core components failed to initialize")?;

        Ok(Arc::new(Self { db, library, node }))
    }

    pub fn shutdown(&self) -> Result<(), CoreError> {
        debug!("core: shutting down");

        self.node
            .send(NodeCommand::Stop)
            .context("failed to send stop command to node thread")?;

        self.library
            .send(LibraryCommand::Stop)
            .context("failed to send stop command to library thread")?;

        Ok(())
    }

    pub fn get_node_model(&self) -> Result<NodeModel, CoreError> {
        Ok(self.node.get_model())
    }

    pub fn get_library_model(&self) -> Result<LibraryModel, CoreError> {
        Ok(self.library.get_model())
    }

    pub async fn connect(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;
        let node_addr = NodeAddr::from(node_id);

        let (callback_tx, callback_rx) = tokio::sync::oneshot::channel();

        self.node
            .send(NodeCommand::Connect {
                addr: node_addr,
                callback: callback_tx,
            })
            .context("failed to send to node thread")?;

        async_std::future::timeout(Duration::from_secs(10), callback_rx)
            .await
            .map_err(|_elapsed| core_error!("connect timed out"))?
            .map_err(|_dropped| core_error!("connect failed, sender dropped"))?
            .map_err(CoreError::from)
    }

    pub fn set_download_directory(&self, download_directory: &str) -> Result<(), CoreError> {
        self.node
            .send(NodeCommand::SetDownloadDirectory(download_directory.into()))
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn download_all(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        self.node
            .send(NodeCommand::DownloadAll { client: node_id })
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn download_partial(
        &self,
        node_id: &str,
        items: Vec<DownloadPartialItemModel>,
    ) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        self.node
            .send(NodeCommand::DownloadPartial {
                client: node_id,
                items,
            })
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn accept_connection(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        self.node
            .send(NodeCommand::AcceptConnection(node_id))
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn accept_connection_and_trust(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        self.node
            .send(NodeCommand::AcceptConnection(node_id))
            .context("failed to send to node thread")?;

        self.node
            .send(NodeCommand::TrustNode(node_id))
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn deny_connection(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        self.node
            .send(NodeCommand::DenyConnection(node_id))
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn close_client(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        self.node
            .send(NodeCommand::CloseClient(node_id))
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn close_server(&self, node_id: &str) -> Result<(), CoreError> {
        let node_id: NodeId = node_id.parse().context("failed to parse node id")?;

        self.node
            .send(NodeCommand::CloseServer(node_id))
            .context("failed to send to node thread")?;

        Ok(())
    }

    pub fn add_library_root(&self, name: String, path: String) -> Result<(), CoreError> {
        self.library
            .send(LibraryCommand::AddRoot { name, path })
            .context("failed to send to library thread")?;

        Ok(())
    }

    pub fn remove_library_root(&self, name: String) -> Result<(), CoreError> {
        self.library
            .send(LibraryCommand::RemoveRoot { name })
            .context("failed to send to library thread")?;

        Ok(())
    }

    // TODO: async wait for completion or return progress somehow
    pub fn rescan_library(&self) -> Result<(), CoreError> {
        self.library
            .send(LibraryCommand::Rescan)
            .context("failed to send to library thread")?;
        Ok(())
    }

    pub fn set_transcode_policy(&self, transcode_policy: TranscodePolicy) -> Result<(), CoreError> {
        self.library
            .send(LibraryCommand::SetTranscodePolicy(transcode_policy))
            .context("failed to send to library thread")?;
        Ok(())
    }

    pub fn reset_database(&self) -> Result<(), CoreError> {
        let db = self
            .db
            .lock()
            .map_err(|_elapsed| core_error!("failed to lock database"))?;

        db.reset()?;

        Ok(())
    }

    pub fn write_test_file(&self, root: String) -> Result<(), CoreError> {
        self.node
            .send(NodeCommand::WriteTestFile(root))
            .context("failed to send to node thread")?;
        Ok(())
    }
}

// on android, expose a jni function to initialize ndk_context
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_app_musicopy_RustNdkContext_init(
    env: jni::JNIEnv,
    _class: jni::objects::JClass,
    context: jni::objects::JObject,
) {
    let java_vm = env.get_java_vm().expect("failed to get java vm");
    let java_vm_ptr = java_vm.get_java_vm_pointer() as *mut std::ffi::c_void;

    // turn the local context reference into a global reference
    let context = env
        .new_global_ref(context)
        .expect("failed to create global ref for context");
    let context_ptr = context.as_raw() as *mut std::ffi::c_void;

    // leak the context global reference so it stays alive forever
    Box::leak(Box::new(context));

    unsafe {
        ndk_context::initialize_android_context(java_vm_ptr, context_ptr);
    }
}

#[uniffi::export]
pub fn log_trace(message: String) {
    log::trace!("compose: {}", message);
}

#[uniffi::export]
pub fn log_debug(message: String) {
    log::debug!("compose: {}", message);
}

#[uniffi::export]
pub fn log_info(message: String) {
    log::info!("compose: {}", message);
}

#[uniffi::export]
pub fn log_warn(message: String) {
    log::warn!("compose: {}", message);
}

#[uniffi::export]
pub fn log_error(message: String) {
    log::error!("compose: {}", message);
}
