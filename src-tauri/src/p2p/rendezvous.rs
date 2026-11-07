// src-tauri/src/p2p/rendezvous.rs
// Automated P2P Swarm Discovery via HTTPS Rendezvous over port 443.
// Enables zero-configuration peer discovery across restrictive campus networks
// (Eduroam, university firewalls, coworking Wi-Fi with client isolation) where
// local mDNS broadcast and raw BitTorrent UDP/DHT are blocked.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use iroh::{Endpoint, EndpointAddr, EndpointId, SecretKey};
use serde::{Deserialize, Serialize};

use crate::p2p::sync::sync_catalog_with_peer;
use crate::storage::db::Database;

/// Payload broadcasted by active peers on the shared swarm topic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RendezvousAnnouncement {
    pub node_id: String,
    pub ticket: String,
    pub timestamp: u64,
    #[serde(default)]
    pub signature: String,
}

impl RendezvousAnnouncement {
    /// Create and cryptographically sign a rendezvous announcement.
    pub fn sign(node_id: String, ticket: String, timestamp: u64, secret_key: &SecretKey) -> Self {
        let payload = format!("{node_id}:{ticket}:{timestamp}");
        let sig = secret_key.sign(payload.as_bytes());
        let signature = hex::encode(sig.to_bytes());
        Self {
            node_id,
            ticket,
            timestamp,
            signature,
        }
    }

    /// Verify the Ed25519 signature of the announcing node (VULN-04).
    pub fn verify(&self) -> bool {
        let Ok(pk) = self.node_id.parse::<iroh::PublicKey>() else { return false };
        let payload = format!("{}:{}:{}", self.node_id, self.ticket, self.timestamp);
        let Ok(sig_bytes) = hex::decode(&self.signature) else { return false };
        if sig_bytes.len() != 64 { return false; }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&sig_bytes);
        let sig = iroh::Signature::from_bytes(&arr);
        pk.verify(payload.as_bytes(), &sig).is_ok()
    }
}

/// Obfuscate ticket payload using a stream derived from the network secret (VULN-09)
pub fn obfuscate_ticket(ticket: &str, secret: &str) -> String {
    let clean = secret.trim();
    let key_phrase = if clean.is_empty() {
        "p2pstud-open-ticket-v1".to_string()
    } else {
        format!("p2pstud-envelope-v1:{clean}")
    };
    let key = iroh_blobs::Hash::new(key_phrase.as_bytes());
    let key_bytes = key.as_bytes();

    let bytes = ticket.as_bytes();
    let masked: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key_bytes[i % key_bytes.len()])
        .collect();
    hex::encode(masked)
}

/// Deobfuscate ticket payload
pub fn deobfuscate_ticket(hex_str: &str, secret: &str) -> Option<String> {
    let Ok(masked) = hex::decode(hex_str) else { return None };
    let clean = secret.trim();
    let key_phrase = if clean.is_empty() {
        "p2pstud-open-ticket-v1".to_string()
    } else {
        format!("p2pstud-envelope-v1:{clean}")
    };
    let key = iroh_blobs::Hash::new(key_phrase.as_bytes());
    let key_bytes = key.as_bytes();

    let bytes: Vec<u8> = masked
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key_bytes[i % key_bytes.len()])
        .collect();
    String::from_utf8(bytes).ok()
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

/// Count active peers seen in the active network recently.
pub fn get_active_peer_count(db: &Database, our_node_id: &str) -> usize {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let cutoff = now - 900; // last 15 minutes
    let conn = db.lock();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT peer_id) FROM (
                SELECT peer_node_id AS peer_id, last_seen_at FROM file_seeds
                UNION
                SELECT node_id AS peer_id, last_seen_at FROM peer_versions
            ) WHERE peer_id != ?1 AND last_seen_at >= ?2",
            rusqlite::params![our_node_id, cutoff],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let in_memory = crate::p2p::sync::get_active_peer_ids()
        .iter()
        .filter(|id| id.to_string() != our_node_id)
        .count();
    (count.max(0) as usize).max(in_memory)
}

/// Spawn the automated swarm rendezvous background task.
/// Periodically publishes our presence and fetches other active peers on the topic.
/// Automatically adapts polling speed: fast (every 15s) when searching for initial peers,
/// and relaxed (every 60s) once other swarm nodes are discovered.
pub fn start_swarm_rendezvous(
    endpoint: Endpoint,
    secret_key: SecretKey,
    db: Arc<Database>,
    store: iroh_blobs::store::fs::FsStore,
) {
    tokio::spawn(async move {
        // Wait briefly (up to 3s) for the endpoint to establish its home relay connection
        let _ = tokio::time::timeout(Duration::from_secs(3), endpoint.online()).await;

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

        let our_node_id = endpoint.id().to_string();

        loop {
            // Adaptive interval: 15s when searching (0 peers), 60s when peers are connected
            let peer_count = get_active_peer_count(&db, &our_node_id);
            let next_delay = if peer_count == 0 {
                Duration::from_secs(15)
            } else {
                Duration::from_secs(60)
            };

            if let Err(e) = run_rendezvous_cycle(&client, &endpoint, &secret_key, &db, &store).await {
                tracing::debug!("Swarm rendezvous cycle notice: {e}");
            }

            tokio::time::sleep(next_delay).await;
        }
    });
}

