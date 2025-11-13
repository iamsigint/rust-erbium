// src/network/discovery.rs

use libp2p::{Multiaddr, PeerId};
use crate::utils::error::{Result, BlockchainError};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Information about a peer on the network
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer address
    pub addr: Multiaddr,
    /// Peer ID
    pub peer_id: PeerId,
    /// Last time the peer was seen
    pub last_seen: Instant,
    /// Protocol version
    pub protocol_version: String,
    /// Height of the most recent block
    pub block_height: u64,
}

/// Peer discovery manager
pub struct PeerDiscovery {
    /// List of known peers
    peers: HashMap<PeerId, PeerInfo>,
    /// Bootstrap peer list
    bootstrap_peers: Vec<Multiaddr>,
    /// Banned peers
    banned_peers: HashSet<PeerId>,
    /// Maximum time without contact to consider a peer inactive
    inactive_threshold: Duration,
}

impl PeerDiscovery {
    /// Create a new discovery manager
    pub fn new(bootstrap_peers: Vec<Multiaddr>) -> Self {
        Self {
            peers: HashMap::new(),
            bootstrap_peers,
            banned_peers: HashSet::new(),
            inactive_threshold: Duration::from_secs(3600), // 1 hour
        }
    }

    /// Adds a new peer to the list of known peers
    pub fn add_peer(&mut self, peer_id: PeerId, addr: Multiaddr, block_height: u64) -> Result<()> {
        if self.banned_peers.contains(&peer_id) {
            return Err(BlockchainError::Network(format!("Peer {} está banido", peer_id)));
        }

        self.peers.insert(peer_id, PeerInfo {
            addr,
            peer_id,
            last_seen: Instant::now(),
            protocol_version: "1.0.0".to_string(), // Standard version
            block_height,
        });

        log::debug!("Adicionado peer: {}", peer_id);
        Ok(())
    }

    /// Updates the information for an existing peer
    pub fn update_peer(&mut self, peer_id: &PeerId, block_height: u64) -> Result<()> {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.last_seen = Instant::now();
            peer.block_height = block_height;
            Ok(())
        } else {
            Err(BlockchainError::Network(format!("Peer {} não encontrado", peer_id)))
        }
    }

    /// Removes a peer from the list of known peers
    pub fn remove_peer(&mut self, peer_id: &PeerId) -> Result<()> {
        if self.peers.remove(peer_id).is_some() {
            log::debug!("Removido peer: {}", peer_id);
            Ok(())
        } else {
            Err(BlockchainError::Network(format!("Peer {} não encontrado", peer_id)))
        }
    }

    /// Ban a peer
    pub fn ban_peer(&mut self, peer_id: PeerId, reason: &str) -> Result<()> {
        self.peers.remove(&peer_id);
        self.banned_peers.insert(peer_id);
        log::warn!("Peer {} banido: {}", peer_id, reason);
        Ok(())
    }

    /// Checks if a peer is banned
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        self.banned_peers.contains(peer_id)
    }

    /// Returns the list of active peers
    pub fn get_active_peers(&self) -> Vec<PeerInfo> {
        let now = Instant::now();
        self.peers
            .values()
            .filter(|peer| now.duration_since(peer.last_seen) < self.inactive_threshold)
            .cloned()
            .collect()
    }

    /// Returns the list of bootstrap peers
    pub fn get_bootstrap_peers(&self) -> &[Multiaddr] {
        &self.bootstrap_peers
    }

    /// Adds a bootstrap peer
    pub fn add_bootstrap_peer(&mut self, addr: Multiaddr) {
        if !self.bootstrap_peers.contains(&addr) {
            self.bootstrap_peers.push(addr);
        }
    }

    /// Cleans inactive peers
    pub fn clean_inactive_peers(&mut self) -> usize {
        let now = Instant::now();
        let inactive_peers: Vec<PeerId> = self.peers
            .iter()
            .filter(|(_, peer)| now.duration_since(peer.last_seen) >= self.inactive_threshold)
            .map(|(peer_id, _)| *peer_id)
            .collect();

        let count = inactive_peers.len();
        for peer_id in inactive_peers {
            self.peers.remove(&peer_id);
            log::debug!("Removido peer inativo: {}", peer_id);
        }

        count
    }

    /// Returns the number of known peers
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Returns the number of banned peers
    pub fn banned_count(&self) -> usize {
        self.banned_peers.len()
    }

    /// Find peers with the highest block height
    pub fn find_highest_block_peers(&self) -> Vec<PeerInfo> {
        if self.peers.is_empty() {
            return Vec::new();
        }

        let max_height = self.peers
            .values()
            .map(|peer| peer.block_height)
            .max()
            .unwrap_or(0);

        self.peers
            .values()
            .filter(|peer| peer.block_height == max_height)
            .cloned()
            .collect()
    }
}
