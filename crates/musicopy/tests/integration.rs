mod common;

use crate::common::{TestCore, TestEventHandler, TestNodeIdExt};
use musicopy::{Core, CoreOptions, library::transcode::TranscodePolicy};
use std::sync::Arc;

#[tokio::test]
async fn start_and_shutdown() {
    let event_handler = Arc::new(TestEventHandler);
    let options = CoreOptions {
        init_logging: false,
        in_memory: true,
        project_dirs: None,
        transcode_policy: TranscodePolicy::IfRequested,
    };
    let core = Core::start(event_handler, options)
        .await
        .expect("should start core");

    core.shutdown().expect("should shutdown core");
}

mod connect {
    use super::*;

    #[tokio::test]
    async fn accept() {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        core_1.wait_for_relay().await;
        core_2.wait_for_relay().await;

        // core 1: connect to core 2
        core_1
            .core
            .connect(&core_2.node_id_str())
            .await
            .expect("should connect");

        // wait for client and server
        core_1.wait_for_client(&core_2).await;
        core_2.wait_for_server(&core_1).await;

        // should be pending
        core_1.wait_for_client_pending(&core_2).await;
        core_2.wait_for_server_pending(&core_1).await;

        // core 2: accept connection
        core_2
            .core
            .accept_connection(&core_1.node_id_str())
            .expect("should accept");

        // should be accepted
        core_1.wait_for_client_accepted(&core_2).await;
        core_2.wait_for_server_accepted(&core_1).await;
    }

    #[tokio::test]
    async fn deny() {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        core_1.wait_for_relay().await;
        core_2.wait_for_relay().await;

        // core 1: connect to core 2
        core_1
            .core
            .connect(&core_2.node_id_str())
            .await
            .expect("should connect");

        // wait for client and server
        core_1.wait_for_client(&core_2).await;
        core_2.wait_for_server(&core_1).await;

        // should be pending
        core_1.wait_for_client_pending(&core_2).await;
        core_2.wait_for_server_pending(&core_1).await;

        // core 2: deny connection
        core_2
            .core
            .deny_connection(&core_1.node_id_str())
            .expect("should deny");

        // should be closed
        core_1.wait_for_client_closed(&core_2).await;
        core_2.wait_for_server_closed(&core_1).await;
    }

    #[tokio::test]
    async fn accept_then_client_close() {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        core_1.wait_for_relay().await;
        core_2.wait_for_relay().await;

        // core 1: connect to core 2
        core_1
            .core
            .connect(&core_2.node_id_str())
            .await
            .expect("should connect");

        // wait for client and server
        core_1.wait_for_client(&core_2).await;
        core_2.wait_for_server(&core_1).await;

        // should be pending
        core_1.wait_for_client_pending(&core_2).await;
        core_2.wait_for_server_pending(&core_1).await;

        // core 2: accept connection
        core_2
            .core
            .accept_connection(&core_1.node_id_str())
            .expect("should accept");

        // should be accepted
        core_1.wait_for_client_accepted(&core_2).await;
        core_2.wait_for_server_accepted(&core_1).await;

        // core 1: close client
        core_1
            .core
            .close_client(&core_2.node_id_str())
            .expect("should close client");

        // should be closed
        core_1.wait_for_client_closed(&core_2).await;
        core_2.wait_for_server_closed(&core_1).await;
    }

    #[tokio::test]
    async fn accept_then_server_close() {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        core_1.wait_for_relay().await;
        core_2.wait_for_relay().await;

        // core 1: connect to core 2
        core_1
            .core
            .connect(&core_2.node_id_str())
            .await
            .expect("should connect");

        // wait for client and server
        core_1.wait_for_client(&core_2).await;
        core_2.wait_for_server(&core_1).await;

        // should be pending
        core_1.wait_for_client_pending(&core_2).await;
        core_2.wait_for_server_pending(&core_1).await;

        // core 2: accept connection
        core_2
            .core
            .accept_connection(&core_1.node_id_str())
            .expect("should accept");

        // should be accepted
        core_1.wait_for_client_accepted(&core_2).await;
        core_2.wait_for_server_accepted(&core_1).await;

        // core 2: close server
        core_2
            .core
            .close_server(&core_1.node_id_str())
            .expect("should close server");

        // should be closed
        core_1.wait_for_client_closed(&core_2).await;
        core_2.wait_for_server_closed(&core_1).await;
    }
}
