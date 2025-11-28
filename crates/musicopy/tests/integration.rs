mod common;

mod connect {
    use crate::common::{TestCore, TestNodeIdExt};
    use musicopy::{device_name::device_name, node::ClientStateModel};
    use std::time::Duration;

    #[tokio::test]
    async fn accept() {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        // core 1: connect to core 2
        core_1.wait_for_relay().await;
        core_2.wait_for_relay().await;
        core_1
            .core
            .connect(&core_2.node_id_str())
            .await
            .expect("should connect");

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

    #[tokio::test]
    async fn accept_then_client_shutdown() {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        // core 1: connect to core 2
        core_1.wait_for_relay().await;
        core_2.wait_for_relay().await;
        core_1
            .core
            .connect(&core_2.node_id_str())
            .await
            .expect("should connect");

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

        // core 1: shutdown
        core_1.core.shutdown().expect("should shutdown");

        // should be closed
        core_2.wait_for_server_closed(&core_1).await;
    }

    #[tokio::test]
    async fn accept_then_server_shutdown() {
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

        // core 2: shutdown
        core_2.core.shutdown().expect("should shutdown");

        // should be closed
        core_1.wait_for_client_closed(&core_2).await;
    }

    #[tokio::test]
    async fn model_trust_untrust() {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        // core 2: trust core 1
        core_2
            .core
            .trust_node(&core_1.node_id_str())
            .expect("should trust");
        core_2
            .wait_for_node_model_condition("trusted nodes contains core 1", |model| {
                model
                    .trusted_nodes
                    .iter()
                    .any(|trusted_node| trusted_node.node_id == core_1.node_id_str())
            })
            .await;

        // core 2: untrust core 1
        core_2
            .core
            .untrust_node(&core_1.node_id_str())
            .expect("should untrust");
        core_2
            .wait_for_node_model_condition("trusted nodes does not contain core 1", |model| {
                !model
                    .trusted_nodes
                    .iter()
                    .any(|trusted_node| trusted_node.node_id == core_1.node_id_str())
            })
            .await;
    }

    #[tokio::test]
    async fn trust() {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        // core 2: trust core 1
        core_2
            .core
            .trust_node(&core_1.node_id_str())
            .expect("should trust");

        // core 1: connect to core 2
        core_1.wait_for_relay().await;
        core_2.wait_for_relay().await;
        core_1
            .core
            .connect(&core_2.node_id_str())
            .await
            .expect("should connect");

        // should be accepted without manual accept
        core_1.wait_for_client_accepted(&core_2).await;
        core_2.wait_for_server_accepted(&core_1).await;
    }

    #[tokio::test]
    async fn untrust() {
        let core_2 = TestCore::start("core 2").await;

        // first run
        let core_1_node_id = {
            let core_1 = TestCore::start("core 1").await;

            // core 2: trust core 1
            core_2
                .core
                .trust_node(&core_1.node_id_str())
                .expect("should trust");

            // core 1: connect to core 2
            core_1.wait_for_relay().await;
            core_2.wait_for_relay().await;
            core_1
                .core
                .connect(&core_2.node_id_str())
                .await
                .expect("should connect");

            // should be accepted without manual accept
            core_1.wait_for_client_accepted(&core_2).await;
            core_2.wait_for_server_accepted(&core_1).await;

            // shutdown and store node id
            core_1.core.shutdown().expect("should shutdown");
            core_1.node_id()
        };

        // core 2: untrust core 1
        core_2
            .core
            .untrust_node(&core_1_node_id.to_string())
            .expect("should untrust");

        // second run
        {
            let core_1 = TestCore::start("core 1").await;

            // should be reusing state
            assert_eq!(core_1.node_id(), core_1_node_id);

            // core 1: connect to core 2
            core_1.wait_for_relay().await;
            core_2.wait_for_relay().await;
            core_1
                .core
                .connect(&core_2.node_id_str())
                .await
                .expect("should connect");

            // should be pending
            core_1.wait_for_client_pending(&core_2).await;
            core_2.wait_for_server_pending(&core_1).await;

            // should not be accepted
            tokio::time::sleep(Duration::from_secs(1)).await;
            core_1
                .check_client_condition("state is not Accepted", &core_2, |client| {
                    !matches!(client.state, ClientStateModel::Accepted)
                })
                .await;
        }
    }

    #[tokio::test]
    async fn model_trusted_nodes_updated() {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        // core 2: trust core 1
        core_2
            .core
            .trust_node(&core_1.node_id_str())
            .expect("should trust");

        // core 1: connect to core 2
        core_1.wait_for_relay().await;
        core_2.wait_for_relay().await;
        core_1
            .core
            .connect(&core_2.node_id_str())
            .await
            .expect("should connect");

        // should be accepted without manual accept
        core_1.wait_for_client_accepted(&core_2).await;
        core_2.wait_for_server_accepted(&core_1).await;

        // core 2: should have connected_at
        core_2
            .wait_for_node_model_condition(
                "trusted nodes contains core 1 with connected_at",
                |model| {
                    model.trusted_nodes.iter().any(|trusted_node| {
                        trusted_node.node_id == core_1.node_id_str()
                            && trusted_node.connected_at.is_some()
                    })
                },
            )
            .await;

        // core 2: trusted node name should be set and match
        // a little weird because the device is the same on both cores
        core_2
            .wait_for_node_model_condition("trusted nodes contains core 1 with name", |model| {
                model.trusted_nodes.iter().any(|trusted_node| {
                    trusted_node.node_id == core_1.node_id_str()
                        && trusted_node.name != "unknown"
                        && trusted_node.name == device_name()
                })
            })
            .await;
    }

    #[tokio::test]
    async fn model_recent_servers_updated() {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        // core 1: connect to core 2
        core_1.wait_for_relay().await;
        core_2.wait_for_relay().await;
        core_1
            .core
            .connect(&core_2.node_id_str())
            .await
            .expect("should connect");

        // should be pending
        core_1.wait_for_client_pending(&core_2).await;
        core_2.wait_for_server_pending(&core_1).await;

        // core 1: should not have recent server entry for core 2
        core_1
            .wait_for_node_model_condition("recent servers does not contain core 2", |model| {
                !model
                    .recent_servers
                    .iter()
                    .any(|recent_server| recent_server.node_id == core_2.node_id_str())
            })
            .await;

        // core 2: accept connection
        core_2
            .core
            .accept_connection(&core_1.node_id_str())
            .expect("should accept");

        // core 1: should have recent server entry for core 2
        core_1
            .wait_for_node_model_condition("recent servers contains core 2", |model| {
                model
                    .recent_servers
                    .iter()
                    .any(|recent_server| recent_server.node_id == core_2.node_id_str())
            })
            .await;

        // core 1: trusted node name should be set and match
        core_1
            .wait_for_node_model_condition("recent servers contains core 2 with name", |model| {
                model.recent_servers.iter().any(|recent_server| {
                    recent_server.node_id == core_2.node_id_str()
                        && recent_server.name != "unknown"
                        && recent_server.name == device_name()
                })
            })
            .await;
    }
}
