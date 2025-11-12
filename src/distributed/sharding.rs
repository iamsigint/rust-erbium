//! Distributed sharding system for horizontal data distribution
//!
//! This module provides intelligent data sharding across multiple nodes,
//! enabling horizontal scaling and improved performance for large datasets.

use crate::utils::error::{Result, BlockchainError};
use crate::core::types::{Address, Hash};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, BTreeMap, VecDeque};
use std::sync::Arc;
use std::hash::{Hash as StdHash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Shard configuration and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardConfig {
    pub num_shards: u32,
    pub replication_factor: u32,
    pub shard_key_strategy: ShardKeyStrategy,
    pub rebalance_threshold: f64, // When to trigger rebalancing (0.0-1.0)
    pub max_shard_size_gb: u64,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            num_shards: 16,
            replication_factor: 3,
            shard_key_strategy: ShardKeyStrategy::HashModulo,
            rebalance_threshold: 0.8,
            max_shard_size_gb: 100,
        }
    }
}

/// Shard key generation strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShardKeyStrategy {
    /// Simple hash modulo (fastest)
    HashModulo,
    /// Consistent hashing (better distribution)
    ConsistentHash,
    /// Range-based sharding (for ordered data)
    RangeBased,
    /// Geography-based (location-aware)
    GeographyBased,
}

/// Shard metadata and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardInfo {
    pub id: u32,
    pub node_id: String,
    pub range_start: Vec<u8>,
    pub range_end: Vec<u8>,
    pub size_bytes: u64,
    pub record_count: u64,
    pub last_updated: u64,
    pub status: ShardStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShardStatus {
    Active,
    Migrating,
    ReadOnly,
    Offline,
}

/// Main sharding manager
pub struct ShardManager {
    config: ShardConfig,
    shards: HashMap<u32, ShardInfo>,
    node_shards: HashMap<String, Vec<u32>>, // node_id -> shard_ids
    consistent_hash_ring: Option<ConsistentHashRing>,
    shard_locks: HashMap<u32, Arc<std::sync::Mutex<()>>>, // Prevent concurrent shard operations
}

impl ShardManager {
    pub fn new(config: ShardConfig) -> Self {
        let mut manager = Self {
            config,
            shards: HashMap::new(),
            node_shards: HashMap::new(),
            consistent_hash_ring: None,
            shard_locks: HashMap::new(),
        };

        // Initialize consistent hashing if needed
        if matches!(manager.config.shard_key_strategy, ShardKeyStrategy::ConsistentHash) {
            manager.consistent_hash_ring = Some(ConsistentHashRing::new(manager.config.num_shards));
        }

        manager.initialize_shards();
        manager
    }

    /// Initialize shards across available nodes
    fn initialize_shards(&mut self) {
        // For now, assume single node "local"
        // In production, this would discover nodes dynamically
        let nodes = vec!["local".to_string()];

        for shard_id in 0..self.config.num_shards {
            let node_id = self.select_node_for_shard(shard_id, &nodes);

            let shard_info = ShardInfo {
                id: shard_id,
                node_id: node_id.clone(),
                range_start: self.calculate_range_start(shard_id),
                range_end: self.calculate_range_end(shard_id),
                size_bytes: 0,
                record_count: 0,
                last_updated: current_timestamp(),
                status: ShardStatus::Active,
            };

            self.shards.insert(shard_id, shard_info);
            self.node_shards.entry(node_id).or_insert_with(Vec::new).push(shard_id);
            self.shard_locks.insert(shard_id, Arc::new(std::sync::Mutex::new(())));
        }
    }

    /// Determine which shard a key belongs to
    pub fn get_shard_for_key(&self, key: &[u8]) -> Result<u32> {
        match self.config.shard_key_strategy {
            ShardKeyStrategy::HashModulo => {
                let hash = self.hash_key(key);
                Ok((hash % self.config.num_shards as u64) as u32)
            }
            ShardKeyStrategy::ConsistentHash => {
                if let Some(ring) = &self.consistent_hash_ring {
                    Ok(ring.get_shard(key))
                } else {
                    Err(BlockchainError::Storage("Consistent hash ring not initialized".to_string()))
                }
            }
            ShardKeyStrategy::RangeBased => {
                self.get_range_shard(key)
            }
            ShardKeyStrategy::GeographyBased => {
                // For now, fall back to hash modulo
                // In production, this would use geo-location data
                let hash = self.hash_key(key);
                Ok((hash % self.config.num_shards as u64) as u32)
            }
        }
    }

