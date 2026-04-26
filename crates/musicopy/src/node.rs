//! Networking.
//!
//! A `Node` is an Iroh node that can perform the client or server end of the protocol.
//!
//! A `Client` is an outgoing connection to a server.
//! Clients request files, primarily used in the mobile app.
//!
//! A `Server` is an incoming connection from a client.
//! Servers send files, primarily used in the desktop app.

#[cfg(feature = "test-hooks")]
use crate::TestHooks;
use crate::{
    EventHandler,
    database::{Database, InsertFile},
    device_name::device_name,
    fs::{OpenMode, TreeFile, TreePath},
    library::{
        Library, LibraryCommand,
        hash::HashCache,
        transcode::{TranscodeFormat, TranscodeStatus, TranscodeStatusCache, estimate_file_size},
    },
    model::CounterModel,
    protocol::{
        ClientMessageV1, DownloadItem, FileSize, IndexItem, IndexUpdateItem, JobStatusItem,
        ServerMessageV1,
    },
};
use anyhow::Context;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt, TryStreamExt};
use iroh::{
    Endpoint, EndpointAddr, EndpointId, SecretKey, Watcher,
    endpoint::{Connection, presets::N0},
    protocol::{AcceptError, ProtocolHandler, Router},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};
use tokio::{
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{Notify, mpsc, oneshot},
};
use tokio_util::{
    bytes::Bytes,
    codec::{FramedRead, FramedWrite, LengthDelimitedCodec},
};
use tracing::{debug, error, info, warn};

/// Model of progress for a transfer job.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum TransferJobProgressModel {
    Requested,
    Transcoding,
    Ready,
    InProgress {
        started_at: u64,
        /// Number of bytes written so far.
        bytes: Arc<CounterModel>,
    },
    Finished {
        finished_at: u64,
    },
    Failed {
        error: String,
    },
}

/// Model of a transfer job.
#[derive(Debug, Clone, uniffi::Record)]
pub struct TransferJobModel {
    pub job_id: u64,
    pub file_root: String,
    pub file_path: String,
    pub file_size: Option<u64>,
    pub progress: TransferJobProgressModel,
}

/// Model of the state of a server connection.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum ServerStateModel {
    Pending,
    Accepted,
    Closed { error: Option<String> },
}

/// Model of an incoming connection.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ServerModel {
    pub name: String,
    pub endpoint_id: String,
    pub connected_at: u64,

    pub state: ServerStateModel,

    pub connection_type: String,
    pub latency_ms: Option<u64>,

    pub transfer_jobs: Vec<TransferJobModel>,
}

/// Model of an unknown, estimated, or actual file size.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum FileSizeModel {
    Unknown,
    Estimated(u64),
    Actual(u64),
}

/// Model of the download status of an item in a client's index.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum IndexItemDownloadStatusModel {
    Waiting,
    InProgress,
    Downloaded,
    Failed,
}

/// Model of an item in the index sent by the server.
#[derive(Debug, Clone, uniffi::Record)]
pub struct IndexItemModel {
    pub endpoint_id: String,
    pub root: String,
    pub path: String,

    pub file_size: FileSizeModel,

    pub download_status: Option<IndexItemDownloadStatusModel>,
}

/// Model of the state of a client connection.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum ClientStateModel {
    Pending,
    Accepted,
    Closed { error: Option<String> },
}

/// Model of an outgoing connection.
#[derive(Debug, Clone, uniffi::Record)]
pub struct ClientModel {
    pub name: String,
    pub endpoint_id: String,
    pub connected_at: u64,

    pub state: ClientStateModel,

    pub connection_type: String,
    pub latency_ms: Option<u64>,

    pub index: Option<Vec<IndexItemModel>>,
    pub transfer_jobs: Vec<TransferJobModel>,
    pub paused: bool,
}

/// Model of a trusted node.
#[derive(Debug, Clone, uniffi::Record)]
pub struct TrustedNodeModel {
    pub endpoint_id: String,
    pub name: String,
    pub connected_at: Option<u64>,
}

/// Model of a recently connected server.
#[derive(Debug, Clone, uniffi::Record)]
pub struct RecentServerModel {
    pub endpoint_id: String,
    pub name: String,
    pub connected_at: u64,
}

/// Node state sent to the UI.
///
/// Needs to be Clone to send snapshots to the UI.
#[derive(Debug, Clone, uniffi::Record)]
pub struct NodeModel {
    pub endpoint_id: String,

    pub home_relay: String,

    pub send_ipv4: u64,
    pub send_ipv6: u64,
    pub send_relay: u64,
    pub recv_ipv4: u64,
    pub recv_ipv6: u64,
    pub recv_relay: u64,
    pub conn_success: u64,
    pub conn_direct: u64,

    pub servers: HashMap<String, ServerModel>,
    pub clients: HashMap<String, ClientModel>,

    pub trusted_nodes: Vec<TrustedNodeModel>,
    pub recent_servers: Vec<RecentServerModel>,
}

/// Model of an item selected to be downloaded.
#[derive(Debug, Clone, uniffi::Record)]
pub struct DownloadRequestModel {
    pub endpoint_id: String,
    pub root: String,
    pub path: String,
}

/// A command sent by the UI to the node.
#[derive(Debug)]
pub enum NodeCommand {
    SetDownloadDirectory(String),

    Connect {
        /// Transcode format for transcoding, or None to transfer original files.
        transcode_format: Option<TranscodeFormat>,
        addr: EndpointAddr,
        callback: oneshot::Sender<anyhow::Result<()>>,
    },

    AcceptConnection(EndpointId),
    DenyConnection(EndpointId),

    CloseClient(EndpointId),
    CloseServer(EndpointId),

    RefreshClientIndex(EndpointId),

    SetDownloads {
        client: EndpointId,
        items: Vec<DownloadRequestModel>,
    },
    PauseDownloads {
        client: EndpointId,
    },

    TrustNode(EndpointId),
    UntrustNode(EndpointId),

    RefreshModel,

    Stop,

    // unused, for debugging filesystem code
    WriteTestFile(String),
}

/// An event sent from a server or client to the node.
enum NodeEvent {
    FilesRequested(TranscodeFormat, HashSet<PathBuf>),

    TrustedNodesChanged,
    RecentServersChanged,

    ServerOpened {
        endpoint_id: EndpointId,
        handle: ServerHandle,

        name: String,
        connected_at: u64,
    },
    ServerChanged {
        endpoint_id: EndpointId,
        update: ServerModelUpdate,
    },
    ServerClosed {
        endpoint_id: EndpointId,
        error: Option<String>,
    },

    ClientOpened {
        endpoint_id: EndpointId,
        handle: ClientHandle,

        name: String,
        connected_at: u64,
    },
    ClientChanged {
        endpoint_id: EndpointId,
        update: ClientModelUpdate,
    },
    ClientClosed {
        endpoint_id: EndpointId,
        error: Option<String>,
    },

    ServerTransferCompleted {
        endpoint_id: EndpointId,
        bytes: u64,
        is_first_transfer: bool,
    },
    ClientTransferCompleted {
        endpoint_id: EndpointId,
        bytes: u64,
        is_first_transfer: bool,
    },
}

/// An update to a server model.
enum ServerModelUpdate {
    Accept,
    UpdateConnectionInfo {
        remote_addr: String,
        rtt_ms: Option<u64>,
    },
    UpdateTransferJobs,
    Close {
        error: Option<String>,
    },
}

/// An update to a client model.
enum ClientModelUpdate {
    Accept,
    UpdateConnectionInfo {
        remote_addr: String,
        rtt_ms: Option<u64>,
    },
    UpdateIndex,
    UpdateTransferJobs,
    UpdatePaused,
    Close {
        error: Option<String>,
    },
}

/// An update to the node model.
enum NodeModelUpdate {
    PollMetrics,
    UpdateHomeRelay {
        home_relay: String,
    },
    UpdateTrustedNodes,
    UpdateRecentServers,

    CreateServer {
        endpoint_id: EndpointId,
        name: String,
        connected_at: u64,
    },
    UpdateServer {
        endpoint_id: EndpointId,
        update: ServerModelUpdate,
    },

    CreateClient {
        endpoint_id: EndpointId,
        name: String,
        connected_at: u64,
    },
    UpdateClient {
        endpoint_id: EndpointId,
        update: ClientModelUpdate,
    },
}

pub struct Node {
    event_handler: Arc<dyn EventHandler>,
    db: Arc<Mutex<Database>>,

    router: Router,

    command_tx: mpsc::UnboundedSender<NodeCommand>,
    event_tx: mpsc::UnboundedSender<NodeEvent>,

    servers: Mutex<HashMap<EndpointId, ServerHandle>>,
    clients: Mutex<HashMap<EndpointId, ClientHandle>>,

    download_directory: Arc<Mutex<Option<String>>>,

    model: Mutex<NodeModel>,

    #[cfg(feature = "test-hooks")]
    test_hooks: Arc<TestHooks>,
}

// stub debug implementation
impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node").finish()
    }
}

/// The resources needed to run the Node run loop.
///
/// This is created by Node::new() and passed linearly to Node::run(). This
/// pattern allows the run loop to own and mutate these resources while hiding
/// the details from the public API.
#[derive(Debug)]
pub struct NodeRun {
    command_rx: mpsc::UnboundedReceiver<NodeCommand>,
    event_rx: mpsc::UnboundedReceiver<NodeEvent>,
}

