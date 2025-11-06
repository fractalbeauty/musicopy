//! Networking.
//!
//! A `Node` is an Iroh node that can perform the client or server end of the protocol.
//!
//! A `Client` is an outgoing connection to a server.
//! Clients request files, primarily used in the mobile app.
//!
//! A `Server` is an incoming connection from a client.
//! Servers send files, primarily used in the desktop app.

use crate::{
    EventHandler,
    database::{Database, InsertFile},
    fs::{OpenMode, TreeFile, TreePath},
    library::{
        Library, LibraryCommand,
        transcode::{TranscodeStatus, TranscodeStatusCache},
    },
    model::CounterModel,
};
use anyhow::Context;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt, TryStreamExt};
use iroh::{
    Endpoint, NodeAddr, NodeId, SecretKey,
    endpoint::Connection,
    protocol::{ProtocolHandler, Router},
};
use log::error;
use n0_future::future::Boxed;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};
use tokio::{
    io::{AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{mpsc, oneshot},
};
use tokio_util::{
    bytes::Bytes,
    codec::{FramedRead, FramedWrite, LengthDelimitedCodec},
    sync::CancellationToken,
};

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
    pub node_id: String,
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

/// Model of an item in the index sent by the server.
#[derive(Debug, Clone, uniffi::Record)]
pub struct IndexItemModel {
    pub node_id: String,
    pub root: String,
    pub path: String,

    pub hash_kind: String,
    pub hash: Vec<u8>,

    pub file_size: FileSizeModel,

    pub downloaded: bool,
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
    pub node_id: String,
    pub connected_at: u64,

    pub state: ClientStateModel,

    pub connection_type: String,
    pub latency_ms: Option<u64>,

    pub index: Option<Vec<IndexItemModel>>,
    pub transfer_jobs: Vec<TransferJobModel>,
}

/// Model of a recently connected server.
#[derive(Debug, Clone, uniffi::Record)]
pub struct RecentServerModel {
    pub node_id: String,
    pub connected_at: u64,
}

/// Node state sent to the UI.
///
/// Needs to be Clone to send snapshots to the UI.
#[derive(Debug, Clone, uniffi::Record)]
pub struct NodeModel {
    pub node_id: String,

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

    pub trusted_nodes: Vec<String>,
    pub recent_servers: Vec<RecentServerModel>,
}

/// Model of an item selected to be downloaded.
#[derive(Debug, uniffi::Record)]
pub struct DownloadPartialItemModel {
    pub node_id: String,
    pub root: String,
    pub path: String,
}

/// A command sent by the UI to the node.
#[derive(Debug)]
pub enum NodeCommand {
    SetDownloadDirectory(String),

    Connect {
        addr: NodeAddr,
        callback: oneshot::Sender<anyhow::Result<()>>,
    },

    AcceptConnection(NodeId),
    DenyConnection(NodeId),

    CloseClient(NodeId),
    CloseServer(NodeId),

    DownloadAll {
        client: NodeId,
    },
    DownloadPartial {
        client: NodeId,
        items: Vec<DownloadPartialItemModel>,
    },

    TrustNode(NodeId),
    UntrustNode(NodeId),

    Stop,

    // unused, for debugging filesystem code
    WriteTestFile(String),
}

/// An event sent from a server or client to the node.
enum NodeEvent {
    FilesRequested(Vec<(String, Vec<u8>)>),

    RecentServersChanged,

    ServerOpened {
        node_id: NodeId,
        handle: ServerHandle,

        name: String,
        connected_at: u64,
    },
    ServerChanged {
        node_id: NodeId,
        update: ServerModelUpdate,
    },
    ServerClosed {
        node_id: NodeId,
        error: Option<String>,
    },

    ClientOpened {
        node_id: NodeId,
        handle: ClientHandle,

        name: String,
        connected_at: u64,
    },
    ClientChanged {
        node_id: NodeId,
        update: ClientModelUpdate,
    },
    ClientClosed {
        node_id: NodeId,
        error: Option<String>,
    },
}

/// An update to a server model.
enum ServerModelUpdate {
    Accept,
    PollRemoteInfo,
    UpdateTransferJobs,
    Close { error: Option<String> },
}

/// An update to a client model.
enum ClientModelUpdate {
    Accept,
    PollRemoteInfo,
    UpdateIndex,
    UpdateTransferJobs,
    Close { error: Option<String> },
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
        node_id: NodeId,
        name: String,
        connected_at: u64,
    },
    UpdateServer {
        node_id: NodeId,
        update: ServerModelUpdate,
    },

    CreateClient {
        node_id: NodeId,
        name: String,
        connected_at: u64,
    },
    UpdateClient {
        node_id: NodeId,
        update: ClientModelUpdate,
    },
}

pub struct Node {
    event_handler: Arc<dyn EventHandler>,
    db: Arc<Mutex<Database>>,
    transcode_status_cache: TranscodeStatusCache,

    router: Router,
    protocol: Protocol,

    command_tx: mpsc::UnboundedSender<NodeCommand>,
    event_tx: mpsc::UnboundedSender<NodeEvent>,

    servers: Mutex<HashMap<NodeId, ServerHandle>>,
    clients: Mutex<HashMap<NodeId, ClientHandle>>,

    download_directory: Arc<Mutex<Option<String>>>,

