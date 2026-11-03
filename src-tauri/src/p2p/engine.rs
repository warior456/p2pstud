// src-tauri/src/p2p/engine.rs
// Manages the lifecycle of the embedded Iroh node.
// The engine is started once at app launch and held for the app's lifetime.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use iroh::key::SecretKey;
use iroh::protocol::Router;
use iroh::{Endpoint, NodeId, RelayMap, RelayMode};
use iroh_blobs::net_protocol::Blobs;
use iroh_blobs::store::fs::Store as BlobStore;
use iroh_blobs::util::local_pool::LocalPool;

use crate::p2p::identity::load_or_create_secret_key;
use crate::p2p::relay::parse_relay_url;
use crate::p2p::sync::{sync_catalog_with_peer, CatalogSyncHandler, SYNC_ALPN};
use crate::storage::db::Database;

/// A running Iroh node with its blob store.
/// Clone is cheap (all fields are Arc-wrapped).
#[derive(Clone)]
pub struct IrohEngine {
    pub endpoint: Endpoint,
    pub blobs: Arc<Blobs<BlobStore>>,
    pub store: BlobStore,
    _router: Arc<Router>,
    _pool: Arc<LocalPool>,
}

impl IrohEngine {
    /// Start an Iroh node. Loads or creates a persistent identity, opens the blob
    /// store, and begins listening for incoming QUIC connections.
    pub async fn start(data_dir: &Path, db: Arc<Database>) -> Result<Self> {
        // 1. Load persistent Ed25519 identity.
        let secret_key: SecretKey = load_or_create_secret_key(data_dir)?;
        let node_id = secret_key.public();

        // 2. Resolve relay URL from settings (falls back to Iroh default).
        let relay_url = db
            .get_setting("relay_url")
            .unwrap_or_default()
            .unwrap_or_default();
        let relay = parse_relay_url(&relay_url)?;

        tracing::info!(%node_id, relay = %relay, "Starting Iroh node");

        // 3. Open local blob store.
        let blob_dir = data_dir.join("blobs");
        std::fs::create_dir_all(&blob_dir)?;
        let pool = LocalPool::default();
        let store = BlobStore::load(&blob_dir).await?;

        // 4. Build QUIC endpoint with relay fallback and multi-layer discovery
        // (Local Swarm / mDNS for campus LAN, PKARR DHT & n0 DNS for WAN).
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .relay_mode(RelayMode::Custom(RelayMap::from_url(relay)))
            .discovery_local_network()
            .discovery_n0()
            .discovery_dht()
            .bind()
            .await?;

        let router = Router::builder(endpoint);
        let blobs = Blobs::builder(store.clone()).build(pool.handle(), router.endpoint());
        let router = router.accept(iroh_blobs::ALPN, blobs.clone());
        let sync_handler = Arc::new(CatalogSyncHandler::new(db.clone()));
        let router = router.accept(SYNC_ALPN, sync_handler);
        let endpoint = router.endpoint().clone();
        let running_router = router.spawn().await?;

        // 5. Spawn background swarm maintenance loop
        // Periodically connects to discovered nodes and synchronizes workspace catalogs.
        let bg_endpoint = endpoint.clone();
        let bg_db = db.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            loop {
                interval.tick().await;

                // Sync with any configured bootstrap/anchor peer
                if let Ok(Some(bootstrap)) = bg_db.get_setting("bootstrap_peer") {
                    let trimmed = bootstrap.trim();
                    if !trimmed.is_empty() {
                        if let Ok(ticket) = trimmed.parse::<iroh::ticket::NodeTicket>() {
                            let _ = bg_endpoint.connect(ticket.node_addr().clone(), iroh_blobs::ALPN).await;
                            let _ = sync_catalog_with_peer(&bg_endpoint, &bg_db, ticket.node_addr().clone()).await;
                        } else if let Ok(id) = trimmed.parse::<NodeId>() {
                            let _ = bg_endpoint.connect(iroh::NodeAddr::new(id), iroh_blobs::ALPN).await;
                            let _ = sync_catalog_with_peer(&bg_endpoint, &bg_db, iroh::NodeAddr::new(id)).await;
                        }
                    }
                }

                // Discovered remotes from local LAN (mDNS) or WAN
                let remotes: Vec<_> = bg_endpoint.remote_info_iter().collect();
                for remote in remotes {
                    let addr = iroh::NodeAddr::new(remote.node_id);
                    let _ = bg_endpoint.connect(addr.clone(), iroh_blobs::ALPN).await;
                    let _ = sync_catalog_with_peer(&bg_endpoint, &bg_db, addr).await;
                }
            }
        });

        Ok(IrohEngine {
            endpoint,
            blobs,
            store,
            _router: Arc::new(running_router),
            _pool: Arc::new(pool),
        })
    }

    /// Returns the public NodeId (Ed25519 public key) of this node.
    pub fn node_id(&self) -> NodeId {
        self.endpoint.node_id()
    }

    /// Returns the number of currently connected peers.
    pub async fn peer_count(&self) -> usize {
        self.endpoint.remote_info_iter().count()
    }
}
