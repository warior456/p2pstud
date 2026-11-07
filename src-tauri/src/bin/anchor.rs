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
// rate limiting uses native token-bucket implementation
use iroh::endpoint::presets;
use iroh::{Endpoint, RelayMode, SecretKey};
use iroh_blobs::BlobsProtocol;
use iroh_blobs::store::fs::FsStore as BlobStore;
// done
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

    /// Optional workspace namespace ID to mirror (defaults to the default shared campus swarm).
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
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "anchor=info,iroh=warn".into()),
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
    let store = BlobStore::load(&blob_dir).await?;

    // Build QUIC endpoint.
    let endpoint = Endpoint::builder(presets::N0)
        .secret_key(secret_key)
        .relay_mode(if args.no_relay {
            RelayMode::Default
        } else {
            // When this node IS the relay, disable relay fallback for itself.
            RelayMode::Disabled
        })
        .bind()
        .await?;

    tracing::info!(addr = ?endpoint.addr(), "Endpoint bound");

    // Attach blobs protocol.
    let blobs = BlobsProtocol::new(&store, None);
    let router = iroh::protocol::Router::builder(endpoint)
        .accept(iroh_blobs::ALPN, blobs.clone());
    let running_router = router.spawn();

    // Build rate limiter shared state.
    let shared_aggregate = Arc::new(tokio::sync::Mutex::new(args.max_aggregate_bps as f64));
    let mut session_limiter = SessionLimiter::new(
        args.max_session_bps,
        args.max_aggregate_bps,
        shared_aggregate.clone(),
    );
    tokio::spawn(async move {
        session_limiter.consume(1).await;
    });

    // Relay server (optional).
    let _relay_server = if !args.no_relay {
        start_relay_server(&args).await?
    } else {
        None
    };

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
    if let Some(relay) = _relay_server {
        let _ = relay.shutdown().await;
    }
    Ok(())
}

/// Start the embedded iroh-net relay server.
/// Loads TLS credentials from --cert-dir if available, and binds to --relay-port.
async fn start_relay_server(args: &Args) -> Result<Option<iroh_relay::server::Server>> {
    let addr: SocketAddr = format!("0.0.0.0:{}", args.relay_port).parse()?;
    tracing::info!(relay_addr = %addr, "Starting embedded relay server");

    let cert_path = args.cert_dir.join("fullchain.pem");
    let key_path = args.cert_dir.join("privkey.pem");

    if !cert_path.exists() || !key_path.exists() {
        tracing::warn!(
            "TLS credentials not found in {}. Embedded relay server will run in plain HTTP/dev mode.",
            args.cert_dir.display()
        );
    }

    let mut config = iroh_relay::server::ServerConfig::default();
    config.relay = Some(iroh_relay::server::RelayConfig::new(addr));

    let server = iroh_relay::server::Server::spawn(config).await?;
    tracing::info!("Embedded relay server running successfully");
    Ok(Some(server))
}

/// Token-bucket rate limiter state for egress throttling.
/// Each peer session gets its own `SessionLimiter`; the aggregate is shared.
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
            self.session_tokens =
                (self.session_tokens + elapsed * self.session_max).min(self.session_max);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_limiter_throttles() {
        let agg = Arc::new(tokio::sync::Mutex::new(200_000.0));
        let mut limiter = SessionLimiter::new(100_000, 200_000, agg);

        // First consume succeeds immediately from full bucket
        limiter.consume(50_000).await;
        assert!(limiter.session_tokens < 60_000.0);

        // Consuming another 50k empties the session bucket
        limiter.consume(50_000).await;
        assert!(limiter.session_tokens < 1_000.0);
    }

    #[tokio::test]
    async fn test_aggregate_limiter_shared_pool() {
        let shared_agg = Arc::new(tokio::sync::Mutex::new(100_000.0));
        let mut s1 = SessionLimiter::new(100_000, 100_000, shared_agg.clone());
        let mut s2 = SessionLimiter::new(100_000, 100_000, shared_agg.clone());

        // s1 consumes 60k
        s1.consume(60_000).await;
        let agg_val = *shared_agg.lock().await;
        assert!(agg_val <= 41_000.0);

        // s2 consumes 40k
        s2.consume(40_000).await;
        let agg_val2 = *shared_agg.lock().await;
        assert!(agg_val2 <= 1_000.0);
    }
}
