// src/network/dht.rs

use crate::utils::error::{Result, BlockchainError};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use tokio::time::{Duration, Instant};
use rand::Rng;
use sha2::{Sha256, Digest};

/// Node ID (160-bit for Kademlia compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId([u8; 20]);

impl NodeId {
    /// Generate random NodeId
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 20];
        rng.fill(&mut bytes);
        Self(bytes)
    }

    /// Create NodeId from bytes
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Create NodeId from hash of data
    pub fn from_hash(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(&result[..20]);
        Self(bytes)
    }

    /// Calculate XOR distance between two NodeIds
    pub fn distance(&self, other: &NodeId) -> NodeId {
        let mut result = [0u8; 20];
        for i in 0..20 {
            result[i] = self.0[i] ^ other.0[i];
        }
        NodeId(result)
    }

    /// Get the bucket index for a distance
    pub fn bucket_index(&self, other: &NodeId) -> usize {
        let dist = self.distance(other);
        // Find the highest bit set
        for i in 0..160 {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            if (dist.0[byte_idx] & (1 << bit_idx)) != 0 {
                return 159 - i;
            }
        }
        0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Get bytes
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// DHT Contact information
#[derive(Debug, Clone)]
pub struct Contact {
    pub node_id: NodeId,
    pub address: SocketAddr,
    pub last_seen: Instant,
    pub failures: u32,
}

impl Contact {
    pub fn new(node_id: NodeId, address: SocketAddr) -> Self {
        Self {
            node_id,
            address,
            last_seen: Instant::now(),
            failures: 0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.failures < 3
    }

    pub fn mark_seen(&mut self) {
        self.last_seen = Instant::now();
        self.failures = 0;
    }

    pub fn mark_failed(&mut self) {
        self.failures += 1;
    }

    pub fn time_since_seen(&self) -> Duration {
        Instant::now().duration_since(self.last_seen)
    }
}

/// K-bucket for storing contacts
#[derive(Debug, Clone)]
pub struct KBucket {
    contacts: Vec<Contact>,
    max_size: usize,
}

impl KBucket {
    pub fn new(max_size: usize) -> Self {
        Self {
            contacts: Vec::new(),
            max_size,
        }
    }

    /// Add or update contact
    pub fn add_contact(&mut self, contact: Contact) {
        // Remove existing contact if present
        self.contacts.retain(|c| c.node_id != contact.node_id);

        // Add new contact at the end (most recently seen)
        self.contacts.push(contact);

        // Keep only the most recent k contacts
        if self.contacts.len() > self.max_size {
            self.contacts.remove(0); // Remove oldest
        }
    }

    /// Get all contacts
    pub fn contacts(&self) -> &[Contact] {
        &self.contacts
    }

    /// Check if bucket is full
    pub fn is_full(&self) -> bool {
        self.contacts.len() >= self.max_size
    }

    /// Get contact by node ID
    pub fn get_contact(&self, node_id: &NodeId) -> Option<&Contact> {
        self.contacts.iter().find(|c| c.node_id == *node_id)
    }

    /// Remove contact
    pub fn remove_contact(&mut self, node_id: &NodeId) {
        self.contacts.retain(|c| c.node_id != *node_id);
    }
}

/// DHT Configuration
#[derive(Debug, Clone)]
pub struct DHTConfig {
    pub k: usize,                    // Bucket size (k)
    pub alpha: usize,               // Parallelism factor (alpha)
    pub id_bits: usize,             // ID bits (160 for Kademlia)
    pub refresh_interval: Duration, // Bucket refresh interval
    pub republish_interval: Duration, // Republish interval
    pub expire_interval: Duration,  // Expire interval
}

impl Default for DHTConfig {
    fn default() -> Self {
        Self {
            k: 20,                          // Standard Kademlia k
            alpha: 3,                       // Standard alpha
            id_bits: 160,                   // 160-bit IDs
            refresh_interval: Duration::from_secs(3600), // 1 hour
            republish_interval: Duration::from_secs(86400), // 24 hours
            expire_interval: Duration::from_secs(86400), // 24 hours
        }
    }
}

/// Kademlia DHT implementation
pub struct DHT {
    config: DHTConfig,
    node_id: NodeId,
    buckets: Vec<KBucket>,
    known_nodes: HashSet<NodeId>,
    pending_queries: HashMap<NodeId, QueryState>,
}

#[derive(Debug)]
struct QueryState {
    target: NodeId,
    closest_nodes: Vec<Contact>,
    active_contacts: HashSet<NodeId>,
    start_time: Instant,
}

impl DHT {
    /// Create new DHT node
    pub fn new(config: DHTConfig) -> Self {
        let node_id = NodeId::random();
        let mut buckets = Vec::with_capacity(config.id_bits);

        for _ in 0..config.id_bits {
            buckets.push(KBucket::new(config.k));
        }

        Self {
            config,
            node_id,
            buckets,
            known_nodes: HashSet::new(),
            pending_queries: HashMap::new(),
        }
    }

    /// Create DHT with specific node ID
    pub fn with_node_id(config: DHTConfig, node_id: NodeId) -> Self {
        let mut buckets = Vec::with_capacity(config.id_bits);

        for _ in 0..config.id_bits {
            buckets.push(KBucket::new(config.k));
        }

        Self {
            config,
            node_id,
            buckets,
            known_nodes: HashSet::new(),
            pending_queries: HashMap::new(),
        }
    }

    /// Get node ID
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Add a known node to bootstrap the network
    pub fn add_bootstrap_node(&mut self, contact: Contact) -> Result<()> {
        if contact.node_id == self.node_id {
            return Err(BlockchainError::Network("Cannot add self as bootstrap node".to_string()));
        }

        self.known_nodes.insert(contact.node_id);
        self.insert_contact(contact);
        Ok(())
    }

    /// Insert contact into appropriate k-bucket
    pub fn insert_contact(&mut self, contact: Contact) {
        let bucket_idx = self.node_id.bucket_index(&contact.node_id);

        if bucket_idx < self.buckets.len() {
            self.buckets[bucket_idx].add_contact(contact);
        }
    }

    /// Find k closest nodes to target
    pub fn find_closest_nodes(&self, target: &NodeId, k: usize) -> Vec<Contact> {
        let mut candidates = Vec::new();

        // Collect candidates from all buckets
        for bucket in &self.buckets {
            for contact in bucket.contacts() {
                candidates.push(contact.clone());
            }
        }

        // Sort by distance to target
        candidates.sort_by_key(|c| target.distance(&c.node_id));

        // Return k closest
        candidates.into_iter().take(k).collect()
    }

    /// Start node lookup
    pub fn start_lookup(&mut self, target: NodeId) -> Result<()> {
        if self.pending_queries.contains_key(&target) {
            return Err(BlockchainError::Network("Lookup already in progress".to_string()));
        }

        let closest_nodes = self.find_closest_nodes(&target, self.config.k);
        let active_contacts = closest_nodes.iter()
            .map(|c| c.node_id)
            .collect();

        let query_state = QueryState {
            target,
            closest_nodes,
            active_contacts,
            start_time: Instant::now(),
        };

        self.pending_queries.insert(target, query_state);

        // TODO: Send FIND_NODE messages to closest nodes
        log::debug!("Started lookup for node {}", target);

        Ok(())
    }

    /// Handle FIND_NODE response
    pub fn handle_find_node_response(&mut self, from: NodeId, contacts: Vec<Contact>) -> Result<()> {
        // Update contact last seen
        self.update_contact_last_seen(&from);

        // Add new contacts to our routing table first
        for contact in &contacts {
            if contact.node_id != self.node_id {
                self.insert_contact(contact.clone());
            }
        }

        // Process response if we have a pending query
        if let Some(query) = self.pending_queries.get_mut(&from) {
            // Update closest nodes
            let mut all_contacts = query.closest_nodes.clone();
            all_contacts.extend(contacts);
            all_contacts.sort_by_key(|c| query.target.distance(&c.node_id));
            all_contacts.dedup_by_key(|c| c.node_id);

            query.closest_nodes = all_contacts.into_iter().take(self.config.k).collect();

            // Mark contact as responded
            query.active_contacts.remove(&from);
        }

        Ok(())
    }

    /// Check if lookup is complete
    pub fn is_lookup_complete(&self, target: &NodeId) -> bool {
        if let Some(query) = self.pending_queries.get(target) {
            query.active_contacts.is_empty() ||
            query.start_time.elapsed() > Duration::from_secs(30) // Timeout after 30 seconds
        } else {
            true
        }
    }

    /// Get lookup results
    pub fn get_lookup_results(&self, target: &NodeId) -> Option<Vec<Contact>> {
        self.pending_queries.get(target)
            .map(|query| query.closest_nodes.clone())
    }

    /// Complete lookup and clean up
    pub fn complete_lookup(&mut self, target: &NodeId) -> Option<Vec<Contact>> {
        self.pending_queries.remove(target)
            .map(|query| query.closest_nodes)
    }

    /// Update contact last seen time
    pub fn update_contact_last_seen(&mut self, node_id: &NodeId) {
        for bucket in &mut self.buckets {
            if let Some(contact) = bucket.contacts.iter_mut()
                .find(|c| c.node_id == *node_id) {
                contact.mark_seen();
                break;
            }
        }
    }

    /// Mark contact as failed
    pub fn mark_contact_failed(&mut self, node_id: &NodeId) {
        for bucket in &mut self.buckets {
            if let Some(contact) = bucket.contacts.iter_mut()
                .find(|c| c.node_id == *node_id) {
                contact.mark_failed();
                break;
            }
        }
    }

    /// Get routing table statistics
    pub fn get_stats(&self) -> DHTStats {
        let mut total_contacts = 0;
        let mut bucket_sizes = Vec::new();

        for bucket in &self.buckets {
            let size = bucket.contacts.len();
            total_contacts += size;
            bucket_sizes.push(size);
        }

        DHTStats {
            node_id: self.node_id,
            total_contacts,
            bucket_sizes,
            pending_queries: self.pending_queries.len(),
        }
    }

    /// Refresh buckets (ping old contacts)
    pub fn refresh_buckets(&self) -> Vec<NodeId> {
        let mut nodes_to_ping = Vec::new();

        for (_bucket_idx, bucket) in self.buckets.iter().enumerate() {
            for contact in bucket.contacts() {
                if contact.time_since_seen() > self.config.refresh_interval {
                    nodes_to_ping.push(contact.node_id);
                }
            }
        }

        nodes_to_ping
    }

    /// Get all known contacts
    pub fn get_all_contacts(&self) -> Vec<Contact> {
        let mut all_contacts = Vec::new();

        for bucket in &self.buckets {
            all_contacts.extend(bucket.contacts().iter().cloned());
        }

        all_contacts
    }

    /// Find contact by node ID
    pub fn find_contact(&self, node_id: &NodeId) -> Option<Contact> {
        for bucket in &self.buckets {
            if let Some(contact) = bucket.get_contact(node_id) {
                return Some(contact.clone());
            }
        }
        None
    }

    /// Remove contact
    pub fn remove_contact(&mut self, node_id: &NodeId) {
        for bucket in &mut self.buckets {
            bucket.remove_contact(node_id);
        }
        self.known_nodes.remove(node_id);
    }
}

/// DHT Statistics
#[derive(Debug, Clone)]
pub struct DHTStats {
    pub node_id: NodeId,
    pub total_contacts: usize,
    pub bucket_sizes: Vec<usize>,
    pub pending_queries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_node_id_distance() {
        let id1 = NodeId::from_bytes([0; 20]);
        let id2 = NodeId::from_bytes([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let dist = id1.distance(&id2);
        assert_eq!(dist.0[0], 1);
        assert_eq!(dist.0[1], 0);
    }

    #[test]
    fn test_bucket_index() {
        let id1 = NodeId::from_bytes([0; 20]);
        let id2 = NodeId::from_bytes([128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

        let bucket_idx = id1.bucket_index(&id2);
        assert_eq!(bucket_idx, 152); // MSB is at bit 7 in byte 0: 159 - 7 = 152
    }

    #[test]
    fn test_dht_creation() {
        let config = DHTConfig::default();
        let dht = DHT::new(config);

        let stats = dht.get_stats();
        assert_eq!(stats.bucket_sizes.len(), 160);
        assert_eq!(stats.total_contacts, 0);
    }

    #[test]
    fn test_add_bootstrap_node() {
        let config = DHTConfig::default();
        let mut dht = DHT::new(config);

        let bootstrap_id = NodeId::random();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 30303);
        let contact = Contact::new(bootstrap_id, addr);

        dht.add_bootstrap_node(contact).unwrap();

        let stats = dht.get_stats();
        assert_eq!(stats.total_contacts, 1);
    }

    #[test]
    fn test_find_closest_nodes() {
        let config = DHTConfig::default();
        let mut dht = DHT::new(config);

        // Add some test contacts
        for i in 0..5 {
            let node_id = NodeId::from_bytes([i as u8; 20]);
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 30303 + i);
            let contact = Contact::new(node_id, addr);
            dht.insert_contact(contact);
        }

        let target = NodeId::from_bytes([0; 20]);
        let closest = dht.find_closest_nodes(&target, 3);

        assert_eq!(closest.len(), 3);
    }
}