impl Node {
    pub async fn new(
        event_handler: Arc<dyn EventHandler>,
        secret_key: SecretKey,
        db: Arc<Mutex<Database>>,
        transcode_status_cache: TranscodeStatusCache,
        hash_cache: HashCache,
        #[cfg(feature = "test-hooks")] test_hooks: Arc<TestHooks>,
    ) -> anyhow::Result<(Arc<Self>, NodeRun)> {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let endpoint = Endpoint::builder(N0).secret_key(secret_key).bind().await?;
        let protocol = Protocol::new(
            db.clone(),
            transcode_status_cache.clone(),
            hash_cache.clone(),
            event_tx.clone(),
        );

        let router = Router::builder(endpoint)
            .accept(Protocol::ALPN, protocol.clone())
            .spawn();

        let model = NodeModel {
            endpoint_id: router.endpoint().id().to_string(),

            home_relay: "none".to_string(), // TODO

            send_ipv4: 0,
            send_ipv6: 0,
            send_relay: 0,
            recv_ipv4: 0,
            recv_ipv6: 0,
            recv_relay: 0,
            conn_success: 0,
            conn_direct: 0,

            servers: HashMap::new(),
            clients: HashMap::new(),

            trusted_nodes: Default::default(),
            recent_servers: Vec::new(),
        };

        let node = Arc::new(Self {
            event_handler,
            db,

            router,

            command_tx,
            event_tx,

            servers: Mutex::new(HashMap::new()),
            clients: Mutex::new(HashMap::new()),

            download_directory: Arc::new(Mutex::new(None)),

            model: Mutex::new(model),

            #[cfg(feature = "test-hooks")]
            test_hooks,
        });

        // initialize model
        // TODO: ideally don't push updates here...
        node.update_model(NodeModelUpdate::PollMetrics);
        node.update_model(NodeModelUpdate::UpdateTrustedNodes);
        node.update_model(NodeModelUpdate::UpdateRecentServers);

        // spawn task to check downloaded remote files
        tokio::spawn({
            let node = node.clone();
            async move {
                if let Err(e) = node.check_remote_files().await {
                    error!("Node::new: failed to check remote files: {e:#}");
                }
            }
        });

        // spawn metrics polling task
        tokio::spawn({
            let node = node.clone();
            async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    node.update_model(NodeModelUpdate::PollMetrics);
                }
            }
        });

        // spawn home relay watcher task
        {
            let node = node.clone();
            let mut addr_stream = node.router.endpoint().watch_addr().stream();
            let endpoint_closed = node.router.endpoint().closed();
            tokio::spawn(endpoint_closed.run_until(async move {
                while let Some(addr) = addr_stream.next().await {
                    let home_relay = addr
                        .relay_urls()
                        .next()
                        .map(|relay_url| relay_url.to_string())
                        .unwrap_or_else(|| "none".to_string());
                    node.update_model(NodeModelUpdate::UpdateHomeRelay { home_relay });
                }
            }));
        }

        let node_run = NodeRun {
            command_rx,
            event_rx,
        };

        Ok((node, node_run))
    }

    pub async fn run(
        self: &Arc<Self>,
        run_token: NodeRun,
        library: Arc<Library>,
    ) -> anyhow::Result<()> {
        let NodeRun {
            mut command_rx,
            mut event_rx,
        } = run_token;

        // Track launch
        {
            let db = self.db.lock().unwrap();
            let _ = db.track_launch();
        }
        self.push_stats_model();

        loop {
            tokio::select! {
                Some(command) = command_rx.recv() => {
                    match command {
                        NodeCommand::SetDownloadDirectory(path) => {
                            let mut download_directory = self.download_directory.lock().unwrap();
                            *download_directory = Some(path);
                        },

                        NodeCommand::Connect { transcode_format, addr, callback } => {
                            let node = self.clone();
                            tokio::task::spawn(async move {
                                debug!("starting connect");
                                let res = node.connect(transcode_format, addr).await;
                                debug!("connect result: {res:?}");
                                if let Err(e) = callback.send(res) {
                                    error!("failed to send res: {e:?}");
                                }
                            });
                        },

                        NodeCommand::AcceptConnection(endpoint_id) => {
                            let servers = self.servers.lock().unwrap();
                            if let Some(server_handle) = servers.get(&endpoint_id) {
                                server_handle.tx.send(ServerCommand::Accept).expect("failed to send ServerCommand::Accept");
                            } else {
                                error!("AcceptConnection: no server found with endpoint_id: {endpoint_id}");
                            }
                        },
                        NodeCommand::DenyConnection(endpoint_id) => {
                            let servers = self.servers.lock().unwrap();
                            if let Some(server_handle) = servers.get(&endpoint_id) {
                                server_handle.tx.send(ServerCommand::Close).expect("failed to send ServerCommand::Close");
                            } else {
                                error!("DenyConnection: no server found with endpoint_id: {endpoint_id}");
                            }
                        },

                        NodeCommand::CloseClient(endpoint_id) => {
                            let clients = self.clients.lock().unwrap();
                            if let Some(client_handle) = clients.get(&endpoint_id) {
                                client_handle.tx.send(ClientCommand::Close).expect("failed to send ClientCommand::Close");
                            } else {
                                error!("CloseClient: no client found with endpoint_id: {endpoint_id}");
                            }
                        }
                        NodeCommand::CloseServer(endpoint_id) => {
                            let servers = self.servers.lock().unwrap();
                            if let Some(server_handle) = servers.get(&endpoint_id) {
                                server_handle.tx.send(ServerCommand::Close).expect("failed to send ServerCommand::Close");
                            } else {
                                error!("CloseServer: no server found with endpoint_id: {endpoint_id}");
                            }
                        },

                        NodeCommand::RefreshClientIndex(endpoint_id) => {
                            self.update_model(NodeModelUpdate::UpdateClient {
                                endpoint_id,
                                update: ClientModelUpdate::UpdateIndex,
                            });
                        }

                        NodeCommand::SetDownloads { client, items } => {
                            // check that download directory is set before downloading
                            {
                                let download_directory = self.download_directory.lock().unwrap();
                                if download_directory.is_none() {
                                    error!("SetDownloads: download directory not set");
                                    continue;
                                }
                            };

                            let clients = self.clients.lock().unwrap();
                            if let Some(client_handle) = clients.get(&client) {
                                client_handle.tx.send(ClientCommand::SetDownloads { items }).expect("failed to send ClientCommand::SetDownloads");
                            } else {
                                error!("SetDownloads: no client found with endpoint_id: {client}");
                            }
                        }
                        NodeCommand::PauseDownloads { client } => {
                            let clients = self.clients.lock().unwrap();
                            if let Some(client_handle) = clients.get(&client) {
                                client_handle.tx.send(ClientCommand::PauseDownloads).expect("failed to send ClientCommand::PauseDownloads");
                            } else {
                                error!("PauseDownloads: no client found with endpoint_id: {client}");
                            }
                        }

                        NodeCommand::TrustNode(endpoint_id) => {
                            // persist to database
                            {
                                let db = self.db.lock().unwrap();
                                if let Err(e) = db.add_trusted_node(endpoint_id) {
                                    error!("failed to add trusted node to database: {e:#}");
                                }
                            }

                            // update model
                            self.update_model(NodeModelUpdate::UpdateTrustedNodes);
                        }
                        NodeCommand::UntrustNode(endpoint_id) => {
                            // persist to database
                            {
                                let db = self.db.lock().unwrap();
                                if let Err(e) = db.remove_trusted_node(endpoint_id) {
                                    error!("failed to remove trusted node from database: {e:#}");
                                }
                            }

                            // update model
                            self.update_model(NodeModelUpdate::UpdateTrustedNodes);
                        }

                        NodeCommand::RefreshModel => {
                            self.update_model(NodeModelUpdate::UpdateRecentServers);
                            self.update_model(NodeModelUpdate::UpdateTrustedNodes);
                        }

                        NodeCommand::WriteTestFile(root) => {
                            warn!("core: received NodeCommand::WriteTestFile");

                            tokio::spawn(async move {
                                debug!("core: inside WriteTestFile task");

                                let mut path = match crate::fs::TreePath::from_root(root) {
                                    Ok(p) => p,
                                    Err(e) => {
                                        error!("core: WriteTestFile failed to create TreePath from root: {e:#}");
                                        return;
                                    }
                                };
                                path.push("test.txt");

                                debug!("core: WriteTestFile opening or creating file at {:?}", path);

                                let mut file = match crate::fs::TreeFile::open_or_create(
                                    &path,
                                    crate::fs::OpenMode::Write,
                                )
                                .await
                                {
                                    Ok(f) => f,
                                    Err(e) => {
                                        error!("core: WriteTestFile failed to open or create file: {e:#}");
                                        return;
                                    }
                                };

                                debug!("core: WriteTestFile opened or created file successfully");

                                if let Err(e) = file.write_all(b"meow meow").await {
                                    error!("core: WriteTestFile failed to write to file: {e:#}");
                                    return;
                                }

                                debug!("core: WriteTestFile wrote to file successfully");
                            });
                        }

                        NodeCommand::Stop => break,
                    }
                }

                Some(event) = event_rx.recv() => {
                    match event {
                        NodeEvent::FilesRequested(transcode_format, files) => {
                            if let Err(e) = library.send(LibraryCommand::RequestTranscodes(transcode_format, files)) {
                                error!("NodeEvent::FilesRequested: failed to send to library: {e:#}");
                            }
                        }

                        NodeEvent::TrustedNodesChanged => {
                            self.update_model(NodeModelUpdate::UpdateTrustedNodes);
                        }
                        NodeEvent::RecentServersChanged => {
                            self.update_model(NodeModelUpdate::UpdateRecentServers);
                        }

                        NodeEvent::ServerOpened { endpoint_id, handle, name, connected_at } => {
                            {
                                let mut servers = self.servers.lock().unwrap();
                                servers.insert(endpoint_id, handle);
                            }

                            self.update_model(NodeModelUpdate::CreateServer { endpoint_id, name, connected_at });
                        }

                        NodeEvent::ServerChanged { endpoint_id, update } => {
                            self.update_model(NodeModelUpdate::UpdateServer { endpoint_id, update });
                        }

                        NodeEvent::ServerClosed { endpoint_id, error } => {
                            {
                                let mut servers = self.servers.lock().unwrap();
                                servers.remove(&endpoint_id);
                            }

                            self.update_model(NodeModelUpdate::UpdateServer { endpoint_id, update: ServerModelUpdate::Close { error } });
                        }

                        NodeEvent::ClientOpened { endpoint_id, handle, name, connected_at } => {
                            {
                                let mut clients = self.clients.lock().unwrap();
                                clients.insert(endpoint_id, handle);
                            }

                            self.update_model(NodeModelUpdate::CreateClient { endpoint_id, name, connected_at });
                        }

                        NodeEvent::ClientChanged { endpoint_id, update } => {
                            self.update_model(NodeModelUpdate::UpdateClient { endpoint_id, update });
                        }

                        NodeEvent::ClientClosed { endpoint_id, error } => {
                            {
                                let mut clients = self.clients.lock().unwrap();
                                clients.remove(&endpoint_id);
                            }

                            self.update_model(NodeModelUpdate::UpdateClient { endpoint_id, update: ClientModelUpdate::Close { error } });
                        }

                        NodeEvent::ServerTransferCompleted { endpoint_id, bytes, is_first_transfer } => {
                            {
                                let db = self.db.lock().unwrap();
                                let _ = db.track_server_transfer(1, bytes);
                                if is_first_transfer {
                                    let _ = db.track_server_session();
                                }
                            }
                            self.push_stats_model();
                        }

                        NodeEvent::ClientTransferCompleted { endpoint_id, bytes, is_first_transfer } => {
                            {
                                let db = self.db.lock().unwrap();
                                let _ = db.track_client_transfer(1, bytes);
                                if is_first_transfer {
                                    let _ = db.track_client_session();
                                }
                            }
                            self.push_stats_model();
                        }
                    }
                }

                else => {
                    warn!("all senders dropped in Node::run, shutting down");
                    break
                }
            }
        }

        let _ = self.router.shutdown().await;

        Ok(())
    }

    pub fn get_model(self: &Arc<Self>) -> NodeModel {
        let model = self.model.lock().unwrap();
        model.clone()
    }

    // TODO: throttle pushing updates?
    fn update_model(self: &Arc<Self>, update: NodeModelUpdate) {
        match update {
            NodeModelUpdate::PollMetrics => {
                let metrics = self.router.endpoint().metrics();

                let mut model = self.model.lock().unwrap();
                model.send_ipv4 = metrics.socket.send_ipv4.get();
                model.send_ipv6 = metrics.socket.send_ipv6.get();
                model.send_relay = metrics.socket.send_relay.get();
                model.recv_ipv4 = metrics.socket.recv_data_ipv4.get();
                model.recv_ipv6 = metrics.socket.recv_data_ipv6.get();
                model.recv_relay = metrics.socket.recv_data_relay.get();
                model.conn_success = metrics.socket.num_conns_opened.get();
                model.conn_direct = metrics.socket.num_conns_direct.get();

                self.event_handler.on_node_model_snapshot(model.clone());
            }

            NodeModelUpdate::UpdateHomeRelay { home_relay } => {
                let mut model = self.model.lock().unwrap();
                model.home_relay = home_relay;

                self.event_handler.on_node_model_snapshot(model.clone());
            }

            NodeModelUpdate::UpdateTrustedNodes => {
                let trusted_nodes = {
                    let db = self.db.lock().unwrap();
                    let trusted_nodes = match db.get_trusted_nodes() {
                        Ok(trusted_nodes) => trusted_nodes,
                        Err(e) => {
                            error!("failed update node model trusted nodes from database: {e:#}");
                            return;
                        }
                    };
                    trusted_nodes
                        .into_iter()
                        .map(|node| TrustedNodeModel {
                            endpoint_id: node.node_id.to_string(),
                            name: node.name.unwrap_or_else(|| "Unknown".to_string()),
                            connected_at: node.connected_at,
                        })
                        .collect()
                };

                let mut model = self.model.lock().unwrap();
                model.trusted_nodes = trusted_nodes;

                self.event_handler.on_node_model_snapshot(model.clone());
            }

            NodeModelUpdate::UpdateRecentServers => {
                let recent_servers = {
                    let db = self.db.lock().unwrap();
                    match db.get_recent_servers() {
                        Ok(recent_servers) => recent_servers
                            .into_iter()
                            .map(|node| RecentServerModel {
                                endpoint_id: node.node_id.to_string(),
                                name: node.name,
                                connected_at: node.connected_at,
                            })
                            .collect(),
                        Err(e) => {
                            error!("failed to get recent servers from database: {e:#}");
                            Vec::new()
                        }
                    }
                };

                let mut model = self.model.lock().unwrap();
                model.recent_servers = recent_servers;

                self.event_handler.on_node_model_snapshot(model.clone());
            }

            NodeModelUpdate::CreateServer {
                endpoint_id,
                name,
                connected_at,
            } => {
                let endpoint_id = endpoint_id.to_string();

                let mut model = self.model.lock().unwrap();
                model.servers.insert(
                    endpoint_id.clone(),
                    ServerModel {
                        name,
                        endpoint_id,
                        connected_at,

                        state: ServerStateModel::Pending,

                        connection_type: "unknown".to_string(),
                        latency_ms: None,

                        transfer_jobs: Vec::new(),
                    },
                );

                self.event_handler.on_node_model_snapshot(model.clone());
            }

            NodeModelUpdate::UpdateServer {
                endpoint_id,
                update,
            } => {
                let endpoint_id_string = endpoint_id.to_string();

                let mut model = self.model.lock().unwrap();
                let Some(server) = model.servers.get_mut(&endpoint_id_string) else {
                    warn!("failed to apply NodeModelUpdate::UpdateServer: no server model found");
                    return;
                };

                match update {
                    ServerModelUpdate::Accept => {
                        server.state = ServerStateModel::Accepted;
                    }
                    ServerModelUpdate::UpdateConnectionInfo {
                        remote_addr,
                        rtt_ms,
                    } => {
                        server.connection_type = remote_addr;
                        server.latency_ms = rtt_ms;
                    }
                    ServerModelUpdate::UpdateTransferJobs => {
                        let server_handles = self.servers.lock().unwrap();
                        let Some(server_handle) = server_handles.get(&endpoint_id) else {
                            warn!(
                                "failed to apply ServerModelUpdate::UpdateTransferJobs: no server handle found"
                            );
                            return;
                        };

                        let transfer_jobs = server_handle
                            .jobs
                            .iter()
                            .map(|entry| {
                                let job = entry.value();

                                let (progress, file_size) = match &job.progress {
                                    ServerTransferJobProgress::Transcoding { .. } => {
                                        (TransferJobProgressModel::Transcoding, None)
                                    }

                                    ServerTransferJobProgress::Ready { file_size, .. } => {
                                        (TransferJobProgressModel::Ready, Some(*file_size))
                                    }

                                    ServerTransferJobProgress::InProgress {
                                        started_at,
                                        file_size,
                                        sent,
                                    } => (
                                        TransferJobProgressModel::InProgress {
                                            started_at: *started_at,
                                            bytes: Arc::new(CounterModel::from(sent)),
                                        },
                                        Some(*file_size),
                                    ),

                                    ServerTransferJobProgress::Finished {
                                        finished_at,
                                        file_size,
                                    } => (
                                        TransferJobProgressModel::Finished {
                                            finished_at: *finished_at,
                                        },
                                        Some(*file_size),
                                    ),

                                    ServerTransferJobProgress::Failed { error } => (
                                        TransferJobProgressModel::Failed {
                                            error: format!("{error:#}"),
                                        },
                                        None,
                                    ),
                                };

                                TransferJobModel {
                                    job_id: *entry.key(),
                                    file_root: job.file_root.clone(),
                                    file_path: job.file_path.clone(),
                                    file_size,
                                    progress,
                                }
                            })
                            .collect();

                        server.transfer_jobs = transfer_jobs;
                    }
                    ServerModelUpdate::Close { error } => {
                        server.state = ServerStateModel::Closed { error };
                    }
                }

                self.event_handler.on_node_model_snapshot(model.clone());
            }

            NodeModelUpdate::CreateClient {
                endpoint_id,
                name,
                connected_at,
            } => {
                let endpoint_id = endpoint_id.to_string();

                let mut model = self.model.lock().unwrap();
                model.clients.insert(
                    endpoint_id.clone(),
                    ClientModel {
                        name,
                        endpoint_id,
                        connected_at,

                        state: ClientStateModel::Pending,

                        connection_type: "unknown".to_string(),
                        latency_ms: None,

                        index: None,
                        transfer_jobs: Vec::new(),
                        paused: false,
                    },
                );

                self.event_handler.on_node_model_snapshot(model.clone());
            }

            NodeModelUpdate::UpdateClient {
                endpoint_id,
                update,
            } => {
                let endpoint_id_string = endpoint_id.to_string();

                let mut model = self.model.lock().unwrap();
                let Some(client) = model.clients.get_mut(&endpoint_id_string) else {
                    warn!("failed to apply NodeModelUpdate::UpdateClient: no client model found");
                    return;
                };

                match update {
                    ClientModelUpdate::Accept => {
                        client.state = ClientStateModel::Accepted;
                    }
                    ClientModelUpdate::UpdateConnectionInfo {
                        remote_addr,
                        rtt_ms,
                    } => {
                        client.connection_type = remote_addr;
                        client.latency_ms = rtt_ms;
                    }
                    ClientModelUpdate::UpdateIndex => {
                        let client_handles = self.clients.lock().unwrap();
                        let Some(client_handle) = client_handles.get(&endpoint_id) else {
                            warn!(
                                "failed to apply ClientModelUpdate::UpdateIndex: no client handle found"
                            );
                            return;
                        };

                        let download_directory = {
                            let download_directory = self.download_directory.lock().unwrap();
                            download_directory.clone()
                        };

                        let index = client_handle.index.lock().unwrap().as_ref().cloned();
                        if let Some(index) = index {
                            let db = self.db.lock().unwrap();

                            let index = index
                                .into_iter()
                                .map(|item| {
                                    // check if file is downloaded to the current download directory
                                    let file_exists = download_directory.as_ref().is_some_and(
                                        |download_directory| {
                                            db.exists_file_by_node_root_path_localtree(
                                                endpoint_id,
                                                &item.root,
                                                &item.path,
                                                download_directory,
                                            )
                                            .unwrap_or(false)
                                        },
                                    );

                                    // determine download status
                                    let download_status = if file_exists {
                                        Some(IndexItemDownloadStatusModel::Downloaded)
                                    } else {
                                        // check if there's an active transfer job for this file
                                        let transfer_job =
                                            client.transfer_jobs.iter().find(|job| {
                                                job.file_root == item.root
                                                    && job.file_path == item.path
                                            });

                                        match transfer_job {
                                            Some(job) => match &job.progress {
                                                TransferJobProgressModel::Requested
                                                | TransferJobProgressModel::Transcoding
                                                | TransferJobProgressModel::Ready => {
                                                    Some(IndexItemDownloadStatusModel::Waiting)
                                                }
                                                TransferJobProgressModel::InProgress { .. } => {
                                                    Some(IndexItemDownloadStatusModel::InProgress)
                                                }
                                                TransferJobProgressModel::Failed { .. } => {
                                                    Some(IndexItemDownloadStatusModel::Failed)
                                                }
                                                TransferJobProgressModel::Finished { .. } => {
                                                    Some(IndexItemDownloadStatusModel::Downloaded)
                                                }
                                            },
                                            None => None,
                                        }
                                    };

                                    IndexItemModel {
                                        endpoint_id: endpoint_id.to_string(),
                                        root: item.root,
                                        path: item.path,

                                        file_size: match item.file_size {
                                            FileSize::Unknown => FileSizeModel::Unknown,
                                            FileSize::Estimated(n) => FileSizeModel::Estimated(n),
                                            FileSize::Actual(n) => FileSizeModel::Actual(n),
                                        },

                                        download_status,
                                    }
                                })
                                .collect();

                            client.index = Some(index);
                        } else {
                            warn!("ClientModelUpdate::UpdateIndex: no index found");
                            client.index = None;
                        }
                    }
                    ClientModelUpdate::UpdateTransferJobs => {
                        let client_handles = self.clients.lock().unwrap();
                        let Some(client_handle) = client_handles.get(&endpoint_id) else {
                            warn!(
                                "failed to apply ClientModelUpdate::UpdateTransferJobs: no client handle found"
                            );
                            return;
                        };

                        let transfer_jobs = client_handle
                            .jobs
                            .iter()
                            .map(|entry| {
                                let job = entry.value();

                                let file_size = match &job.progress {
                                    ClientTransferJobProgress::Requested
                                    | ClientTransferJobProgress::Transcoding => None,

                                    ClientTransferJobProgress::Ready { file_size }
                                    | ClientTransferJobProgress::InProgress { file_size, .. }
                                    | ClientTransferJobProgress::Finished { file_size, .. } => {
                                        Some(*file_size)
                                    }

                                    ClientTransferJobProgress::Failed { .. } => None,
                                };

                                let progress = match &job.progress {
                                    ClientTransferJobProgress::Requested => {
                                        TransferJobProgressModel::Requested
                                    }
                                    ClientTransferJobProgress::Transcoding => {
                                        TransferJobProgressModel::Transcoding
                                    }
                                    ClientTransferJobProgress::Ready { .. } => {
                                        TransferJobProgressModel::Ready
                                    }

                                    ClientTransferJobProgress::InProgress {
                                        started_at,
                                        written,
                                        ..
                                    } => TransferJobProgressModel::InProgress {
                                        started_at: *started_at,
                                        bytes: Arc::new(CounterModel::from(written)),
                                    },

                                    // Finished jobs are always shown as Finished
                                    ClientTransferJobProgress::Finished { finished_at, .. } => {
                                        TransferJobProgressModel::Finished {
                                            finished_at: *finished_at,
                                        }
                                    }

                                    // Failed jobs are always shown as Failed
                                    ClientTransferJobProgress::Failed { error } => {
                                        TransferJobProgressModel::Failed {
                                            error: error.clone(),
                                        }
                                    }
                                };

                                TransferJobModel {
                                    job_id: *entry.key(),
                                    file_root: job.file_root.clone(),
                                    file_path: job.file_path.clone(),
                                    file_size,
                                    progress,
                                }
                            })
                            .collect();

                        client.transfer_jobs = transfer_jobs;
                    }
                    ClientModelUpdate::UpdatePaused => {
                        let client_handles = self.clients.lock().unwrap();
                        let Some(client_handle) = client_handles.get(&endpoint_id) else {
                            warn!(
                                "failed to apply ClientModelUpdate::UpdatePaused: no client handle found"
                            );
                            return;
                        };

                        let is_paused = client_handle.paused.load(Ordering::Relaxed);
                        client.paused = is_paused;
                    }
                    ClientModelUpdate::Close { error } => {
                        client.state = ClientStateModel::Closed { error };
                    }
                }

                self.event_handler.on_node_model_snapshot(model.clone());
            }
        }
    }

    fn push_stats_model(&self) {
        let db = self.db.lock().unwrap();
        if let Ok(stats) = db.get_stats() {
            self.event_handler.on_stats_model_snapshot(stats);
        }
    }

    // TODO: maybe replace with methods?
    pub fn send(self: &Arc<Self>, command: NodeCommand) -> anyhow::Result<()> {
        self.command_tx
            .send(command)
            .map_err(|e| anyhow::anyhow!("failed to send command: {e:?}"))
    }

    async fn connect(
        self: &Arc<Self>,
        transcode_format: Option<TranscodeFormat>,
        addr: EndpointAddr,
    ) -> anyhow::Result<()> {
        // connect before spawning the task, so we can return an error immediately
        let connection = self.router.endpoint().connect(addr, Protocol::ALPN).await?;

        let endpoint_id = connection.remote_id();
        info!("opened connection to {endpoint_id}");

        let db = self.db.clone();
        let event_tx = self.event_tx.clone();
        let download_directory = self.download_directory.clone();
        #[cfg(feature = "test-hooks")]
        let test_hooks = self.test_hooks.clone();
        tokio::spawn(async move {
            let client = Client::new(
                db,
                event_tx.clone(),
                connection,
                transcode_format,
                download_directory,
                #[cfg(feature = "test-hooks")]
                test_hooks,
            );

            let res = client.run().await;
            if let Err(e) = &res {
                error!("error during client.run(): {e:#}");
            }

            // notify node
            event_tx
                .send(NodeEvent::ClientClosed {
                    endpoint_id,
                    error: res.err().map(|e| format!("{e:#}")),
                })
                .expect("failed to send NodeEvent::ClientClosed");
        });

        Ok(())
    }

    /// Check if stored remote files still exist locally.
    async fn check_remote_files(self: &Arc<Self>) -> anyhow::Result<()> {
        // get remote files by getting files where endpoint ID is not the local endpoint ID
        let remote_files = {
            let db = self.db.lock().unwrap();
            db.get_files_by_ne_node_id(self.router.endpoint().id())?
        };

        // check if files exist
        let mut missing_files = Vec::new();
        for remote_file in remote_files {
            let path = TreePath::new(
                remote_file.local_tree.clone(),
                remote_file.local_path.clone().into(),
            );

            // check for error without failing
            if let Err(e) = &path {
                warn!("check_remote_files: failed to create TreePath: {e:#}");
            }

            if !path.is_ok_and(|p| p.exists()) {
                missing_files.push((remote_file.local_tree, remote_file.local_path));
            }
        }

        // remove from db
        if !missing_files.is_empty() {
            warn!(
                "removing {} missing remote files from database",
                missing_files.len()
            );
            {
                let db = self.db.lock().unwrap();
                db.remove_files_by_local_treepath(missing_files.into_iter())?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Protocol {
    db: Arc<Mutex<Database>>,
    transcode_status_cache: TranscodeStatusCache,
    hash_cache: HashCache,

    event_tx: mpsc::UnboundedSender<NodeEvent>,
}

impl Protocol {
    const ALPN: &'static [u8] = b"musicopy/1";

    fn new(
        db: Arc<Mutex<Database>>,
        transcode_status_cache: TranscodeStatusCache,
        hash_cache: HashCache,

        event_tx: mpsc::UnboundedSender<NodeEvent>,
    ) -> Self {
        Self {
            db,
            transcode_status_cache,
            hash_cache,

            event_tx,
        }
    }
}

impl ProtocolHandler for Protocol {
    async fn accept(&self, connection: iroh::endpoint::Connection) -> Result<(), AcceptError> {
        let endpoint_id = connection.remote_id();
        info!("accepted connection from {endpoint_id}");

        let server = Server::new(
            self.db.clone(),
            self.transcode_status_cache.clone(),
            self.hash_cache.clone(),
            connection,
            self.event_tx.clone(),
        );

        let res = server.run().await;
        if let Err(e) = &res {
            error!("error during server.run(): {e:#}");
        }

        // notify node
        self.event_tx
            .send(NodeEvent::ServerClosed {
                endpoint_id,
                error: res.err().map(|e| format!("{e:#}")),
            })
            .expect("failed to send NodeEvent::ServerClosed");

        Ok(())
    }
}

/// A message sent by the client at the start of a file transfer stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransferRequest {
    job_id: u64,
}

/// A message sent by the server in a file transfer stream in response to a TransfrRequest.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum TransferResponse {
    /// The job is ready to be downloaded and will be sent by the server.
    Ok { file_size: u64 },
    /// The job was unable to be downloaded.
    Error { error: String },
}

#[derive(Debug)]
struct ServerTransferJob {
    progress: ServerTransferJobProgress,

    // for UI
    file_endpoint_id: EndpointId,
    file_root: String,
    file_path: String,
}

#[derive(Debug)]
enum ServerTransferJobProgress {
    /// The server is waiting for the file to be transcoded.
    Transcoding { local_path: PathBuf },
    /// The server is ready to send the file.
    Ready {
        transcode_path: PathBuf,
        file_size: u64,
    },
    /// The server has started sending the file.
    InProgress {
        started_at: u64,
        file_size: u64,
        sent: Arc<AtomicU64>,
    },
    /// The server has finished sending the file.
    Finished { finished_at: u64, file_size: u64 },
    /// The server failed to send the file.
    Failed { error: anyhow::Error },
}

#[derive(Debug)]
enum ServerCommand {
    Accept,

    Close,

    /// Send a message to the client.
    ///
    /// This is sort of a hack, but it's used by the task that watches for
    /// finished transcodes to send JobStatus messages to the client.
    ServerMessage(ServerMessageV1),
}

#[derive(Debug, Clone)]
struct ServerHandle {
    tx: mpsc::UnboundedSender<ServerCommand>,

    jobs: Arc<DashMap<u64, ServerTransferJob>>,
}

struct Server {
    db: Arc<Mutex<Database>>,
    transcode_status_cache: TranscodeStatusCache,
    hash_cache: HashCache,

    connection: Connection,
    event_tx: mpsc::UnboundedSender<NodeEvent>,

    connected_at: u64,

    jobs: Arc<DashMap<u64, ServerTransferJob>>,
}

impl Server {
    fn new(
        db: Arc<Mutex<Database>>,
        transcode_status_cache: TranscodeStatusCache,
        hash_cache: HashCache,

        connection: Connection,
        event_tx: mpsc::UnboundedSender<NodeEvent>,
    ) -> Self {
        Self {
            db,
            transcode_status_cache,
            hash_cache,

            connection,
            event_tx,

            connected_at: unix_epoch_now_secs(),

            jobs: Arc::new(DashMap::new()),
        }
    }

    async fn run(self) -> anyhow::Result<()> {
        let remote_endpoint_id = self.connection.remote_id();

        // spawn connection info watcher task
        // TODO: check that this is cancelled/cleaned up correctly?
        let mut paths_stream = self.connection.paths().stream();
        tokio::spawn({
            let event_tx = self.event_tx.clone();
            async move {
                while let Some(paths) = paths_stream.next().await {
                    let selected_path = paths.iter().find(|path| path.is_selected());
                    let (remote_addr, rtt_ms) = match selected_path {
                        Some(selected_path) => {
                            let remote_addr = selected_path.remote_addr().to_string();
                            let rtt_ms = selected_path.rtt().map(|d| d.as_millis() as u64);
                            (remote_addr, rtt_ms)
                        }
                        None => ("unknown".to_string(), None),
                    };

                    event_tx
                        .send(NodeEvent::ServerChanged {
                            endpoint_id: remote_endpoint_id,
                            update: ServerModelUpdate::UpdateConnectionInfo {
                                remote_addr,
                                rtt_ms,
                            },
                        })
                        .expect("failed to send ServerModelUpdate::UpdateConnectionInfo");
                }
            }
        });

        let (tx, mut rx) = mpsc::unbounded_channel();

        // accept bidirectional control stream
        let (send, recv) = self.connection.accept_bi().await?;

        // wrap in framed codecs
        let mut send = FramedWrite::new(send, LengthDelimitedCodec::new()).with_flat_map(
            |message: ServerMessageV1| {
                let buf: Vec<u8> =
                    postcard::to_stdvec(&message).expect("failed to serialize message");
                futures::stream::once(futures::future::ready(Ok(Bytes::from(buf))))
            },
        );
        let mut recv = FramedRead::new(recv, LengthDelimitedCodec::new())
            .map_err(|e| anyhow::anyhow!("failed to read from connection: {e:?}"))
            .map(|res| {
                res.and_then(|bytes| {
                    postcard::from_bytes::<ClientMessageV1>(&bytes)
                        .map_err(|e| anyhow::anyhow!("failed to deserialize message: {e:?}"))
                })
            });

        // wait for client Identify
        let Some(Ok(message)) = recv.next().await else {
            error!("failed to receive Identify message");
            return Ok(());
        };
        let (client_name, transcode_format) = match message {
            ClientMessageV1::Identify {
                name,
                transcode_format,
            } => (name, transcode_format),
            _ => {
                error!("unexpected message, expected Identify: {message:?}");
                return Ok(());
            }
        };

        // send server Identify
        send.send(ServerMessageV1::Identify(device_name().to_string()))
            .await
            .expect("failed to send Identify message");

        // handshake finished, send handle to Node
        let handle = ServerHandle {
            tx: tx.clone(),

            jobs: self.jobs.clone(),
        };
        self.event_tx
            .send(NodeEvent::ServerOpened {
                endpoint_id: remote_endpoint_id,
                handle,

                name: client_name.clone(),
                connected_at: self.connected_at,
            })
            .expect("failed to send NodeEvent::ServerOpened");

        // check if remote node is trusted
        let is_trusted = {
            let db = self.db.lock().unwrap();
            db.is_node_trusted(remote_endpoint_id)?
        };

        if is_trusted {
            info!("accepting connection from trusted node {remote_endpoint_id}");
        } else {
            // waiting loop, wait for user to accept or deny the connection
            info!(
                "waiting for accept or deny of connection from untrusted node {remote_endpoint_id}",
            );
            loop {
                tokio::select! {
                    Some(command) = rx.recv() => {
                        match command {
                            ServerCommand::Accept => {
                                // continue to next state
                                break;
                            },
                            ServerCommand::Close => {
                                self.connection.close(0u32.into(), b"close");
                                return Ok(());
                            },
                            ServerCommand::ServerMessage(message) => {
                                send.send(message)
                                    .await
                                    .expect("failed to send ServerMessageV1");
                            }
                        }
                    }

                    next_message = recv.next() => {
                        match next_message {
                            Some(Ok(message)) => {
                                debug!("unexpected message (not accepted): {message:?}");
                            },
                            Some(Err(e)) => {
                                error!("error receiving message: {e}");
                            },
                            None => {
                                info!("control stream closed, shutting down server");
                                return Ok(());
                            },
                        }
                    }

                    else => {
                        anyhow::bail!("stream and receiver closed while waiting for Accept");
                    }
                }
            }
        }

        // mark as accepted
        self.event_tx
            .send(NodeEvent::ServerChanged {
                endpoint_id: remote_endpoint_id,
                update: ServerModelUpdate::Accept,
            })
            .expect("failed to send ServerModelUpdate::Accept");

        // send Accepted message
        send.send(ServerMessageV1::Accepted)
            .await
            .expect("failed to send Accepted message");

        // send Index message
        let mut index = self.get_index(transcode_format)?;
        send.send(ServerMessageV1::Index(
            index.iter().map(|(_, item)| item.clone()).collect(),
        ))
        .await
        .expect("failed to send Index message");

        // update name and connected_at for trusted nodes
        {
            let db = self.db.lock().unwrap();
            db.update_trusted_node(remote_endpoint_id, &client_name, self.connected_at)
                .context("failed to update trusted node in database")?;
        }
        self.event_tx
            .send(NodeEvent::TrustedNodesChanged)
            .expect("failed to send NodeEvent::TrustedNodesChanged");

        // spawn task to watch for finished transcodes
        // TODO: shutdown signal
        // TODO: could maybe be a timer instead of a task with a sleep loop
        tokio::spawn({
            let jobs = self.jobs.clone();
            let transcode_status_cache = self.transcode_status_cache.clone();
            let hash_cache = self.hash_cache.clone();
            let event_tx = self.event_tx.clone();
            async move {
                loop {
                    let mut ready_jobs = Vec::new();
                    let mut failed_jobs = Vec::new();

                    // check for jobs with Transcoding status
                    for job in jobs.iter() {
                        if let ServerTransferJobProgress::Transcoding { local_path } =
                            &job.value().progress
                        {
                            // read cache key from metadata
                            let Ok(key) = hash_cache.read_cache_key(local_path) else {
                                // TODO: failing to read metadata is likely an error
                                continue;
                            };

                            // If no transcode format was specified, mark the job as ready
                            let Some(transcode_format) = transcode_format else {
                                ready_jobs.push((*job.key(), local_path.clone(), key.file_size()));
                                continue;
                            };

                            // check for cached hash
                            let Ok(Some((hash_kind, hash))) = hash_cache.get_cached_hash(&key)
                            else {
                                // error or not hashed yet, still transcoding
                                continue;
                            };

                            // get transcode status
                            let Some(status) =
                                transcode_status_cache.get(transcode_format, &hash_kind, hash)
                            else {
                                // no status yet, still transcoding
                                continue;
                            };

                            match &*status {
                                // if transcode status is Ready, set job status to Ready
                                TranscodeStatus::Ready {
                                    transcode_path,
                                    file_size,
                                } => {
                                    ready_jobs.push((
                                        *job.key(),
                                        transcode_path.clone(),
                                        *file_size,
                                    ));
                                }

                                // if transcode status is Failed, set job status to Failed
                                TranscodeStatus::Failed { error } => {
                                    error!("transcoding failed for job {}: {}", job.key(), error);

                                    failed_jobs.push((
                                        *job.key(),
                                        anyhow::anyhow!("transcoding failed: {error}"),
                                    ));
                                }
                            }
                        }
                    }

                    // create status changes for ready jobs
                    let ready_jobs =
                        ready_jobs
                            .into_iter()
                            .map(|(job_id, transcode_path, file_size)| {
                                // set job status to Ready
                                // needs to happen outside the loop, since jobs.iter() already holds the entry's lock
                                jobs.alter(&job_id, |_, mut job| {
                                    job.progress = ServerTransferJobProgress::Ready {
                                        transcode_path,
                                        file_size,
                                    };
                                    job
                                });

                                (job_id, JobStatusItem::Ready { file_size })
                            });

                    // create status changes for failed jobs
                    let failed_jobs = failed_jobs.into_iter().map(|(job_id, error)| {
                        let error_string = format!("{error}");

                        // set job status to Failed
                        // needs to happen outside the loop, since jobs.iter() already holds the entry's lock
                        jobs.alter(&job_id, |_, mut job| {
                            job.progress = ServerTransferJobProgress::Failed { error };
                            job
                        });

                        (
                            job_id,
                            JobStatusItem::Failed {
                                error: error_string,
                            },
                        )
                    });

                    let status_changes = ready_jobs.chain(failed_jobs).collect::<HashMap<_, _>>();
                    if !status_changes.is_empty() {
                        // send status changes to client via ServerCommand::ServerMessage
                        if let Err(e) = tx.send(ServerCommand::ServerMessage(
                            ServerMessageV1::JobStatus(status_changes),
                        )) {
                            warn!("transcode watcher failed to send JobStatus message: {e}");
                        }

                        // update model
                        event_tx
                            .send(NodeEvent::ServerChanged {
                                endpoint_id: remote_endpoint_id,
                                update: ServerModelUpdate::UpdateTransferJobs,
                            })
                            .expect("failed to send ServerModelUpdate::UpdateTransferJobs");
                    }

                    // sleep before checking again
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });

        let mut index_update_interval = tokio::time::interval(Duration::from_secs(1));

        // Track whether the first transfer has completed, for counting sessions with >1 transfer.
        // When tracking transferred files we indicate whether it's the first of this session.
        let is_first_transfer = Arc::new(AtomicBool::new(true));

        // main loop
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ServerCommand::Accept => {
                            warn!("unexpected Accept command in main loop");
                        },
                        ServerCommand::Close => {
                            self.connection.close(0u32.into(), b"close");
                            break;
                        },
                        ServerCommand::ServerMessage(message) => {
                            send.send(message)
                                .await
                                .expect("failed to send ServerMessage");
                        }
                    }
                }

                next_message = recv.next() => {
                    match next_message {
                        Some(Ok(message)) => {
                            match message {
                                ClientMessageV1::Identify { .. } => {
                                    warn!("unexpected ClientMessageV1::Identify in main loop");
                                }

                                ClientMessageV1::Download(items) => {
                                    // get file local paths
                                    // TODO: this could be better
                                    let files = {
                                        let db = self.db.lock().expect("failed to lock database");
                                        db.get_files_by_node_root_path(
                                            items.iter().map(|item| (item.endpoint_id, item.root.clone(), item.path.clone()))
                                        )?.into_iter().map(|f| ((f.node_id, f.root.clone(), f.path.clone()), f)).collect::<HashMap<_, _>>()
                                    };

                                    let status_changes = items.into_iter().map(|item| {
                                        // TODO: wasteful clones
                                        let file = files.get(&(item.endpoint_id, item.root.clone(), item.path.clone()));

                                        // get file for requested item
                                        let Some(file) = file else {
                                            self.jobs.insert(item.job_id, ServerTransferJob {
                                                progress: ServerTransferJobProgress::Failed { error: anyhow::anyhow!("file not found") },
                                                file_endpoint_id: item.endpoint_id,
                                                file_root: item.root,
                                                file_path: item.path,
                                            });

                                            return (item.job_id, JobStatusItem::Failed {
                                                error: "file not found".to_string(),
                                            });
                                        };

                                        let local_path = PathBuf::from(&file.local_path);

                                        // check for cached hash
                                        let Ok(key) = self.hash_cache.read_cache_key(&local_path) else {
                                            // This is probably unrecoverable, but we still add a
                                            // job with Transcoding status which should reach Failed
                                            // eventually.

                                            // create job
                                            self.jobs.insert(item.job_id, ServerTransferJob {
                                                progress: ServerTransferJobProgress::Transcoding { local_path },
                                                file_endpoint_id: item.endpoint_id,
                                                file_root: item.root,
                                                file_path: item.path,
                                            });

                                            return (item.job_id, JobStatusItem::Transcoding);
                                        };

                                        // If no transcode format was specified, create a ready job
                                        let Some(transcode_format) = transcode_format else {
                                            self.jobs.insert(item.job_id, ServerTransferJob {
                                                progress: ServerTransferJobProgress::Ready {
                                                    transcode_path: local_path.clone(),
                                                    file_size: key.file_size(),
                                                },
                                                file_endpoint_id: item.endpoint_id,
                                                file_root: item.root,
                                                file_path: item.path,
                                            });

                                            return (item.job_id, JobStatusItem::Ready {
                                                file_size: key.file_size(),
                                            });
                                        };

                                        let Ok(Some((hash_kind, hash))) =
                                            self.hash_cache.get_cached_hash(&key)
                                        else {
                                            // error or not hashed yet, still transcoding

                                            // create job
                                            self.jobs.insert(item.job_id, ServerTransferJob {
                                                progress: ServerTransferJobProgress::Transcoding { local_path },
                                                file_endpoint_id: item.endpoint_id,
                                                file_root: item.root,
                                                file_path: item.path,
                                            });

                                            return (item.job_id, JobStatusItem::Transcoding);
                                        };

                                        // get transcode status
                                        let transcode_status = self.transcode_status_cache.get(transcode_format, &hash_kind, hash);

                                        match transcode_status.as_deref() {
                                            Some(TranscodeStatus::Ready { transcode_path, file_size }) => {
                                                // file is already transcoded

                                                // create job
                                                self.jobs.insert(item.job_id, ServerTransferJob {
                                                    progress: ServerTransferJobProgress::Ready {
                                                        transcode_path: transcode_path.clone(),
                                                        file_size: *file_size,
                                                    },
                                                    file_endpoint_id: item.endpoint_id,
                                                    file_root: item.root,
                                                    file_path: item.path,
                                                });

                                                (item.job_id, JobStatusItem::Ready {
                                                    file_size: *file_size,
                                                })
                                            }

                                            Some(TranscodeStatus::Failed { error }) => {
                                                // transcoding failed

                                                // create job
                                                self.jobs.insert(item.job_id, ServerTransferJob {
                                                    progress: ServerTransferJobProgress::Failed {
                                                        error: anyhow::anyhow!("transcoding failed: {error}"),
                                                    },
                                                    file_endpoint_id: item.endpoint_id,
                                                    file_root: item.root,
                                                    file_path: item.path,
                                                });

                                                (item.job_id, JobStatusItem::Failed {
                                                    error: format!("{error:#}"),
                                                })
                                            }

                                            None => {
                                                // still transcoding

                                                // create job
                                                self.jobs.insert(item.job_id, ServerTransferJob {
                                                    progress: ServerTransferJobProgress::Transcoding { local_path },
                                                    file_endpoint_id: item.endpoint_id,
                                                    file_root: item.root,
                                                    file_path: item.path,
                                                });

                                                (item.job_id, JobStatusItem::Transcoding)
                                            }
                                        }

                                    }).collect::<HashMap<_, _>>();

                                    // send job status to client
                                    send.send(ServerMessageV1::JobStatus(status_changes))
                                        .await
                                        .expect("failed to send JobStatus message");

                                    // update model
                                    self.event_tx.send(NodeEvent::ServerChanged {
                                        endpoint_id: remote_endpoint_id,
                                        update: ServerModelUpdate::UpdateTransferJobs,
                                    }).expect("failed to send ServerModelUpdate::UpdateTransferJobs");

                                    // prioritize transcodes
                                    if let Some(transcode_format) = transcode_format {
                                        let requested_paths = files.into_values().map(|f| PathBuf::from(f.local_path)).collect::<HashSet<_>>();
                                        self.event_tx.send(NodeEvent::FilesRequested(transcode_format, requested_paths)).expect("failed to send NodeEvent::FilesRequested");
                                    }
                                }
                            }
                        },
                        Some(Err(e)) => {
                            error!("error receiving message: {e}");
                        },
                        None => {
                            info!("control stream closed, shutting down server");
                            break;
                        },
                    }
                }

                // handle file transfer streams
                accept_result = self.connection.accept_bi() => {
                    match accept_result {
                        Ok((mut send, mut recv)) => {
                            let jobs = self.jobs.clone();
                            let event_tx = self.event_tx.clone();
                            let is_first_transfer = is_first_transfer.clone();
                            tokio::spawn(async move {
                                // receive transfer request with job id
                                let transfer_req_len = recv.read_u32().await?;
                                let mut transfer_req_buf = vec![0; transfer_req_len as usize];
                                recv
                                    .read_exact(&mut transfer_req_buf)
                                    .await
                                    .context("failed to read transfer request")?;
                                let transfer_req: TransferRequest =
                                    postcard::from_bytes(&transfer_req_buf).context("failed to deserialize transfer request")?;

                                // check job status
                                let (transfer_res, ready) = {
                                    let Some(job) = jobs.get(&transfer_req.job_id) else {
                                        anyhow::bail!("transfer request job id not found: {}", transfer_req.job_id);
                                    };

                                    match &job.progress {
                                        ServerTransferJobProgress::Ready { transcode_path, file_size } => {
                                            (TransferResponse::Ok { file_size: *file_size }, Some((transcode_path.clone(), *file_size)))
                                        }
                                        _ => {
                                            (TransferResponse::Error { error: "job not ready".to_string() }, None)
                                        }
                                    }
                                };

                                // send transfer response
                                let transfer_res_buf = postcard::to_stdvec(&transfer_res)
                                    .context("failed to serialize transfer response")?;
                                send.write_u32(transfer_res_buf.len() as u32)
                                    .await
                                    .context("failed to write transfer response length")?;
                                send.write_all(&transfer_res_buf)
                                    .await
                                    .context("failed to write transfer response")?;

                                // TODO: could maybe be nicer
                                let Some((transcode_path, file_size)) = ready else {
                                    return Ok(());
                                };

                                // check local file exists
                                if !transcode_path.exists() {
                                    // TODO: set job to failed and respond with error
                                    anyhow::bail!("file at transcode_path does not exist: {}", transcode_path.display());
                                }

                                let sent_counter = Arc::new(AtomicU64::new(0));

                                // set job status to InProgress
                                jobs.alter(&transfer_req.job_id, |_, mut job| {
                                    job.progress = ServerTransferJobProgress::InProgress {
                                        started_at: unix_epoch_now_secs(),
                                        file_size,
                                        sent: sent_counter.clone()
                                    };
                                    job
                                });

                                // update model
                                event_tx.send(NodeEvent::ServerChanged {
                                    endpoint_id: remote_endpoint_id,
                                    update: ServerModelUpdate::UpdateTransferJobs,
                                }).expect("failed to send ServerModelUpdate::UpdateTransferJobs");

                                // read file to buffer
                                // TODO: stream instead of reading into memory?
                                let file_content = tokio::fs::read(transcode_path).await?;

                                // TODO: handle errors during send
                                let mut send_progress = WriteProgress::new(sent_counter.clone(), send);
                                send_progress.write_all(&file_content).await?;

                                // set job status to Finished
                                jobs.alter(&transfer_req.job_id, |_, mut job| {
                                    job.progress = ServerTransferJobProgress::Finished { finished_at: unix_epoch_now_secs(), file_size };
                                    job
                                });

                                // update model
                                event_tx.send(NodeEvent::ServerChanged {
                                    endpoint_id: remote_endpoint_id,
                                    update: ServerModelUpdate::UpdateTransferJobs,
                                }).expect("failed to send ServerModelUpdate::UpdateTransferJobs");

                                let is_first_transfer = is_first_transfer.swap(false, Ordering::SeqCst);
                                let _ = event_tx.send(NodeEvent::ServerTransferCompleted {
                                    endpoint_id: remote_endpoint_id,
                                    bytes: file_size,
                                    is_first_transfer,
                                });

                                Ok::<(), anyhow::Error>(())
                            });
                        }

                        Err(e) => {
                            error!("accept_bi error: {e}");
                        }
                    }
                }

                // periodically check for index updates
                _ = index_update_interval.tick() => {
                    let mut updates = Vec::new();

                    for (local_path, item) in index.iter_mut() {
                        // if the client doesn't have the actual file size
                        if !matches!(item.file_size, FileSize::Actual(_)) {
                            // check for actual size from transcode cache, then estimated size from database
                            let file_size = match self.hash_cache.read_cache_key(local_path) {
                                Ok(key) => {
                                    match transcode_format {
                                        Some(transcode_format) => {
                                            if let Some(actual_size) = self.hash_cache.get_cached_hash(&key)
                                                .ok().flatten()
                                                .and_then(|(hash_kind, hash)| self.transcode_status_cache.get(transcode_format, &hash_kind, hash))
                                                .and_then(|entry| match &*entry {
                                                    TranscodeStatus::Ready { file_size, .. } => Some(*file_size),
                                                    _ => None,
                                                }) {
                                                Some(FileSize::Actual(actual_size))
                                            } else {
                                                self.hash_cache.get_cached_duration(&key)
                                                    .ok().flatten().map(|duration| estimate_file_size(transcode_format, duration)).map(FileSize::Estimated)
                                            }
                                        }
                                        None => {
                                            // use original file size
                                            Some(FileSize::Actual(key.file_size()))
                                        }
                                    }
                                }
                                Err(_) => None,
                            };

                            // if we now have an updated file size, update the client
                            if let Some(file_size) = file_size && file_size != item.file_size {
                                // store client's view so we don't send the same update again
                                item.file_size = file_size;

                                updates.push(IndexUpdateItem::FileSize {
                                    endpoint_id: item.endpoint_id,
                                    root: item.root.clone(),
                                    path: item.path.clone(),

                                    file_size,
                                });
                            }
                        }
                    }

                    if !updates.is_empty() {
                        send.send(ServerMessageV1::IndexUpdate(updates))
                            .await
                            .expect("failed to send IndexUpdate message");
                    }
                }

                else => {
                    warn!("all senders dropped in Server::run, shutting down");
                    break;
                }
            }
        }

        self.connection.closed().await;

        Ok(())
    }

    /// Gets the index to send to the client.
    ///
    /// Also returns the local paths for the files, which are used to check for index updates
    /// (i.e. file size updates). Local paths are not sent to the client.
    fn get_index(
        &self,
        transcode_format: Option<TranscodeFormat>,
    ) -> anyhow::Result<Vec<(PathBuf, IndexItem)>> {
        let files = {
            let db = self.db.lock().unwrap();
            db.get_files()?
        };

        let index = files
            .into_iter()
            .map(|file| {
                let local_path = PathBuf::from(file.local_path);

                let file_size = if let Some(transcode_format) = transcode_format {
                    // Get cached estimated size without checking validity. Validating the cached size
                    // requires accessing the file to read its metadata, which can be expensive. We want
                    // this to be fast since it's on the user's critical path. We can tolerate the
                    // estimated sizes very rarely being incorrect, and we also set the size to
                    // Estimated, so the periodic index update will send the actual size shortly after.
                    match self.hash_cache.get_cached_duration_unvalidated(&local_path) {
                        Ok(Some(duration)) => {
                            FileSize::Estimated(estimate_file_size(transcode_format, duration))
                        }
                        _ => FileSize::Unknown,
                    }
                } else {
                    // Get cached original file size without accessing the file.
                    match self
                        .hash_cache
                        .get_cached_file_size_unvalidated(&local_path)
                    {
                        // This could be stale if the files were modified, so we set the size to
                        // Estimated, and the periodic index update will send the actual size shortly after.
                        Ok(Some(size)) => FileSize::Estimated(size),
                        _ => FileSize::Unknown,
                    }
                };

                (
                    local_path,
                    IndexItem {
                        endpoint_id: file.node_id,
                        root: file.root,
                        path: file.path,

                        file_size,
                    },
                )
            })
            .collect::<Vec<_>>();

        Ok(index)
    }
}

