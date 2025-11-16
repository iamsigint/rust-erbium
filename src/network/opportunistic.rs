use crate::network::dht::{Contact, NodeId, DHT};
use crate::network::discovery::PeerDiscovery;
use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{watch, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};

const DISCOVERY_MAGIC: &str = "ERBIUM_OPPORTUNISTIC_V1";

#[derive(Clone)]
pub struct OpportunisticDiscoveryConfig {
    pub port: u16,
    pub interval: Duration,
    pub max_packet_size: usize,
}

impl Default for OpportunisticDiscoveryConfig {
    fn default() -> Self {
        Self {
            port: 30_303,
            interval: Duration::from_secs(5),
            max_packet_size: 2048,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoveryPacket {
    magic: String,
    node_id: String,
    address: String,
    timestamp: u64,
}

pub struct OpportunisticDiscovery;

impl OpportunisticDiscovery {
    pub async fn spawn(
        config: OpportunisticDiscoveryConfig,
        advertised_addr: SocketAddr,
        node_id: NodeId,
        dht: Arc<RwLock<DHT>>,
        peer_discovery: Arc<RwLock<PeerDiscovery>>,
        shutdown: watch::Receiver<bool>,
    ) -> Result<Vec<JoinHandle<()>>> {
        let listen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.port);
        let listen_socket = UdpSocket::bind(listen_addr).await.map_err(|e| {
            BlockchainError::Network(format!(
                "Failed to bind opportunistic discovery listener on {}: {}",
                listen_addr, e
            ))
        })?;

        // Allow receiving broadcast packets on some platforms
        if let Err(e) = listen_socket.set_broadcast(true) {
            log::warn!(
                "Failed to enable broadcast reception for opportunistic discovery: {}",
                e
            );
        }

        let listen_socket = Arc::new(listen_socket);

        let send_socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0))
            .await
            .map_err(|e| {
                BlockchainError::Network(format!(
                    "Failed to bind opportunistic discovery sender socket: {}",
                    e
                ))
            })?;
        send_socket.set_broadcast(true).map_err(|e| {
            BlockchainError::Network(format!(
                "Failed to enable UDP broadcast for opportunistic discovery: {}",
                e
            ))
        })?;
        let send_socket = Arc::new(send_socket);

        let broadcast_target = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), config.port);
        let packet_template = DiscoveryPacket {
            magic: DISCOVERY_MAGIC.to_string(),
            node_id: node_id.to_hex(),
            address: advertised_addr.to_string(),
            timestamp: current_timestamp(),
        };

        // Broadcast task
        let sender_socket = Arc::clone(&send_socket);
        let mut send_shutdown = shutdown.clone();
        let mut broadcast_packet = packet_template.clone();
        broadcast_packet.timestamp = current_timestamp();
        let broadcast_task = tokio::spawn(async move {
            let mut interval = time::interval(config.interval);
            loop {
                tokio::select! {
                    changed = send_shutdown.changed() => {
                        if changed.is_ok() && *send_shutdown.borrow() {
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        broadcast_packet.timestamp = current_timestamp();
                        match serde_json::to_vec(&broadcast_packet) {
                            Ok(payload) => {
                                if let Err(e) = sender_socket.send_to(&payload, broadcast_target).await {
                                    log::debug!(
                                        "Failed to send opportunistic discovery packet to {}: {}",
                                        broadcast_target,
                                        e
                                    );
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to serialize opportunistic discovery packet: {}", e);
                            }
                        }
                    }
                }
            }
            log::debug!("Opportunistic discovery broadcast task stopped");
        });

        // Listener task
        let listener_socket = Arc::clone(&listen_socket);
        let mut recv_shutdown = shutdown;
        let listener_dht = Arc::clone(&dht);
        let listener_discovery = Arc::clone(&peer_discovery);
        let listener_task = tokio::spawn(async move {
            let mut buffer = vec![0u8; config.max_packet_size];

            loop {
                tokio::select! {
                    changed = recv_shutdown.changed() => {
                        if changed.is_ok() && *recv_shutdown.borrow() {
                            break;
                        }
                    }
                    recv_result = listener_socket.recv_from(&mut buffer) => {
                        match recv_result {
                            Ok((len, source)) => {
                                if len == 0 {
                                    continue;
                                }

                                let payload = &buffer[..len];
                                match serde_json::from_slice::<DiscoveryPacket>(payload) {
                                    Ok(packet) => {
                                        if packet.magic != DISCOVERY_MAGIC {
                                            continue;
                                        }

                                        if packet.node_id == node_id.to_hex() {
                                            continue;
                                        }

                                        match NodeId::from_hex(&packet.node_id) {
                                            Ok(remote_node_id) => {
                                                let advertised_socket = match packet.address.parse::<SocketAddr>() {
                                                    Ok(addr) => addr,
                                                    Err(_) => source,
                                                };

                                                if let Err(e) = register_contact(
                                                    listener_dht.clone(),
                                                    listener_discovery.clone(),
                                                    remote_node_id,
                                                    advertised_socket,
                                                ).await {
                                                    log::debug!(
                                                        "Failed to register opportunistic peer {} at {}: {}",
                                                        packet.node_id,
                                                        advertised_socket,
                                                        e
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                log::debug!(
                                                    "Invalid node id {} received from {}: {}",
                                                    packet.node_id,
                                                    source,
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::debug!(
                                            "Failed to decode opportunistic discovery payload from {}: {}",
                                            source,
                                            e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                log::debug!("Opportunistic discovery listener error: {}", e);
                                time::sleep(Duration::from_secs(1)).await;
                            }
                        }
                    }
                }
            }

            log::debug!("Opportunistic discovery listener task stopped");
        });

        Ok(vec![broadcast_task, listener_task])
    }
}

async fn register_contact(
    dht: Arc<RwLock<DHT>>,
    peer_discovery: Arc<RwLock<PeerDiscovery>>,
    remote_node_id: NodeId,
    advertised_socket: SocketAddr,
) -> Result<()> {
    {
        let mut discovery_guard = peer_discovery.write().await;
        if !discovery_guard.add_opportunistic_peer(advertised_socket) {
            // Already known
            return Ok(());
        }
    }

    let contact = Contact::new(remote_node_id, advertised_socket);
    let mut dht_guard = dht.write().await;

    // It's fine if the contact already exists; insert_contact will update recency
    if let Err(e) = dht_guard.add_bootstrap_node(contact.clone()) {
        if !matches!(e, BlockchainError::Network(_)) {
            log::debug!(
                "Failed to add bootstrap node from opportunistic discovery: {}",
                e
            );
        }
    }

    dht_guard.insert_contact(contact);
    log::info!(
        "Opportunistic discovery found peer {} at {}",
        remote_node_id.to_hex(),
        advertised_socket
    );

    Ok(())
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
