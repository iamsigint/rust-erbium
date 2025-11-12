// src/network/discovery.rs

use libp2p::{Multiaddr, PeerId};
use crate::utils::error::{Result, BlockchainError};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Informações sobre um peer na rede
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Endereço do peer
    pub addr: Multiaddr,
    /// ID do peer
    pub peer_id: PeerId,
    /// Última vez que o peer foi visto
    pub last_seen: Instant,
    /// Versão do protocolo
    pub protocol_version: String,
    /// Altura do bloco mais recente
    pub block_height: u64,
}

/// Gerenciador de descoberta de peers
pub struct PeerDiscovery {
    /// Lista de peers conhecidos
    peers: HashMap<PeerId, PeerInfo>,
    /// Lista de peers de bootstrap
    bootstrap_peers: Vec<Multiaddr>,
    /// Peers banidos
    banned_peers: HashSet<PeerId>,
    /// Tempo máximo sem contato para considerar um peer inativo
    inactive_threshold: Duration,
}

impl PeerDiscovery {
    /// Cria um novo gerenciador de descoberta
    pub fn new(bootstrap_peers: Vec<Multiaddr>) -> Self {
        Self {
            peers: HashMap::new(),
            bootstrap_peers,
            banned_peers: HashSet::new(),
            inactive_threshold: Duration::from_secs(3600), // 1 hora
        }
    }

    /// Adiciona um novo peer à lista de peers conhecidos
    pub fn add_peer(&mut self, peer_id: PeerId, addr: Multiaddr, block_height: u64) -> Result<()> {
        if self.banned_peers.contains(&peer_id) {
            return Err(BlockchainError::Network(format!("Peer {} está banido", peer_id)));
        }

        self.peers.insert(peer_id, PeerInfo {
            addr,
            peer_id,
            last_seen: Instant::now(),
            protocol_version: "1.0.0".to_string(), // Versão padrão
            block_height,
        });

        log::debug!("Adicionado peer: {}", peer_id);
        Ok(())
    }

    /// Atualiza as informações de um peer existente
    pub fn update_peer(&mut self, peer_id: &PeerId, block_height: u64) -> Result<()> {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.last_seen = Instant::now();
            peer.block_height = block_height;
            Ok(())
        } else {
            Err(BlockchainError::Network(format!("Peer {} não encontrado", peer_id)))
        }
    }

    /// Remove um peer da lista de peers conhecidos
    pub fn remove_peer(&mut self, peer_id: &PeerId) -> Result<()> {
        if self.peers.remove(peer_id).is_some() {
            log::debug!("Removido peer: {}", peer_id);
            Ok(())
        } else {
            Err(BlockchainError::Network(format!("Peer {} não encontrado", peer_id)))
        }
    }

    /// Bane um peer
    pub fn ban_peer(&mut self, peer_id: PeerId, reason: &str) -> Result<()> {
        self.peers.remove(&peer_id);
        self.banned_peers.insert(peer_id);
        log::warn!("Peer {} banido: {}", peer_id, reason);
        Ok(())
    }

    /// Verifica se um peer está banido
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        self.banned_peers.contains(peer_id)
    }

    /// Retorna a lista de peers ativos
    pub fn get_active_peers(&self) -> Vec<PeerInfo> {
        let now = Instant::now();
        self.peers
            .values()
            .filter(|peer| now.duration_since(peer.last_seen) < self.inactive_threshold)
            .cloned()
            .collect()
    }

    /// Retorna a lista de peers de bootstrap
    pub fn get_bootstrap_peers(&self) -> &[Multiaddr] {
        &self.bootstrap_peers
    }

    /// Adiciona um peer de bootstrap
    pub fn add_bootstrap_peer(&mut self, addr: Multiaddr) {
        if !self.bootstrap_peers.contains(&addr) {
            self.bootstrap_peers.push(addr);
        }
    }

    /// Limpa peers inativos
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

    /// Retorna o número de peers conhecidos
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Retorna o número de peers banidos
    pub fn banned_count(&self) -> usize {
        self.banned_peers.len()
    }

    /// Encontra peers com a maior altura de bloco
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
