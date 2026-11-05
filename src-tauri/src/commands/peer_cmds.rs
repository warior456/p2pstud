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

    let my_node_id = engine.node_id().to_string();
    let peer_ids: Vec<String> = {
        let conn = state.db.lock();
        let mut stmt = conn
            .prepare("SELECT DISTINCT peer_node_id FROM file_seeds WHERE peer_node_id != ?1 AND last_seen_at >= ?2")
            .map_err(|e| e.to_string())?;
        let thirty_mins_ago = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64 - 1800;
        let rows = stmt
            .query_map(rusqlite::params![&my_node_id, thirty_mins_ago], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let mut peers = Vec::new();
    for peer_id_str in peer_ids {
        if let Ok(endpoint_id) = peer_id_str.parse::<iroh::EndpointId>() {
            let remote_info = engine.endpoint.remote_info(endpoint_id).await;
            let relay_url = remote_info.as_ref().and_then(|info| {
                info.addrs().find_map(|a| match a.addr() {
                    iroh::TransportAddr::Relay(url) => Some(url.to_string()),
                    _ => None,
                })
            });
            peers.push(PeerInfo {
                node_id: peer_id_str,
                relay_url,
                latency_ms: None,
            });
        }
    }

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
    let addr = engine.endpoint.addr();
    let ticket = iroh_tickets::endpoint::EndpointTicket::new(addr);
    Ok(ticket.to_string())
}

/// Return this node's complete NodeAddr (direct socket addresses + relay URL) as a JSON string.
#[tauri::command]
pub async fn get_node_addr(state: State<'_, AppState>) -> Result<String, String> {
    let guard = state.engine.read().await;
    let engine = guard.as_ref().ok_or("Iroh engine not yet started")?;
    let addr = engine.endpoint.addr();
    serde_json::to_string(&addr).map_err(|e| e.to_string())
}

/// Parse any user-provided peer address representation into an `iroh::EndpointAddr`.
/// Supports:
/// 1. NodeTicket / EndpointTicket
/// 2. JSON EndpointAddr
/// 3. node_id@ip:port or node_id@relay_url
/// 4. Bare NodeId + optional relay_url
fn parse_peer_address(input: &str, relay_url: Option<String>) -> Result<iroh::EndpointAddr, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Peer address cannot be empty".to_string());
    }

    // 1. Try parsing as EndpointTicket
    if let Ok(ticket) = iroh_tickets::endpoint::EndpointTicket::from_str(trimmed) {
        return Ok(ticket.into());
    }

    // 2. Try parsing as JSON EndpointAddr
    if trimmed.starts_with('{') {
        if let Ok(addr) = serde_json::from_str::<iroh::EndpointAddr>(trimmed) {
            return Ok(addr);
        }
    }

    // 3. Try parsing node_id@endpoint
    if let Some((node_str, endpoint_str)) = trimmed.split_once('@') {
        let node_id = node_str
            .trim()
            .parse::<iroh::EndpointId>()
            .map_err(|e| format!("Invalid Node ID in '@' format: {e}"))?;
        let mut addr = iroh::EndpointAddr::from(node_id);
        let ep = endpoint_str.trim();
        if let Ok(socket) = ep.parse::<std::net::SocketAddr>() {
            addr = addr.with_ip_addr(socket);
        } else if let Ok(relay) = ep.parse::<iroh::RelayUrl>() {
            addr = addr.with_relay_url(relay);
        }
        return Ok(addr);
    }

    // 4. Try parsing as bare NodeId
    let node_id = trimmed
        .parse::<iroh::EndpointId>()
        .map_err(|e| format!("Could not parse as Node Ticket, JSON address, or Node ID: {e}"))?;

    let mut addr = iroh::EndpointAddr::from(node_id);
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
    let _ = sync_catalog_with_peer(&engine.endpoint, &state.db, Some(&engine.store), addr).await;

    tracing::info!(node_id = %node_id, "Connected to peer and synchronized catalog successfully");
    Ok(())
}

