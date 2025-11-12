// src/network/firewall.rs

use libp2p::PeerId;
use crate::utils::error::Result;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Tipos de regras de firewall
#[derive(Debug, Clone, PartialEq)]
pub enum FirewallRule {
    /// Permitir conexão
    Allow,
    /// Bloquear conexão
    Block,
    /// Limitar taxa de conexões
    RateLimit(u32, Duration), // (max_connections, time_window)
}

/// Tipos de eventos de segurança
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityEvent {
    /// Tentativa de conexão excessiva
    ConnectionFlood { count: u32, window: Duration },
    /// Mensagens malformadas
    MalformedMessage { count: u32 },
    /// Comportamento de validador suspeito
    SuspiciousValidatorBehavior { description: String },
    /// Tentativa de eclipse attack
    EclipseAttackAttempt,
    /// Transações inválidas repetidas
    RepeatedInvalidTransactions { count: u32 },
}

/// Gerenciador de firewall para a rede blockchain
pub struct NetworkFirewall {
    /// Regras por endereço IP
    ip_rules: HashMap<IpAddr, FirewallRule>,
    /// Regras por peer ID
    peer_rules: HashMap<PeerId, FirewallRule>,
    /// IPs banidos
    banned_ips: HashSet<IpAddr>,
    /// Peers banidos
    banned_peers: HashSet<PeerId>,
    /// Contadores de conexão por IP
    connection_counters: HashMap<IpAddr, (u32, Instant)>,
    /// Limite global de conexões por IP
    global_rate_limit: (u32, Duration),
    /// Whitelist de IPs
    whitelisted_ips: HashSet<IpAddr>,
    /// Whitelist de peers
    whitelisted_peers: HashSet<PeerId>,
}

impl NetworkFirewall {
    /// Cria um novo gerenciador de firewall
    pub fn new() -> Self {
        Self {
            ip_rules: HashMap::new(),
            peer_rules: HashMap::new(),
            banned_ips: HashSet::new(),
            banned_peers: HashSet::new(),
            connection_counters: HashMap::new(),
            global_rate_limit: (100, Duration::from_secs(60)), // 100 conexões por minuto
            whitelisted_ips: HashSet::new(),
            whitelisted_peers: HashSet::new(),
        }
    }

    /// Adiciona uma regra para um endereço IP
    pub fn add_ip_rule(&mut self, ip: IpAddr, rule: FirewallRule) {
        self.ip_rules.insert(ip, rule);
    }

    /// Adiciona uma regra para um peer ID
    pub fn add_peer_rule(&mut self, peer_id: PeerId, rule: FirewallRule) {
        self.peer_rules.insert(peer_id, rule);
    }

    /// Adiciona um IP à whitelist
    pub fn whitelist_ip(&mut self, ip: IpAddr) {
        self.whitelisted_ips.insert(ip);
        self.banned_ips.remove(&ip);
    }

    /// Adiciona um peer à whitelist
    pub fn whitelist_peer(&mut self, peer_id: PeerId) {
        self.whitelisted_peers.insert(peer_id);
        self.banned_peers.remove(&peer_id);
    }

    /// Bane um endereço IP
    pub fn ban_ip(&mut self, ip: IpAddr, reason: &str) {
        if !self.whitelisted_ips.contains(&ip) {
            self.banned_ips.insert(ip);
            log::warn!("IP {} banido: {}", ip, reason);
        }
    }

    /// Bane um peer ID
    pub fn ban_peer(&mut self, peer_id: PeerId, reason: &str) {
        if !self.whitelisted_peers.contains(&peer_id) {
            self.banned_peers.insert(peer_id);
            log::warn!("Peer {} banido: {}", peer_id, reason);
        }
    }

    /// Verifica se um endereço IP está banido
    pub fn is_ip_banned(&self, ip: &IpAddr) -> bool {
        self.banned_ips.contains(ip)
    }

    /// Verifica se um peer está banido
    pub fn is_peer_banned(&self, peer_id: &PeerId) -> bool {
        self.banned_peers.contains(peer_id)
    }