/// Execute a single rendezvous cycle: publish presence and dial discovered peers.
pub async fn run_rendezvous_cycle(
    client: &reqwest::Client,
    endpoint: &Endpoint,
    secret_key: &SecretKey,
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

    // 3. Prepare our signed & ticket-obfuscated announcement (VULN-04 & VULN-09)
    let our_node_id = endpoint.id().to_string();
    let mut our_addr = endpoint.addr();
    if our_addr.relay_urls().next().is_none() {
        if let Ok(relay) = crate::p2p::relay::EU_RELAY_URL.parse::<iroh::RelayUrl>() {
            our_addr = our_addr.with_relay_url(relay);
        }
    }
    let our_ticket = iroh_tickets::endpoint::EndpointTicket::new(our_addr).to_string();
    let masked_ticket = obfuscate_ticket(&our_ticket, &secret);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let announcement = RendezvousAnnouncement::sign(our_node_id.clone(), masked_ticket, now, secret_key);

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
    let mut dials_attempted = 0usize;
    const MAX_DIALS_PER_CYCLE: usize = 6; // Rate limiter prevents dial storms (VULN-04)

    for line in text.lines() {
        if dials_attempted >= MAX_DIALS_PER_CYCLE {
            break;
        }
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
                        // VULN-04: Cryptographic verification of announcement signature
                        if !peer.verify() {
                            tracing::warn!(peer = %peer.node_id, "Rejected rendezvous announcement with invalid signature");
                            continue;
                        }

                        // VULN-09: Deobfuscate ticket using network secret
                        let ticket_str = deobfuscate_ticket(&peer.ticket, &secret)
                            .unwrap_or(peer.ticket);

                        connect_and_sync_discovered_peer(endpoint, db, store, &ticket_str, &peer.node_id).await;
                        dials_attempted += 1;
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
    let mut addr = if let Ok(ticket) = ticket_str.parse::<iroh_tickets::endpoint::EndpointTicket>() {
        EndpointAddr::from(ticket)
    } else if let Ok(id) = node_id_str.parse::<EndpointId>() {
        EndpointAddr::from(id)
    } else {
        return;
    };

    // If addr has no relay URLs, augment with local endpoint's relay URLs or standard EU relay
    if addr.relay_urls().next().is_none() {
        for relay in endpoint.addr().relay_urls() {
            addr = addr.with_relay_url(relay.clone());
        }
        if addr.relay_urls().next().is_none() {
            if let Ok(relay) = crate::p2p::relay::EU_RELAY_URL.parse::<iroh::RelayUrl>() {
                addr = addr.with_relay_url(relay);
            }
        }
    }

    // Connect using swarm sync ALPN directly
    if let Ok(()) = sync_catalog_with_peer(endpoint, db, Some(store), addr.clone()).await {
        tracing::info!(peer = %node_id_str, "Successfully auto-connected to peer via swarm rendezvous!");
        // Warm up blobs connection in background for fast file transfers
        let bg_ep = endpoint.clone();
        tokio::spawn(async move {
            let _ = bg_ep.connect(addr, iroh_blobs::ALPN).await;
        });
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
        let key = SecretKey::generate();
        let ann = RendezvousAnnouncement::sign(
            key.public().to_string(),
            "nodeticketxyz".to_string(),
            1793929935,
            &key,
        );
        assert!(ann.verify());

        let json = serde_json::to_string(&ann).expect("must serialize");
        let parsed: RendezvousAnnouncement = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(ann, parsed);
        assert!(parsed.verify());
    }

    #[test]
    fn test_ticket_obfuscation_roundtrip() {
        let secret = "my-secret-passphrase";
        let original_ticket = "endpoint-ticket-content-12345";
        let masked = obfuscate_ticket(original_ticket, secret);
        assert_ne!(masked, original_ticket);

        let restored = deobfuscate_ticket(&masked, secret).expect("must restore");
        assert_eq!(restored, original_ticket);
    }

    #[test]
    fn test_get_active_peer_count_includes_peer_versions() {
        use tempfile::TempDir;
        let dir = TempDir::new().unwrap();
        let db = Database::open(&dir.path().join("test_count.db")).unwrap();
        let our_id = "our-node-id-123";
        let peer_id = "peer-node-id-456";

        // Initially 0 peers
        assert_eq!(get_active_peer_count(&db, our_id), 0);

        // Record peer version (even when no files are seeded yet)
        crate::storage::blob_tracker::record_peer_version(&db, peer_id, "0.1.30").unwrap();

        // Now must count 1 active peer
        assert_eq!(get_active_peer_count(&db, our_id), 1);
    }
}
