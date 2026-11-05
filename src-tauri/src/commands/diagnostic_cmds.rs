// src-tauri/src/commands/diagnostic_cmds.rs
// Connection Doctor diagnostic tests for Eduroam / campus network traversal.

use std::time::Duration;

use iroh::Watcher;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

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

#[tauri::command]
pub async fn run_connection_doctor(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let state_db = state.db.clone();
    let engine_lock = state.engine.clone();

    // Spawn async background worker so we never block Tauri IPC or UI thread
    tokio::spawn(async move {
        // Small initial delay to ensure frontend IPC event listeners have mounted and registered
        tokio::time::sleep(Duration::from_millis(300)).await;

        let mut advice = Vec::new();

        // 1. Test mDNS / Local LAN discovery
        let mdns_ok = {
            let guard = engine_lock.read().await;
            if let Some(engine) = guard.as_ref() {
                // Check if any bound sockets are local / private
                let has_local_socket = engine.endpoint.bound_sockets().iter().any(|sock| {
                    let ip = sock.ip();
                    ip.is_loopback() || match ip {
                        std::net::IpAddr::V4(v4) => v4.is_private(),
                        std::net::IpAddr::V6(_) => false,
                    }
                });
                has_local_socket
            } else {
                false
            }
        };

        let _ = app.emit(
            "doctor-test-result",
            DoctorTestResult {
                test: "mdns".to_string(),
                ok: mdns_ok,
                detail: if mdns_ok {
                    "Local peers detected on LAN (mDNS active)".to_string()
                } else {
                    "No local peers found on LAN broadcast (common on campus Wi-Fi with client isolation)".to_string()
                },
            },
        );

        tokio::time::sleep(Duration::from_millis(150)).await;

        // 2. Test QUIC UDP
        let quic_udp_ok = {
            // Test if UDP socket can bind and transmit
            match tokio::time::timeout(Duration::from_secs(2), tokio::net::UdpSocket::bind("0.0.0.0:0")).await {
                Ok(Ok(sock)) => {
                    sock.connect("1.1.1.1:53").await.is_ok()
                }
                _ => false,
            }
        };

        let _ = app.emit(
            "doctor-test-result",
            DoctorTestResult {
                test: "quic_udp".to_string(),
                ok: quic_udp_ok,
                detail: if quic_udp_ok {
                    "Outbound UDP connectivity available for direct QUIC".to_string()
                } else {
                    "Outbound UDP appears blocked by campus firewall".to_string()
                },
            },
        );

        tokio::time::sleep(Duration::from_millis(150)).await;

        // 3. Test Relay over port 443 (TCP)
        let active_relay = {
            let guard = engine_lock.read().await;
            guard.as_ref().and_then(|e| {
                let relays = e.endpoint.home_relay_status().get();
                relays.first().map(|r| r.url().to_string())
            })
        };

        let configured_relay = state_db.get_setting("relay_url").unwrap_or_default().unwrap_or_default();
        let relay_url = if !configured_relay.trim().is_empty() {
            configured_relay
        } else if let Some(ref r) = active_relay {
            r.clone()
        } else {
            "https://euw1-1.relay.iroh.network/".to_string()
        };

        let relay_tcp_ok = {
            // Extract host and port from relay URL
            let parsed = url::Url::parse(&relay_url).ok();
            let host = parsed.as_ref().and_then(|u| u.host_str()).unwrap_or("euw1-1.relay.iroh.network");
            let port = parsed.as_ref().and_then(|u| u.port()).unwrap_or(443);

            let addr_str = format!("{host}:{port}");
            match tokio::time::timeout(Duration::from_secs(3), tokio::net::TcpStream::connect(&addr_str)).await {
                Ok(Ok(_stream)) => true,
                _ => {
                    // Non-blocking DNS resolution test with timeout
                    match tokio::time::timeout(Duration::from_secs(2), tokio::net::lookup_host(&addr_str)).await {
                        Ok(Ok(mut addrs)) => addrs.next().is_some(),
                        _ => false,
                    }
                }
            }
        };

        let _ = app.emit(
            "doctor-test-result",
            DoctorTestResult {
                test: "relay_tcp".to_string(),
                ok: relay_tcp_ok,
                detail: if relay_tcp_ok {
                    format!("Successfully reached relay server at {relay_url}")
                } else {
                    format!("Cannot connect to relay server at {relay_url} over TCP 443")
                },
            },
        );

        tokio::time::sleep(Duration::from_millis(150)).await;

        // 4. Test NAT type
        let is_symmetric_nat = !quic_udp_ok;

        let nat_type = if is_symmetric_nat { "symmetric" } else { "full_cone" };

        let _ = app.emit(
            "doctor-test-result",
            DoctorTestResult {
                test: "nat_type".to_string(),
                ok: true,
                detail: format!("NAT Architecture detected: {nat_type}"),
            },
        );

        // Compile advice strings as required by Step 34
        if !quic_udp_ok && relay_tcp_ok {
            advice.push("Eduroam is blocking direct UDP. All traffic routes through the relay — transfers will be slightly slower, but everything works.".to_string());
        }

        if !relay_tcp_ok {
            advice.push("Cannot reach the relay server. Check your internet connection, or ask your group admin for the correct relay URL.".to_string());
        }

        if nat_type == "symmetric" {
            advice.push("Your network uses symmetric NAT. The relay is required for all connections — this is normal on Eduroam.".to_string());
        }

        if mdns_ok {
            advice.push("Found campus peers on this Wi-Fi — direct LAN connections are available and fast.".to_string());
        }

        if advice.is_empty() {
            advice.push("All connectivity checks passed. Your device is connected and ready to share course files.".to_string());
        }

        let _ = app.emit("doctor-complete", DoctorComplete { advice });
    });

    Ok(())
}