#[derive(Debug)]
struct ClientTransferJob {
    progress: ClientTransferJobProgress,

    file_endpoint_id: EndpointId,
    file_root: String,
    file_path: String,
}

#[derive(Debug)]
enum ClientTransferJobProgress {
    /// The client sent the request and is waiting for its status.
    Requested,
    /// The client is waiting for the file to be transcoded.
    Transcoding,
    /// The client is ready to download the file.
    Ready { file_size: u64 },
    /// The client has started downloading the file.
    InProgress {
        started_at: u64,
        file_size: u64,
        written: Arc<AtomicU64>,
    },
    /// The client has finished downloading the file.
    Finished { finished_at: u64, file_size: u64 },
    /// The client failed to download the file.
    Failed { error: String },
}

#[derive(Debug)]
enum ClientCommand {
    Close,

    SetDownloads { items: Vec<DownloadRequestModel> },
    PauseDownloads,
}

#[derive(Debug, Clone)]
struct ClientHandle {
    tx: mpsc::UnboundedSender<ClientCommand>,

    index: Arc<Mutex<Option<Vec<IndexItem>>>>,
    jobs: Arc<DashMap<u64, ClientTransferJob>>,
    paused: Arc<AtomicBool>,
}

struct Client {
    db: Arc<Mutex<Database>>,
    download_directory: Arc<Mutex<Option<String>>>,
    transcode_format: Option<TranscodeFormat>,

