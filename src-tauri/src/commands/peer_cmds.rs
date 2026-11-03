// src-tauri/src/commands/peer_cmds.rs
// Tauri IPC commands exposing peer connectivity and swarm management.

use std::str::FromStr;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::p2p::sync::sync_catalog_with_peer;
use crate::AppState;

/// Condensed information about a connected remote peer.
#[derive(Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub node_id: String,
    pub relay_url: Option<String>,
    pub latency_ms: Option<u64>,
}

/// Return a list of currently connected peers.
#[tauri::command]
pub async fn list_peers(state: State<'_, AppState>) -> Result<Vec<PeerInfo>, String> {
    let guard = state.engine.read().await;
    let Some(engine) = guard.as_ref() else {
        return Ok(vec![]);
    };

    let peers: Vec<PeerInfo> = engine
        .endpoint
        .remote_info_iter()
        .map(|info| PeerInfo {
            node_id: info.node_id.to_string(),
            relay_url: info.relay_url.as_ref().map(|u| u.relay_url.to_string()),
            latency_ms: info.latency.map(|d| d.as_millis() as u64),
        })
        .collect();

    Ok(peers)
}

/// Return this node's public NodeId (hex-encoded Ed25519 public key).
#[tauri::command]
pub async fn get_node_id(state: State<'_, AppState>) -> Result<String, String> {
    let guard = state.engine.read().await;
    match guard.as_ref() {
        Some(engine) => Ok(engine.node_id().to_string()),
        None => Err("Iroh engine not yet started".to_string()),
    }
}

/// Return this node's full NodeTicket (string starting with "node") for easy sharing.
#[tauri::command]
pub async fn get_node_ticket(state: State<'_, AppState>) -> Result<String, String> {
    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;
    let addr = engine.endpoint.node_addr().await.map_err(|e| e.to_string())?;
    let ticket = iroh::ticket::NodeTicket::new(addr);
    Ok(ticket.to_string())
}

/// Return this node's complete NodeAddr (direct socket addresses + relay URL) as a JSON string.
#[tauri::command]
pub async fn get_node_addr(state: State<'_, AppState>) -> Result<String, String> {
    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;
    let addr = engine.endpoint.node_addr().await.map_err(|e| e.to_string())?;
    serde_json::to_string(&addr).map_err(|e| e.to_string())
}

/// Parse any user-provided peer address representation into an `iroh::NodeAddr`.
/// Supports:
/// 1. NodeTicket ("node...")
/// 2. JSON NodeAddr ({"node_id":"...","info":{...}})
/// 3. node_id@ip:port or node_id@relay_url
/// 4. Bare NodeId + optional relay_url
fn parse_peer_address(input: &str, relay_url: Option<String>) -> Result<iroh::NodeAddr, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Peer address cannot be empty".to_string());
    }

    // 1. Try parsing as NodeTicket
    if let Ok(ticket) = iroh::ticket::NodeTicket::from_str(trimmed) {
        return Ok(ticket.node_addr().clone());
    }

    // 2. Try parsing as JSON NodeAddr
    if trimmed.starts_with('{') {
        if let Ok(addr) = serde_json::from_str::<iroh::NodeAddr>(trimmed) {
            return Ok(addr);
        }
    }

    // 3. Try parsing node_id@endpoint
    if let Some((node_str, endpoint_str)) = trimmed.split_once('@') {
        let node_id = node_str
            .trim()
            .parse::<iroh::NodeId>()
            .map_err(|e| format!("Invalid Node ID in '@' format: {e}"))?;
        let mut addr = iroh::NodeAddr::new(node_id);
        let ep = endpoint_str.trim();
        if let Ok(socket) = ep.parse::<std::net::SocketAddr>() {
            addr = addr.with_direct_addresses(vec![socket]);
        } else if let Ok(relay) = ep.parse::<iroh::RelayUrl>() {
            addr = addr.with_relay_url(relay);
        }
        return Ok(addr);
    }

    // 4. Try parsing as bare NodeId
    let node_id = trimmed
        .parse::<iroh::NodeId>()
        .map_err(|e| format!("Could not parse as Node Ticket, JSON address, or Node ID: {e}"))?;

    let mut addr = iroh::NodeAddr::new(node_id);
    if let Some(r) = relay_url {
        let r_trimmed = r.trim();
        if !r_trimmed.is_empty() {
            if let Ok(relay) = r_trimmed.parse::<iroh::RelayUrl>() {
                addr = addr.with_relay_url(relay);
            }
        }
    }

    Ok(addr)
}

/// Connect to a remote peer or anchor node by Ticket, JSON address, or NodeId.
#[tauri::command]
pub async fn connect_peer(
    node_id: String,
    relay_url: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let addr = parse_peer_address(&node_id, relay_url)?;

    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;

    // Connect using Iroh Blobs ALPN
    engine
        .endpoint
        .connect(addr.clone(), iroh_blobs::ALPN)
        .await
        .map_err(|e| format!("Connection to peer failed: {e}"))?;

    // Save as bootstrap peer in settings so connection is restored on restart
    let _ = state.db.set_setting("bootstrap_peer", node_id.trim());

    // Synchronize workspace catalog immediately
    let _ = sync_catalog_with_peer(&engine.endpoint, &state.db, addr).await;

    tracing::info!(node_id = %node_id, "Connected to peer and synchronized catalog successfully");
    Ok(())
}