    model: Mutex<NodeModel>,
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
    ) -> anyhow::Result<(Arc<Self>, NodeRun)> {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .bind()
            .await?;
        let protocol = Protocol::new(db.clone(), transcode_status_cache.clone(), event_tx.clone());

        let router = Router::builder(endpoint)
            .accept(Protocol::ALPN, protocol.clone())
            .spawn();

        let model = NodeModel {
            node_id: router.endpoint().node_id().to_string(),

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
            transcode_status_cache,

            router,
            protocol,

            command_tx,
            event_tx,

            servers: Mutex::new(HashMap::new()),
            clients: Mutex::new(HashMap::new()),

            download_directory: Arc::new(Mutex::new(None)),

            model: Mutex::new(model),
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

        // TODO: observe iroh changes instead of polling
        // spawn iroh polling task
        tokio::spawn({
            let node = node.clone();
            async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let home_relay = node
                        .router
                        .endpoint()
                        .home_relay()
                        .get()
                        .ok()
                        .flatten()
                        .map(|url| url.to_string())
                        .unwrap_or_else(|| "none".to_string());
                    node.update_model(NodeModelUpdate::UpdateHomeRelay { home_relay });
                }
            }
        });

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

        loop {
            tokio::select! {
                Some(command) = command_rx.recv() => {
                    match command {
                        NodeCommand::SetDownloadDirectory(path) => {
                            let mut download_directory = self.download_directory.lock().unwrap();
                            *download_directory = Some(path);
                        },

                        NodeCommand::Connect { addr, callback } => {
                            let node = self.clone();
                            tokio::task::spawn(async move {
                                log::debug!("starting connect");
                                let res = node.connect(addr).await;
                                log::debug!("connect result: {res:?}");
                                if let Err(e) = callback.send(res) {
                                    log::error!("failed to send res: {e:?}");
                                }
                            });
                        },

                        NodeCommand::AcceptConnection(node_id) => {
                            let servers = self.servers.lock().unwrap();
                            if let Some(server_handle) = servers.get(&node_id) {
                                server_handle.tx.send(ServerCommand::Accept).expect("failed to send ServerCommand::Accept");
                            } else {
                                log::error!("AcceptConnection: no server found with node_id: {node_id}");
                            }
                        },
                        NodeCommand::DenyConnection(node_id) => {
                            let servers = self.servers.lock().unwrap();
                            if let Some(server_handle) = servers.get(&node_id) {
                                server_handle.tx.send(ServerCommand::Close).expect("failed to send ServerCommand::Close");
                            } else {
                                log::error!("DenyConnection: no server found with node_id: {node_id}");
                            }
                        },

                        NodeCommand::CloseClient(node_id) => {
                            let clients = self.clients.lock().unwrap();
                            if let Some(client_handle) = clients.get(&node_id) {
                                client_handle.tx.send(ClientCommand::Close).expect("failed to send ClientCommand::Close");
                            } else {
                                log::error!("CloseClient: no client found with node_id: {node_id}");
                            }
                        }
                        NodeCommand::CloseServer(node_id) => {
                            let servers = self.servers.lock().unwrap();
                            if let Some(server_handle) = servers.get(&node_id) {
                                server_handle.tx.send(ServerCommand::Close).expect("failed to send ServerCommand::Close");
                            } else {
                                log::error!("CloseServer: no server found with node_id: {node_id}");
                            }
                        },

                        NodeCommand::DownloadAll { client } => {
                            // check that download directory is set before downloading
                            {
                                let download_directory = self.download_directory.lock().unwrap();
                                if download_directory.is_none() {
                                    log::error!("DownloadAll: download directory not set");
                                    continue;
                                }
                            };

                            // send command to target client
                            let clients = self.clients.lock().unwrap();
                            if let Some(client_handle) = clients.get(&client) {
                                client_handle.tx.send(ClientCommand::DownloadAll).expect("failed to send ClientCommand::DownloadAll");
                            } else {
                                log::error!("DownloadAll: no client found with node_id: {client}");
                            }
                        },
                        NodeCommand::DownloadPartial { client, items } => {
                            // check that download directory is set before downloading
                            {
                                let download_directory = self.download_directory.lock().unwrap();
                                if download_directory.is_none() {
                                    log::error!("DownloadPartial: download directory not set");
                                    continue;
                                }
                            };

                            // send command to target client
                            let clients = self.clients.lock().unwrap();
                            if let Some(client_handle) = clients.get(&client) {
                                client_handle.tx.send(ClientCommand::DownloadPartial { items }).expect("failed to send ClientCommand::DownloadPartial");
                            } else {
                                log::error!("DownloadPartial: no client found with node_id: {client}");
                            }
                        }

                        NodeCommand::TrustNode(node_id) => {
                            // persist to database
                            {
                                let db = self.db.lock().unwrap();
                                if let Err(e) = db.add_trusted_node(node_id) {
                                    log::error!("failed to add trusted node to database: {e:#}");
                                }
                            }

                            // update model
                            self.update_model(NodeModelUpdate::UpdateTrustedNodes);
                        }
                        NodeCommand::UntrustNode(node_id) => {
                            // persist to database
                            {
                                let db = self.db.lock().unwrap();
                                if let Err(e) = db.remove_trusted_node(node_id) {
                                    log::error!("failed to remove trusted node from database: {e:#}");
                                }
                            }

                            // update model
                            self.update_model(NodeModelUpdate::UpdateTrustedNodes);
                        }

                        NodeCommand::WriteTestFile(root) => {
                            log::warn!("core: received NodeCommand::WriteTestFile");

                            tokio::spawn(async move {
                                log::debug!("core: inside WriteTestFile task");

                                let mut path = match crate::fs::TreePath::from_root(root) {
                                    Ok(p) => p,
                                    Err(e) => {
                                        log::error!("core: WriteTestFile failed to create TreePath from root: {e:#}");
                                        return;
                                    }
                                };
                                path.push("test.txt");

                                log::debug!("core: WriteTestFile opening or creating file at {:?}", path);

                                let mut file = match crate::fs::TreeFile::open_or_create(
                                    &path,
                                    crate::fs::OpenMode::Write,
                                )
                                .await
                                {
                                    Ok(f) => f,
                                    Err(e) => {
                                        log::error!("core: WriteTestFile failed to open or create file: {e:#}");
                                        return;
                                    }
                                };

                                log::debug!("core: WriteTestFile opened or created file successfully");

                                if let Err(e) = file.write_all(b"meow meow").await {
                                    log::error!("core: WriteTestFile failed to write to file: {e:#}");
                                    return;
                                }

                                log::debug!("core: WriteTestFile wrote to file successfully");
                            });
                        }

                        NodeCommand::Stop => break,
                    }
                }

                Some(event) = event_rx.recv() => {
                    match event {
                        NodeEvent::FilesRequested(files) => {
                            if let Err(e) = library.send(LibraryCommand::PrioritizeTranscodes(files.clone())) {
                                error!("NodeEvent::FilesRequested: failed to send to library: {e:#}");
                            }
                        }

                        NodeEvent::RecentServersChanged => {
                            self.update_model(NodeModelUpdate::UpdateRecentServers);
                        }

                        NodeEvent::ServerOpened { node_id, handle, name, connected_at } => {
                            {
                                let mut servers = self.servers.lock().unwrap();
                                servers.insert(node_id, handle);
                            }

                            self.update_model(NodeModelUpdate::CreateServer { node_id, name, connected_at });
                        }

                        NodeEvent::ServerChanged { node_id, update } => {
                            self.update_model(NodeModelUpdate::UpdateServer { node_id, update });
                        }

                        NodeEvent::ServerClosed { node_id, error } => {
                            {
                                let mut servers = self.servers.lock().unwrap();
                                servers.remove(&node_id);
                            }

                            self.update_model(NodeModelUpdate::UpdateServer { node_id, update: ServerModelUpdate::Close { error } });
                        }

                        NodeEvent::ClientOpened { node_id, handle, name, connected_at } => {
                            {
                                let mut clients = self.clients.lock().unwrap();
                                clients.insert(node_id, handle);
                            }

                            self.update_model(NodeModelUpdate::CreateClient { node_id, name, connected_at });
                        }

                        NodeEvent::ClientChanged { node_id, update } => {
                            self.update_model(NodeModelUpdate::UpdateClient { node_id, update });
                        }

                        NodeEvent::ClientClosed { node_id, error } => {
                            {
                                let mut clients = self.clients.lock().unwrap();
                                clients.remove(&node_id);
                            }

                            self.update_model(NodeModelUpdate::UpdateClient { node_id, update: ClientModelUpdate::Close { error } });
                        }
                    }
                }

                else => {
                    log::warn!("all senders dropped in Node::run, shutting down");
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
                model.send_ipv4 = metrics.magicsock.send_ipv4.get();
                model.send_ipv6 = metrics.magicsock.send_ipv6.get();
                model.send_relay = metrics.magicsock.send_relay.get();
                model.recv_ipv4 = metrics.magicsock.recv_data_ipv4.get();
                model.recv_ipv6 = metrics.magicsock.recv_data_ipv6.get();
                model.recv_relay = metrics.magicsock.recv_data_relay.get();
                model.conn_success = metrics.magicsock.connection_handshake_success.get();
                model.conn_direct = metrics.magicsock.connection_became_direct.get();

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
                        .iter()
                        .map(|node_id| node_id.to_string())
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
                                node_id: node.node_id.to_string(),
                                connected_at: node.connected_at,
                            })
                            .collect(),
                        Err(e) => {
                            log::error!("failed to get recent servers from database: {e:#}");
                            Vec::new()
                        }
                    }
                };

                let mut model = self.model.lock().unwrap();
                model.recent_servers = recent_servers;

                self.event_handler.on_node_model_snapshot(model.clone());
            }

            NodeModelUpdate::CreateServer {
                node_id,
                name,
                connected_at,
            } => {
                let node_id = node_id.to_string();

                let mut model = self.model.lock().unwrap();
                model.servers.insert(
                    node_id.clone(),
                    ServerModel {
                        name,
                        node_id,
                        connected_at,

                        state: ServerStateModel::Pending,

                        connection_type: "unknown".to_string(),
                        latency_ms: None,

                        transfer_jobs: Vec::new(),
                    },
                );

                self.event_handler.on_node_model_snapshot(model.clone());
            }

            NodeModelUpdate::UpdateServer { node_id, update } => {
                let node_id_string = node_id.to_string();

                let mut model = self.model.lock().unwrap();
                let Some(server) = model.servers.get_mut(&node_id_string) else {
                    log::warn!(
                        "failed to apply NodeModelUpdate::UpdateServer: no server model found"
                    );
                    return;
                };

                match update {
                    ServerModelUpdate::Accept => {
                        server.state = ServerStateModel::Accepted;
                    }
                    ServerModelUpdate::PollRemoteInfo => {
                        let remote_info = self.router.endpoint().remote_info(node_id);
                        let connection_type = remote_info
                            .as_ref()
                            .map(|info| info.conn_type.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        let latency_ms = remote_info
                            .and_then(|info| info.latency)
                            .map(|latency| latency.as_millis() as u64);

                        server.connection_type = connection_type;
                        server.latency_ms = latency_ms;
                    }
                    ServerModelUpdate::UpdateTransferJobs => {
                        let server_handles = self.servers.lock().unwrap();
                        let Some(server_handle) = server_handles.get(&node_id) else {
                            log::warn!(
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
                node_id,
                name,
                connected_at,
            } => {
                let node_id = node_id.to_string();

                let mut model = self.model.lock().unwrap();
                model.clients.insert(
                    node_id.clone(),
                    ClientModel {
                        name,
                        node_id,
                        connected_at,

                        state: ClientStateModel::Pending,

                        connection_type: "unknown".to_string(),
                        latency_ms: None,

                        index: None,
                        transfer_jobs: Vec::new(),
                    },
                );

                self.event_handler.on_node_model_snapshot(model.clone());
            }

            NodeModelUpdate::UpdateClient { node_id, update } => {
                let node_id_string = node_id.to_string();

                let mut model = self.model.lock().unwrap();
                let Some(client) = model.clients.get_mut(&node_id_string) else {
                    log::warn!(
                        "failed to apply NodeModelUpdate::UpdateClient: no client model found"
                    );
                    return;
                };

                match update {
                    ClientModelUpdate::Accept => {
                        client.state = ClientStateModel::Accepted;
                    }
                    ClientModelUpdate::PollRemoteInfo => {
                        let remote_info = self.router.endpoint().remote_info(node_id);
                        let connection_type = remote_info
                            .as_ref()
                            .map(|info| info.conn_type.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        let latency_ms = remote_info
                            .and_then(|info| info.latency)
                            .map(|latency| latency.as_millis() as u64);

                        client.connection_type = connection_type;
                        client.latency_ms = latency_ms;
                    }
                    ClientModelUpdate::UpdateIndex => {
                        let client_handles = self.clients.lock().unwrap();
                        let Some(client_handle) = client_handles.get(&node_id) else {
                            log::warn!(
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
                                    let downloaded = download_directory.as_ref().is_some_and(
                                        |download_directory| {
                                            db.exists_file_by_node_root_path_localtree(
                                                node_id,
                                                &item.root,
                                                &item.path,
                                                &download_directory,
                                            )
                                            .unwrap_or(false)
                                        },
                                    );

                                    IndexItemModel {
                                        node_id: node_id.to_string(),
                                        root: item.root,
                                        path: item.path,

                                        hash_kind: item.hash_kind,
                                        hash: item.hash,

                                        file_size: match item.file_size {
                                            FileSize::Unknown => FileSizeModel::Unknown,
                                            FileSize::Estimated(n) => FileSizeModel::Estimated(n),
                                            FileSize::Actual(n) => FileSizeModel::Actual(n),
                                        },

                                        downloaded,
                                    }
                                })
                                .collect();

                            client.index = Some(index);
                        } else {
                            log::warn!("ClientModelUpdate::UpdateIndex: no index found");
                            client.index = None;
                        }
                    }
                    ClientModelUpdate::UpdateTransferJobs => {
                        let client_handles = self.clients.lock().unwrap();
                        let Some(client_handle) = client_handles.get(&node_id) else {
                            log::warn!(
                                "failed to apply ClientModelUpdate::UpdateTransferJobs: no client handle found"
                            );
                            return;
                        };

                        let transfer_jobs = client_handle
                            .jobs
                            .iter()
                            .map(|entry| {
                                let job = entry.value();

                                let (progress, file_size) = match &job.progress {
                                    ClientTransferJobProgress::Requested => {
                                        (TransferJobProgressModel::Requested, None)
                                    }

                                    ClientTransferJobProgress::Transcoding => {
                                        (TransferJobProgressModel::Transcoding, None)
                                    }

                                    ClientTransferJobProgress::Ready { file_size } => {
                                        (TransferJobProgressModel::Ready, Some(*file_size))
                                    }

                                    ClientTransferJobProgress::InProgress {
                                        started_at,
                                        file_size,
                                        written,
                                    } => (
                                        TransferJobProgressModel::InProgress {
                                            started_at: *started_at,
                                            bytes: Arc::new(CounterModel::from(written)),
                                        },
                                        Some(*file_size),
                                    ),

                                    ClientTransferJobProgress::Finished {
                                        finished_at,
                                        file_size,
                                    } => (
                                        TransferJobProgressModel::Finished {
                                            finished_at: *finished_at,
                                        },
                                        Some(*file_size),
                                    ),

                                    ClientTransferJobProgress::Failed { error } => (
                                        TransferJobProgressModel::Failed {
                                            error: error.clone(),
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

                        client.transfer_jobs = transfer_jobs;
                    }
                    ClientModelUpdate::Close { error } => {
                        client.state = ClientStateModel::Closed { error };
                    }
                }

                self.event_handler.on_node_model_snapshot(model.clone());
            }
        }
    }

    // TODO: maybe replace with methods?
    pub fn send(self: &Arc<Self>, command: NodeCommand) -> anyhow::Result<()> {
        self.command_tx
            .send(command)
            .map_err(|e| anyhow::anyhow!("failed to send command: {e:?}"))
    }

    async fn connect(self: &Arc<Self>, addr: NodeAddr) -> anyhow::Result<()> {
        // connect before spawning the task, so we can return an error immediately
        let connection = self.router.endpoint().connect(addr, Protocol::ALPN).await?;

        let node_id = connection.remote_node_id()?;
        log::info!("opened connection to {node_id}");

        let db = self.db.clone();
        let event_tx = self.event_tx.clone();
        let download_directory = self.download_directory.clone();
        tokio::spawn(async move {
            let client = Client::new(db, event_tx.clone(), connection, download_directory);

            let res = client.run().await;
            if let Err(e) = &res {
                log::error!("error during client.run(): {e:#}");
            }

            // notify node
            event_tx
                .send(NodeEvent::ClientClosed {
                    node_id,
                    error: res.err().map(|e| format!("{e:#}")),
                })
                .expect("failed to send NodeEvent::ClientClosed");
        });

        Ok(())
    }

    /// Check if stored remote files still exist locally.
    async fn check_remote_files(self: &Arc<Self>) -> anyhow::Result<()> {
        // get remote files by getting files where node ID is not the local node ID
        let remote_files = {
            let db = self.db.lock().unwrap();
            db.get_files_by_ne_node_id(self.router.endpoint().node_id())?
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
                log::warn!("check_remote_files: failed to create TreePath: {e:#}");
            }

            if !path.is_ok_and(|p| p.exists()) {
                missing_files.push((remote_file.local_tree, remote_file.local_path));
            }
        }

        // remove from db
        if !missing_files.is_empty() {
            log::warn!(
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

    event_tx: mpsc::UnboundedSender<NodeEvent>,
}

impl Protocol {
    const ALPN: &'static [u8] = b"musicopy/0";

    fn new(
        db: Arc<Mutex<Database>>,
        transcode_status_cache: TranscodeStatusCache,

        event_tx: mpsc::UnboundedSender<NodeEvent>,
    ) -> Self {
        Self {
            db,
            transcode_status_cache,

            event_tx,
        }
    }
}

impl ProtocolHandler for Protocol {
    fn accept(&self, connection: iroh::endpoint::Connection) -> Boxed<anyhow::Result<()>> {
        let db = self.db.clone();
        let transcode_status_cache = self.transcode_status_cache.clone();
        let event_tx = self.event_tx.clone();
        Box::pin(async move {
            let node_id = connection.remote_node_id()?;
            log::info!("accepted connection from {node_id}");

            let server = Server::new(db, transcode_status_cache, connection, event_tx.clone());

            let res = server.run().await;
            if let Err(e) = &res {
                log::error!("error during server.run(): {e:#}");
            }

            // notify node
            event_tx
                .send(NodeEvent::ServerClosed {
                    node_id,
                    error: res.err().map(|e| format!("{e:#}")),
                })
                .expect("failed to send NodeEvent::ServerClosed");

            Ok(())
        })
    }
}

/// An item requested for downloading by the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadItem {
    job_id: u64,

    node_id: NodeId,
    root: String,
    path: String,
}

/// A message sent by the client end of a connection on the control stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ClientMessage {
    /// Identify the client with a friendly name.
    Identify(String),
    /// Request to download files.
    Download(Vec<DownloadItem>),
}

/// An unknown, estimated, or actual file size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileSize {
    Unknown,
    Estimated(u64),
    Actual(u64),
}

/// An item available for downloading from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexItem {
    node_id: NodeId,
    root: String,
    path: String,

    hash_kind: String,
    hash: Vec<u8>,

    file_size: FileSize,
}

/// An update to an item in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum IndexUpdateItem {
    FileSize {
        hash_kind: String,
        hash: Vec<u8>,

        file_size: FileSize,
    },
}

/// A job that changed status.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum JobStatusItem {
    Transcoding,
    Ready { file_size: u64 },
    Failed { error: String },
}

/// A message sent by the server end of a connection on the control stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ServerMessage {
    /// Identify the server with a friendly name.
    Identify(String),
    /// Notify the client that the connection has been accepted.
    Accepted,
    /// Inform the client of available files.
    Index(Vec<IndexItem>),
    /// Inform the client of updates to the index.
    IndexUpdate(Vec<IndexUpdateItem>),
    /// Notify the client that the statuses of jobs have changed.
    JobStatus(HashMap<u64, JobStatusItem>),
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
    file_node_id: NodeId,
    file_root: String,
    file_path: String,
}

#[derive(Debug)]
enum ServerTransferJobProgress {
    /// The server is waiting for the file to be transcoded.
    Transcoding { hash_kind: String, hash: Vec<u8> },
    /// The server is ready to send the file.
    Ready { local_path: PathBuf, file_size: u64 },
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
    ServerMessage(ServerMessage),
}

#[derive(Debug, Clone)]
struct ServerHandle {
    tx: mpsc::UnboundedSender<ServerCommand>,

    jobs: Arc<DashMap<u64, ServerTransferJob>>,
}

struct Server {
    db: Arc<Mutex<Database>>,
    transcode_status_cache: TranscodeStatusCache,

    connection: Connection,
    event_tx: mpsc::UnboundedSender<NodeEvent>,

    connected_at: u64,

    jobs: Arc<DashMap<u64, ServerTransferJob>>,
}

impl Server {
    fn new(
        db: Arc<Mutex<Database>>,
        transcode_status_cache: TranscodeStatusCache,

        connection: Connection,
        event_tx: mpsc::UnboundedSender<NodeEvent>,
    ) -> Self {
        Self {
            db,
            transcode_status_cache,

            connection,
            event_tx,

            connected_at: unix_epoch_now_secs(),

            jobs: Arc::new(DashMap::new()),
        }
    }

    async fn run(self) -> anyhow::Result<()> {
        let remote_node_id = self.connection.remote_node_id()?;

        // spawn remote info polling task
        let cancel_token = CancellationToken::new();
        let _cancel_guard = cancel_token.clone().drop_guard();
        tokio::spawn({
            let event_tx = self.event_tx.clone();
            async move {
                loop {
                    event_tx
                        .send(NodeEvent::ServerChanged {
                            node_id: remote_node_id,
                            update: ServerModelUpdate::PollRemoteInfo,
                        })
                        .expect("failed to send ServerModelUpdate::PollRemoteInfo");

                    tokio::time::sleep(Duration::from_secs(1)).await;

                    if cancel_token.is_cancelled() {
                        log::debug!(
                            "remote info polling task cancelled for server {remote_node_id}"
                        );
                        break;
                    }
                }
            }
        });

        let (tx, mut rx) = mpsc::unbounded_channel();

        // accept bidirectional control stream
        let (send, recv) = self.connection.accept_bi().await?;

        // wrap in framed codecs
        let mut send = FramedWrite::new(send, LengthDelimitedCodec::new()).with_flat_map(
            |message: ServerMessage| {
                let buf: Vec<u8> =
                    postcard::to_stdvec(&message).expect("failed to serialize message");
                futures::stream::once(futures::future::ready(Ok(Bytes::from(buf))))
            },
        );
        let mut recv = FramedRead::new(recv, LengthDelimitedCodec::new())
            .map_err(|e| anyhow::anyhow!("failed to read from connection: {e:?}"))
            .map(|res| {
                res.and_then(|bytes| {
                    postcard::from_bytes::<ClientMessage>(&bytes)
                        .map_err(|e| anyhow::anyhow!("failed to deserialize message: {e:?}"))
                })
            });

        // wait for client Identify
        let Some(Ok(message)) = recv.next().await else {
            log::error!("failed to receive Identify message");
            return Ok(());
        };
        match message {
            ClientMessage::Identify(name) => {
                // TODO: store
                log::debug!("client identified as {name}");
            }
            _ => {
                log::error!("unexpected message, expected Identify: {message:?}");
                return Ok(());
            }
        }

        // send server Identify
        // TODO: real name
        send.send(ServerMessage::Identify("server".to_string()))
            .await
            .expect("failed to send Identify message");

        // handshake finished, send handle to Node
        let handle = ServerHandle {
            tx: tx.clone(),

            jobs: self.jobs.clone(),
        };
        self.event_tx
            .send(NodeEvent::ServerOpened {
                node_id: remote_node_id,
                handle,

                name: "unknown".to_string(), // TODO: real name
                connected_at: self.connected_at,
            })
            .expect("failed to send NodeEvent::ServerOpened");

        // check if remote node is trusted
        let is_trusted = {
            let db = self.db.lock().unwrap();
            db.is_node_trusted(remote_node_id)?
        };

        if is_trusted {
            log::info!("accepting connection from trusted node {remote_node_id}");
        } else {
            // waiting loop, wait for user to accept or deny the connection
            log::info!(
                "waiting for accept or deny of connection from untrusted node {remote_node_id}",
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
                                    .expect("failed to send ServerMessage");
                            }
                        }
                    }

                    next_message = recv.next() => {
                        match next_message {
                            Some(Ok(message)) => {
                                log::debug!("unexpected message (not accepted): {message:?}");
                            },
                            Some(Err(e)) => {
                                log::error!("error receiving message: {e}");
                            },
                            None => {
                                log::info!("control stream closed, shutting down server");
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
                node_id: remote_node_id,
                update: ServerModelUpdate::Accept,
            })
            .expect("failed to send ServerModelUpdate::Accept");

        // send Accepted message
        send.send(ServerMessage::Accepted)
            .await
            .expect("failed to send Accepted message");

        // send Index message
        let mut index = self.get_index()?;
        send.send(ServerMessage::Index(index.clone()))
            .await
            .expect("failed to send Index message");

        // spawn task to watch for finished transcodes
        // TODO: shutdown signal
        // TODO: could maybe be a timer instead of a task with a sleep loop
        tokio::spawn({
            let jobs = self.jobs.clone();
            let transcode_status_cache = self.transcode_status_cache.clone();
            let event_tx = self.event_tx.clone();
            async move {
                loop {
                    let mut ready_jobs = Vec::new();
                    let mut failed_jobs = Vec::new();

                    // check for jobs with Transcoding status
                    for job in jobs.iter() {
                        if let ServerTransferJobProgress::Transcoding { hash_kind, hash } =
                            &job.value().progress
                        {
                            // get transcode status
                            let Some(status) = transcode_status_cache.get(hash_kind, hash) else {
                                continue;
                            };

                            match &*status {
                                TranscodeStatus::Waiting { .. } => {
                                    // job is still queued
                                }

                                // if transcode status is Ready, set job status to Ready
                                TranscodeStatus::Ready {
                                    local_path,
                                    file_size,
                                } => {
                                    ready_jobs.push((*job.key(), local_path.clone(), *file_size));
                                }

                                // if transcode status is Failed, set job status to Failed
                                TranscodeStatus::Failed { error } => {
                                    log::error!(
                                        "transcoding failed for job {}: {}",
                                        job.key(),
                                        error
                                    );

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
                            .map(|(job_id, local_path, file_size)| {
                                // set job status to Ready
                                // needs to happen outside the loop, since jobs.iter() already holds the entry's lock
                                jobs.alter(&job_id, |_, mut job| {
                                    job.progress = ServerTransferJobProgress::Ready {
                                        local_path,
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
                            ServerMessage::JobStatus(status_changes),
                        )) {
                            log::warn!("transcode watcher failed to send JobStatus message: {e}");
                        }

                        // update model
                        event_tx
                            .send(NodeEvent::ServerChanged {
                                node_id: remote_node_id,
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

        // main loop
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ServerCommand::Accept => {
                            log::warn!("unexpected Accept command in main loop");
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
                                ClientMessage::Identify(_) => {
                                    log::warn!("unexpected ClientMessage::Identify in main loop");
                                }

                                ClientMessage::Download(items) => {
                                    // get file hashes
                                    // TODO: this could be better
                                    let files = {
                                        let db = self.db.lock().expect("failed to lock database");
                                        db.get_files_by_node_root_path(
                                            items.iter().map(|item| (item.node_id, item.root.clone(), item.path.clone()))
                                        )?.into_iter().map(|f| ((f.node_id, f.root.clone(), f.path.clone()), f)).collect::<HashMap<_, _>>()
                                    };

                                    let status_changes = items.into_iter().map(|item| {
                                        // TODO: wasteful clones
                                        let file = files.get(&(item.node_id, item.root.clone(), item.path.clone()));

                                        let Some(file) = file else {
                                            self.jobs.insert(item.job_id, ServerTransferJob {
                                                progress: ServerTransferJobProgress::Failed { error: anyhow::anyhow!("file not found") },
                                                file_node_id: item.node_id,
                                                file_root: item.root,
                                                file_path: item.path,
                                            });

                                            return (item.job_id, JobStatusItem::Failed {
                                                error: "file not found".to_string(),
                                            });
                                        };

                                        // get transcode status
                                        let transcode_status = self.transcode_status_cache.get(&file.hash_kind, &file.hash);

                                        let Some(transcode_status) = transcode_status else {
                                            // file exists, but transcode status is missing
                                            log::warn!("file {}/{}/{} exists but transcode status is missing, defaulting to Queued",
                                                item.node_id, item.root, item.path);

                                            // create job
                                            self.jobs.insert(item.job_id, ServerTransferJob {
                                                progress: ServerTransferJobProgress::Transcoding { hash_kind: file.hash_kind.clone(), hash: file.hash.clone() },
                                                file_node_id: item.node_id,
                                                file_root: item.root,
                                                file_path: item.path,
                                            });

                                            return (item.job_id, JobStatusItem::Transcoding);
                                        };

                                        match &*transcode_status {
                                            TranscodeStatus::Waiting { .. } => {
                                                // file is queued for transcoding

                                                // create job
                                                self.jobs.insert(item.job_id, ServerTransferJob {
                                                    progress: ServerTransferJobProgress::Transcoding { hash_kind: file.hash_kind.clone(), hash: file.hash.clone() },
                                                    file_node_id: item.node_id,
                                                    file_root: item.root,
                                                    file_path: item.path,
                                                });

                                                (item.job_id, JobStatusItem::Transcoding)
                                            }

                                            TranscodeStatus::Ready { local_path, file_size } => {
                                                // file is already transcoded

                                                // create job
                                                self.jobs.insert(item.job_id, ServerTransferJob {
                                                    progress: ServerTransferJobProgress::Ready {
                                                        local_path: local_path.clone(),
                                                        file_size: *file_size,
                                                    },
                                                    file_node_id: item.node_id,
                                                    file_root: item.root,
                                                    file_path: item.path,
                                                });

                                                (item.job_id, JobStatusItem::Ready {
                                                    file_size: *file_size,
                                                })
                                            }

                                            TranscodeStatus::Failed { error } => {
                                                // transcoding failed

                                                // create job
                                                self.jobs.insert(item.job_id, ServerTransferJob {
                                                    progress: ServerTransferJobProgress::Failed {
                                                        error: anyhow::anyhow!("transcoding failed: {error}"),
                                                    },
                                                    file_node_id: item.node_id,
                                                    file_root: item.root,
                                                    file_path: item.path,
                                                });

                                                (item.job_id, JobStatusItem::Failed {
                                                    error: format!("{error:#}"),
                                                })
                                            }
                                        }
                                    }).collect::<HashMap<_, _>>();

                                    // send job status to client
                                    send.send(ServerMessage::JobStatus(status_changes))
                                        .await
                                        .expect("failed to send JobStatus message");

                                    // update model
                                    self.event_tx.send(NodeEvent::ServerChanged {
                                        node_id: remote_node_id,
                                        update: ServerModelUpdate::UpdateTransferJobs,
                                    }).expect("failed to send ServerModelUpdate::UpdateTransferJobs");

                                    // prioritize transcodes
                                    let hashes = files.values().map(|f| (f.hash_kind.clone(), f.hash.clone())).collect::<Vec<_>>();
                                    self.event_tx.send(NodeEvent::FilesRequested(hashes)).expect("failed to send NodeEvent::FilesRequested");
                                }
                            }
                        },
                        Some(Err(e)) => {
                            log::error!("error receiving message: {e}");
                        },
                        None => {
                            log::info!("control stream closed, shutting down server");
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
                                        ServerTransferJobProgress::Ready { local_path, file_size } => {
                                            (TransferResponse::Ok { file_size: *file_size }, Some((local_path.clone(), *file_size)))
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
                                let Some((local_path, file_size)) = ready else {
                                    return Ok(());
                                };

                                // check local file exists
                                if !local_path.exists() {
                                    // TODO: set job to failed and respond with error
                                    anyhow::bail!("file at local_path does not exist: {}", local_path.display());
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
                                    node_id: remote_node_id,
                                    update: ServerModelUpdate::UpdateTransferJobs,
                                }).expect("failed to send ServerModelUpdate::UpdateTransferJobs");

                                // read file to buffer
                                // TODO: stream instead of reading into memory?
                                let file_content = tokio::fs::read(local_path).await?;

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
                                    node_id: remote_node_id,
                                    update: ServerModelUpdate::UpdateTransferJobs,
                                }).expect("failed to send ServerModelUpdate::UpdateTransferJobs");

                                Ok::<(), anyhow::Error>(())
                            });
                        }

                        Err(e) => {
                            log::error!("accept_bi error: {e}");
                        }
                    }
                }

                // periodically check for index updates
                _ = index_update_interval.tick() => {
                    let mut updates = Vec::new();

                    for item in index.iter_mut() {
                        // if the client doesn't have the actual file size
                        if !matches!(item.file_size, FileSize::Actual(_)) {
                            // try to get file size from transcode status cache
                            let file_size = self
                                .transcode_status_cache
                                .get(&item.hash_kind, &item.hash)
                                .and_then(|entry| match &*entry {
                                    TranscodeStatus::Waiting { estimated_size } => estimated_size.map(FileSize::Estimated),
                                    TranscodeStatus::Ready { file_size, .. } => Some(FileSize::Actual(*file_size)),
                                    _ => None,
                                });

                            // if we now have an updated file size, update the client
                            if let Some(file_size) = file_size && file_size != item.file_size {
                                // store client's view so we don't send the same update again
                                item.file_size = file_size;

                                updates.push(IndexUpdateItem::FileSize {
                                    hash_kind: item.hash_kind.clone(),
                                    hash: item.hash.clone(),

                                    file_size,
                                });
                            }
                        }
                    }

                    if !updates.is_empty() {
                        send.send(ServerMessage::IndexUpdate(updates))
                            .await
                            .expect("failed to send IndexUpdate message");
                    }
                }

                else => {
                    log::warn!("all senders dropped in Server::run, shutting down");
                    break;
                }
            }
        }

        self.connection.closed().await;

        Ok(())
    }

    fn get_index(&self) -> anyhow::Result<Vec<IndexItem>> {
        let files = {
            let db = self.db.lock().unwrap();
            db.get_files()?
        };

        let index = files
            .into_iter()
            .map(|file| {
                // try to get file size from transcode status cache
                let file_size = self
                    .transcode_status_cache
                    .get(&file.hash_kind, &file.hash)
                    .and_then(|entry| match &*entry {
                        TranscodeStatus::Waiting { estimated_size } => {
                            estimated_size.map(FileSize::Estimated)
                        }
                        TranscodeStatus::Ready { file_size, .. } => {
                            Some(FileSize::Actual(*file_size))
                        }
                        _ => None,
                    })
                    .unwrap_or(FileSize::Unknown);

                IndexItem {
                    node_id: file.node_id,
                    root: file.root,
                    path: file.path,

                    hash_kind: file.hash_kind,
                    hash: file.hash,

                    file_size,
                }
            })
            .collect::<Vec<_>>();

        Ok(index)
    }
}

#[derive(Debug)]
struct ClientTransferJob {
    progress: ClientTransferJobProgress,

    file_hash_kind: String,
    file_hash: Vec<u8>,
    file_node_id: NodeId,
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

    DownloadAll,
    DownloadPartial {
        items: Vec<DownloadPartialItemModel>,
    },
}

#[derive(Debug, Clone)]
struct ClientHandle {
    tx: mpsc::UnboundedSender<ClientCommand>,

    index: Arc<Mutex<Option<Vec<IndexItem>>>>,
    jobs: Arc<DashMap<u64, ClientTransferJob>>,
}

struct Client {
    db: Arc<Mutex<Database>>,
    download_directory: Arc<Mutex<Option<String>>>,

    event_tx: mpsc::UnboundedSender<NodeEvent>,
    connection: Connection,

    connected_at: u64,

    next_job_id: Arc<AtomicU64>,
    ready_tx: mpsc::UnboundedSender<u64>,

    index: Arc<Mutex<Option<Vec<IndexItem>>>>,
    jobs: Arc<DashMap<u64, ClientTransferJob>>,
}

impl Client {
    fn new(
        db: Arc<Mutex<Database>>,
        event_tx: mpsc::UnboundedSender<NodeEvent>,
        connection: Connection,
        download_directory: Arc<Mutex<Option<String>>>,
    ) -> Self {
        let jobs = Arc::new(DashMap::<u64, ClientTransferJob>::new());

        // spawn a task to handle ready jobs and spawn more tasks to download them
        // Client::run() receives ServerMessage::JobStatus messages. jobs marked Ready are sent to this channel
        let (ready_tx, mut ready_rx) = mpsc::unbounded_channel::<u64>();
        tokio::spawn({
            let db = db.clone();
            let jobs = jobs.clone();
            let event_tx = event_tx.clone();
            let connection = connection.clone();
            let download_directory = download_directory.clone();
            async move {
                // convert channel receiver of ready job IDs into a stream for use with buffer_unordered
                let ready_stream = async_stream::stream! {
                    while let Some(job_id) = ready_rx.recv().await {
                        yield job_id;
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
                        async move {
                            let remote_node_id = connection.remote_node_id()?;

                            // check if download directory is set
                            // we need to do this inside the async block so that the return type of the closure is always the async block's anonymous future
                            let Some(download_directory) = download_directory else {
                                anyhow::bail!("download directory is None, cannot download");
                            };

                            // check job exists and get details
                            let (file_hash_kind, file_hash, file_node_id, file_root, file_path) = {
                                let Some(job) = jobs.get(&job_id) else {
                                    anyhow::bail!("received ready for unknown job ID {job_id}");
                                };

                                (
                                    job.file_hash_kind.clone(),
                                    job.file_hash.clone(),
                                    job.file_node_id,
                                    job.file_root.clone(),
                                    job.file_path.clone(),
                                )
                            };

                            log::debug!("downloading file: {file_root}/{file_path}");

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
                                    node_id: remote_node_id,
                                    update: ClientModelUpdate::UpdateTransferJobs,
                                })
                                .expect("failed to send ClientModelUpdate::UpdateTransferJobs");

                            // build file path
                            let local_path = {
                                let root_dir_name =
                                    format!("musicopy-{}-{}", &file_node_id, &file_root);
                                let mut local_path =
                                    TreePath::new(download_directory, root_dir_name.into())?;
                                local_path.push(&file_path);
                                local_path.set_extension("ogg");
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
                                    remote_node_id,
                                    InsertFile {
                                        hash_kind: &file_hash_kind,
                                        hash: &file_hash,
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
                                    node_id: remote_node_id,
                                    update: ClientModelUpdate::UpdateTransferJobs,
                                })
                                .expect("failed to send ClientModelUpdate::UpdateTransferJobs");

                            log::debug!("saved file to {local_path:?}");

                            Ok::<(), anyhow::Error>(())
                        }
                    })
                    .buffer_unordered(4);

                tokio::pin!(buffer);

                // poll the stream to download items with limited concurrency
                while let Some(res) = buffer.next().await {
                    if let Err(e) = res {
                        log::error!("error downloading item: {e:#}");
                    }
                }
            }
        });

        Self {
            db,
            download_directory,

            event_tx,
            connection,

            connected_at: unix_epoch_now_secs(),

            next_job_id: Arc::new(AtomicU64::new(0)),
            ready_tx,

            index: Arc::new(Mutex::new(None)),
            jobs,
        }
    }

    async fn run(self) -> anyhow::Result<()> {
        let remote_node_id = self.connection.remote_node_id()?;

        // spawn remote info polling task
        let cancel_token = CancellationToken::new();
        let _cancel_guard = cancel_token.clone().drop_guard();
        tokio::spawn({
            let event_tx = self.event_tx.clone();
            async move {
                loop {
                    event_tx
                        .send(NodeEvent::ClientChanged {
                            node_id: remote_node_id,
                            update: ClientModelUpdate::PollRemoteInfo,
                        })
                        .expect("failed to send ClientModelUpdate::PollRemoteInfo");

                    tokio::time::sleep(Duration::from_secs(1)).await;

                    if cancel_token.is_cancelled() {
                        log::debug!(
                            "remote info polling task cancelled for client {remote_node_id}"
                        );
                        break;
                    }
                }
            }
        });

        let (tx, mut rx) = mpsc::unbounded_channel();

        // open a bidirectional QUIC stream
        let (send, recv) = self.connection.open_bi().await?;

        // wrap in framed codecs
        let mut send = FramedWrite::new(send, LengthDelimitedCodec::new()).with_flat_map(
            |message: ClientMessage| {
                let buf: Vec<u8> =
                    postcard::to_stdvec(&message).expect("failed to serialize message");
                futures::stream::once(futures::future::ready(Ok(Bytes::from(buf))))
            },
        );
        let mut recv = FramedRead::new(recv, LengthDelimitedCodec::new())
            .map_err(|e| anyhow::anyhow!("failed to read from connection: {e:?}"))
            .map(|res| {
                res.and_then(|bytes| {
                    postcard::from_bytes::<ServerMessage>(&bytes)
                        .map_err(|e| anyhow::anyhow!("failed to deserialize message: {e:?}"))
                })
            });

        // send client Identify
        // TODO: real name
        send.send(ClientMessage::Identify("client".to_string()))
            .await
            .expect("failed to send Identify message");

        // wait for server Identify
        // TODO: also wait for commands
        let Some(Ok(message)) = recv.next().await else {
            log::error!("failed to receive Identify message");
            return Ok(());
        };
        match message {
            ServerMessage::Identify(name) => {
                // TODO: store
                log::info!("server identified as {name}");
            }
            _ => {
                log::error!("unexpected message, expected Identify: {message:?}");
                return Ok(());
            }
        }

        // handshake finished, send handle to Node
        let handle = ClientHandle {
            tx,

            index: self.index.clone(),
            jobs: self.jobs.clone(),
        };
        self.event_tx
            .send(NodeEvent::ClientOpened {
                node_id: remote_node_id,
                handle,

                name: "unknown".to_string(), // TODO: real name
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

                        ClientCommand::DownloadAll => {
                            log::warn!("unexpected DownloadAll command in waiting loop");
                        }
                        ClientCommand::DownloadPartial { .. } => {
                            log::warn!("unexpected DownloadPartial command in waiting loop");
                        }
                    }
                }

                next_message = recv.next() => {
                    match next_message {
                        Some(Ok(message)) => {
                            match message {
                                ServerMessage::Accepted => {
                                    log::info!("server accepted the connection");

                                    // continue to next state
                                    break;
                                }
                                _ => {
                                    log::debug!("unexpected message (waiting for Accepted): {message:?}");
                                }
                            }
                        }
                        Some(Err(e)) => {
                            log::error!("error receiving message: {e}");
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
                node_id: remote_node_id,
                update: ClientModelUpdate::Accept,
            })
            .expect("failed to send ClientModelUpdate::Accept");

        // update recent servers in database
        {
            let db = self.db.lock().unwrap();
            db.update_recent_server(remote_node_id, self.connected_at)
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

                        ClientCommand::DownloadAll => {
                            // get index
                            let index = {
                                let index = self.index.lock().unwrap();
                                index.clone()
                            };
                            let Some(index) = index else {
                                log::error!("DownloadAll: no index available, cannot download");
                                continue;
                            };

                            // get download directory
                            let download_directory = {
                                let download_directory = self.download_directory.lock().unwrap();
                                download_directory.clone()
                            };

                            // create jobs and download request items
                            let download_requests = {
                                let db = self.db.lock().unwrap();
                                index.into_iter().flat_map(|file| {
                                    // check if file is downloaded to the current download directory
                                    let downloaded = download_directory.as_ref().is_some_and(|download_directory| {
                                        db.exists_file_by_node_root_path_localtree(
                                            file.node_id, &file.root, &file.path, download_directory,
                                        )
                                        .unwrap_or(false)
                                    });
                                    if downloaded {
                                        return None;
                                    }

                                    let job_id = self.next_job_id.fetch_add(1, Ordering::Relaxed);

                                    self.jobs.insert(job_id, ClientTransferJob {
                                        progress: ClientTransferJobProgress::Requested,
                                        file_hash_kind: file.hash_kind.clone(),
                                        file_hash: file.hash.clone(),
                                        file_node_id: file.node_id,
                                        file_root: file.root.clone(),
                                        file_path: file.path.clone(),
                                    });

                                    Some(DownloadItem {
                                        job_id,

                                        node_id: file.node_id,
                                        root: file.root,
                                        path: file.path,
                                    })
                                }).collect::<Vec<_>>()
                            };

                            // send download request
                            send.send(ClientMessage::Download(download_requests))
                                .await
                                .expect("failed to send Download message");

                            // update model
                            self.event_tx.send(NodeEvent::ClientChanged {
                                node_id: remote_node_id,
                                update: ClientModelUpdate::UpdateTransferJobs,
                            }).expect("failed to send ClientModelUpdate::UpdateTransferJobs");
                        }
                        ClientCommand::DownloadPartial { items } => {
                            // get index
                            let index = {
                                let index = self.index.lock().unwrap();
                                index.clone()
                            };
                            let Some(index) = index else {
                                log::error!("DownloadPartial: no index available, cannot download");
                                continue;
                            };

                            // create jobs and download request items
                            let download_requests = items.into_iter().flat_map(|item| {
                                let Ok(file_node_id) = item.node_id.parse() else {
                                    log::warn!("DownloadPartial: invalid node ID");
                                    return None;
                                };

                                // find item in index
                                let Some(index_item) = index.iter().find(|i| i.node_id == file_node_id && i.root == item.root && i.path == item.path) else {
                                    log::warn!("DownloadPartial: item not found in index: {item:?}");
                                    return None;
                                };

                                let job_id = self.next_job_id.fetch_add(1, Ordering::Relaxed);

                                self.jobs.insert(job_id, ClientTransferJob {
                                    progress: ClientTransferJobProgress::Requested,
                                    file_hash_kind: index_item.hash_kind.clone(),
                                    file_hash: index_item.hash.clone(),
                                    file_node_id,
                                    file_root: item.root.clone(),
                                    file_path: item.path.clone(),
                                });

                                Some(DownloadItem {
                                    job_id,

                                    node_id: file_node_id,
                                    root: item.root,
                                    path: item.path,
                                })
                            }).collect::<Vec<_>>();

                            // send download request
                            send.send(ClientMessage::Download(download_requests))
                                .await
                                .expect("failed to send Download message");

                            // update model
                            self.event_tx.send(NodeEvent::ClientChanged {
                                node_id: remote_node_id,
                                update: ClientModelUpdate::UpdateTransferJobs,
                            }).expect("failed to send ClientModelUpdate::UpdateTransferJobs");
                        }
                    }
                }

                next_message = recv.next() => {
                    match next_message {
                        Some(Ok(message)) => {
                            match message {
                                ServerMessage::Index(new_index) => {
                                    log::info!("received index with {} items", new_index.len());
                                    {
                                        let mut index = self.index.lock().unwrap();
                                        *index = Some(new_index);
                                    }

                                    // update model
                                    self.event_tx.send(NodeEvent::ClientChanged {
                                        node_id: remote_node_id,
                                        update: ClientModelUpdate::UpdateIndex,
                                    }).expect("failed to send ClientModelUpdate::UpdateIndex");
                                }

                                ServerMessage::IndexUpdate(updates) => {
                                    log::info!("received index update with {} items", updates.len());
                                    {
                                        let mut index = self.index.lock().unwrap();
                                        if let Some(index) = index.as_mut() {
                                            for update in updates {
                                                match update {
                                                    IndexUpdateItem::FileSize { hash_kind, hash, file_size } => {
                                                        // TODO: don't be exponential
                                                        for item in index.iter_mut() {
                                                            if item.hash_kind == hash_kind && item.hash == hash {
                                                                item.file_size = file_size;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            log::warn!("received index update but index is None, ignoring");
                                        }
                                    }

                                    // update model
                                    self.event_tx.send(NodeEvent::ClientChanged {
                                        node_id: remote_node_id,
                                        update: ClientModelUpdate::UpdateIndex,
                                    }).expect("failed to send ClientModelUpdate::UpdateIndex");
                                }

                                ServerMessage::JobStatus(status_changes) => {
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
                                        node_id: remote_node_id,
                                        update: ClientModelUpdate::UpdateTransferJobs,
                                    }).expect("failed to send ClientModelUpdate::UpdateTransferJobs");
                                }

                                _ => {
                                    log::debug!("unexpected message in main loop: {message:?}");
                                }
                            }
                        }
                        Some(Err(e)) => {
                            log::error!("error receiving message: {e}");
                        }
                        None => {
                            log::info!("control stream closed, shutting down client");
                            break;
                        }
                    }
                }

                _ = self.connection.closed() => {
                    log::info!("connection closed");
                    break;
                }

                else => {
                    log::warn!("all senders dropped in Client::run, shutting down");
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