    /// Verifica se uma conexão deve ser permitida
    pub fn check_connection(&mut self, ip: &IpAddr, peer_id: Option<&PeerId>) -> Result<bool> {
        // Verifica whitelist
        if self.whitelisted_ips.contains(ip) || 
           (peer_id.is_some() && self.whitelisted_peers.contains(peer_id.unwrap())) {
            return Ok(true);
        }

        // Verifica banimentos
        if self.banned_ips.contains(ip) || 
           (peer_id.is_some() && self.banned_peers.contains(peer_id.unwrap())) {
            return Ok(false);
        }

        // Verifica regras específicas
        if let Some(rule) = self.ip_rules.get(ip) {
            match rule {
                FirewallRule::Allow => return Ok(true),
                FirewallRule::Block => return Ok(false),
                FirewallRule::RateLimit(max, window) => {
                    return self.check_rate_limit(ip, *max, *window);
                }
            }
        }

        if let Some(peer) = peer_id {
            if let Some(rule) = self.peer_rules.get(peer) {
                match rule {
                    FirewallRule::Allow => return Ok(true),
                    FirewallRule::Block => return Ok(false),
                    FirewallRule::RateLimit(max, window) => {
                        return self.check_rate_limit(ip, *max, *window);
                    }
                }
            }
        }

        // Aplica regra global de limite de taxa
        self.check_rate_limit(ip, self.global_rate_limit.0, self.global_rate_limit.1)
    }

    /// Verifica o limite de taxa para um IP
    fn check_rate_limit(&mut self, ip: &IpAddr, max: u32, window: Duration) -> Result<bool> {
        let now = Instant::now();
        
        if let Some((count, timestamp)) = self.connection_counters.get_mut(ip) {
            if now.duration_since(*timestamp) > window {
                // Reinicia o contador se o período expirou
                *count = 1;
                *timestamp = now;
                Ok(true)
            } else if *count >= max {
                // Excedeu o limite
                log::warn!("IP {} excedeu o limite de taxa: {} em {:?}", ip, *count, window);
                Ok(false)
            } else {
                // Incrementa o contador
                *count += 1;
                Ok(true)
            }
        } else {
            // Primeiro contato deste IP
            self.connection_counters.insert(*ip, (1, now));
            Ok(true)
        }
    }

    /// Processa um evento de segurança
    pub fn process_security_event(&mut self, ip: IpAddr, peer_id: Option<PeerId>, event: SecurityEvent) -> Result<()> {
        match event {
            SecurityEvent::ConnectionFlood { count, window } => {
                if count > self.global_rate_limit.0 * 2 {
                    // Bane temporariamente por flood de conexão
                    self.ban_ip(ip, &format!("Flood de conexão: {} em {:?}", count, window));
                    if let Some(peer) = peer_id {
                        self.ban_peer(peer, &format!("Flood de conexão: {} em {:?}", count, window));
                    }
                }
            },
            SecurityEvent::MalformedMessage { count } => {
                if count > 5 {
                    // Bane após múltiplas mensagens malformadas
                    self.ban_ip(ip, &format!("Mensagens malformadas: {}", count));
                    if let Some(peer) = peer_id {
                        self.ban_peer(peer, &format!("Mensagens malformadas: {}", count));
                    }
                }
            },
            SecurityEvent::SuspiciousValidatorBehavior { description } => {
                log::warn!("Comportamento suspeito de validador detectado: {}", description);
                // Aqui poderíamos implementar lógica para reportar ao sistema de slashing
            },
            SecurityEvent::EclipseAttackAttempt => {
                log::warn!("Possível tentativa de eclipse attack detectada de {}", ip);
                self.ban_ip(ip, "Tentativa de eclipse attack");
                if let Some(peer) = peer_id {
                    self.ban_peer(peer, "Tentativa de eclipse attack");
                }
            },
            SecurityEvent::RepeatedInvalidTransactions { count } => {
                if count > 10 {
                    log::warn!("Múltiplas transações inválidas de {}: {}", ip, count);
                    self.ban_ip(ip, &format!("Transações inválidas: {}", count));
                    if let Some(peer) = peer_id {
                        self.ban_peer(peer, &format!("Transações inválidas: {}", count));
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Define o limite global de taxa
    pub fn set_global_rate_limit(&mut self, max: u32, window: Duration) {
        self.global_rate_limit = (max, window);
    }

    /// Limpa contadores expirados
    pub fn clean_expired_counters(&mut self) {
        let now = Instant::now();
        self.connection_counters.retain(|_, (_, timestamp)| {
            now.duration_since(*timestamp) <= self.global_rate_limit.1
        });
    }

    /// Retorna o número de IPs banidos
    pub fn banned_ip_count(&self) -> usize {
        self.banned_ips.len()
    }

    /// Retorna o número de peers banidos
    pub fn banned_peer_count(&self) -> usize {
        self.banned_peers.len()
    }
}

impl Default for NetworkFirewall {
    fn default() -> Self {
        Self::new()
    }
}