    /// Get all nodes that should have a copy of the data
    pub fn get_replica_nodes(&self, shard_id: u32) -> Vec<String> {
        // Simple replication strategy: primary + N-1 replicas
        // In production, this would use more sophisticated placement
        let mut nodes = Vec::new();

        if let Some(shard) = self.shards.get(&shard_id) {
            nodes.push(shard.node_id.clone());

            // Add replica nodes (simplified)
            for i in 1..self.config.replication_factor {
                let replica_node = format!("node_{}", (shard_id as usize + i as usize) % 10);
                nodes.push(replica_node);
            }
        }

        nodes
    }

    /// Check if shard needs rebalancing
    pub fn needs_rebalancing(&self, shard_id: u32) -> bool {
        if let Some(shard) = self.shards.get(&shard_id) {
            let size_ratio = shard.size_bytes as f64 / (self.config.max_shard_size_gb * 1024 * 1024 * 1024) as f64;
            size_ratio > self.config.rebalance_threshold
        } else {
            false
        }
    }

    /// Rebalance shards across nodes
    pub async fn rebalance_shards(&mut self) -> Result<()> {
        log::info!("Starting shard rebalancing...");

        let mut shards_to_move = Vec::new();

        // Find overloaded shards
        for (shard_id, shard) in &self.shards {
            if self.needs_rebalancing(*shard_id) {
                shards_to_move.push(*shard_id);
            }
        }

        // Split overloaded shards
        for shard_id in shards_to_move {
            self.split_shard(shard_id).await?;
        }

        log::info!("Shard rebalancing completed");
        Ok(())
    }

    /// Split a shard into two
    async fn split_shard(&mut self, shard_id: u32) -> Result<()> {
        let _lock = self.shard_locks.get(&shard_id)
            .ok_or_else(|| BlockchainError::Storage("Shard lock not found".to_string()))?
            .lock().unwrap();

        if let Some(shard) = self.shards.get_mut(&shard_id) {
            shard.status = ShardStatus::Migrating;

            // Calculate split point (middle of range)
            let split_point = self.calculate_split_point(&shard.range_start, &shard.range_end);

            // Create new shard
            let new_shard_id = self.config.num_shards;
            self.config.num_shards += 1;

            let new_shard = ShardInfo {
                id: new_shard_id,
                node_id: self.select_best_node(),
                range_start: split_point.clone(),
                range_end: shard.range_end.clone(),
                size_bytes: shard.size_bytes / 2,
                record_count: shard.record_count / 2,
                last_updated: current_timestamp(),
                status: ShardStatus::Active,
            };

            // Update existing shard
            shard.range_end = split_point;
            shard.size_bytes /= 2;
            shard.record_count /= 2;
            shard.status = ShardStatus::Active;

            // Register new shard
            self.shards.insert(new_shard_id, new_shard);
            self.shard_locks.insert(new_shard_id, Arc::new(std::sync::Mutex::new(())));

            log::info!("Split shard {} into {} and {}", shard_id, shard_id, new_shard_id);
        }

        Ok(())
    }

    /// Update shard statistics
    pub fn update_shard_stats(&mut self, shard_id: u32, size_delta: i64, record_delta: i64) {
        if let Some(shard) = self.shards.get_mut(&shard_id) {
            shard.size_bytes = (shard.size_bytes as i64 + size_delta).max(0) as u64;
            shard.record_count = (shard.record_count as i64 + record_delta).max(0) as u64;
            shard.last_updated = current_timestamp();
        }
    }

    /// Get shard information
    pub fn get_shard_info(&self, shard_id: u32) -> Option<&ShardInfo> {
        self.shards.get(&shard_id)
    }

    /// Get all shard information
    pub fn get_all_shards(&self) -> Vec<&ShardInfo> {
        self.shards.values().collect()
    }