    event_tx: mpsc::UnboundedSender<NodeEvent>,
    connection: Connection,

    connected_at: u64,

    next_job_id: Arc<AtomicU64>,
    ready_tx: mpsc::UnboundedSender<u64>,

    index: Arc<Mutex<Option<Vec<IndexItem>>>>,
    jobs: Arc<DashMap<u64, ClientTransferJob>>,
    paused: Arc<AtomicBool>,
    pause_notify: Arc<Notify>,
}

impl Client {
    fn new(
        db: Arc<Mutex<Database>>,
        event_tx: mpsc::UnboundedSender<NodeEvent>,
        connection: Connection,
        transcode_format: Option<TranscodeFormat>,
        download_directory: Arc<Mutex<Option<String>>>,
        #[cfg(feature = "test-hooks")] test_hooks: Arc<TestHooks>,
    ) -> Self {
        let jobs = Arc::new(DashMap::<u64, ClientTransferJob>::new());
        let paused = Arc::new(AtomicBool::new(false));
        let pause_notify = Arc::new(Notify::new());

        // Track whether the first transfer has completed, for counting sessions with >1 transfer.
        // When tracking transferred files we indicate whether it's the first of this session.
        let is_first_transfer = Arc::new(AtomicBool::new(true));

        // spawn a task to handle ready jobs and spawn more tasks to download them
        // Client::run() receives ServerMessageV1::JobStatus messages. jobs marked Ready are sent to this channel
        let (ready_tx, mut ready_rx) = mpsc::unbounded_channel::<u64>();
        tokio::spawn({
            let db = db.clone();
            let jobs = jobs.clone();
            let event_tx = event_tx.clone();
            let connection = connection.clone();
            let download_directory = download_directory.clone();
            let paused = paused.clone();
            let pause_notify = pause_notify.clone();
            async move {
                // convert channel receiver of ready job IDs into a stream for use with buffer_unordered
                let ready_stream = {
                    let jobs = jobs.clone();
                    async_stream::stream! {
                        while let Some(job_id) = ready_rx.recv().await {
                            // if compiled with test hooks, wait for a download permit
                            #[cfg(feature = "test-hooks")]
                            test_hooks.wait_for_download_permit().await;

                            // if paused, wait for unpause before processing received item
                            while paused.load(Ordering::Relaxed) {
                                pause_notify.notified().await;
                            }

                            // check job exists: it may have been removed while paused
                            if jobs.get(&job_id).is_none() {
                                debug!("job {job_id} removed while paused, skipping");
                                continue;
                            }

                            // once yielded, the job will start and can't be cancelled
                            yield job_id;
                        }
                    }
                };

                // map stream of ready ids to futures that download the files
                let buffer = ready_stream
                    .map(|job_id| {
                        // get download directory
                        let download_directory = {
                            let download_directory = download_directory.lock().unwrap();
                            download_directory.clone()
                        };

                        let db = db.clone();
                        let jobs = jobs.clone();
                        let event_tx = event_tx.clone();
                        let connection = connection.clone();
                        let is_first_transfer = is_first_transfer.clone();
                        async move {
                            let remote_endpoint_id = connection.remote_id();

                            // check if download directory is set
                            // we need to do this inside the async block so that the return type of the closure is always the async block's anonymous future
                            let Some(download_directory) = download_directory else {
                                anyhow::bail!("download directory is None, cannot download");
                            };

                            // check job exists and get details
                            let (file_endpoint_id, file_root, file_path) = {
                                let Some(job) = jobs.get(&job_id) else {
                                    anyhow::bail!("received ready for unknown job ID {job_id}");
                                };

                                (
                                    job.file_endpoint_id,
                                    job.file_root.clone(),
                                    job.file_path.clone(),
                                )
                            };

                            debug!("downloading file: {file_root}/{file_path}");

                            // open a bidirectional stream
                            let (mut send, mut recv) = connection.open_bi().await?;

                            // send transfer request with job id
                            let transfer_req = TransferRequest { job_id };
                            let transfer_req_buf = postcard::to_stdvec(&transfer_req)
                                .context("failed to serialize transfer request")?;
                            send.write_u32(transfer_req_buf.len() as u32)
                                .await
                                .context("failed to write transfer request length")?;
                            send.write_all(&transfer_req_buf)
                                .await
                                .context("failed to write transfer request")?;

                            // receive transfer response with metadata
                            let transfer_res_len = recv.read_u32().await?;
                            let mut transfer_res_buf = vec![0; transfer_res_len as usize];
                            recv.read_exact(&mut transfer_res_buf)
                                .await
                                .context("failed to read transfer response")?;
                            let transfer_res: TransferResponse =
                                postcard::from_bytes(&transfer_res_buf)
                                    .context("failed to deserialize transfer response")?;

                            // check transfer response
                            let file_size = match transfer_res {
                                TransferResponse::Ok { file_size } => file_size,
                                TransferResponse::Error { error } => {
                                    // set job status to Failed
                                    jobs.alter(&job_id, |_, mut job| {
                                        job.progress = ClientTransferJobProgress::Failed { error };
                                        job
                                    });

                                    return Ok(());
                                }
                            };

                            // set job status to InProgress
                            let written = Arc::new(AtomicU64::new(0));
                            jobs.alter(&job_id, |_, mut job| {
                                job.progress = ClientTransferJobProgress::InProgress {
                                    started_at: unix_epoch_now_secs(),
                                    file_size,
                                    written: written.clone(),
                                };

                                job
                            });

                            // update model
                            event_tx
                                .send(NodeEvent::ClientChanged {
                                    endpoint_id: remote_endpoint_id,
                                    update: ClientModelUpdate::UpdateTransferJobs,
                                })
                                .expect("failed to send ClientModelUpdate::UpdateTransferJobs");

                            // build file path
                            let local_path = {
                                let root_dir_name =
                                    format!("musicopy-{}-{}", &file_endpoint_id, &file_root);
                                let mut local_path =
                                    TreePath::new(download_directory, root_dir_name.into())?;
                                local_path.push(&file_path);
                                // If transcoding, overwrite the transferred file's extension
                                if let Some(transcode_format) = transcode_format {
                                    local_path.set_extension(transcode_format.extension());
                                }
                                local_path
                            };

                            // create parent directories
                            let parent_dir_path = local_path.parent();
                            if let Some(parent) = parent_dir_path {
                                crate::fs::create_dir_all(&parent)
                                    .await
                                    .context("failed to create directory for root")?;
                            }

                            // open file for writing
                            let file = TreeFile::open_or_create(&local_path, OpenMode::Write)
                                .await
                                .context("failed to open file")?;

                            // copy from stream to file
                            let mut file_progress = WriteProgress::new(written.clone(), file);
                            tokio::io::copy(&mut recv.take(file_size), &mut file_progress).await?;

                            // TODO: handle errors above and update job status

                            // insert or update file in database
                            {
                                let mut db = db.lock().unwrap();
                                db.insert_remote_file(
                                    remote_endpoint_id,
                                    InsertFile {
                                        root: &file_root,
                                        path: &file_path,
                                        local_tree: local_path.root(),
                                        local_path: &local_path.path(),
                                    },
                                )
                                .context("failed to insert remote file in database")?;
                            }

                            // set job status to Finished
                            jobs.alter(&job_id, |_, mut job| {
                                job.progress = ClientTransferJobProgress::Finished {
                                    finished_at: unix_epoch_now_secs(),
                                    file_size,
                                };
                                job
                            });

                            // update model
                            event_tx
                                .send(NodeEvent::ClientChanged {
                                    endpoint_id: remote_endpoint_id,
                                    update: ClientModelUpdate::UpdateTransferJobs,
                                })
                                .expect("failed to send ClientModelUpdate::UpdateTransferJobs");

                            let is_first_transfer = is_first_transfer.swap(false, Ordering::SeqCst);
                            let _ = event_tx.send(NodeEvent::ClientTransferCompleted {
                                endpoint_id: remote_endpoint_id,
                                bytes: file_size,
                                is_first_transfer,
                            });

                            debug!("saved file to {local_path:?}");

                            Ok::<(), anyhow::Error>(())
                        }
                    })
                    .buffer_unordered(4);

                tokio::pin!(buffer);

                // poll the stream to download items with limited concurrency
                while let Some(res) = buffer.next().await {
                    if let Err(e) = res {
                        error!("error downloading item: {e:#}");
                    }
                }
            }
        });

        Self {
            db,
            download_directory,
            transcode_format,

            event_tx,
            connection,

            connected_at: unix_epoch_now_secs(),

            next_job_id: Arc::new(AtomicU64::new(0)),
            ready_tx,

            index: Arc::new(Mutex::new(None)),
            jobs,
            paused,
            pause_notify,
        }
    }

