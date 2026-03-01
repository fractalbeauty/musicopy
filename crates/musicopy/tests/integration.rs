#[cfg(not(feature = "test-hooks"))]
compile_error!("Integration tests require the `test-hooks` feature");

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

        // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

        // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

        // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

        // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

        // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

        // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

        // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

            // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

            // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

        // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

        // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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
    use crate::common::{LibraryFixture, TestCore};

    #[tokio::test]
    async fn add_root_with_files() {
        let core = TestCore::start("core").await;

        let root_dir = LibraryFixture::Minimal.path();

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
        let fixture_path = LibraryFixture::Minimal.path();
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

        let fixture_path = LibraryFixture::Minimal.path();
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
        let fixture_path = LibraryFixture::Minimal.path();
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

        let fixture_path = LibraryFixture::Minimal.path();
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
    use crate::common::{LibraryFixture, TestCore, TestNodeIdExt};
    use musicopy::node::{
        DownloadRequestModel, IndexItemDownloadStatusModel, TransferJobProgressModel,
    };

    /// Prepares two TestCores for transfer tests.
    ///
    /// - 1: set download directory
    /// - 2: add library root with fixture files
    /// - 2: wait for library model to update with root and files
    /// - 1: connect to 2
    /// - 2: accept connection
    /// - 1 and 2: wait for accepted state
    async fn prepare(fixture: LibraryFixture) -> (TestCore, TestCore) {
        let core_1 = TestCore::start("core 1").await;
        let core_2 = TestCore::start("core 2").await;

        // set up download directory
        std::fs::create_dir_all(&core_1.download_dir).expect("should create download dir");
        core_1
            .core
            .set_download_directory(&core_1.download_dir.to_string_lossy())
            .expect("should set download directory");

        // set up core 2 library
        core_2
            .core
            .add_library_root("foo".into(), fixture.path().to_string_lossy().to_string())
            .expect("should add library root");

        // wait for file
        core_2
            .wait_for_library_model_condition("model has root", |model| {
                model.local_roots.len() == 1
            })
            .await;
        core_2
            .wait_for_library_model_condition("root has files", |model| {
                let root = model.local_roots.first().unwrap();
                root.num_files == fixture.num_items() as u64
            })
            .await;

        // core 1: connect to core 2
        core_1.wait_for_relay().await;
        core_2.wait_for_relay().await;

        // HACK: use StaticProvider/MemoryLookup to fix discovery flakiness
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

        (core_1, core_2)
    }

    /// Calls `prepare`, then waits for core 1 to receive the index and returns a vec of download
    /// items for convenience.
    async fn prepare_with_index(
        fixture: LibraryFixture,
    ) -> (TestCore, TestCore, Vec<DownloadRequestModel>) {
        let (core_1, core_2) = prepare(fixture).await;

        // wait for index with correct number of items
        core_1
            .wait_for_client_condition("index has items", &core_2, |client| {
                client
                    .index
                    .as_ref()
                    .is_some_and(|idx| idx.len() == fixture.num_items())
            })
            .await;

        // get download items
        let download_items = core_1
            .client_model(&core_2)
            .index
            .unwrap()
            .into_iter()
            .map(|item| DownloadRequestModel {
                node_id: item.node_id.clone(),
                root: item.root.clone(),
                path: item.path.clone(),
            })
            .collect::<Vec<_>>();

        (core_1, core_2, download_items)
    }

    #[tokio::test]
    async fn transfer() {
        let (core_1, core_2) = prepare(LibraryFixture::Minimal).await;

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
            assert!(item.download_status.is_none());
        }

        // core 1: download file
        core_1
            .core
            .set_downloads(
                &core_2.node_id_str(),
                vec![DownloadRequestModel {
                    node_id: core_2.node_id_str(),
                    root: "foo".into(),
                    path: "test.mp3".into(),
                }],
            )
            .expect("should set downloads");

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
                    client.transfer_jobs.first().map(|j| &j.progress),
                    Some(TransferJobProgressModel::Finished { .. })
                )
            })
            .await;

        // file should exist in download directory
        let downloaded_file_path = core_1
            .download_dir
            .join(format!("musicopy-{}-foo/test.ogg", core_2.node_id_str()));
        assert!(downloaded_file_path.exists());
    }

    /// Test pausing downloads:
    /// - Request both items
    /// - Both jobs should reach Ready
    /// - Both index items should reach InProgress
    /// - TestHooks: allow first item
    /// - 1st job should reach Finished
    /// - 1st index item should reach Downloaded
    /// - 2nd job should still be Ready
    /// - 2nd index item should still be InProgress
    /// - Pause
    /// - 1st job should still be Finished
    /// - 1st index item should still be Downloaded
    /// - 2nd job should become Paused
    /// - 2nd index item should become Paused
    /// - Resume (using SetDownloads with the Paused item)
    /// - 2nd job should reach Finished
    /// - 2nd index item should reach Downloaded
    #[tokio::test]
    async fn pause() {
        let (core_1, core_2, download_items) = prepare_with_index(LibraryFixture::Multiple).await;
        core_1.test_hooks.enable_download_gate();

        // request both items
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // both jobs should reach Ready
        core_1
            .wait_for_client_condition("both jobs are Ready", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Ready))
            })
            .await;
        // both index items should reach InProgress
        core_1
            .core
            .refresh_client_index(&core_2.node_id_str())
            .expect("should refresh client index");
        core_1
            .wait_for_client_condition("both index items are InProgress", &core_2, |client| {
                client.index.as_ref().unwrap().len() == 2
                    && client.index.as_ref().unwrap().iter().all(|item| {
                        matches!(
                            item.download_status,
                            Some(IndexItemDownloadStatusModel::InProgress)
                        )
                    })
            })
            .await;

        // allow first item to download
        core_1.test_hooks.add_download_permits(1);

        // first job should reach Finished, second should still be Ready
        core_1
            .wait_for_client_condition(
                "one job is Finished, other job is still Ready",
                &core_2,
                |client| {
                    client.transfer_jobs.len() == 2
                        && client.transfer_jobs.iter().any(|j| {
                            matches!(j.progress, TransferJobProgressModel::Finished { .. })
                        })
                        && client
                            .transfer_jobs
                            .iter()
                            .any(|j| matches!(j.progress, TransferJobProgressModel::Ready))
                },
            )
            .await;
        let finished_job_path = core_1
            .client_model(&core_2)
            .transfer_jobs
            .iter()
            .find(|j| matches!(j.progress, TransferJobProgressModel::Finished { .. }))
            .map(|j| j.file_path.clone())
            .unwrap();
        // finished index item should reach Downloaded, other should still be InProgress
        core_1
            .core
            .refresh_client_index(&core_2.node_id_str())
            .expect("should refresh client index");
        core_1
            .wait_for_client_condition(
                "finished index item is Downloaded, other index item is still InProgress",
                &core_2,
                |client| {
                    client.index.as_ref().unwrap().len() == 2
                        && matches!(
                            client
                                .index
                                .as_ref()
                                .unwrap()
                                .iter()
                                .find(|i| i.path == finished_job_path)
                                .unwrap()
                                .download_status,
                            Some(IndexItemDownloadStatusModel::Downloaded)
                        )
                        && matches!(
                            client
                                .index
                                .as_ref()
                                .unwrap()
                                .iter()
                                .find(|i| i.path != finished_job_path)
                                .unwrap()
                                .download_status,
                            Some(IndexItemDownloadStatusModel::InProgress)
                        )
                },
            )
            .await;

        // pause downloads
        core_1
            .core
            .pause_downloads(&core_2.node_id_str())
            .expect("should pause downloads");

        // finished job should still be Finished
        core_1
            .wait_for_client_condition("finished job is still Finished", &core_2, |client| {
                matches!(
                    client
                        .transfer_jobs
                        .iter()
                        .find(|j| j.file_path == finished_job_path)
                        .map(|j| &j.progress),
                    Some(TransferJobProgressModel::Finished { .. })
                )
            })
            .await;
        // finished index item should still be Downloaded
        core_1
            .core
            .refresh_client_index(&core_2.node_id_str())
            .expect("should refresh client index");
        core_1
            .wait_for_client_condition(
                "finished index item is still Downloaded",
                &core_2,
                |client| {
                    client.index.as_ref().unwrap().len() == 2
                        && matches!(
                            client
                                .index
                                .as_ref()
                                .unwrap()
                                .iter()
                                .find(|i| i.path == finished_job_path)
                                .unwrap()
                                .download_status,
                            Some(IndexItemDownloadStatusModel::Downloaded)
                        )
                },
            )
            .await;

        // other job should become Paused
        core_1
            .wait_for_client_condition("other job is Paused", &core_2, |client| {
                matches!(
                    client
                        .transfer_jobs
                        .iter()
                        .find(|j| j.file_path != finished_job_path)
                        .map(|j| &j.progress),
                    Some(TransferJobProgressModel::Paused)
                )
            })
            .await;
        // other index item should become Paused
        core_1
            .core
            .refresh_client_index(&core_2.node_id_str())
            .expect("should refresh client index");
        core_1
            .wait_for_client_condition("other index item is Paused", &core_2, |client| {
                client.index.as_ref().unwrap().len() == 2
                    && matches!(
                        client
                            .index
                            .as_ref()
                            .unwrap()
                            .iter()
                            .find(|i| i.path != finished_job_path)
                            .unwrap()
                            .download_status,
                        Some(IndexItemDownloadStatusModel::Paused)
                    )
            })
            .await;

        // resume downloads using SetDownloads
        let not_finished_download_item = download_items
            .into_iter()
            .find(|item| item.path != finished_job_path)
            .unwrap();
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), vec![not_finished_download_item])
            .expect("should set downloads");

        // other job should be Ready
        core_1
            .wait_for_client_condition("other job is Ready", &core_2, |client| {
                matches!(
                    client
                        .transfer_jobs
                        .iter()
                        .find(|j| j.file_path != finished_job_path)
                        .map(|j| &j.progress),
                    Some(TransferJobProgressModel::Ready)
                )
            })
            .await;
        // other index item should be InProgress
        core_1
            .core
            .refresh_client_index(&core_2.node_id_str())
            .expect("should refresh client index");
        core_1
            .wait_for_client_condition("other index item is InProgress", &core_2, |client| {
                client.index.as_ref().unwrap().len() == 2
                    && matches!(
                        client
                            .index
                            .as_ref()
                            .unwrap()
                            .iter()
                            .find(|i| i.path != finished_job_path)
                            .unwrap()
                            .download_status,
                        Some(IndexItemDownloadStatusModel::InProgress)
                    )
            })
            .await;

        // allow second item to download
        core_1.test_hooks.add_download_permits(1);

        // other job should reach Finished
        core_1
            .wait_for_client_condition("other job is Finished", &core_2, |client| {
                matches!(
                    client
                        .transfer_jobs
                        .iter()
                        .find(|j| j.file_path != finished_job_path)
                        .map(|j| &j.progress),
                    Some(TransferJobProgressModel::Finished { .. })
                )
            })
            .await;
        // other index item should reach Downloaded
        core_1
            .core
            .refresh_client_index(&core_2.node_id_str())
            .expect("should refresh client index");
        core_1
            .wait_for_client_condition("other index item is Downloaded", &core_2, |client| {
                client.index.as_ref().unwrap().len() == 2
                    && matches!(
                        client
                            .index
                            .as_ref()
                            .unwrap()
                            .iter()
                            .find(|i| i.path != finished_job_path)
                            .unwrap()
                            .download_status,
                        Some(IndexItemDownloadStatusModel::Downloaded)
                    )
            })
            .await;
    }

    /// Test set downloads with the same items while waiting:
    /// - Request both items
    /// - Both jobs should be Ready
    /// - Request both items again
    /// - Both jobs should still be Ready (only 2 jobs)
    #[tokio::test]
    async fn set_same_downloads_while_waiting() {
        let (core_1, core_2, download_items) = prepare_with_index(LibraryFixture::Multiple).await;
        core_1.test_hooks.enable_download_gate();

        // request both items
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // should have two Ready jobs
        core_1
            .wait_for_client_condition("has 2 Ready jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Ready))
            })
            .await;

        // request both items again
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // should still have two Ready jobs
        core_1
            .wait_for_client_condition("still has 2 Ready jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Ready))
            })
            .await;
    }

    /// Test set downloads with the same items while paused:
    /// - Request both items
    /// - Both jobs should be Ready
    /// - Pause downloads
    /// - Both jobs should be Paused
    /// - Request both items again
    /// - Both jobs should still be Paused (only 2 jobs)
    #[tokio::test]
    async fn set_same_downloads_while_paused() {
        let (core_1, core_2, download_items) = prepare_with_index(LibraryFixture::Multiple).await;
        core_1.test_hooks.enable_download_gate();

        // request both items
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // should have two Ready jobs
        core_1
            .wait_for_client_condition("has 2 Ready jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Ready))
            })
            .await;

        // pause downloads
        core_1
            .core
            .pause_downloads(&core_2.node_id_str())
            .expect("should pause downloads");

        // should have two Paused jobs
        core_1
            .wait_for_client_condition("has 2 Paused jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Paused))
            })
            .await;

        // request both items again
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // should still have two Paused jobs
        core_1
            .wait_for_client_condition("still has 2 Paused jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Paused))
            })
            .await;
    }

    /// Test set downloads with the same items while finished:
    /// - Request both items
    /// - Both jobs should be Ready
    /// - Allow both to download
    /// - Both jobs should reach Finished
    /// - Request both items again
    /// - Both jobs should still be Finished (only 2 jobs)
    #[tokio::test]
    async fn set_same_downloads_while_finished() {
        let (core_1, core_2, download_items) = prepare_with_index(LibraryFixture::Multiple).await;
        core_1.test_hooks.enable_download_gate();

        // request both items
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // should have two Ready jobs
        core_1
            .wait_for_client_condition("has 2 Ready jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Ready))
            })
            .await;

        // allow both to download
        core_1.test_hooks.add_download_permits(2);

        // both jobs should reach Finished
        core_1
            .wait_for_client_condition("both jobs are Finished", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Finished { .. }))
            })
            .await;

        // request both items again
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // should still have two Finished jobs
        core_1
            .wait_for_client_condition("still has 2 Finished jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Finished { .. }))
            })
            .await;
    }

    /// Test adding items while waiting using SetDownloads:
    /// - Request first item
    /// - Should be one Ready job
    /// - Request second item
    /// - Should be two Ready jobs
    /// - Allow both to download
    /// - Both jobs should reach Finished
    #[tokio::test]
    async fn add_items_while_waiting() {
        let (core_1, core_2, download_items) = prepare_with_index(LibraryFixture::Multiple).await;
        core_1.test_hooks.enable_download_gate();

        // request first item
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), vec![download_items[0].clone()])
            .expect("should set downloads");

        // should have one Ready job
        core_1
            .wait_for_client_condition("has 1 Ready job", &core_2, |client| {
                client.transfer_jobs.len() == 1
                    && matches!(
                        client.transfer_jobs.first().map(|j| &j.progress),
                        Some(TransferJobProgressModel::Ready)
                    )
            })
            .await;

        // request second item
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // should have two Ready jobs
        core_1
            .wait_for_client_condition("has 2 Ready jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Ready))
            })
            .await;

        // allow both to download
        core_1.test_hooks.add_download_permits(2);

        // both jobs should reach Finished
        core_1
            .wait_for_client_condition("both jobs are Finished", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Finished { .. }))
            })
            .await;
    }

    /// Test adding items while paused using SetDownloads:
    /// - Request first item
    /// - Should be one Ready job
    /// - Pause downloads
    /// - Should be one Paused job
    /// - Request second item (should unpause)
    /// - Should be two Ready jobs
    /// - Allow both to download
    /// - Both jobs should reach Finished
    #[tokio::test]
    async fn add_items_while_paused() {
        let (core_1, core_2, download_items) = prepare_with_index(LibraryFixture::Multiple).await;
        core_1.test_hooks.enable_download_gate();

        // request first item
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), vec![download_items[0].clone()])
            .expect("should set downloads");

        // should have one Ready job
        core_1
            .wait_for_client_condition("has 1 Ready job", &core_2, |client| {
                client.transfer_jobs.len() == 1
                    && matches!(
                        client.transfer_jobs.first().map(|j| &j.progress),
                        Some(TransferJobProgressModel::Ready)
                    )
            })
            .await;

        // pause downloads
        core_1
            .core
            .pause_downloads(&core_2.node_id_str())
            .expect("should pause downloads");

        // should have one Paused job
        core_1
            .wait_for_client_condition("has 1 Paused job", &core_2, |client| {
                client.transfer_jobs.len() == 1
                    && matches!(
                        client.transfer_jobs.first().map(|j| &j.progress),
                        Some(TransferJobProgressModel::Paused)
                    )
            })
            .await;

        // request second item (should unpause)
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // should have two Ready jobs
        core_1
            .wait_for_client_condition("has 2 Ready jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Ready))
            })
            .await;

        // allow both to download
        core_1.test_hooks.add_download_permits(2);

        // both jobs should reach Finished
        core_1
            .wait_for_client_condition("both jobs are Finished", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Finished { .. }))
            })
            .await;
    }

    /// Test adding items while finished using SetDownloads:
    /// - Request first item
    /// - Should be one Ready job
    /// - Allow it to download
    /// - Should reach Finished
    /// - Request second item
    /// - Should be one Ready job
    /// - Allow it to download
    /// - Should reach Finished
    #[tokio::test]
    async fn add_items_while_finished() {
        let (core_1, core_2, download_items) = prepare_with_index(LibraryFixture::Multiple).await;
        core_1.test_hooks.enable_download_gate();

        // request first item
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), vec![download_items[0].clone()])
            .expect("should set downloads");

        // should have one Ready job
        core_1
            .wait_for_client_condition("has 1 Ready job", &core_2, |client| {
                client.transfer_jobs.len() == 1
                    && matches!(
                        client.transfer_jobs.first().map(|j| &j.progress),
                        Some(TransferJobProgressModel::Ready)
                    )
            })
            .await;

        // allow first item to download
        core_1.test_hooks.add_download_permits(1);

        // should reach Finished
        core_1
            .wait_for_client_condition("job is Finished", &core_2, |client| {
                client.transfer_jobs.len() == 1
                    && matches!(
                        client.transfer_jobs.first().map(|j| &j.progress),
                        Some(TransferJobProgressModel::Finished { .. })
                    )
            })
            .await;

        // request second item
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), vec![download_items[1].clone()])
            .expect("should set downloads");

        // first job should still be Finished, second should be Ready
        core_1
            .wait_for_client_condition(
                "first job is Finished, second job is Ready",
                &core_2,
                |client| {
                    client.transfer_jobs.len() == 2
                        && matches!(
                            client
                                .transfer_jobs
                                .iter()
                                .find(|j| j.job_id == 0)
                                .map(|j| &j.progress),
                            Some(TransferJobProgressModel::Finished { .. })
                        )
                        && matches!(
                            client
                                .transfer_jobs
                                .iter()
                                .find(|j| j.job_id == 1)
                                .map(|j| &j.progress),
                            Some(TransferJobProgressModel::Ready)
                        )
                },
            )
            .await;

        // allow second item to download
        core_1.test_hooks.add_download_permits(1);

        // both jobs should reach Finished
        core_1
            .wait_for_client_condition("both jobs are Finished", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Finished { .. }))
            })
            .await;
    }

    /// Test removing items while waiting:
    /// - Request both items
    /// - Should be two Ready jobs
    /// - Request only first item
    /// - Should still be two jobs, since we only remove jobs when paused
    #[tokio::test]
    async fn remove_items_while_waiting() {
        let (core_1, core_2, download_items) = prepare_with_index(LibraryFixture::Multiple).await;
        core_1.test_hooks.enable_download_gate();

        // request both items
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // should have two Ready jobs
        core_1
            .wait_for_client_condition("has 2 Ready jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Ready))
            })
            .await;

        // request only first item
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), vec![download_items[0].clone()])
            .expect("should set downloads");

        // should still have two jobs, since we only remove jobs when paused
        core_1
            .wait_for_client_condition("still has 2 jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
            })
            .await;
    }

    /// Test removing items while paused:
    /// - Request both items
    /// - Should be two Ready jobs
    /// - Pause downloads
    /// - Should be two Paused jobs
    /// - Request only first item (should unpause and remove second item)
    /// - Should be only one Ready job for the first item
    #[tokio::test]
    async fn remove_items_while_paused() {
        let (core_1, core_2, download_items) = prepare_with_index(LibraryFixture::Multiple).await;
        core_1.test_hooks.enable_download_gate();

        // request both items
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // should have two Ready jobs
        core_1
            .wait_for_client_condition("has 2 Ready jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Ready))
            })
            .await;

        // pause downloads
        core_1
            .core
            .pause_downloads(&core_2.node_id_str())
            .expect("should pause downloads");

        // should have two Paused jobs
        core_1
            .wait_for_client_condition("has 2 Paused jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Paused))
            })
            .await;

        // request only first item
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), vec![download_items[0].clone()])
            .expect("should set downloads");

        // should be only one Ready job for the first item
        core_1
            .wait_for_client_condition(
                "only has 1 Ready job for the first item",
                &core_2,
                |client| {
                    client.transfer_jobs.len() == 1
                        && matches!(
                            client.transfer_jobs.first().map(|j| &j.progress),
                            Some(TransferJobProgressModel::Ready)
                        )
                        && client.transfer_jobs.first().unwrap().file_path == download_items[0].path
                },
            )
            .await;
    }

    /// Test removing items while finished:
    /// - Request both items
    /// - Should be two Ready jobs
    /// - Allow both to download
    /// - Should reach Finished
    /// - Request only first item
    /// - Should still be two Finished jobs, since we don't remove finished jobs
    #[tokio::test]
    async fn remove_items_while_finished() {
        let (core_1, core_2, download_items) = prepare_with_index(LibraryFixture::Multiple).await;
        core_1.test_hooks.enable_download_gate();

        // request both items
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), download_items.clone())
            .expect("should set downloads");

        // should have two Ready jobs
        core_1
            .wait_for_client_condition("has 2 Ready jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Ready))
            })
            .await;

        // allow both to download
        core_1.test_hooks.add_download_permits(2);

        // both jobs should reach Finished
        core_1
            .wait_for_client_condition("both jobs are Finished", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Finished { .. }))
            })
            .await;

        // request only first item
        core_1
            .core
            .set_downloads(&core_2.node_id_str(), vec![download_items[0].clone()])
            .expect("should set downloads");

        // should still have two Finished jobs, since we don't remove finished jobs
        core_1
            .wait_for_client_condition("still has 2 Finished jobs", &core_2, |client| {
                client.transfer_jobs.len() == 2
                    && client
                        .transfer_jobs
                        .iter()
                        .all(|j| matches!(j.progress, TransferJobProgressModel::Finished { .. }))
            })
            .await;
    }
}
