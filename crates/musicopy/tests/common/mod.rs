use iroh::EndpointId;
use musicopy::{
    Core, CoreOptions, EventHandler, ProjectDirsOptions, StatsModel, TestHooks,
    library::LibraryModel,
    node::{ClientModel, ClientStateModel, NodeModel, ServerModel, ServerStateModel},
};
use std::{borrow::Cow, path::PathBuf, sync::Arc};
use tracing::{debug, warn};

#[derive(Debug, Clone, Copy)]
pub enum LibraryFixture {
    Minimal,
    Multiple,
}

impl LibraryFixture {
    pub fn num_items(&self) -> usize {
        match self {
            Self::Minimal => 1,
            Self::Multiple => 2,
        }
    }

    pub fn path(&self) -> PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");

        let mut p = PathBuf::from(manifest_dir);
        p.push("tests/fixtures");
        match self {
            Self::Minimal => p.push("minimal"),
            Self::Multiple => p.push("multiple"),
        }

        assert!(p.exists(), "fixture path does not exist: {p:?}");

        p
    }
}

async fn wait_until(msg: &str, condition: impl Fn() -> bool, on_fail: impl Fn()) {
    debug!("wait_until: waiting for condition: {}", msg);
    let start = std::time::Instant::now();
    while !condition() {
        if start.elapsed().as_secs_f64() > 10.0 {
            on_fail();
            panic!("timed out waiting for condition: {msg}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

pub struct TestEventHandler;

impl EventHandler for TestEventHandler {
    fn on_library_model_snapshot(&self, _model: LibraryModel) {}

    fn on_node_model_snapshot(&self, _model: NodeModel) {}

    fn on_stats_model_snapshot(&self, _model: StatsModel) {}
}

#[derive(Clone)]
pub struct TestCore {
    pub label: String,

    pub instance_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub download_dir: PathBuf,

    pub event_handler: Arc<TestEventHandler>,
    pub core: Arc<Core>,
    #[cfg(feature = "test-hooks")]
    pub test_hooks: Arc<TestHooks>,
}

impl TestCore {
    pub async fn start(label: &str) -> Self {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,musicopy=debug")),
            )
            .try_init();

        let event_handler = Arc::new(TestEventHandler);

        let test_dir = testdir::testdir!();
        let instance_dir = test_dir.join(label);
        let data_dir = instance_dir.join("data");
        let cache_dir = instance_dir.join("cache");
        let download_dir = instance_dir.join("downloads");

        let project_dirs = ProjectDirsOptions {
            data_dir: data_dir.to_string_lossy().to_string(),
            cache_dir: cache_dir.to_string_lossy().to_string(),
        };

        let options = CoreOptions {
            init_logging: false,
            in_memory: false,
            project_dirs: Some(project_dirs),
        };

        #[cfg(feature = "test-hooks")]
        let test_hooks = Arc::new(TestHooks::default());

        let core = Core::start_inner(
            event_handler.clone(),
            options,
            #[cfg(feature = "test-hooks")]
            test_hooks.clone(),
        )
        .await
        .expect("should start core");

        Self {
            label: label.to_string(),

            instance_dir,
            cache_dir,
            download_dir,

            event_handler,
            core,
            #[cfg(feature = "test-hooks")]
            test_hooks,
        }
    }

    /// Wait until we have a home relay, so discovery and connections should work
    pub async fn wait_for_relay(&self) {
        let full_msg = format!("{} has relay", self.label);
        wait_until(
            &full_msg,
            || {
                let model = self.core.get_node_model().expect("should get node model");
                model.home_relay != "none"
            },
            || {},
        )
        .await;
    }

    /// Wait until the node model satisfies the given condition
    pub async fn wait_for_node_model_condition(
        &self,
        msg: &str,
        condition: impl Fn(&NodeModel) -> bool,
    ) {
        let full_msg = format!("{} node model where {}", self.label, msg);
        wait_until(
            &full_msg,
            || {
                let model = self.core.get_node_model().expect("should get node model");
                condition(&model)
            },
            || {
                warn!(
                    "wait_for_node_model_condition: {full_msg}: failed, node model: {:?}",
                    self.core.get_node_model(),
                );
            },
        )
        .await;
    }

    /// Wait until the library model satisfies the given condition
    pub async fn wait_for_library_model_condition(
        &self,
        msg: &str,
        condition: impl Fn(&LibraryModel) -> bool,
    ) {
        let full_msg = format!("{} library model where {}", self.label, msg);
        wait_until(
            &full_msg,
            || {
                let model = self
                    .core
                    .get_library_model()
                    .expect("should get library model");
                condition(&model)
            },
            || {
                warn!(
                    "wait_for_library_model_condition: {full_msg}: failed, library model: {:?}",
                    self.core.get_library_model(),
                );
            },
        )
        .await;
    }

    /// Wait until we have a client with the given endpoint id
    pub async fn wait_for_client(&self, other: impl TestEndpointIdExt) {
        let full_msg = format!("{} has client for {}", self.label, other.label());
        wait_until(
            &full_msg,
            || {
                let model = self.core.get_node_model().expect("should get node model");
                model.clients.contains_key(&other.endpoint_id_str())
            },
            || {
                warn!(
                    "wait_for_client: {full_msg}: failed, node model: {:?}",
                    self.core.get_node_model(),
                );
            },
        )
        .await;
    }

    /// Wait until we have a client with the given endpoint id, satisfying the given condition
    pub async fn wait_for_client_condition(
        &self,
        msg: &str,
        other: impl TestEndpointIdExt,
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
        wait_until(
            &full_msg,
            || {
                let model = self.core.get_node_model().expect("should get node model");
                if let Some(client) = model.clients.get(&other.endpoint_id_str()) {
                    condition(client)
                } else {
                    eprintln!(
                        "wait_for_client_condition: {full_msg}: client for {} missing?",
                        other.label()
                    );
                    false
                }
            },
            || {
                warn!(
                    "wait_for_client_condition: {full_msg}: failed, node model: {:?}",
                    self.core.get_node_model(),
                );
            },
        )
        .await;
    }

    /// Wait until we have a client with the given endpoint id, with state Pending
    pub async fn wait_for_client_pending(&self, other: impl TestEndpointIdExt) {
        self.wait_for_client_condition("state is Pending", other, |client| {
            matches!(client.state, ClientStateModel::Pending)
        })
        .await;
    }

    /// Wait until we have a client with the given endpoint id, with state Accepted
    pub async fn wait_for_client_accepted(&self, other: impl TestEndpointIdExt) {
        self.wait_for_client_condition("state is Accepted", other, |client| {
            matches!(client.state, ClientStateModel::Accepted)
        })
        .await;
    }

    /// Wait until we have a client with the given endpoint id, with state Closed
    pub async fn wait_for_client_closed(&self, other: impl TestEndpointIdExt) {
        self.wait_for_client_condition("state is Closed", other, |client| {
            matches!(client.state, ClientStateModel::Closed { .. })
        })
        .await;
    }

    /// Wait until we have a server with the given endpoint id
    pub async fn wait_for_server(&self, other: impl TestEndpointIdExt) {
        let full_msg = format!("{} has server for {}", self.label, other.label());
        wait_until(
            &full_msg,
            || {
                let model = self.core.get_node_model().expect("should get node model");
                model.servers.contains_key(&other.endpoint_id_str())
            },
            || {
                warn!(
                    "wait_for_server: {full_msg}: failed, node model: {:?}",
                    self.core.get_node_model(),
                );
            },
        )
        .await;
    }

    /// Wait until we have a server with the given endpoint id, satisfying the given condition
    pub async fn wait_for_server_condition(
        &self,
        msg: &str,
        other: impl TestEndpointIdExt,
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
        wait_until(
            &full_msg,
            || {
                let model = self.core.get_node_model().expect("should get node model");
                if let Some(server) = model.servers.get(&other.endpoint_id_str()) {
                    condition(server)
                } else {
                    eprintln!(
                        "wait_for_server_condition: {full_msg}: server for {} missing?",
                        other.label()
                    );
                    false
                }
            },
            || {
                warn!(
                    "wait_for_server_condition: {full_msg}: failed, node model: {:?}",
                    self.core.get_node_model(),
                );
            },
        )
        .await;
    }

    /// Wait until we have a server with the given endpoint id, with state Pending
    pub async fn wait_for_server_pending(&self, other: impl TestEndpointIdExt) {
        self.wait_for_server_condition("state is Pending", other, |server| {
            matches!(server.state, ServerStateModel::Pending)
        })
        .await;
    }

    /// Wait until we have a server with the given endpoint id, with state Accepted
    pub async fn wait_for_server_accepted(&self, other: impl TestEndpointIdExt) {
        self.wait_for_server_condition("state is Accepted", other, |server| {
            matches!(server.state, ServerStateModel::Accepted)
        })
        .await;
    }

    /// Wait until we have a server with the given endpoint id, with state Closed
    pub async fn wait_for_server_closed(&self, other: impl TestEndpointIdExt) {
        self.wait_for_server_condition("state is Closed", other, |server| {
            matches!(server.state, ServerStateModel::Closed { .. })
        })
        .await;
    }

    /// Check a client condition immediately
    pub async fn check_client_condition(
        &self,
        msg: &str,
        other: impl TestEndpointIdExt,
        condition: impl Fn(&ClientModel) -> bool,
    ) {
        let full_msg = format!(
            "{} has client for {}, where {}",
            self.label,
            other.label(),
            msg
        );

        let model = self.core.get_node_model().expect("should get node model");
        let Some(client) = model.clients.get(&other.endpoint_id_str()) else {
            panic!(
                "check_client_condition: {}: client for {} missing?",
                full_msg,
                other.label()
            );
        };
        assert!(condition(client), "{full_msg}");
    }

    /// Wait until the stats satisfy the given condition
    pub async fn wait_for_stats_condition(
        &self,
        msg: &str,
        condition: impl Fn(&StatsModel) -> bool,
    ) {
        let full_msg = format!("{} stats where {}", self.label, msg);
        wait_until(
            &full_msg,
            || {
                let stats = self.core.get_stats_model().expect("should get stats");
                condition(&stats)
            },
            || {
                warn!(
                    "wait_for_stats_condition: {full_msg}: failed, stats: {:?}",
                    self.core.get_stats_model(),
                );
            },
        )
        .await;
    }

    pub fn endpoint_id(&self) -> EndpointId {
        let model = self.core.get_node_model().expect("should get node model");
        let bytes = hex::decode(&model.endpoint_id).expect("should decode endpoint id hex");
        EndpointId::from_bytes(&bytes.try_into().expect("should have correct length"))
            .expect("should parse endpoint id")
    }

    pub fn client_model(&self, other: impl TestEndpointIdExt) -> ClientModel {
        let model = self.core.get_node_model().expect("should get node model");
        model
            .clients
            .get(&other.endpoint_id_str())
            .unwrap_or_else(|| panic!("client_model: client for {} missing?", other.label()))
            .clone()
    }
}

/// Helper trait to pass a TestCore or EndpointId and get a label and EndpointId
pub trait TestEndpointIdExt: Clone {
    fn label(&self) -> Cow<'_, str>;
    fn endpoint_id(&self) -> EndpointId;

    fn endpoint_id_str(&self) -> String {
        self.endpoint_id().to_string()
    }
}

impl TestEndpointIdExt for TestCore {
    fn label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.label)
    }

    fn endpoint_id(&self) -> EndpointId {
        TestCore::endpoint_id(self)
    }
}

impl TestEndpointIdExt for &TestCore {
    fn label(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.label)
    }

    fn endpoint_id(&self) -> EndpointId {
        TestCore::endpoint_id(self)
    }
}

impl TestEndpointIdExt for EndpointId {
    fn label(&self) -> Cow<'_, str> {
        Cow::Owned(self.to_string())
    }

    fn endpoint_id(&self) -> EndpointId {
        *self
    }
}
