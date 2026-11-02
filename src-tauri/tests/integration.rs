// src-tauri/tests/integration.rs
// End-to-end P2P integration test transferring blobs between two local Iroh nodes.
// Feature-gated behind `integration` to allow running without network setup in basic CI.

#[cfg(feature = "integration")]
mod integration {
    use std::time::Duration;
    use bytes::Bytes;
    use iroh::key::SecretKey;
    use iroh::protocol::Router;
    use iroh::Endpoint;
    use iroh_blobs::net_protocol::Blobs;
    use iroh_blobs::store::fs::Store as BlobStore;
    use iroh_blobs::util::local_pool::LocalPool;
    use tempfile::TempDir;

    #[tokio::test]
    async fn two_nodes_transfer_blob() -> anyhow::Result<()> {
        // Setup Provider (Node A)
        let dir_a = TempDir::new()?;
        let store_a_dir = dir_a.path().join("blobs");
        std::fs::create_dir_all(&store_a_dir)?;
        let pool_a = LocalPool::default();
        let store_a = BlobStore::load(&store_a_dir).await?;

        let secret_a = SecretKey::generate();
        let endpoint_a = Endpoint::builder()
            .secret_key(secret_a)
            .bind()
            .await?;

        let router_a = Router::builder(endpoint_a);
        let blobs_a = Blobs::builder(store_a)
            .build(pool_a.handle(), router_a.endpoint());
        let router_a = router_a.accept(iroh_blobs::ALPN, blobs_a.clone());
        let node_a = router_a.spawn().await?;

        // Add a test blob to Node A
        let test_payload = b"Campus Workspace P2P Blob Exchange Integrity Verification Test Payload 2026";
        let outcome = blobs_a.client().add_bytes(Bytes::from_static(test_payload)).await?;
        let hash = outcome.hash;

        let node_a_addr = node_a.endpoint().node_addr().await?;

        // Setup Consumer (Node B)
        let dir_b = TempDir::new()?;
        let store_b_dir = dir_b.path().join("blobs");
        std::fs::create_dir_all(&store_b_dir)?;
        let pool_b = LocalPool::default();
        let store_b = BlobStore::load(&store_b_dir).await?;

        let secret_b = SecretKey::generate();
        let endpoint_b = Endpoint::builder()
            .secret_key(secret_b)
            .bind()
            .await?;

        let router_b = Router::builder(endpoint_b);
        let blobs_b = Blobs::builder(store_b)
            .build(pool_b.handle(), router_b.endpoint());
        let router_b = router_b.accept(iroh_blobs::ALPN, blobs_b.clone());
        let node_b = router_b.spawn().await?;

        let client_b = blobs_b.client();

        // Verify Node B does not have the blob initially
        let initial_read = client_b.read_to_bytes(hash).await;
        assert!(initial_read.is_err(), "Node B should not have blob before transfer");

        // Request download from Node A to Node B with timeout
        let client_b_dl = client_b.clone();
        let download_future = async move {
            let stream = client_b_dl.download(hash, node_a_addr).await?;
            stream.await
        };

        tokio::time::timeout(Duration::from_secs(30), download_future).await??;

        // Verify Node B now has the blob and byte contents match exactly
        let received_bytes = client_b
            .read_to_bytes(hash)
            .await?;

        assert_eq!(&received_bytes[..], test_payload, "Received payload must match transmitted payload");

        // Clean up
        let _ = node_a.shutdown().await;
        let _ = node_b.shutdown().await;

        Ok(())
    }
}
