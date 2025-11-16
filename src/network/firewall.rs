// src/network/firewall.rs

use crate::utils::error::Result;
use libp2p::PeerId;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Firewall rule types
#[derive(Debug, Clone, PartialEq)]
pub enum FirewallRule {
    /// Allow connection
    Allow,
    /// Block connection
    Block,
    /// Limit connection rate
    RateLimit(u32, Duration), // (max_connections, time_window)
}

/// Types of security events
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityEvent {
    /// Excessive connection attempts
    ConnectionFlood { count: u32, window: Duration },
    /// Malformed messages
    MalformedMessage { count: u32 },
    /// Suspicious validator behavior
    SuspiciousValidatorBehavior { description: String },
    /// Eclipse attack attempt
    EclipseAttackAttempt,
    /// Repeated invalid transactions
    RepeatedInvalidTransactions { count: u32 },
}

/// Firewall manager for the blockchain network
pub struct NetworkFirewall {
    /// Rules by IP address
    ip_rules: HashMap<IpAddr, FirewallRule>,
    /// Rules by peer ID
    peer_rules: HashMap<PeerId, FirewallRule>,
    /// Banned IPs
    banned_ips: HashSet<IpAddr>,
    /// Banned peers
    banned_peers: HashSet<PeerId>,
    /// Connection counters per IP
    connection_counters: HashMap<IpAddr, (u32, Instant)>,
    /// Global rate limit per IP
    global_rate_limit: (u32, Duration),
    /// Whitelisted IPs
    whitelisted_ips: HashSet<IpAddr>,
    /// Whitelisted peers
    whitelisted_peers: HashSet<PeerId>,
}

impl NetworkFirewall {
    /// Creates a new firewall manager
    pub fn new() -> Self {
        Self {
            ip_rules: HashMap::new(),
            peer_rules: HashMap::new(),
            banned_ips: HashSet::new(),
            banned_peers: HashSet::new(),
            connection_counters: HashMap::new(),
            global_rate_limit: (100, Duration::from_secs(60)), // 100 connections per minute
            whitelisted_ips: HashSet::new(),
            whitelisted_peers: HashSet::new(),
        }
    }

    /// Adds a rule for an IP address
    pub fn add_ip_rule(&mut self, ip: IpAddr, rule: FirewallRule) {
        self.ip_rules.insert(ip, rule);
    }

    /// Adds a rule for a peer ID
    pub fn add_peer_rule(&mut self, peer_id: PeerId, rule: FirewallRule) {
        self.peer_rules.insert(peer_id, rule);
    }

    /// Adds an IP to the whitelist
    pub fn whitelist_ip(&mut self, ip: IpAddr) {
        self.whitelisted_ips.insert(ip);
        self.banned_ips.remove(&ip);
    }

    /// Adds a peer to the whitelist
    pub fn whitelist_peer(&mut self, peer_id: PeerId) {
        self.whitelisted_peers.insert(peer_id);
        self.banned_peers.remove(&peer_id);
    }

    /// Bans an IP address
    pub fn ban_ip(&mut self, ip: IpAddr, reason: &str) {
        if !self.whitelisted_ips.contains(&ip) {
            self.banned_ips.insert(ip);
            log::warn!("IP {} banned: {}", ip, reason);
        }
    }

    /// Bans a peer ID
    pub fn ban_peer(&mut self, peer_id: PeerId, reason: &str) {
        if !self.whitelisted_peers.contains(&peer_id) {
            self.banned_peers.insert(peer_id);
            log::warn!("Peer {} banned: {}", peer_id, reason);
        }
    }

    /// Checks if an IP address is banned
    pub fn is_ip_banned(&self, ip: &IpAddr) -> bool {
        self.banned_ips.contains(ip)
    }

    /// Checks if a peer is banned
    pub fn is_peer_banned(&self, peer_id: &PeerId) -> bool {
        self.banned_peers.contains(peer_id)
    }

