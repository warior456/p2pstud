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
    pub secret_key: SecretKey,
    _router: Arc<Router>,
}

impl IrohEngine {
    /// Update the active sync ALPN and re-spawn the protocol router when switching swarm networks.
    pub fn update_network_alpn(&mut self, db: Arc<Database>) {
        let new_alpn = get_sync_alpn(&db);
        self.sync_alpn = new_alpn.clone();
        let sync_handler = Arc::new(CatalogSyncHandler::new(db, Some(self.store.clone()), self.node_id().to_string()));
        let router = Router::builder(self.endpoint.clone())
            .accept(iroh_blobs::ALPN, (*self.blobs).clone())
            .accept(new_alpn, sync_handler);
        self._router = Arc::new(router.spawn());
    }
    /// Load or rebuild the local redb BlobStore safely once.
    pub async fn load_store(blob_dir: &Path) -> Result<BlobStore> {
        std::fs::create_dir_all(blob_dir)?;
        match BlobStore::load(blob_dir).await {
            Ok(s) => Ok(s),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("redb v2") || err_str.contains("migration") || err_str.contains("Database error") {
                    tracing::warn!("Incompatible legacy blob store index detected ({err_str}). Backing up legacy index and rebuilding redb store...");
                    let old_db = blob_dir.join("blobs.db");
                    if old_db.exists() {
                        let backup = blob_dir.join("blobs.db.v2.bak");
                        let _ = std::fs::rename(&old_db, &backup);
                    }
                    Ok(BlobStore::load(blob_dir).await?)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    /// Start an Iroh node using a preloaded BlobStore.
    /// Loads or creates a persistent identity and begins listening for incoming QUIC connections.
    pub async fn start(data_dir: &Path, db: Arc<Database>, store: BlobStore) -> Result<Self> {
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

        // 3. Build QUIC endpoint with relay fallback and presets.
        // Disable portmapper (UPnP / NAT-PMP) by default to prevent triggering campus IDS/firewall
        // alerts (such as unauthorized UPnP port forwards classified as BitTorrent distribution).
        // Check current network profile: on Campus networks, use AddrFilter::relay_only() so all
        // peer traffic strictly tunnels through port 443 HTTPS relay, avoiding suspicious high-port UDP probes.
        let (initial_profile, _) = crate::p2p::network_profile::resolve_network_profile(
            &db,
            crate::p2p::network_profile::get_cached_or_detect_ssid().as_deref(),
        );

        let initial_addr_filter = match initial_profile {
            crate::p2p::network_profile::NetworkProfile::Campus => {
                tracing::info!("Campus network profile active at startup: using AddrFilter::relay_only() for stealth port 443 tunneling");
                iroh::endpoint_info::AddrFilter::relay_only()
            }
            crate::p2p::network_profile::NetworkProfile::Trusted => {
                iroh::endpoint_info::AddrFilter::unfiltered()
            }
        };

        let endpoint = match Endpoint::builder(presets::N0)
            .secret_key(secret_key.clone())
            .relay_mode(relay_mode)
            .addr_filter(initial_addr_filter.clone())
            .portmapper_config(iroh::endpoint::PortmapperConfig::Disabled)
            .bind()
            .await
        {
            Ok(ep) => ep,
            Err(e) => {
                tracing::warn!("Failed to bind endpoint with configured relay: {}. Trying fallback with RelayMode::Default...", e);
                match Endpoint::builder(presets::N0)
                    .secret_key(secret_key.clone())
                    .relay_mode(iroh::RelayMode::Default)
                    .addr_filter(initial_addr_filter.clone())
                    .portmapper_config(iroh::endpoint::PortmapperConfig::Disabled)
                    .bind()
                    .await
                {
                    Ok(ep) => ep,
                    Err(e2) => {
                        tracing::warn!("Failed to bind with default relay: {}. Falling back to local-only RelayMode::Disabled...", e2);
                        Endpoint::builder(presets::N0)
                            .secret_key(secret_key.clone())
                            .relay_mode(iroh::RelayMode::Disabled)
                            .addr_filter(initial_addr_filter)
                            .portmapper_config(iroh::endpoint::PortmapperConfig::Disabled)
                            .bind()
                            .await?
                    }
                }
            }
        };

        let sync_alpn = get_sync_alpn(&db);
        let blobs = Arc::new(BlobsProtocol::new(&store, None));
        let sync_handler = Arc::new(CatalogSyncHandler::new(db.clone(), Some(store.clone()), node_id.to_string()));
        let router = Router::builder(endpoint.clone())
            .accept(iroh_blobs::ALPN, (*blobs).clone())
            .accept(sync_alpn.clone(), sync_handler);
        let running_router = router.spawn();

        // 4. Spawn background swarm maintenance loop
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
                        if let Ok(addr) = crate::commands::peer_cmds::parse_peer_address(trimmed, None) {
                            let sanitized = crate::p2p::network_profile::sanitize_endpoint_addr(
                                &bg_db,
                                addr,
                                bg_endpoint.addr().relay_urls().cloned(),
                            );
                            let _ = sync_catalog_with_peer(&bg_endpoint, &bg_db, Some(&bg_store), sanitized).await;
                        }
                    }
                }

                // Also maintain periodic catalog synchronization with all active peers
                for peer_id in crate::p2p::sync::get_active_peer_ids() {
                    if peer_id != bg_endpoint.id() {
                        let addr = EndpointAddr::from(peer_id);
                        let sanitized = crate::p2p::network_profile::sanitize_endpoint_addr(
                            &bg_db,
                            addr,
                            bg_endpoint.addr().relay_urls().cloned(),
                        );
                        let _ = sync_catalog_with_peer(&bg_endpoint, &bg_db, Some(&bg_store), sanitized).await;
                    }
                }
            }
        });

        // 5. Start automated discovery (Local LAN UDP broadcast + HTTPS Swarm Rendezvous)
        crate::p2p::lan_discovery::start_lan_discovery(endpoint.clone(), secret_key.clone(), db.clone(), store.clone());
        crate::p2p::rendezvous::start_swarm_rendezvous(endpoint.clone(), secret_key.clone(), db.clone(), store.clone());

        Ok(IrohEngine {
            endpoint,
            blobs,
            store,
            sync_alpn,
            secret_key,
            _router: Arc::new(running_router),
        })
    }

    /// Returns the public NodeId (Ed25519 public key) of this node.
    pub fn node_id(&self) -> NodeId {
        self.endpoint.id()
    }

    /// Returns the number of currently connected peers.
    pub async fn peer_count(&self) -> usize {
        crate::p2p::sync::get_active_peer_ids().len()
    }
}
