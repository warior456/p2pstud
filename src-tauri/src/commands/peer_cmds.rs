// src-tauri/src/commands/peer_cmds.rs
// Tauri IPC commands exposing peer connectivity and swarm management.

use serde::{Deserialize, Serialize};
use tauri::State;

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

/// Return this node's complete NodeAddr (direct socket addresses + relay URL) as a JSON string.
#[tauri::command]
pub async fn get_node_addr(state: State<'_, AppState>) -> Result<String, String> {
    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;
    let addr = engine.endpoint.node_addr().await.map_err(|e| e.to_string())?;
    serde_json::to_string(&addr).map_err(|e| e.to_string())
}

/// Connect to a remote peer or anchor node by NodeId and optional RelayUrl.
#[tauri::command]
pub async fn connect_peer(
    node_id: String,
    relay_url: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let parsed_id = node_id
        .trim()
        .parse::<iroh::NodeId>()
        .map_err(|e| format!("Invalid Node ID: {e}"))?;

    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;

    let mut addr = iroh::NodeAddr::new(parsed_id);
    if let Some(r) = relay_url {
        let trimmed = r.trim();
        if !trimmed.is_empty() {
            if let Ok(relay) = trimmed.parse::<iroh::RelayUrl>() {
                addr = addr.with_relay_url(relay);
            }
        }
    }

    // Connect using Iroh Blobs ALPN
    engine
        .endpoint
        .connect(addr, iroh_blobs::ALPN)
        .await
        .map_err(|e| format!("Connection to peer failed: {e}"))?;

    tracing::info!(node_id = %node_id, "Connected to peer successfully");
    Ok(())
}
