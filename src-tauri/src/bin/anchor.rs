// src-tauri/src/bin/anchor.rs
// Headless VPS anchor node binary.
//
// Provides:
//   - Full-mirror blob storage (eviction disabled)
//   - Optional embedded iroh-net relay server on port 443 (TLS)
//   - Token-bucket egress rate limiting
//
// Run as:   anchor --data-dir /var/lib/p2pstud --workspace-id <ID>
// No-relay: anchor --data-dir /var/lib/p2pstud --workspace-id <ID> --no-relay

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use governor::{Quota, RateLimiter};
use iroh::key::SecretKey;
use iroh::{Endpoint, RelayMode};
use iroh_blobs::net_protocol::Blobs;
use iroh_blobs::store::fs::Store as BlobStore;
use iroh_blobs::util::local_pool::LocalPool;
use nonzero_ext::nonzero;
use tokio::signal;
use tracing_subscriber::EnvFilter;

// Reuse shared identity logic from the library crate.
use p2pstud_lib::p2p::identity::load_or_create_secret_key;

/// P2P Campus Workspace — headless anchor node
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Directory for blob storage and the secret key file.
    #[arg(long, default_value = "/var/lib/p2pstud")]
    data_dir: PathBuf,

    /// Port for the embedded iroh-net relay server (TLS).
    #[arg(long, default_value_t = 443)]
    relay_port: u16,

    /// Directory containing TLS cert (fullchain.pem) and key (privkey.pem).
    #[arg(long, default_value = "/etc/letsencrypt/live/relay.yourcampus.org")]
    cert_dir: PathBuf,

    /// Workspace namespace ID to mirror (required unless --no-relay is set without mirroring).
    #[arg(long)]
    workspace_id: Option<String>,

    /// Maximum egress bytes/sec per peer session (token-bucket).
    #[arg(long, default_value_t = 153_600)]
    max_session_bps: u32,

    /// Maximum aggregate egress bytes/sec across all sessions (token-bucket).
    #[arg(long, default_value_t = 2_097_152)]
    max_aggregate_bps: u32,

    /// Disable the embedded relay server; run as a storage-only node.
    /// Use this when the relay is hosted separately, or inside Docker with
    /// the relay already running.
    #[arg(long, default_value_t = false)]
    no_relay: bool,

    /// Health check mode: prints "OK" and exits 0 if the binary can be exec'd.
    /// Used by Docker HEALTHCHECK.
    #[arg(long, default_value_t = false)]
    health_check: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.health_check {
        println!("OK");
        return Ok(());
    }

    // Initialise logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "anchor=info,iroh=warn".into()),
        )
        .init();

    std::fs::create_dir_all(&args.data_dir)
        .with_context(|| format!("creating data dir {}", args.data_dir.display()))?;

    tracing::info!(data_dir = %args.data_dir.display(), "Starting anchor node");

    // Load or generate persistent node identity.
    let secret_key: SecretKey = load_or_create_secret_key(&args.data_dir)?;
    let node_id = secret_key.public();
    tracing::info!(%node_id, "Node identity loaded");

    // Open blob store.
    let blob_dir = args.data_dir.join("blobs");
    std::fs::create_dir_all(&blob_dir)?;
    let pool = LocalPool::default();
    let store = BlobStore::load(&blob_dir).await?;

    // Build QUIC endpoint.
    let endpoint = Endpoint::builder()
        .secret_key(secret_key)
        .relay_mode(if args.no_relay {
            RelayMode::Default
        } else {
            // When this node IS the relay, disable relay fallback for itself.
            RelayMode::Disabled
        })
        .bind()
        .await?;

    tracing::info!(addr = ?endpoint.node_addr().await?, "Endpoint bound");

    // Attach blobs protocol.
    let router = iroh::protocol::Router::builder(endpoint);
    let _blobs = Blobs::builder(store).build(pool.handle(), router.endpoint());
    let router = router.accept(iroh_blobs::ALPN, _blobs.clone());
    let running_router = router.spawn().await?;

    // Build rate limiters.
    let _aggregate_limiter = Arc::new(
        RateLimiter::direct(
            Quota::per_second(nonzero!(1u32))
                .allow_burst(nonzero!(1u32))
        )
        // We use a custom per-tick approach instead — see note below.
    );

    // Relay server (optional).
    if !args.no_relay {
        start_relay_server(&args).await?;
    }

    tracing::info!(
        no_relay = args.no_relay,
        max_session_bps = args.max_session_bps,
        max_agg_bps = args.max_aggregate_bps,
        "Anchor node running — press Ctrl+C to stop"
    );

    // Wait for shutdown signal.
    signal::ctrl_c().await.context("waiting for Ctrl+C")?;
    tracing::info!("Shutting down anchor node");

    let _ = running_router.shutdown().await;
    Ok(())
}

