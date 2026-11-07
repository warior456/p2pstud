// src-tauri/tests/integration.rs
// End-to-end P2P integration test transferring blobs and syncing catalogs between two local Iroh nodes.
// Feature-gated behind `integration` to allow running without network setup in basic CI.

#[cfg(feature = "integration")]
mod integration {
    use std::sync::Arc;
    use bytes::Bytes;
    use iroh::endpoint::presets;
    use iroh::protocol::Router;
    use iroh::{Endpoint, SecretKey};
    use iroh_blobs::BlobsProtocol;
    use iroh_blobs::store::fs::FsStore as BlobStore;
    use p2pstud_lib::storage::db::Database;
    use tempfile::TempDir;

    #[tokio::test]
    async fn two_nodes_transfer_blob() -> anyhow::Result<()> {
        // Setup Provider (Node A)
        let dir_a = TempDir::new()?;
        let store_a_dir = dir_a.path().join("blobs");
        std::fs::create_dir_all(&store_a_dir)?;
        let store_a = BlobStore::load(&store_a_dir).await?;

        let secret_a = SecretKey::generate();
        let endpoint_a = Endpoint::builder(presets::N0)
            .secret_key(secret_a)
            .relay_mode(iroh::RelayMode::Disabled)
            .bind()
            .await?;

        let blobs_a = Arc::new(BlobsProtocol::new(&store_a, None));
        let router_a = Router::builder(endpoint_a.clone())
            .accept(iroh_blobs::ALPN, (*blobs_a).clone());
        let _node_a = router_a.spawn();

        // Add a test blob to Node A
        let test_payload =
            b"Campus Workspace P2P Blob Exchange Integrity Verification Test Payload 2026";
        let outcome = store_a
            .blobs()
            .add_bytes(Bytes::from_static(test_payload))
            .await?;
        let hash = outcome.hash;

        let node_a_addr = endpoint_a.addr();

        // Setup Consumer (Node B)
        let dir_b = TempDir::new()?;
        let store_b_dir = dir_b.path().join("blobs");
        std::fs::create_dir_all(&store_b_dir)?;
        let store_b = BlobStore::load(&store_b_dir).await?;

        let secret_b = SecretKey::generate();
        let endpoint_b = Endpoint::builder(presets::N0)
            .secret_key(secret_b)
            .relay_mode(iroh::RelayMode::Disabled)
            .bind()
            .await?;

        let blobs_b = Arc::new(BlobsProtocol::new(&store_b, None));
        let router_b = Router::builder(endpoint_b.clone())
            .accept(iroh_blobs::ALPN, (*blobs_b).clone());
        let _node_b = router_b.spawn();

        // Verify Node B does not have the blob initially
        let initial_status = store_b.blobs().status(hash).await?;
        assert!(
            !matches!(initial_status, iroh_blobs::api::blobs::BlobStatus::Complete { .. }),
            "Node B should not have blob before transfer"
        );

        // Connect Node B to Node A
        let _conn = endpoint_b.connect(node_a_addr, iroh_blobs::ALPN).await?;

        // Request download from Node A to Node B
        let downloader = store_b.downloader(&endpoint_b);
        downloader.download(hash, vec![endpoint_a.id()]).await?;

        // Verify Node B now has the complete blob
        let final_status = store_b.blobs().status(hash).await?;
        assert!(
            matches!(final_status, iroh_blobs::api::blobs::BlobStatus::Complete { .. }),
            "Node B must have blob after transfer"
        );

        let read_bytes = store_b.blobs().get_bytes(hash).await?;
        assert_eq!(&read_bytes[..], test_payload);

        Ok(())
    }

    #[tokio::test]
    async fn two_nodes_catalog_sync_and_version_exchange() -> anyhow::Result<()> {
        let dir_a = TempDir::new()?;
        let db_a = Arc::new(Database::open(&dir_a.path().join("db_a.db"))?);
        let store_a = BlobStore::load(&dir_a.path().join("blobs")).await?;
        let secret_a = SecretKey::generate();
        let endpoint_a = Endpoint::builder(presets::N0)
            .secret_key(secret_a)
            .relay_mode(iroh::RelayMode::Disabled)
            .bind()
            .await?;

        let dir_b = TempDir::new()?;
        let db_b = Arc::new(Database::open(&dir_b.path().join("db_b.db"))?);
        let store_b = BlobStore::load(&dir_b.path().join("blobs")).await?;
        let secret_b = SecretKey::generate();
        let endpoint_b = Endpoint::builder(presets::N0)
            .secret_key(secret_b)
            .relay_mode(iroh::RelayMode::Disabled)
            .bind()
            .await?;

        let sync_alpn = p2pstud_lib::p2p::sync::get_sync_alpn(&db_a);

        // Setup sync handler on Node A
        let handler_a = Arc::new(p2pstud_lib::p2p::sync::CatalogSyncHandler::new(
            db_a.clone(),
            Some(store_a.clone()),
            endpoint_a.id().to_string(),
        ));
        let blobs_a = Arc::new(BlobsProtocol::new(&store_a, None));
        let router_a = Router::builder(endpoint_a.clone())
            .accept(iroh_blobs::ALPN, (*blobs_a).clone())
            .accept(sync_alpn.clone(), handler_a);
        let _router_a_guard = router_a.spawn();

        // Setup sync handler on Node B
        let handler_b = Arc::new(p2pstud_lib::p2p::sync::CatalogSyncHandler::new(
            db_b.clone(),
            Some(store_b.clone()),
            endpoint_b.id().to_string(),
        ));
        let blobs_b = Arc::new(BlobsProtocol::new(&store_b, None));
        let router_b = Router::builder(endpoint_b.clone())
            .accept(iroh_blobs::ALPN, (*blobs_b).clone())
            .accept(sync_alpn.clone(), handler_b);
        let _router_b_guard = router_b.spawn();

        // Node B initiates sync with Node A
        p2pstud_lib::p2p::sync::sync_catalog_with_peer(
            &endpoint_b,
            &db_b,
            Some(&store_b),
            endpoint_a.addr(),
        ).await?;

        // Both nodes must have registered each other in active peers
        let peers = p2pstud_lib::p2p::sync::get_active_peer_ids();
        assert!(peers.contains(&endpoint_a.id()));

        // Both nodes must have recorded each other in peer_versions table
        let conn_b = db_b.lock();
        let count_b: i64 = conn_b.query_row(
            "SELECT COUNT(*) FROM peer_versions WHERE node_id = ?1",
            rusqlite::params![endpoint_a.id().to_string()],
            |r| r.get(0),
        )?;
        assert_eq!(count_b, 1, "Node B must record Node A in peer_versions");

        let conn_a = db_a.lock();
        let count_a: i64 = conn_a.query_row(
            "SELECT COUNT(*) FROM peer_versions WHERE node_id = ?1",
            rusqlite::params![endpoint_b.id().to_string()],
            |r| r.get(0),
        )?;
        assert_eq!(count_a, 1, "Node A must record Node B in peer_versions via NodeAnnounce");

        Ok(())
    }
}
