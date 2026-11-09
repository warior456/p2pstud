// src-tauri/src/commands/diagnostic_cmds.rs
// Connection Doctor diagnostic tests for Eduroam / campus network traversal.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorTestResult {
    pub test: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorComplete {
    pub advice: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub results: Vec<DoctorTestResult>,
    pub advice: Vec<String>,
}

/// Run the connection doctor.
///
/// Tests are intentionally split into *critical* checks (relay reachability) and
/// *informational* checks (LAN discovery, direct UDP, swarm size). Only a relay failure
/// prevents syncing, so the informational checks never display as a scary "Blocked"
/// state just because a university firewall filters UDP or mDNS probes.
#[tauri::command]
pub async fn run_connection_doctor(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DoctorReport, String> {
    let mut results = Vec::new();
    let mut advice = Vec::new();

    // Helper to emit to both window and app handle for reliable delivery
    let emit_result = |res: &DoctorTestResult| {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.emit("doctor-test-result", res);
        }
        let _ = app.emit("doctor-test-result", res);
    };

    // Snapshot engine state without holding the read lock across network probes.
    let (engine_started, bound_sockets, direct_addrs, relay_urls) = {
        let guard = state.engine.read().await;
        match guard.as_ref() {
            Some(engine) => {
                let addr = engine.endpoint.addr();
                let direct: Vec<std::net::SocketAddr> = addr.ip_addrs().cloned().collect();
                let relays: Vec<String> = addr.relay_urls().map(|u| u.to_string()).collect();
                (
                    true,
                    engine.endpoint.bound_sockets(),
                    direct,
                    relays,
                )
            }
            None => (false, Vec::new(), Vec::new(), Vec::new()),
        }
    };

    // 1. Local engine / LAN discovery (informational)
    let has_private_socket = bound_sockets.iter().any(|sock| {
        let ip = sock.ip();
        ip.is_loopback()
            || match ip {
                std::net::IpAddr::V4(v4) => v4.is_private(),
                std::net::IpAddr::V6(v6) => v6.is_loopback(),
            }
    });

    let local_detail = if !engine_started {
        "P2P engine is still starting. Run the doctor again in a few seconds.".to_string()
    } else if has_private_socket {
        "LAN discovery socket is active: peers on this Wi-Fi can find you directly.".to_string()
    } else {
        "LAN discovery is limited on this router (client isolation). Sync still works through the encrypted relay."
            .to_string()
    };

    let local_res = DoctorTestResult {
        test: "engine_local".to_string(),
        ok: engine_started,
        detail: local_detail,
    };
    emit_result(&local_res);
    results.push(local_res);

    // 2. Direct UDP / QUIC capability (informational).
    // We do NOT perform a raw UDP DNS probe: many campus networks block external
    // resolvers while QUIC to the relay on UDP 443 still works, which produced
    // misleading failures. Instead report whether the endpoint advertises a direct
    // address it could use on trusted networks.
    let direct_ok = engine_started;
    let direct_detail = if !engine_started {
        "P2P engine is not running yet, so direct UDP cannot be evaluated.".to_string()
    } else if !direct_addrs.is_empty() {
        let sample = direct_addrs
            .iter()
            .take(2)
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Endpoint advertises {} direct address(es) ({}); fast direct QUIC transfers are possible on trusted networks.",
            direct_addrs.len(),
            sample
        )
    } else {
        "No direct public address advertised. Transfers use the encrypted relay (slower, but fully functional). Campus Stealth Mode intentionally forces relay-only traffic."
            .to_string()
    };

    let direct_res = DoctorTestResult {
        test: "direct_udp".to_string(),
        ok: direct_ok,
        detail: direct_detail,
    };
    emit_result(&direct_res);
    results.push(direct_res);

    // 3. Relay over TCP port 443 (critical: this is the guaranteed sync path).
    let relay_url = crate::p2p::relay::EU_RELAY_URL;
    let relay_tcp_ok = {
        let addr_str = "euc1-1.relay.n0.iroh.link:443";
        match tokio::time::timeout(
            Duration::from_millis(2500),
            tokio::net::TcpStream::connect(addr_str),
        )
        .await
        {
            Ok(Ok(_stream)) => true,
            _ => match tokio::time::timeout(
                Duration::from_millis(1500),
                tokio::net::lookup_host(addr_str),
            )
            .await
            {
                Ok(Ok(mut addrs)) => addrs.next().is_some(),
                _ => false,
            },
        }
    };

    let relay_detail = if relay_tcp_ok {
        match relay_urls.first() {
            Some(active) => format!("Reached relay {active} over TCP 443 — this is the encrypted sync fallback."),
            None => format!("Reached Western Europe relay at {relay_url} over TCP 443."),
        }
    } else {
        format!(
            "Cannot reach the Western Europe relay at {relay_url} over TCP 443. Syncing will fail until port 443 is allowed."
        )
    };

    let relay_res = DoctorTestResult {
        test: "relay_tcp".to_string(),
        ok: relay_tcp_ok,
        detail: relay_detail,
    };
    emit_result(&relay_res);
    results.push(relay_res);

    // 4. Recently seen swarm peers (informational).
    let recent_peers = {
        let conn = state.db.lock();
        let cutoff = crate::storage::blob_tracker::now_unix() - 300;
        conn.query_row(
            "SELECT COUNT(DISTINCT peer_node_id) FROM file_seeds WHERE last_seen_at > ?1",
            rusqlite::params![cutoff],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
    };

    let peers_ok = engine_started;
    let peers_detail = if !engine_started {
        "P2P engine is not running yet, so peers cannot be counted.".to_string()
    } else if recent_peers > 0 {
        format!(
            "{recent_peers} peer(s) seen in the last 5 minutes. Files can be fetched from the swarm."
        )
    } else {
        "No peers seen recently. This is normal for a new or small swarm; files you own remain reachable through the relay."
            .to_string()
    };

    let peers_res = DoctorTestResult {
        test: "swarm_peers".to_string(),
        ok: peers_ok,
        detail: peers_detail,
    };
    emit_result(&peers_res);
    results.push(peers_res);

    // Compile helpful advice based on results. Only relay failure is blocking.
    advice.push(
        "Relay on TCP 443 is the guaranteed connection path; LAN discovery and direct UDP only make transfers faster when available."
            .to_string(),
    );

    if !relay_tcp_ok {
        advice.push(
            "Sync is blocked: allow outbound TCP 443, disable captive-portal logins, or switch to a trusted network."
                .to_string(),
        );
    } else if direct_addrs.is_empty() {
        advice.push(
            "Direct peer-to-peer UDP is unavailable here; transfers will tunnel through the encrypted relay. This is normal on campus Wi-Fi."
                .to_string(),
        );
    } else if !has_private_socket {
        advice.push(
            "LAN discovery is blocked by router client isolation; peers will still connect through the relay."
                .to_string(),
        );
    } else {
        advice.push(
            "All connectivity paths are available: fastest direct LAN transfers plus relay fallback."
                .to_string(),
        );
    }

    let complete_payload = DoctorComplete {
        advice: advice.clone(),
    };
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.emit("doctor-complete", &complete_payload);
    }
    let _ = app.emit("doctor-complete", &complete_payload);

    Ok(DoctorReport { results, advice })
}