    /// Checks if a connection should be allowed
    pub fn check_connection(&mut self, ip: &IpAddr, peer_id: Option<&PeerId>) -> Result<bool> {
        // Check whitelist
        if self.whitelisted_ips.contains(ip)
            || (peer_id.is_some() && self.whitelisted_peers.contains(peer_id.unwrap()))
        {
            return Ok(true);
        }

        // Check bans
        if self.banned_ips.contains(ip)
            || (peer_id.is_some() && self.banned_peers.contains(peer_id.unwrap()))
        {
            return Ok(false);
        }

        // Check specific rules
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

        // Apply global rate limit rule
        self.check_rate_limit(ip, self.global_rate_limit.0, self.global_rate_limit.1)
    }

    /// Checks rate limit for an IP
    fn check_rate_limit(&mut self, ip: &IpAddr, max: u32, window: Duration) -> Result<bool> {
        let now = Instant::now();

        if let Some((count, timestamp)) = self.connection_counters.get_mut(ip) {
            if now.duration_since(*timestamp) > window {
                // Reset counter if window expired
                *count = 1;
                *timestamp = now;
                Ok(true)
            } else if *count >= max {
                // Limit exceeded
                log::warn!("IP {} exceeded rate limit: {} in {:?}", ip, *count, window);
                Ok(false)
            } else {
                // Increment counter
                *count += 1;
                Ok(true)
            }
        } else {
            // First contact from this IP
            self.connection_counters.insert(*ip, (1, now));
            Ok(true)
        }
    }

    /// Processes a security event
    pub fn process_security_event(
        &mut self,
        ip: IpAddr,
        peer_id: Option<PeerId>,
        event: SecurityEvent,
    ) -> Result<()> {
        match event {
            SecurityEvent::ConnectionFlood { count, window } => {
                if count > self.global_rate_limit.0 * 2 {
                    // Temporarily ban due to connection flood
                    self.ban_ip(ip, &format!("Connection flood: {} in {:?}", count, window));
                    if let Some(peer) = peer_id {
                        self.ban_peer(
                            peer,
                            &format!("Connection flood: {} in {:?}", count, window),
                        );
                    }
                }
            }
            SecurityEvent::MalformedMessage { count } => {
                if count > 5 {
                    // Ban after multiple malformed messages
                    self.ban_ip(ip, &format!("Malformed messages: {}", count));
                    if let Some(peer) = peer_id {
                        self.ban_peer(peer, &format!("Malformed messages: {}", count));
                    }
                }
            }
            SecurityEvent::SuspiciousValidatorBehavior { description } => {
                log::warn!("Suspicious validator behavior detected: {}", description);
                // Logic to report to slashing system could be implemented here
            }
            SecurityEvent::EclipseAttackAttempt => {
                log::warn!("Possible eclipse attack attempt detected from {}", ip);
                self.ban_ip(ip, "Eclipse attack attempt");
                if let Some(peer) = peer_id {
                    self.ban_peer(peer, "Eclipse attack attempt");
                }
            }
            SecurityEvent::RepeatedInvalidTransactions { count } => {
                if count > 10 {
                    log::warn!("Multiple invalid transactions from {}: {}", ip, count);
                    self.ban_ip(ip, &format!("Invalid transactions: {}", count));
                    if let Some(peer) = peer_id {
                        self.ban_peer(peer, &format!("Invalid transactions: {}", count));
                    }
                }
            }
        }

        Ok(())
    }

    /// Sets the global rate limit
    pub fn set_global_rate_limit(&mut self, max: u32, window: Duration) {
        self.global_rate_limit = (max, window);
    }

    /// Cleans expired counters
    pub fn clean_expired_counters(&mut self) {
        let now = Instant::now();
        self.connection_counters
            .retain(|_, (_, timestamp)| now.duration_since(*timestamp) <= self.global_rate_limit.1);
    }

    /// Returns the number of banned IPs
    pub fn banned_ip_count(&self) -> usize {
        self.banned_ips.len()
    }

    /// Returns the number of banned peers
    pub fn banned_peer_count(&self) -> usize {
        self.banned_peers.len()
    }
}

impl Default for NetworkFirewall {
    fn default() -> Self {
        Self::new()
    }
}