    async fn run(self) -> anyhow::Result<()> {
        let remote_endpoint_id = self.connection.remote_id();

        // spawn connection info watcher task
        // TODO: check that this is cancelled/cleaned up correctly?
        let mut paths_stream = self.connection.paths().stream();
        tokio::spawn({
            let event_tx = self.event_tx.clone();
            async move {
                while let Some(paths) = paths_stream.next().await {
                    let selected_path = paths.iter().find(|path| path.is_selected());
                    let (remote_addr, rtt_ms) = match selected_path {
                        Some(selected_path) => {
                            let remote_addr = selected_path.remote_addr().to_string();
                            let rtt_ms = selected_path.rtt().map(|d| d.as_millis() as u64);
                            (remote_addr, rtt_ms)
                        }
                        None => ("unknown".to_string(), None),
                    };

                    event_tx
                        .send(NodeEvent::ClientChanged {
                            endpoint_id: remote_endpoint_id,
                            update: ClientModelUpdate::UpdateConnectionInfo {
                                remote_addr,
                                rtt_ms,
                            },
                        })
                        .expect("failed to send ClientModelUpdate::UpdateConnectionInfo");
                }
            }
        });

        let (tx, mut rx) = mpsc::unbounded_channel();

        // open a bidirectional QUIC stream
        let (send, recv) = self.connection.open_bi().await?;

        // wrap in framed codecs
        let mut send = FramedWrite::new(send, LengthDelimitedCodec::new()).with_flat_map(
            |message: ClientMessageV1| {
                let buf: Vec<u8> =
                    postcard::to_stdvec(&message).expect("failed to serialize message");
                futures::stream::once(futures::future::ready(Ok(Bytes::from(buf))))
            },
        );
        let mut recv = FramedRead::new(recv, LengthDelimitedCodec::new())
            .map_err(|e| anyhow::anyhow!("failed to read from connection: {e:?}"))
            .map(|res| {
                res.and_then(|bytes| {
                    postcard::from_bytes::<ServerMessageV1>(&bytes)
                        .map_err(|e| anyhow::anyhow!("failed to deserialize message: {e:?}"))
                })
            });

        // send client Identify
        send.send(ClientMessageV1::Identify {
            name: device_name().to_string(),
            transcode_format: self.transcode_format,
        })
        .await
        .expect("failed to send Identify message");

        // wait for server Identify
        // TODO: also wait for commands
        let Some(Ok(message)) = recv.next().await else {
            error!("failed to receive Identify message");
            return Ok(());
        };
        let server_name = match message {
            ServerMessageV1::Identify(name) => name,
            _ => {
                error!("unexpected message, expected Identify: {message:?}");
                return Ok(());
            }
        };

        // handshake finished, send handle to Node
        let handle = ClientHandle {
            tx,

            index: self.index.clone(),
            jobs: self.jobs.clone(),
            paused: self.paused.clone(),
        };
        self.event_tx
            .send(NodeEvent::ClientOpened {
                endpoint_id: remote_endpoint_id,
                handle,

                name: server_name.clone(),
                connected_at: self.connected_at,
            })
            .expect("failed to send NodeEvent::ClientOpened");

        // waiting loop, wait for server Accepted
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ClientCommand::Close => {
                            return Ok(());
                        }

                        ClientCommand::SetDownloads { .. } => {
                            warn!("unexpected SetDownloads command in waiting loop");
                        }
                        ClientCommand::PauseDownloads => {
                            warn!("unexpected PauseDownloads command in waiting loop");
                        }
                    }
                }

                next_message = recv.next() => {
                    match next_message {
                        Some(Ok(message)) => {
                            match message {
                                ServerMessageV1::Accepted => {
                                    info!("server accepted the connection");

                                    // continue to next state
                                    break;
                                }
                                _ => {
                                    debug!("unexpected message (waiting for Accepted): {message:?}");
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("error receiving message: {e}");
                        }
                        None => {
                            anyhow::bail!("control stream closed, shutting down client");
                        }
                    }
                }

                else => {
                    anyhow::bail!("stream and receiver closed while waiting for Accepted");
                }
            }
        }

        // mark as accepted
        self.event_tx
            .send(NodeEvent::ClientChanged {
                endpoint_id: remote_endpoint_id,
                update: ClientModelUpdate::Accept,
            })
            .expect("failed to send ClientModelUpdate::Accept");

        // update recent servers in database
        {
            let db = self.db.lock().unwrap();
            db.update_recent_server(remote_endpoint_id, &server_name, self.connected_at)
                .context("failed to update recent server in database")?;
        }
        self.event_tx
            .send(NodeEvent::RecentServersChanged)
            .expect("failed to send NodeEvent::RecentServersChanged");

        // main loop
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ClientCommand::Close => {
                            self.connection.close(0u32.into(), b"close");
                            break;
                        }

                        ClientCommand::SetDownloads { items } => {
                            info!("setting downloads: {} items", items.len());

                            // get index
                            let index = {
                                let index = self.index.lock().unwrap();
                                index.clone()
                            };
                            let Some(index) = index else {
                                error!("SetDownloads: no index available");
                                continue;
                            };

                            // only remove jobs if paused
                            let is_paused = self.paused.load(Ordering::Relaxed);
                            if is_paused {
                                // build set of requested keys
                                let requested_keys: HashSet<(&str, &str)> = items.iter()
                                    .map(|item| (item.root.as_str(), item.path.as_str()))
                                    .collect();

                                // remove jobs that aren't in the new set of requested items and aren't
                                // already in progress, finished, or failed
                                let jobs_to_remove: Vec<u64> = self.jobs.iter()
                                    .filter(|entry| {
                                        let job = entry.value();
                                        let key = (job.file_root.as_str(), job.file_path.as_str());
                                        let should_remove = !requested_keys.contains(&key);
                                        let is_started = matches!(
                                            job.progress,
                                            ClientTransferJobProgress::InProgress { .. }
                                            | ClientTransferJobProgress::Finished { .. }
                                            | ClientTransferJobProgress::Failed { .. }
                                        );
                                        should_remove && !is_started
                                    })
                                    .map(|entry| *entry.key())
                                    .collect();

                                for job_id in jobs_to_remove {
                                    if let Some((_, job)) = self.jobs.remove(&job_id) {
                                        debug!("removed job {job_id}: {}/{}", job.file_root, job.file_path);
                                    }
                                }
                            }

                            // build set of existing jobs
                            let existing_keys: HashSet<(String, String)> = self.jobs.iter()
                                .map(|entry| {
                                    let job = entry.value();
                                    (job.file_root.clone(), job.file_path.clone())
                                })
                                .collect();

                            // get download directory
                            let download_directory = {
                                let download_directory = self.download_directory.lock().unwrap();
                                download_directory.clone()
                            };

                            // create jobs for new items
                            let download_requests = {
                                let db = self.db.lock().unwrap();
                                items.into_iter().flat_map(|item| {
                                    let Ok(file_endpoint_id) = item.endpoint_id.parse() else {
                                        warn!("SetDownloads: invalid endpoint ID");
                                        return None;
                                    };

                                    // skip if job already exists for this (root, path)
                                    if existing_keys.contains(&(item.root.clone(), item.path.clone())) {
                                        return None;
                                    }

                                    // find item in index
                                    let Some(_index_item) = index.iter().find(|i| {
                                        i.endpoint_id == file_endpoint_id && i.root == item.root && i.path == item.path
                                    }) else {
                                        warn!("SetDownloads: item not found in index: {item:?}");
                                        return None;
                                    };

                                    // check if file is downloaded to the current download directory
                                    let downloaded = download_directory.as_ref().is_some_and(|download_directory| {
                                        db.exists_file_by_node_root_path_localtree(
                                            file_endpoint_id, &item.root, &item.path, download_directory,
                                        )
                                        .unwrap_or(false)
                                    });
                                    if downloaded {
                                        // TODO: maybe add a finished job instead
                                        return None;
                                    }

                                    let job_id = self.next_job_id.fetch_add(1, Ordering::Relaxed);
                                    self.jobs.insert(job_id, ClientTransferJob {
                                        progress: ClientTransferJobProgress::Requested,
                                        file_endpoint_id,
                                        file_root: item.root.clone(),
                                        file_path: item.path.clone(),
                                    });

                                    Some(DownloadItem {
                                        job_id,
                                        endpoint_id: file_endpoint_id,
                                        root: item.root,
                                        path: item.path,
                                    })
                                }).collect::<Vec<_>>()
                            };

                            // send download request for new jobs
                            if !download_requests.is_empty() {
                                send.send(ClientMessageV1::Download(download_requests))
                                    .await
                                    .expect("failed to send Download message");
                            }

                            // unpause
                            self.paused.store(false, Ordering::Relaxed);
                            self.pause_notify.notify_waiters();

                            // update model
                            self.event_tx.send(NodeEvent::ClientChanged {
                                endpoint_id: remote_endpoint_id,
                                update: ClientModelUpdate::UpdateTransferJobs,
                            }).expect("failed to send ClientModelUpdate::UpdateTransferJobs");
                            self.event_tx.send(NodeEvent::ClientChanged {
                                endpoint_id: remote_endpoint_id,
                                update: ClientModelUpdate::UpdateIndex,
                            }).expect("failed to send ClientModelUpdate::UpdateIndex");
                            self.event_tx.send(NodeEvent::ClientChanged {
                                endpoint_id: remote_endpoint_id,
                                update: ClientModelUpdate::UpdatePaused,
                            }).expect("failed to send ClientModelUpdate::UpdatePaused");
                        }

                        ClientCommand::PauseDownloads => {
                            info!("pausing downloads");

                            // pause
                            self.paused.store(true, Ordering::Relaxed);
                            self.pause_notify.notify_waiters();

                            // update model
                            self.event_tx.send(NodeEvent::ClientChanged {
                                endpoint_id: remote_endpoint_id,
                                update: ClientModelUpdate::UpdateTransferJobs,
                            }).expect("failed to send ClientModelUpdate::UpdateTransferJobs");
                            self.event_tx.send(NodeEvent::ClientChanged {
                                endpoint_id: remote_endpoint_id,
                                update: ClientModelUpdate::UpdateIndex,
                            }).expect("failed to send ClientModelUpdate::UpdateIndex");
                            self.event_tx.send(NodeEvent::ClientChanged {
                                endpoint_id: remote_endpoint_id,
                                update: ClientModelUpdate::UpdatePaused,
                            }).expect("failed to send ClientModelUpdate::UpdatePaused");
                        }
                    }
                }

                next_message = recv.next() => {
                    match next_message {
                        Some(Ok(message)) => {
                            match message {
                                ServerMessageV1::Index(new_index) => {
                                    info!("received index with {} items", new_index.len());
                                    {
                                        let mut index = self.index.lock().unwrap();
                                        *index = Some(new_index);
                                    }

                                    // update model
                                    self.event_tx.send(NodeEvent::ClientChanged {
                                        endpoint_id: remote_endpoint_id,
                                        update: ClientModelUpdate::UpdateIndex,
                                    }).expect("failed to send ClientModelUpdate::UpdateIndex");
                                }

                                ServerMessageV1::IndexUpdate(updates) => {
                                    info!("received index update with {} items", updates.len());
                                    {
                                        let mut index = self.index.lock().unwrap();
                                        if let Some(index) = index.as_mut() {
                                            for update in updates {
                                                match update {
                                                    IndexUpdateItem::FileSize { endpoint_id, root, path, file_size } => {
                                                        // TODO: don't be exponential
                                                        for item in index.iter_mut() {
                                                            if item.endpoint_id == endpoint_id && item.root == root && item.path == path {
                                                                item.file_size = file_size;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            warn!("received index update but index is None, ignoring");
                                        }
                                    }

                                    // update model
                                    self.event_tx.send(NodeEvent::ClientChanged {
                                        endpoint_id: remote_endpoint_id,
                                        update: ClientModelUpdate::UpdateIndex,
                                    }).expect("failed to send ClientModelUpdate::UpdateIndex");
                                }

                                ServerMessageV1::JobStatus(status_changes) => {
                                    for (job_id, status) in status_changes {
                                        match status {
                                            JobStatusItem::Transcoding => {
                                                // set job status to Transcoding
                                                self.jobs.alter(&job_id, |_, mut job| {
                                                    job.progress = ClientTransferJobProgress::Transcoding;
                                                    job
                                                });
                                            },
                                            JobStatusItem::Ready { file_size } => {
                                                // set job status to Ready
                                                self.jobs.alter(&job_id, |_, mut job| {
                                                    job.progress = ClientTransferJobProgress::Ready { file_size };
                                                    job
                                                });

                                                // send job id to ready channel
                                                self.ready_tx.send(job_id).context("failed to send job id to ready channel")?;
                                            },
                                            JobStatusItem::Failed { error } => {
                                                // set job status to Failed
                                                self.jobs.alter(&job_id, |_, mut job| {
                                                    job.progress = ClientTransferJobProgress::Failed { error };
                                                    job
                                                });
                                            },
                                        }
                                    }

                                    // update model
                                    self.event_tx.send(NodeEvent::ClientChanged {
                                        endpoint_id: remote_endpoint_id,
                                        update: ClientModelUpdate::UpdateTransferJobs,
                                    }).expect("failed to send ClientModelUpdate::UpdateTransferJobs");
                                }

                                _ => {
                                    debug!("unexpected message in main loop: {message:?}");
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("error receiving message: {e}");
                        }
                        None => {
                            info!("control stream closed, shutting down client");
                            break;
                        }
                    }
                }

                _ = self.connection.closed() => {
                    info!("connection closed");
                    break;
                }

                else => {
                    warn!("all senders dropped in Client::run, shutting down");
                    break;
                }
            }
        }

        self.connection.closed().await;

        Ok(())
    }
}

/// Returns the current system time in seconds since the Unix epoch.
fn unix_epoch_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Wrapper for `T: AsyncWrite` that tracks the number of bytes written in an `Arc<AtomicU64>`.
struct WriteProgress<T> {
    inner: T,
    written: Arc<AtomicU64>,
}

impl<T> WriteProgress<T> {
    fn new(written: Arc<AtomicU64>, inner: T) -> Self {
        Self { inner, written }
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for WriteProgress<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let res = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let std::task::Poll::Ready(Ok(size)) = &res {
            self.written.fetch_add(*size as u64, Ordering::Relaxed);
        }
        res
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }

    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let res = Pin::new(&mut self.inner).poll_write_vectored(cx, bufs);
        if let std::task::Poll::Ready(Ok(size)) = &res {
            self.written.fetch_add(*size as u64, Ordering::Relaxed);
        }
        res
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }
}
