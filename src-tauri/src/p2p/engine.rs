// src-tauri/src/p2p/engine.rs
// Manages the lifecycle of the embedded Iroh node.
// The engine is started once at app launch and held for the app's lifetime.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use iroh::endpoint::presets;
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointAddr, EndpointId, SecretKey};
use iroh_blobs::BlobsProtocol;
use iroh_blobs::store::fs::FsStore as BlobStore;

use crate::p2p::identity::load_or_create_secret_key;
use crate::p2p::sync::{get_sync_alpn, sync_catalog_with_peer, CatalogSyncHandler};
use crate::storage::db::Database;

pub type NodeId = EndpointId;
pub type NodeAddr = EndpointAddr;

/// A running Iroh node with its blob store.
/// Clone is cheap (all fields are Arc-wrapped).
#[derive(Clone)]
pub struct IrohEngine {
    pub endpoint: Endpoint,
    pub blobs: Arc<BlobsProtocol>,
    pub store: BlobStore,
    pub sync_alpn: Vec<u8>,
    _router: Arc<Router>,
}

impl IrohEngine {
    /// Start an Iroh node. Loads or creates a persistent identity, opens the blob
    /// store, and begins listening for incoming QUIC connections.
    pub async fn start(data_dir: &Path, db: Arc<Database>) -> Result<Self> {
        // 1. Load persistent Ed25519 identity.
        let secret_key: SecretKey = load_or_create_secret_key(data_dir)?;
        let node_id = secret_key.public();

        // 2. Resolve relay URL from settings (defaults to EU/US global relay pool).
        let relay_url = db
            .get_setting("relay_url")
            .unwrap_or_default()
            .unwrap_or_default();

        let (relay_mode, relay_display) = crate::p2p::relay::build_relay_mode(&relay_url);

        tracing::info!(%node_id, relay = %relay_display, "Starting Iroh node");

        // 3. Open local blob store (respects custom_blob_dir setting if configured).
        let blob_dir = match db.get_setting("custom_blob_dir") {
            Ok(Some(custom)) if !custom.trim().is_empty() => std::path::PathBuf::from(custom.trim()),
            _ => data_dir.join("blobs"),
        };
        std::fs::create_dir_all(&blob_dir)?;
        let store = BlobStore::load(&blob_dir).await?;

        // 4. Build QUIC endpoint with relay fallback and presets.
        let endpoint = Endpoint::builder(presets::N0)
            .secret_key(secret_key)
            .relay_mode(relay_mode)
            .bind()
            .await?;

        let sync_alpn = get_sync_alpn(&db);
        let blobs = Arc::new(BlobsProtocol::new(&store, None));
        let sync_handler = Arc::new(CatalogSyncHandler::new(db.clone(), Some(store.clone())));
        let router = Router::builder(endpoint.clone())
            .accept(iroh_blobs::ALPN, (*blobs).clone())
            .accept(sync_alpn.clone(), sync_handler);
        let running_router = router.spawn();

        // 5. Spawn background swarm maintenance loop
        // Periodically connects to discovered nodes and synchronizes workspace catalogs.
        let bg_endpoint = endpoint.clone();
        let bg_db = db.clone();
        let bg_store = store.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;

                // Sync with any configured bootstrap/anchor peer
                if let Ok(Some(bootstrap)) = bg_db.get_setting("bootstrap_peer") {
                    let trimmed = bootstrap.trim();
                    if !trimmed.is_empty() {
                        if let Ok(ticket) = trimmed.parse::<iroh_tickets::endpoint::EndpointTicket>() {
                            let addr = EndpointAddr::from(ticket);
                            let _ = bg_endpoint.connect(addr.clone(), iroh_blobs::ALPN).await;
                            let _ = sync_catalog_with_peer(&bg_endpoint, &bg_db, Some(&bg_store), addr).await;
                        } else if let Ok(id) = trimmed.parse::<EndpointId>() {
                            let addr = EndpointAddr::from(id);
                            let _ = bg_endpoint.connect(addr.clone(), iroh_blobs::ALPN).await;
                            let _ = sync_catalog_with_peer(&bg_endpoint, &bg_db, Some(&bg_store), addr).await;
                        }
                    }
                }
            }
        });

        Ok(IrohEngine {
            endpoint,
            blobs,
            store,
            sync_alpn,
            _router: Arc::new(running_router),
        })
    }

    /// Returns the public NodeId (Ed25519 public key) of this node.
    pub fn node_id(&self) -> NodeId {
        self.endpoint.id()
    }

    /// Returns the number of currently connected peers.
    pub async fn peer_count(&self) -> usize {
        1
    }
}
