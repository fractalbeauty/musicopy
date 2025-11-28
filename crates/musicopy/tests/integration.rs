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

mod library {
    use crate::common::{TestCore, fixture_path};

    #[tokio::test]
    async fn add_root_with_files() {
        let core = TestCore::start("core").await;

        let fixture_path = fixture_path("minimal");
        let root_dir = fixture_path;

        core.core
            .add_library_root("foo".into(), root_dir.to_string_lossy().to_string())
            .expect("should add library root");

        core.wait_for_library_model_condition("model has root", |model| {
            model.local_roots.len() == 1
        })
        .await;
        core.wait_for_library_model_condition("root has 1 file", |model| {
            let root = model.local_roots.first().unwrap();
            root.num_files == 1
        })
        .await;
    }

    #[tokio::test]
    async fn add_root_without_files() {
        let core = TestCore::start("core").await;

        let root_dir = core.instance_dir.join("library/root1");
        std::fs::create_dir_all(&root_dir).expect("should create root dir");

        // add library root
        core.core
            .add_library_root("bar".into(), root_dir.to_string_lossy().to_string())
            .expect("should add library root");

        // should have 0 files
        core.wait_for_library_model_condition("model has root", |model| {
            model.local_roots.len() == 1
        })
        .await;
        core.wait_for_library_model_condition("root has 0 files", |model| {
            let root = model.local_roots.first().unwrap();
            root.num_files == 0
        })
        .await;

        // copy fixture file to root dir
        let fixture_path = fixture_path("minimal");
        std::fs::copy(fixture_path.join("test.mp3"), root_dir.join("test.mp3"))
            .expect("should copy fixture files");

        // rescan
        core.core.rescan_library().expect("should rescan library");

        // should have 1 file
        core.wait_for_library_model_condition("root has 1 file", |model| {
            let root = model.local_roots.first().unwrap();
            root.num_files == 1
        })
        .await;
    }

    #[tokio::test]
    async fn remove_root() {
        let core = TestCore::start("core").await;

        let fixture_path = fixture_path("minimal");
        let root_dir = fixture_path;

        // add library root
        core.core
            .add_library_root("foo".into(), root_dir.to_string_lossy().to_string())
            .expect("should add library root");

        // should have 1 file
        core.wait_for_library_model_condition("model has root", |model| {
            model.local_roots.len() == 1
        })
        .await;
        core.wait_for_library_model_condition("root has 1 file", |model| {
            let root = model.local_roots.first().unwrap();
            root.num_files == 1
        })
        .await;

        // remove library root
        core.core
            .remove_library_root("foo".into())
            .expect("should remove library root");

        // should have 0 roots
        core.wait_for_library_model_condition("model has 0 roots", |model| {
            model.local_roots.is_empty()
        })
        .await;
    }

    #[tokio::test]
    async fn delete_file() {
        let core = TestCore::start("core").await;

        let root_dir = core.instance_dir.join("library/root1");
        let file_path = root_dir.join("test.mp3");
        std::fs::create_dir_all(&root_dir).expect("should create root dir");

        // copy fixture file to root dir
        let fixture_path = fixture_path("minimal");
        std::fs::copy(fixture_path.join("test.mp3"), &file_path)
            .expect("should copy fixture files");

        // add library root
        core.core
            .add_library_root("foo".into(), root_dir.to_string_lossy().to_string())
            .expect("should add library root");

        // should have 1 file
        core.wait_for_library_model_condition("model has root", |model| {
            model.local_roots.len() == 1
        })
        .await;
        core.wait_for_library_model_condition("root has 1 file", |model| {
            let root = model.local_roots.first().unwrap();
            root.num_files == 1
        })
        .await;

        // delete file
        std::fs::remove_file(&file_path).expect("should delete file");

        // rescan
        core.core.rescan_library().expect("should rescan library");

        // should have 0 files
        core.wait_for_library_model_condition("root has 0 files", |model| {
            let root = model.local_roots.first().unwrap();
            root.num_files == 0
        })
        .await;
    }

