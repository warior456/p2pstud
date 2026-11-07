// src-tauri/src/p2p/lan_discovery.rs
// Local Area Network (LAN) & loopback peer discovery via UDP broadcast.
// Discovers swarm peers on the same local network within seconds, completely
// offline without any external server dependency.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use iroh::{Endpoint, SecretKey};
use socket2::{Domain, Protocol, Socket, Type};

use crate::p2p::network_profile::{
    get_cached_or_detect_ssid, get_sanitized_our_addr, resolve_network_profile, NetworkProfile,
};
use crate::p2p::rendezvous::{
    connect_and_sync_discovered_peer, deobfuscate_ticket, get_active_peer_count,
    obfuscate_ticket, RendezvousAnnouncement,
};
use crate::storage::db::Database;

pub const LAN_DISCOVERY_PORT: u16 = 27888;

/// Create a reusable broadcast UDP socket.
fn create_discovery_socket(port: u16) -> Option<tokio::net::UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).ok()?;
    let _ = socket.set_reuse_address(true);
    let _ = socket.set_broadcast(true);
    let _ = socket.set_nonblocking(true);

    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port);
    if socket.bind(&bind_addr.into()).is_ok() {
        let std_socket: std::net::UdpSocket = socket.into();
        tokio::net::UdpSocket::from_std(std_socket).ok()
    } else {
        // Fallback: bind to an ephemeral port to allow outgoing broadcast even if port is occupied
        let fallback_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0);
        let _ = socket.bind(&fallback_addr.into());
        let std_socket: std::net::UdpSocket = socket.into();
        tokio::net::UdpSocket::from_std(std_socket).ok()
    }
}

/// Start local LAN & loopback peer discovery in the background.
pub fn start_lan_discovery(
    endpoint: Endpoint,
    secret_key: SecretKey,
    db: Arc<Database>,
    store: iroh_blobs::store::fs::FsStore,
) {
    let recv_socket = match create_discovery_socket(LAN_DISCOVERY_PORT) {
        Some(s) => Arc::new(s),
        None => {
            tracing::warn!("Failed to bind LAN discovery socket on port {}", LAN_DISCOVERY_PORT);
            return;
        }
    };

    let send_socket = recv_socket.clone();
    let our_node_id = endpoint.id().to_string();

    // 1. Broadcast Task (announces our presence to 255.255.255.255 and 127.0.0.1)
    let ep_send = endpoint.clone();
    let db_send = db.clone();
    let sk_send = secret_key.clone();
    let node_id_broadcast = our_node_id.clone();

    tokio::spawn(async move {
        let broadcast_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::BROADCAST, LAN_DISCOVERY_PORT));
        let loopback_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, LAN_DISCOVERY_PORT));

        loop {
            let (profile, _) = resolve_network_profile(&db_send, get_cached_or_detect_ssid().as_deref());
            let is_campus = profile == NetworkProfile::Campus;

            let peer_count = get_active_peer_count(&db_send, &node_id_broadcast);
            let next_delay = if is_campus {
                Duration::from_secs(15)
            } else if peer_count == 0 {
                Duration::from_secs(3)
            } else {
                Duration::from_secs(12)
            };

            let secret = db_send
                .get_setting("network_secret")
                .unwrap_or_default()
                .unwrap_or_default();

            let our_addr = get_sanitized_our_addr(&ep_send, &db_send);
            let our_ticket = iroh_tickets::endpoint::EndpointTicket::new(our_addr).to_string();
            let masked_ticket = obfuscate_ticket(&our_ticket, &secret);
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let announcement = RendezvousAnnouncement::sign(
                node_id_broadcast.clone(),
                masked_ticket,
                now,
                &sk_send,
            );

            if let Ok(payload) = serde_json::to_vec(&announcement) {
                // If on Campus/Eduroam, NEVER broadcast to 255.255.255.255!
                // Sending subnet-wide broadcast on port 27888 triggers campus WIDS/WIPS IDS alerts.
                // Only send to 127.0.0.1 on loopback for multi-instance testing on the same host.
                if !is_campus {
                    let _ = send_socket.send_to(&payload, broadcast_addr).await;
                }
                let _ = send_socket.send_to(&payload, loopback_addr).await;
            }

            tokio::time::sleep(next_delay).await;
        }
    });

    // 2. Receiver Task (listens for peer broadcasts and auto-connects)
    let ep_recv = endpoint.clone();
    let db_recv = db.clone();
    let store_recv = store.clone();

    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match recv_socket.recv_from(&mut buf).await {
                Ok((len, from_addr)) => {
                    let (profile, _) = resolve_network_profile(&db_recv, get_cached_or_detect_ssid().as_deref());
                    // On campus networks, ignore non-loopback broadcast packets to avoid suspicious peer mesh dialing
                    if profile == NetworkProfile::Campus && !from_addr.ip().is_loopback() {
                        continue;
                    }

                    if let Ok(peer) = serde_json::from_slice::<RendezvousAnnouncement>(&buf[..len]) {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        if peer.node_id != our_node_id && peer.timestamp + 300 >= now {
                            if !peer.verify() {
                                continue;
                            }

                            let secret = db_recv
                                .get_setting("network_secret")
                                .unwrap_or_default()
                                .unwrap_or_default();

                            let ticket_str = deobfuscate_ticket(&peer.ticket, &secret)
                                .unwrap_or(peer.ticket);

                            connect_and_sync_discovered_peer(
                                &ep_recv,
                                &db_recv,
                                &store_recv,
                                &ticket_str,
                                &peer.node_id,
                            )
                            .await;
                        }
                    }
                }
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
    });
}
