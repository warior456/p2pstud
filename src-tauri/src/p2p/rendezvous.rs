// src-tauri/src/p2p/rendezvous.rs
// Automated P2P Swarm Discovery via HTTPS Rendezvous over port 443.
// Enables zero-configuration peer discovery across restrictive campus networks
// (Eduroam, university firewalls, coworking Wi-Fi with client isolation) where
// local mDNS broadcast and raw BitTorrent UDP/DHT are blocked.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use iroh::{Endpoint, EndpointAddr, EndpointId};
use serde::{Deserialize, Serialize};

use crate::p2p::sync::sync_catalog_with_peer;
use crate::storage::db::Database;

/// Payload broadcasted by active peers on the shared swarm topic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RendezvousAnnouncement {
    pub node_id: String,
    pub ticket: String,
    pub timestamp: u64,
}

/// Derive a deterministic, opaque rendezvous topic string from the swarm passphrase.
///
/// If no secret is configured (default open network), derives a consistent open topic.
/// If a private passphrase is set, derives a unique BLAKE3-hashed topic known only
/// to students who possess that exact passphrase.
pub fn derive_rendezvous_topic(swarm_secret: &str) -> String {
    let clean = swarm_secret.trim();
    let input = if clean.is_empty() {
        "p2pstud-open-swarm-topic-v1".to_string()
    } else {
        format!("p2pstud-private-swarm-v1:{clean}")
    };

    let hash = iroh_blobs::Hash::new(input.as_bytes());
    let hex = hash.to_hex();
    // Valid topic name for HTTPS rendezvous: alphanumeric with prefix
    format!("p2pstud-{}", &hex[..24])
}

/// Spawn the automated swarm rendezvous background task.
/// Periodically publishes our presence and fetches other active peers on the topic.
pub fn start_swarm_rendezvous(
    endpoint: Endpoint,
    db: Arc<Database>,
    store: iroh_blobs::store::fs::FsStore,
) {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to initialize rendezvous HTTP client: {e}");
                return;
            }
        };

        let mut interval = tokio::time::interval(Duration::from_secs(20));
        loop {
            interval.tick().await;

            if let Err(e) = run_rendezvous_cycle(&client, &endpoint, &db, &store).await {
                tracing::debug!("Swarm rendezvous cycle notice: {e}");
            }
        }
    });
}

/// Execute a single rendezvous cycle: publish presence and dial discovered peers.
pub async fn run_rendezvous_cycle(
    client: &reqwest::Client,
    endpoint: &Endpoint,
    db: &Arc<Database>,
    store: &iroh_blobs::store::fs::FsStore,
) -> Result<()> {
    // 1. Determine active rendezvous server URL (default: https://ntfy.sh)
    let rendezvous_base = db
        .get_setting("rendezvous_url")
        .unwrap_or_default()
        .unwrap_or_else(|| "https://ntfy.sh".to_string());
    let base_url = rendezvous_base.trim().trim_end_matches('/');
    if base_url.is_empty() {
        return Ok(());
    }

    // 2. Derive topic name from current swarm secret
    let secret = db
        .get_setting("network_secret")
        .unwrap_or_default()
        .unwrap_or_default();
    let topic = derive_rendezvous_topic(&secret);

    // 3. Prepare our announcement
    let our_node_id = endpoint.id().to_string();
    let our_addr = endpoint.addr();
    let our_ticket = iroh_tickets::endpoint::EndpointTicket::new(our_addr).to_string();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let announcement = RendezvousAnnouncement {
        node_id: our_node_id.clone(),
        ticket: our_ticket,
        timestamp: now,
    };

    let payload = serde_json::to_string(&announcement)?;

    // 4. Publish our presence to the topic
    let publish_url = format!("{base_url}/{topic}");
    let _ = client
        .post(&publish_url)
        .header("Title", "p2pstud-peer")
        .body(payload)
        .send()
        .await;

    // 5. Poll for other active peers on the topic (within last 3 minutes)
    let poll_url = format!("{base_url}/{topic}/json?since=3m&poll=1");
    let resp = match client.get(&poll_url).send().await {
        Ok(r) if r.status().is_success() => r,
        _ => return Ok(()),
    };

    let text = resp.text().await.unwrap_or_default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Each line from ntfy is a JSON object with { event, message, time, ... }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            if val.get("event").and_then(|v| v.as_str()) != Some("message") {
                continue;
            }
            if let Some(msg_str) = val.get("message").and_then(|v| v.as_str()) {
                if let Ok(peer) = serde_json::from_str::<RendezvousAnnouncement>(msg_str) {
                    if peer.node_id != our_node_id && peer.timestamp + 300 >= now {
                        connect_and_sync_discovered_peer(endpoint, db, store, &peer.ticket, &peer.node_id).await;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Connect to a discovered peer by Ticket or NodeId and synchronize workspace catalog.
async fn connect_and_sync_discovered_peer(
    endpoint: &Endpoint,
    db: &Arc<Database>,
    store: &iroh_blobs::store::fs::FsStore,
    ticket_str: &str,
    node_id_str: &str,
) {
    let addr = if let Ok(ticket) = ticket_str.parse::<iroh_tickets::endpoint::EndpointTicket>() {
        EndpointAddr::from(ticket)
    } else if let Ok(id) = node_id_str.parse::<EndpointId>() {
        EndpointAddr::from(id)
    } else {
        return;
    };

    // Connect using Iroh Blobs ALPN
    if let Ok(_conn) = endpoint.connect(addr.clone(), iroh_blobs::ALPN).await {
        tracing::info!(peer = %node_id_str, "Successfully auto-connected to peer via swarm rendezvous!");
        let _ = sync_catalog_with_peer(endpoint, db, Some(store), addr).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_rendezvous_topic_open_swarm() {
        let topic1 = derive_rendezvous_topic("");
        let topic2 = derive_rendezvous_topic("   ");
        assert_eq!(topic1, topic2);
        assert!(topic1.starts_with("p2pstud-"));
        assert_eq!(topic1.len(), 32);
    }

    #[test]
    fn test_derive_rendezvous_topic_private_swarm() {
        let topic_a = derive_rendezvous_topic("course-group-2026");
        let topic_b = derive_rendezvous_topic("different-group-2026");
        let topic_a_repeat = derive_rendezvous_topic("course-group-2026");

        assert_ne!(topic_a, topic_b);
        assert_eq!(topic_a, topic_a_repeat);
        assert_ne!(topic_a, derive_rendezvous_topic(""));
    }

    #[test]
    fn test_announcement_serialization() {
        let ann = RendezvousAnnouncement {
            node_id: "testnode123".to_string(),
            ticket: "nodeticketxyz".to_string(),
            timestamp: 1793929935,
        };

        let json = serde_json::to_string(&ann).expect("must serialize");
        let parsed: RendezvousAnnouncement = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(ann, parsed);
    }
}