    #[tokio::test]
    async fn prioritize_transcodes() {
        let core = TestCore::start("core").await;

        let fixture_path = fixture_path("minimal");
        let root_dir = fixture_path;
        let file_path = root_dir.join("test.mp3");

        let transcodes_dir = core.cache_dir.join("transcodes");

        // add library root
        core.core
            .add_library_root("foo".into(), root_dir.to_string_lossy().to_string())
            .expect("should add library root");

        // wait for file
        core.wait_for_library_model_condition("model has root", |model| {
            model.local_roots.len() == 1
        })
        .await;
        core.wait_for_library_model_condition("root has 1 file", |model| {
            let root = model.local_roots.first().unwrap();
            root.num_files == 1
        })
        .await;

        // should be 0 transcodes
        assert_eq!(
            transcodes_dir
                .read_dir()
                .expect("should read transcodes dir")
                .count(),
            0
        );

        // prioritize transcodes
        core.core
            .prioritize_transcodes(vec![file_path.to_string_lossy().to_string()])
            .expect("should prioritize transcodes");

        // should have 1 not-ready transcode
        core.wait_for_library_model_condition("1 inprogress/queued transcode", |model| {
            model.transcode_count_inprogress.get() + model.transcode_count_queued.get() == 1
        })
        .await;

        // should have 1 ready transcode
        core.wait_for_library_model_condition("1 ready transcode", |model| {
            model.transcode_count_ready.get() == 1
        })
        .await;

        // should be 1 transcode
        assert_eq!(
            transcodes_dir
                .read_dir()
                .expect("should read transcodes dir")
                .count(),
            1
        );
    }
}

mod transfer {
    use crate::common::{TestCore, TestNodeIdExt, fixture_path};
    use musicopy::node::TransferJobProgressModel;

    #[tokio::test]
    async fn transfer() {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        // set up download directory
        let download_dir = core_1.instance_dir.join("downloads");
        std::fs::create_dir_all(&download_dir).expect("should create download dir");
        core_1
            .core
            .set_download_directory(&download_dir.to_string_lossy())
            .expect("should set download directory");

        // set up core 2 library
        let fixture_path = fixture_path("minimal");
        let root_dir = fixture_path;
        core_2
            .core
            .add_library_root("foo".into(), root_dir.to_string_lossy().to_string())
            .expect("should add library root");

        // wait for file
        core_2
            .wait_for_library_model_condition("model has root", |model| {
                model.local_roots.len() == 1
            })
            .await;
        core_2
            .wait_for_library_model_condition("root has 1 file", |model| {
                let root = model.local_roots.first().unwrap();
                root.num_files == 1
            })
            .await;

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

        // core 1: should have index
        core_1
            .wait_for_client_condition("index is Some", &core_2, |client| client.index.is_some())
            .await;
        {
            let model = core_1.core.get_node_model().expect("should get node model");
            let client = model
                .clients
                .get(&core_2.node_id_str())
                .expect("should have client");

            let index = client.index.as_ref().expect("should have index");
            assert_eq!(index.len(), 1);

            let item = index.first().unwrap();
            assert_eq!(item.node_id, core_2.node_id_str());
            assert_eq!(item.root, "foo");
            assert_eq!(item.path, "test.mp3");
            assert!(!item.downloaded);
        }

        // core 1: download all
        core_1
            .core
            .download_all(&core_2.node_id_str())
            .expect("should download all");

        // should have transfer jobs
        core_1
            .wait_for_client_condition("has transfer job", &core_2, |client| {
                client.transfer_jobs.len() == 1
            })
            .await;
        core_2
            .wait_for_server_condition("has transfer job", &core_1, |server| {
                server.transfer_jobs.len() == 1
            })
            .await;

        // wait for transfer to finish
        core_1
            .wait_for_client_condition("job is finished", &core_2, |client| {
                matches!(
                    client.transfer_jobs.first().unwrap().progress,
                    TransferJobProgressModel::Finished { .. }
                )
            })
            .await;

        // file should exist in download directory
        let downloaded_file_path =
            download_dir.join(format!("musicopy-{}-foo/test.ogg", core_2.node_id_str()));
        assert!(downloaded_file_path.exists());
    }
}
