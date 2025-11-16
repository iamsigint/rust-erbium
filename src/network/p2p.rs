// src/network/p2p.rs

use crate::utils::error::{BlockchainError, Result};
use futures::StreamExt;
use libp2p::{
    dns, identity, ping,
    swarm::{ConnectionDenied, NetworkBehaviour, SwarmEvent, ToSwarm},
    tcp, Multiaddr, PeerId, Swarm, Transport,
};
use std::collections::HashSet;
use tokio::sync::mpsc;

pub struct NodeBehaviour {
    pub ping: ping::Behaviour,
}

impl NetworkBehaviour for NodeBehaviour {
    type ConnectionHandler = <ping::Behaviour as NetworkBehaviour>::ConnectionHandler;
    type ToSwarm = NodeBehaviourEvent;

    fn handle_pending_inbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> std::result::Result<(), ConnectionDenied> {
        Ok(())
    }

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _peer_id: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> std::result::Result<Self::ConnectionHandler, ConnectionDenied> {
        self.ping.handle_established_inbound_connection(
            _connection_id,
            _peer_id,
            _local_addr,
            _remote_addr,
        )
    }

    fn handle_pending_outbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _maybe_peer: Option<PeerId>,
        _addresses: &[Multiaddr],
        _effective_role: libp2p::core::Endpoint,
    ) -> std::result::Result<Vec<Multiaddr>, ConnectionDenied> {
        Ok(vec![])
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _peer_id: PeerId,
        _addr: &Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> std::result::Result<Self::ConnectionHandler, ConnectionDenied> {
        self.ping.handle_established_outbound_connection(
            _connection_id,
            _peer_id,
            _addr,
            _role_override,
        )
    }

    fn on_swarm_event(&mut self, event: libp2p::swarm::FromSwarm) {
        self.ping.on_swarm_event(event);
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: libp2p::swarm::ConnectionId,
        event: <Self::ConnectionHandler as libp2p::swarm::ConnectionHandler>::ToBehaviour,
    ) {
        self.ping
            .on_connection_handler_event(peer_id, connection_id, event);
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        ToSwarm<
            Self::ToSwarm,
            <Self::ConnectionHandler as libp2p::swarm::ConnectionHandler>::FromBehaviour,
        >,
    > {
        // Delegate to inner ping behaviour; map event type if needed
        self.ping
            .poll(cx)
            .map(|poll| poll.map_out(NodeBehaviourEvent::Ping))
    }
}

#[derive(Debug)]
pub enum NodeBehaviourEvent {
    Ping(ping::Event),
}

impl From<ping::Event> for NodeBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        NodeBehaviourEvent::Ping(event)
    }
}

pub struct P2PNode {
    swarm: Option<Swarm<NodeBehaviour>>,
    connected_peers: HashSet<PeerId>,
    message_receiver: mpsc::UnboundedReceiver<Vec<u8>>,
}

impl P2PNode {
    pub async fn new() -> Result<Self> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        log::info!("Local peer id: {}", local_peer_id);

        // Create transport with TCP and DNS
        let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default());
        let dns_transport = dns::tokio::Transport::system(tcp_transport).map_err(|e| {
            BlockchainError::Network(format!("DNS transport creation failed: {}", e))
        })?;

        let transport = dns_transport
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(
                libp2p::noise::Config::new(&local_key)
                    .map_err(|e| BlockchainError::Network(format!("Noise config failed: {}", e)))?,
            )
            .multiplex(libp2p::yamux::Config::default())
            .boxed();

        let ping_behaviour = ping::Behaviour::new(ping::Config::new());
        let behaviour = NodeBehaviour {
            ping: ping_behaviour,
        };

        let swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            libp2p::swarm::Config::with_tokio_executor(),
        );

        let (_message_sender, message_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            swarm: Some(swarm),
            connected_peers: HashSet::new(),
            message_receiver,
        })
    }

    pub async fn start_listening(&mut self, addr: Multiaddr) -> Result<()> {
        if let Some(swarm) = &mut self.swarm {
            match swarm.listen_on(addr.clone()) {
                Ok(_) => {
                    log::info!("Started listening on {}", addr);
                    Ok(())
                }
                Err(e) => Err(BlockchainError::Network(format!(
                    "Failed to listen on address: {}",
                    e
                ))),
            }
        } else {
            Err(BlockchainError::Network(
                "Swarm not initialized".to_string(),
            ))
        }
    }

    pub async fn dial_peer(&mut self, peer_addr: Multiaddr) -> Result<()> {
        if let Some(swarm) = &mut self.swarm {
            match swarm.dial(peer_addr.clone()) {
                Ok(_) => {
                    log::info!("Dialing peer at {}", peer_addr);
                    Ok(())
                }
                Err(e) => Err(BlockchainError::Network(format!(
                    "Failed to dial peer: {}",
                    e
                ))),
            }
        } else {
            Err(BlockchainError::Network(
                "Swarm not initialized".to_string(),
            ))
        }
    }

    pub async fn broadcast_block(&mut self, _block_data: Vec<u8>) -> Result<()> {
        log::info!("Broadcasting block to {} peers", self.connected_peers.len());
        Ok(())
    }

    pub async fn broadcast_transaction(&mut self, _tx_data: Vec<u8>) -> Result<()> {
        log::info!(
            "Broadcasting transaction to {} peers",
            self.connected_peers.len()
        );
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut swarm = match self.swarm.take() {
            Some(swarm) => swarm,
            None => {
                return Err(BlockchainError::Network(
                    "Swarm not initialized".to_string(),
                ))
            }
        };

        loop {
            tokio::select! {
                event = swarm.select_next_some() => {
                    self.handle_swarm_event(event).await?;
                }
                Some(message) = self.message_receiver.recv() => {
                    self.handle_custom_message(message).await?;
                }
                else => {
                    log::info!("Message receiver channel closed.");
                    break;
                }
            }
        }

        self.swarm = Some(swarm);
        Ok(())
    }

    async fn handle_swarm_event(
        &mut self,
        event: SwarmEvent<<NodeBehaviour as NetworkBehaviour>::ToSwarm>,
    ) -> Result<()> {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                log::info!("Listening on {}", address);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                log::info!("Connected to {}", peer_id);
                self.connected_peers.insert(peer_id);
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                log::info!("Disconnected from {}", peer_id);
                self.connected_peers.remove(&peer_id);
            }
            _ => {
                // Ignore other events for now
            }
        }
        Ok(())
    }

    async fn handle_custom_message(&mut self, _message: Vec<u8>) -> Result<()> {
        log::debug!("Handling custom internal message");
        Ok(())
    }

    pub fn get_connected_peers_count(&self) -> usize {
        self.connected_peers.len()
    }

    pub fn is_initialized(&self) -> bool {
        self.swarm.is_some()
    }
}