/// Retrieve current network profile, detected SSID, and active bandwidth throttle state.
#[tauri::command]
pub async fn get_network_profile(
    state: State<'_, AppState>,
) -> Result<crate::p2p::network_profile::NetworkProfileInfo, String> {
    let ssid = crate::p2p::network_profile::get_cached_or_detect_ssid();
    let (profile, override_active) =
        crate::p2p::network_profile::resolve_network_profile(&state.db, ssid.as_deref());
    let is_lecture = crate::p2p::network_profile::is_current_lecture_hours();

    let cap = match profile {
        crate::p2p::network_profile::NetworkProfile::Campus => {
            if is_lecture {
                crate::p2p::network_profile::MAX_LECTURE_BACKGROUND_BPS
            } else {
                crate::p2p::network_profile::MAX_CAMPUS_BACKGROUND_BPS
            }
        }
        crate::p2p::network_profile::NetworkProfile::Trusted => u64::MAX,
    };

    let is_known = ssid
        .as_ref()
        .map(|s| crate::p2p::network_profile::is_ssid_classified(&state.db, s))
        .unwrap_or(true)
        || override_active.is_some();

    Ok(crate::p2p::network_profile::NetworkProfileInfo {
        profile: profile.as_str().to_string(),
        ssid,
        lecture_throttle_active: is_lecture
            && profile == crate::p2p::network_profile::NetworkProfile::Campus,
        override_active,
        background_cap_bps: cap,
        is_known_network: is_known,
    })
}

/// Manually force a profile override ('auto', 'campus', or 'trusted').
#[tauri::command]
pub async fn set_network_profile_override(
    profile: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let p = profile.trim().to_lowercase();
    if !["auto", "campus", "trusted"].contains(&p.as_str()) {
        return Err("Override must be 'auto', 'campus', or 'trusted'".to_string());
    }
    state
        .db
        .set_setting("network_profile_override", &p)
        .map_err(|e| e.to_string())?;
    crate::p2p::network_profile::invalidate_ssid_cache();
    Ok(())
}

/// Add or update an SSID classification rule ('campus' or 'trusted').
#[tauri::command]
pub async fn add_ssid_rule(
    ssid: String,
    profile: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::p2p::network_profile::save_ssid_rule(&state.db, &ssid, &profile).map_err(|e| e.to_string())?;
    crate::p2p::network_profile::invalidate_ssid_cache();
    Ok(())
}

/// List all user-configured and default SSID classification rules.
#[tauri::command]
pub async fn list_ssid_rules(
    state: State<'_, AppState>,
) -> Result<Vec<crate::p2p::network_profile::SsidRule>, String> {
    crate::p2p::network_profile::list_all_ssid_rules(&state.db).map_err(|e| e.to_string())
}

/// Delete an SSID classification rule.
#[tauri::command]
pub async fn delete_ssid_rule(ssid: String, state: State<'_, AppState>) -> Result<(), String> {
    crate::p2p::network_profile::delete_ssid_rule_db(&state.db, &ssid).map_err(|e| e.to_string())?;
    crate::p2p::network_profile::invalidate_ssid_cache();
    Ok(())
}

/// Return all files for which this node is the sole known seed in the swarm.
#[tauri::command]
pub async fn get_sole_seed_files(
    state: State<'_, AppState>,
) -> Result<Vec<crate::storage::blob_tracker::SoleSeedFile>, String> {
    let guard = state.engine.read().await;
    let our_node_id = match guard.as_ref() {
        Some(e) => e.node_id().to_string(),
        None => return Ok(vec![]),
    };
    crate::storage::blob_tracker::get_sole_seed_files(&state.db, &our_node_id)
        .map_err(|e| e.to_string())
}

/// Forcibly trigger an immediate swarm reconnection cycle.
#[tauri::command]
pub async fn reconnect_swarm(state: State<'_, AppState>) -> Result<(), String> {
    let guard = state.engine.read().await;
    let Some(engine) = guard.as_ref() else {
        return Err("Iroh engine not started".to_string());
    };
    if let Ok(Some(bootstrap)) = state.db.get_setting("bootstrap_peer") {
        let trimmed = bootstrap.trim();
        if !trimmed.is_empty() {
            if let Ok(addr) = parse_peer_address(trimmed, None) {
                let _ = engine
                    .endpoint
                    .connect(addr.clone(), iroh_blobs::ALPN)
                    .await;
                let _ = sync_catalog_with_peer(
                    &engine.endpoint,
                    &state.db,
                    Some(&engine.store),
                    addr,
                )
                .await;
            }
        }
    }
    Ok(())
}

/// Retrieve the current local version and calculated median application version across the active swarm.
#[tauri::command]
pub async fn get_swarm_version_info(
    state: State<'_, AppState>,
) -> Result<crate::storage::blob_tracker::SwarmVersionInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION");
    Ok(crate::storage::blob_tracker::calculate_swarm_median_version(&state.db, current_version))
}