    /// Get shards for a specific node
    pub fn get_node_shards(&self, node_id: &str) -> Vec<&ShardInfo> {
        self.node_shards.get(node_id)
            .map(|shard_ids| {
                shard_ids.iter()
                    .filter_map(|id| self.shards.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    // Helper methods
    fn hash_key(&self, key: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn select_node_for_shard(&self, shard_id: u32, nodes: &[String]) -> String {
        // Simple round-robin for now
        // In production, this would consider load, geography, etc.
        let node_index = shard_id as usize % nodes.len();
        nodes[node_index].clone()
    }

    fn select_best_node(&self) -> String {
        // For now, return local node
        // In production, this would select based on load, capacity, etc.
        "local".to_string()
    }

    fn calculate_range_start(&self, shard_id: u32) -> Vec<u8> {
        // Simple range calculation
        // In production, this would be more sophisticated
        let range_size = u64::MAX / self.config.num_shards as u64;
        let start = range_size * shard_id as u64;
        start.to_be_bytes().to_vec()
    }

    fn calculate_range_end(&self, shard_id: u32) -> Vec<u8> {
        let range_size = u64::MAX / self.config.num_shards as u64;
        let end = if shard_id == self.config.num_shards - 1 {
            u64::MAX
        } else {
            range_size * (shard_id as u64 + 1) - 1
        };
        end.to_be_bytes().to_vec()
    }

    fn calculate_split_point(&self, start: &[u8], end: &[u8]) -> Vec<u8> {
        // Calculate midpoint
        let start_val = u64::from_be_bytes(start[..8].try_into().unwrap_or([0; 8]));
        let end_val = u64::from_be_bytes(end[..8].try_into().unwrap_or([u8::MAX; 8]));
        let mid = start_val + (end_val - start_val) / 2;
        mid.to_be_bytes().to_vec()
    }

    fn get_range_shard(&self, key: &[u8]) -> Result<u32> {
        for (shard_id, shard) in &self.shards {
            if key >= &shard.range_start && key < &shard.range_end {
                return Ok(*shard_id);
            }
        }
        Err(BlockchainError::Storage("No shard found for key".to_string()))
    }
}

/// Consistent hashing ring for better load distribution
pub struct ConsistentHashRing {
    ring: BTreeMap<u64, u32>, // hash -> shard_id
    replicas: u32,
}

impl ConsistentHashRing {
    pub fn new(num_shards: u32) -> Self {
        let mut ring = BTreeMap::new();
        let replicas = 3; // Virtual nodes per shard

        for shard_id in 0..num_shards {
            for replica in 0..replicas {
                let key = format!("shard_{}_{}", shard_id, replica);
                let hash = Self::hash_key(key.as_bytes());
                ring.insert(hash, shard_id);
            }
        }

        Self { ring, replicas }
    }

    pub fn get_shard(&self, key: &[u8]) -> u32 {
        let hash = Self::hash_key(key);

        // Find the first shard with hash >= key_hash
        self.ring.range(hash..).next()
            .map(|(_, shard_id)| *shard_id)
            .unwrap_or_else(|| {
                // Wrap around to first shard
                self.ring.values().next().copied().unwrap_or(0)
            })
    }

    fn hash_key(key: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_manager_creation() {
        let config = ShardConfig::default();
        let manager = ShardManager::new(config);

        assert_eq!(manager.shards.len(), 16); // Default num_shards
        assert!(manager.node_shards.contains_key("local"));
    }

    #[test]
    fn test_shard_key_assignment() {
        let config = ShardConfig::default();
        let manager = ShardManager::new(config);

        let shard1 = manager.get_shard_for_key(b"key1").unwrap();
        let shard2 = manager.get_shard_for_key(b"key2").unwrap();

        assert!(shard1 < 16);
        assert!(shard2 < 16);
        // Different keys should often map to different shards
    }

    #[test]
    fn test_replica_nodes() {
        let config = ShardConfig {
            replication_factor: 3,
            ..Default::default()
        };
        let manager = ShardManager::new(config);

        let replicas = manager.get_replica_nodes(0);
        assert_eq!(replicas.len(), 3);
        assert_eq!(replicas[0], "local");
    }

    #[test]
    fn test_consistent_hash_ring() {
        let ring = ConsistentHashRing::new(4);

        let shard1 = ring.get_shard(b"key1");
        let shard2 = ring.get_shard(b"key2");

        assert!(shard1 < 4);
        assert!(shard2 < 4);
    }

    #[test]
    fn test_shard_stats_update() {
        let config = ShardConfig::default();
        let mut manager = ShardManager::new(config);

        manager.update_shard_stats(0, 1000, 10);

        if let Some(shard) = manager.get_shard_info(0) {
            assert_eq!(shard.size_bytes, 1000);
            assert_eq!(shard.record_count, 10);
        }
    }
}
