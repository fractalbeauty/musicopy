use std::{borrow::Cow, sync::Arc};

use iroh::NodeId;
use musicopy::{
    Core, CoreOptions, EventHandler, ProjectDirsOptions,
    library::{LibraryModel, transcode::TranscodePolicy},
    node::{ClientModel, ClientStateModel, NodeModel, ServerModel, ServerStateModel},
};

pub async fn wait_until(msg: &str, condition: impl Fn() -> bool) {
    let start = std::time::Instant::now();
    while !condition() {
        if start.elapsed().as_secs_f64() > 5.0 {
            panic!("timed out waiting for condition: {msg}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

pub struct TestEventHandler;

impl EventHandler for TestEventHandler {
    fn on_library_model_snapshot(&self, _model: LibraryModel) {}

    fn on_node_model_snapshot(&self, _model: NodeModel) {}
}

#[derive(Clone)]
pub struct TestCore {
    pub label: String,
    pub event_handler: Arc<TestEventHandler>,
    pub core: Arc<Core>,
}

impl TestCore {
    pub async fn start(label: &str) -> Self {
        let event_handler = Arc::new(TestEventHandler);

        let test_dir = testdir::testdir!();
        let instance_dir = test_dir.join(label);
        let data_dir = instance_dir.join("data");
        let cache_dir = instance_dir.join("cache");
        let project_dirs = ProjectDirsOptions {
            data_dir: data_dir.to_string_lossy().to_string(),
            cache_dir: cache_dir.to_string_lossy().to_string(),
        };

        let options = CoreOptions {
            init_logging: false,
            in_memory: false,
            project_dirs: Some(project_dirs),
            transcode_policy: TranscodePolicy::IfRequested,
        };

        let core = Core::start(event_handler.clone(), options)
            .await
            .expect("should start core");

        Self {
            label: label.to_string(),
            event_handler,
            core,
        }
    }

    /// Wait until we have a home relay, so discovery and connections should work
    pub async fn wait_for_relay(&self) {
        wait_until(&format!("{} has relay", self.label), || {
            let model = self.core.get_node_model().expect("should get node model");
            model.home_relay != "none"
        })
        .await;
    }

    /// Wait until we have a client with the given node id
    pub async fn wait_for_client(&self, other: impl TestNodeIdExt) {
        wait_until(
            &format!("{} has client for {}", self.label, other.label()),
            || {
                let model = self.core.get_node_model().expect("should get node model");
                model.clients.contains_key(&other.node_id_str())
            },
        )
        .await;
    }

    /// Wait until we have a client with the given node id, satisfying the given condition
    pub async fn wait_for_client_condition(
        &self,
        msg: &str,
        other: impl TestNodeIdExt,
        condition: impl Fn(&ClientModel) -> bool,
    ) {
        let full_msg = format!(
            "{} has client for {}, where {}",
            self.label,
            other.label(),
            msg
        );

        // wait for client first for clearer error messages
        self.wait_for_client(other.clone()).await;

        // wait for client with condition
        wait_until(&full_msg, || {
            let model = self.core.get_node_model().expect("should get node model");
            if let Some(client) = model.clients.get(&other.node_id_str()) {
                condition(client)
            } else {
                eprintln!(
                    "wait_for_client_condition: {full_msg}: client for {} missing?",
                    other.label()
                );
                false
            }
        })
        .await;
    }

    /// Wait until we have a client with the given node id, with state Pending
    pub async fn wait_for_client_pending(&self, other: impl TestNodeIdExt) {
        self.wait_for_client_condition("state is Pending", other, |client| {
            matches!(client.state, ClientStateModel::Pending)
        })
        .await;
    }

    /// Wait until we have a client with the given node id, with state Accepted
    pub async fn wait_for_client_accepted(&self, other: impl TestNodeIdExt) {
        self.wait_for_client_condition("state is Accepted", other, |client| {
            matches!(client.state, ClientStateModel::Accepted)
        })
        .await;
    }

    /// Wait until we have a client with the given node id, with state Closed
    pub async fn wait_for_client_closed(&self, other: impl TestNodeIdExt) {
        self.wait_for_client_condition("state is Closed", other, |client| {
            matches!(client.state, ClientStateModel::Closed { .. })
        })
        .await;
    }

    /// Wait until we have a server with the given node id
    pub async fn wait_for_server(&self, other: impl TestNodeIdExt) {
        wait_until(
            &format!("{} has server for {}", self.label, other.label()),
            || {
                let model = self.core.get_node_model().expect("should get node model");
                model.servers.contains_key(&other.node_id_str())
            },
        )
        .await;
    }

    /// Wait until we have a server with the given node id, satisfying the given condition
    pub async fn wait_for_server_condition(
        &self,
        msg: &str,
        other: impl TestNodeIdExt,
        condition: impl Fn(&ServerModel) -> bool,
    ) {
        let full_msg = format!(
            "{} has server for {}, where {}",
            self.label,
            other.label(),
            msg
        );

        // wait for server first for clearer error messages
        self.wait_for_server(other.clone()).await;

        // wait for server with condition
        wait_until(&full_msg, || {
            let model = self.core.get_node_model().expect("should get node model");
            if let Some(server) = model.servers.get(&other.node_id_str()) {
                condition(server)
            } else {
                eprintln!(
                    "wait_for_server_condition: {full_msg}: server for {} missing?",
                    other.label()
                );
                false
            }
        })
        .await;
    }

    /// Wait until we have a server with the given node id, with state Pending
    pub async fn wait_for_server_pending(&self, other: impl TestNodeIdExt) {
        self.wait_for_server_condition("state is Pending", other, |server| {
            matches!(server.state, ServerStateModel::Pending)
        })
        .await;
    }

    /// Wait until we have a server with the given node id, with state Accepted
    pub async fn wait_for_server_accepted(&self, other: impl TestNodeIdExt) {
        self.wait_for_server_condition("state is Accepted", other, |server| {
            matches!(server.state, ServerStateModel::Accepted)
        })
        .await;
    }

    /// Wait until we have a server with the given node id, with state Closed
    pub async fn wait_for_server_closed(&self, other: impl TestNodeIdExt) {
        self.wait_for_server_condition("state is Closed", other, |server| {
            matches!(server.state, ServerStateModel::Closed { .. })
        })
        .await;
    }

    pub fn node_id(&self) -> NodeId {
        let model = self.core.get_node_model().expect("should get node model");
        let bytes = hex::decode(&model.node_id).expect("should decode node id hex");
        NodeId::from_bytes(&bytes.try_into().expect("should have correct length"))
            .expect("should parse node id")
    }
}

/// Helper trait to pass a TestCore or NodeId and get a label and NodeId
pub trait TestNodeIdExt: Clone {
    fn label(&self) -> Cow<'_, str>;
    fn node_id(&self) -> NodeId;

    fn node_id_str(&self) -> String {
        self.node_id().to_string()
    }
}

impl TestNodeIdExt for TestCore {
    fn label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.label)
    }

    fn node_id(&self) -> NodeId {
        TestCore::node_id(self)
    }
}

impl TestNodeIdExt for &TestCore {
    fn label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.label)
    }

    fn node_id(&self) -> NodeId {
        TestCore::node_id(self)
    }
}

impl TestNodeIdExt for NodeId {
    fn label(&self) -> Cow<'_, str> {
        Cow::Owned(self.to_string())
    }

    fn node_id(&self) -> NodeId {
        *self
    }
}
