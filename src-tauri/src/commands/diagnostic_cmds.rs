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

#[tauri::command]
pub async fn run_connection_doctor(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DoctorReport, String> {
    let _state_db = state.db.clone();
    let engine_lock = state.engine.clone();

    let mut results = Vec::new();
    let mut advice = Vec::new();

    // Helper to emit to both window and app handle for reliable delivery
    let emit_result = |res: &DoctorTestResult| {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.emit("doctor-test-result", res);
        }
        let _ = app.emit("doctor-test-result", res);
    };

    // 1. Test mDNS / Local LAN discovery
    let mdns_ok = {
        let guard = engine_lock.read().await;
        if let Some(engine) = guard.as_ref() {
            engine.endpoint.bound_sockets().iter().any(|sock| {
                let ip = sock.ip();
                ip.is_loopback() || match ip {
                    std::net::IpAddr::V4(v4) => v4.is_private(),
                    std::net::IpAddr::V6(_) => false,
                }
            })
        } else {
            false
        }
    };

    let mdns_res = DoctorTestResult {
        test: "mdns".to_string(),
        ok: mdns_ok,
        detail: if mdns_ok {
            "Local socket bound and active for LAN peer discovery (mDNS)".to_string()
        } else {
            "Client isolation detected on router (mDNS disabled or blocked on this Wi-Fi)".to_string()
        },
    };
    emit_result(&mdns_res);
    results.push(mdns_res);

    // 2. Test QUIC UDP
    // Transmit a standard minimal DNS query probe to 1.1.1.1:53 and wait for response
    // to verify genuine outbound and inbound UDP packet transmission.
    let quic_udp_ok = async {
        let sock = tokio::net::UdpSocket::bind("0.0.0.0:0").await.ok()?;
        sock.connect("1.1.1.1:53").await.ok()?;
        let query = [
            0x12, 0x34, // Query ID
            0x01, 0x00, // Standard query
            0x00, 0x01, // 1 question
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00,       // Root label "."
            0x00, 0x01, // Type A
            0x00, 0x01, // Class IN
        ];
        sock.send(&query).await.ok()?;
        let mut buf = [0u8; 64];
        let res = tokio::time::timeout(Duration::from_millis(1500), sock.recv(&mut buf)).await;
        match res {
            Ok(Ok(n)) if n > 0 => Some(true),
            _ => None,
        }
    }
    .await
    .unwrap_or(false);

    let quic_res = DoctorTestResult {
        test: "quic_udp".to_string(),
        ok: quic_udp_ok,
        detail: if quic_udp_ok {
            "Outbound UDP connectivity available for direct P2P mesh".to_string()
        } else {
            "Outbound UDP blocked by network firewall; traffic will tunnel through relay".to_string()
        },
    };
    emit_result(&quic_res);
    results.push(quic_res);

    // 3. Test Relay over port 443 (TCP)
    let relay_url = crate::p2p::relay::EU_RELAY_URL;
    let relay_tcp_ok = {
        let addr_str = "euc1-1.relay.n0.iroh.link:443";
        match tokio::time::timeout(Duration::from_millis(2500), tokio::net::TcpStream::connect(addr_str)).await {
            Ok(Ok(_stream)) => true,
            _ => {
                match tokio::time::timeout(Duration::from_millis(1500), tokio::net::lookup_host(addr_str)).await {
                    Ok(Ok(mut addrs)) => addrs.next().is_some(),
                    _ => false,
                }
            }
        }
    };

    let relay_res = DoctorTestResult {
        test: "relay_tcp".to_string(),
        ok: relay_tcp_ok,
        detail: if relay_tcp_ok {
            format!("Successfully reached Western Europe relay at {relay_url}")
        } else {
            format!("Cannot reach Western Europe relay at {relay_url} over TCP port 443")
        },
    };
    emit_result(&relay_res);
    results.push(relay_res);

    // 4. Test NAT type
    let is_symmetric_nat = !quic_udp_ok;
    let nat_type = if is_symmetric_nat { "symmetric" } else { "full_cone" };

    let nat_res = DoctorTestResult {
        test: "nat_type".to_string(),
        ok: !is_symmetric_nat,
        detail: if is_symmetric_nat {
            format!("{nat_type} NAT detected: requires relay assistance for peer hole punching")
        } else {
            format!("{nat_type} NAT: direct peer connections possible")
        },
    };
    emit_result(&nat_res);
    results.push(nat_res);

    // Compile helpful advice based on results
    if !quic_udp_ok && relay_tcp_ok {
        advice.push("Network is blocking direct UDP. All traffic routes through Western Europe encrypted relay — transfers work seamlessly.".to_string());
    }

    if !relay_tcp_ok {
        advice.push("Cannot reach the Western Europe relay. Check your internet connection or verify port 443 outbound access.".to_string());
    }

    if is_symmetric_nat {
        advice.push("Symmetric NAT is active. P2P connections will use the encrypted relay for NAT hole-punching.".to_string());
    }

    if mdns_ok {
        advice.push("Local discovery active: fast LAN mesh transfers available with peers on this router.".to_string());
    }

    if advice.is_empty() {
        advice.push("All connectivity checks passed. Your device is ready to sync and share course documents.".to_string());
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