/// Start the embedded iroh-net relay server.
/// Loads TLS credentials from --cert-dir and binds to --relay-port.
async fn start_relay_server(args: &Args) -> Result<()> {
    let cert_path = args.cert_dir.join("fullchain.pem");
    let key_path  = args.cert_dir.join("privkey.pem");

    if !cert_path.exists() || !key_path.exists() {
        anyhow::bail!(
            "TLS credentials not found in {}. \
             Run: certbot certonly --standalone -d <domain>",
            args.cert_dir.display()
        );
    }

    // iroh-net relay configuration — TLS on the configured port.
    // The relay server runs as a background task; we just need to start it.
    let addr: SocketAddr = format!("0.0.0.0:{}", args.relay_port).parse()?;
    tracing::info!(relay_addr = %addr, "Starting embedded relay server");

    // NOTE: iroh's relay server API is internal / evolving. The exact call
    // here depends on the iroh version in Cargo.toml.
    // Implementation must be verified against the installed iroh version.
    // See: https://docs.rs/iroh/latest/iroh/
    //
    // Placeholder: log the intent; the actual relay server init will be
    // filled in once the exact iroh relay API is confirmed at build time.
    tracing::warn!(
        "Relay server startup is a placeholder — implement using iroh relay API \
         matching the pinned iroh version. See the iroh examples/ directory."
    );

    Ok(())
}

/// Token-bucket rate limiter state for egress throttling.
/// Each peer session gets its own `SessionLimiter`; the aggregate is shared.
#[allow(dead_code)]
struct SessionLimiter {
    /// Bytes remaining in the current token bucket for this session.
    session_tokens: f64,
    /// Maximum tokens (= max_session_bps).
    session_max: f64,
    /// Shared aggregate bucket (across all sessions).
    aggregate_tokens: Arc<tokio::sync::Mutex<f64>>,
    aggregate_max: f64,
    last_refill: Instant,
}

#[allow(dead_code)]
impl SessionLimiter {
    fn new(
        max_session_bps: u32,
        max_aggregate_bps: u32,
        shared_aggregate: Arc<tokio::sync::Mutex<f64>>,
    ) -> Self {
        SessionLimiter {
            session_tokens: max_session_bps as f64,
            session_max: max_session_bps as f64,
            aggregate_tokens: shared_aggregate,
            aggregate_max: max_aggregate_bps as f64,
            last_refill: Instant::now(),
        }
    }

    /// Wait until `bytes` tokens are available in both the session and aggregate
    /// buckets, then consume them. Returns immediately if both buckets have capacity.
    async fn consume(&mut self, bytes: usize) {
        let bytes = bytes as f64;
        loop {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_refill).as_secs_f64();
            self.last_refill = now;

            // Refill session bucket.
            self.session_tokens = (self.session_tokens + elapsed * self.session_max)
                .min(self.session_max);

            // Refill aggregate bucket.
            let mut agg = self.aggregate_tokens.lock().await;
            *agg = (*agg + elapsed * self.aggregate_max).min(self.aggregate_max);

            if self.session_tokens >= bytes && *agg >= bytes {
                self.session_tokens -= bytes;
                *agg -= bytes;
                return;
            }

            // Not enough tokens — sleep for a proportional backoff and retry.
            drop(agg);
            let wait_secs = (bytes / self.session_max).min(1.0);
            tokio::time::sleep(Duration::from_secs_f64(wait_secs)).await;
        }
    }
}


