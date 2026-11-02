// src-tauri/src/commands/peer_cmds.rs
// Tauri IPC commands exposing peer connectivity information.

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
/// The frontend displays this in Settings so users can share their node address.
#[tauri::command]
pub async fn get_node_id(state: State<'_, AppState>) -> Result<String, String> {
    let guard = state.engine.read().await;
    match guard.as_ref() {
        Some(engine) => Ok(engine.node_id().to_string()),
        None => Err("Iroh engine not yet started".to_string()),
    }
}
