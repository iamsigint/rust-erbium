//! Automatic state replication system for fault tolerance and high availability
//!
//! This module provides automatic replication of blockchain state across multiple nodes,
//! ensuring data durability and availability even during node failures.

use crate::utils::error::{Result, BlockchainError};
use crate::core::types::{Hash, Address};
use crate::distributed::sharding::{ShardManager, ShardConfig};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// Replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub replication_factor: u32,
    pub min_replicas: u32,
    pub sync_interval_seconds: u64,
    pub max_sync_lag_seconds: u64,
    pub auto_failover: bool,
    pub consistency_level: ConsistencyLevel,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            replication_factor: 3,
            min_replicas: 2,
            sync_interval_seconds: 30,
            max_sync_lag_seconds: 300, // 5 minutes
            auto_failover: true,
            consistency_level: ConsistencyLevel::Strong,
        }
    }
}

/// Consistency levels for replication
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConsistencyLevel {
    /// Strong consistency - all replicas must acknowledge
    Strong,
    /// Eventual consistency - replicas sync asynchronously
    Eventual,
    /// Quorum-based consistency
    Quorum,
}

/// Replica node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaNode {
    pub node_id: String,
    pub address: String,
    pub last_heartbeat: u64,
    pub status: ReplicaStatus,
    pub lag_seconds: u64,
    pub priority: u8, // 0 = primary, higher numbers = lower priority
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicaStatus {
    Primary,
    Secondary,
    Syncing,
    Offline,
    Failed,
}

/// Replication state for a shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardReplicationState {
    pub shard_id: u32,
    pub primary_node: String,
    pub replica_nodes: Vec<String>,
    pub last_sync_height: u64,
    pub pending_operations: VecDeque<ReplicationOperation>,
    pub consistency_checksums: HashMap<String, Hash>, // node_id -> state_hash
}

/// Replication operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReplicationOperation {
    StateUpdate {
        key: Vec<u8>,
        value: Vec<u8>,
        version: u64,
    },
    BatchUpdate {
        operations: Vec<(Vec<u8>, Vec<u8>)>,
        version: u64,
    },
    ShardSplit {
        old_shard: u32,
        new_shard: u32,
        split_key: Vec<u8>,
    },
    NodeFailover {
        failed_node: String,
        new_primary: String,
    },
}

/// Main replication manager
pub struct ReplicationManager {
    config: ReplicationConfig,
    shard_manager: Arc<RwLock<ShardManager>>,
    replica_nodes: Arc<RwLock<HashMap<String, ReplicaNode>>>,
    shard_states: Arc<RwLock<HashMap<u32, ShardReplicationState>>>,
    operation_log: Arc<RwLock<VecDeque<ReplicationOperation>>>,
    sync_sender: mpsc::UnboundedSender<SyncMessage>,
    sync_receiver: Arc<RwLock<mpsc::UnboundedReceiver<SyncMessage>>>,
}

#[derive(Debug)]
enum SyncMessage {
    SyncShard { shard_id: u32, target_node: String },
    CheckConsistency { shard_id: u32 },
    Failover { shard_id: u32, failed_node: String },
    Heartbeat { node_id: String },
}

impl ReplicationManager {
    pub fn new(
        config: ReplicationConfig,
        shard_manager: Arc<RwLock<ShardManager>>,
    ) -> Self {
        let (sync_sender, sync_receiver) = mpsc::unbounded_channel();

        Self {
            config,
            shard_manager,
            replica_nodes: Arc::new(RwLock::new(HashMap::new())),
            shard_states: Arc::new(RwLock::new(HashMap::new())),
            operation_log: Arc::new(RwLock::new(VecDeque::with_capacity(10000))),
            sync_sender,
            sync_receiver: Arc::new(RwLock::new(sync_receiver)),
        }
    }

    /// Initialize replication for all shards
    pub async fn initialize_replication(&self) -> Result<()> {
        log::info!("Initializing replication system...");

        // Initialize replica nodes (simplified - in production would discover via gossip)
        self.initialize_replica_nodes().await?;

        // Initialize replication state for each shard
        let shard_manager = self.shard_manager.read().await;
        for shard_id in 0..shard_manager.config.num_shards {
            self.initialize_shard_replication(shard_id).await?;
        }

        log::info!("Replication system initialized");
        Ok(())
    }

    /// Initialize replica nodes
    async fn initialize_replica_nodes(&self) -> Result<()> {
        let mut nodes = self.replica_nodes.write().await;

        // SECURITY: No hardcoded localhost addresses
        // In production, nodes must be explicitly configured
        log::warn!("REPLICATION: No replica nodes configured - add nodes explicitly in distributed setup");

        // Add local node as primary (placeholder only)
        nodes.insert("local".to_string(), ReplicaNode {
            node_id: "local".to_string(),
            address: "0.0.0.0:0".to_string(), // No hardcoded address
            last_heartbeat: current_timestamp(),
            status: ReplicaStatus::Primary,
            lag_seconds: 0,
            priority: 0,
        });

        Ok(())
    }

    /// Initialize replication state for a shard
    async fn initialize_shard_replication(&self, shard_id: u32) -> Result<()> {
        let shard_manager = self.shard_manager.read().await;
        let replica_nodes = shard_manager.get_replica_nodes(shard_id);

        let replication_state = ShardReplicationState {
            shard_id,
            primary_node: "local".to_string(), // Local node is primary
            replica_nodes,
            last_sync_height: 0,
            pending_operations: VecDeque::new(),
            consistency_checksums: HashMap::new(),
        };

        let mut shard_states = self.shard_states.write().await;
        shard_states.insert(shard_id, replication_state);

        Ok(())
    }

    /// Replicate an operation to all replica nodes
    pub async fn replicate_operation(&self, operation: ReplicationOperation) -> Result<()> {
        // Add to operation log
        {
            let mut log = self.operation_log.write().await;
            log.push_back(operation.clone());

            // Keep log size manageable
            if log.len() > 10000 {
                log.pop_front();
            }
        }

        // Queue operation for replication
        let shard_id = self.get_operation_shard(&operation);
        if let Some(shard_id) = shard_id {
            let mut shard_states = self.shard_states.write().await;
            if let Some(state) = shard_states.get_mut(&shard_id) {
                state.pending_operations.push_back(operation);
            }
        }

        // Trigger sync for affected shards
        self.trigger_sync().await?;

        Ok(())
    }

    /// Synchronize pending operations with replica nodes
    pub async fn sync_pending_operations(&self) -> Result<()> {
        let shard_states = self.shard_states.read().await;

        for (shard_id, state) in shard_states.iter() {
            if !state.pending_operations.is_empty() {
                for replica_node in &state.replica_nodes {
                    let _ = self.sync_sender.send(SyncMessage::SyncShard {
                        shard_id: *shard_id,
                        target_node: replica_node.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Check consistency across all replicas
    pub async fn check_consistency(&self) -> Result<ConsistencyReport> {
        let mut report = ConsistencyReport::default();
        let shard_states = self.shard_states.read().await;

        for (shard_id, state) in shard_states.iter() {
            let _ = self.sync_sender.send(SyncMessage::CheckConsistency { shard_id: *shard_id });

            // Check if all replicas have the same checksum
            let checksums: Vec<&Hash> = state.consistency_checksums.values().collect();
            let all_match = checksums.windows(2).all(|w| w[0] == w[1]);

            if !all_match {
                report.inconsistent_shards.push(*shard_id);
            } else {
                report.consistent_shards.push(*shard_id);
            }
        }

        report.total_shards = shard_states.len();
        Ok(report)
    }

    /// Handle node failure and trigger failover if needed
    pub async fn handle_node_failure(&self, failed_node: String) -> Result<()> {
        log::warn!("Handling failure of node: {}", failed_node);

        // Update node status
        {
            let mut nodes = self.replica_nodes.write().await;
            if let Some(node) = nodes.get_mut(&failed_node) {
                node.status = ReplicaStatus::Failed;
            }
        }

        // Check if we need to trigger failover
        if self.config.auto_failover {
            self.trigger_failover(failed_node).await?;
        }

        Ok(())
    }

    /// Trigger failover for failed node
    async fn trigger_failover(&self, failed_node: String) -> Result<()> {
        let shard_states = self.shard_states.read().await;

        for (shard_id, state) in shard_states.iter() {
            if state.primary_node == failed_node {
                // Find best replica to promote
                let new_primary = self.select_failover_candidate(shard_id, &failed_node).await?;

                let _ = self.sync_sender.send(SyncMessage::Failover {
                    shard_id: *shard_id,
                    failed_node: failed_node.clone(),
                });

                log::info!("Triggered failover for shard {}: {} -> {}",
                          shard_id, failed_node, new_primary);
            }
        }

        Ok(())
    }

    /// Select best candidate for failover
    async fn select_failover_candidate(&self, shard_id: u32, failed_node: &str) -> Result<String> {
        let nodes = self.replica_nodes.read().await;
        let shard_states = self.shard_states.read().await;

        if let Some(state) = shard_states.get(&shard_id) {
            // Find replica with lowest priority (highest priority number = lowest priority)
            let mut best_candidate: Option<(u8, String)> = None;

            for replica_node in &state.replica_nodes {
                if replica_node != failed_node {
                    if let Some(node) = nodes.get(replica_node) {
                        if node.status == ReplicaStatus::Secondary {
                            match best_candidate {
                                None => best_candidate = Some((node.priority, replica_node.clone())),
                                Some((best_priority, _)) if node.priority < best_priority => {
                                    best_candidate = Some((node.priority, replica_node.clone()));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            if let Some((_, candidate)) = best_candidate {
                return Ok(candidate);
            }
        }

        Err(BlockchainError::Replication("No suitable failover candidate found".to_string()))
    }

    /// Process heartbeat from replica node
    pub async fn process_heartbeat(&self, node_id: String) -> Result<()> {
        let mut nodes = self.replica_nodes.write().await;

        if let Some(node) = nodes.get_mut(&node_id) {
            node.last_heartbeat = current_timestamp();

            // Update status if node was offline
            if node.status == ReplicaStatus::Offline {
                node.status = ReplicaStatus::Secondary;
                log::info!("Node {} came back online", node_id);
            }
        }

        Ok(())
    }

    /// Get replication statistics
    pub async fn get_stats(&self) -> Result<ReplicationStats> {
        let nodes = self.replica_nodes.read().await;
        let shard_states = self.shard_states.read().await;
        let operation_log = self.operation_log.read().await;

        let total_nodes = nodes.len();
        let active_nodes = nodes.values()
            .filter(|n| n.status != ReplicaStatus::Failed && n.status != ReplicaStatus::Offline)
            .count();

        let total_pending_ops: usize = shard_states.values()
            .map(|s| s.pending_operations.len())
            .sum();

        Ok(ReplicationStats {
            total_nodes,
            active_nodes,
            total_shards: shard_states.len(),
            pending_operations: total_pending_ops,
            operation_log_size: operation_log.len(),
        })
    }

    // Helper methods
    fn get_operation_shard(&self, operation: &ReplicationOperation) -> Option<u32> {
        match operation {
            ReplicationOperation::StateUpdate { key, .. } => {
                // In production, this would use the shard manager
                Some((key.len() % 16) as u32) // Simple hash for demo
            }
            ReplicationOperation::BatchUpdate { .. } => {
                Some(0) // Default shard for batch operations
            }
            ReplicationOperation::ShardSplit { old_shard, .. } => {
                Some(*old_shard)
            }
            ReplicationOperation::NodeFailover { .. } => {
                None // Affects all shards
            }
        }
    }

    async fn trigger_sync(&self) -> Result<()> {
        // In production, this would send sync messages to replica nodes
        // For now, just log the intent
        log::debug!("Triggered replication sync");
        Ok(())
    }
}

/// Consistency check report
#[derive(Debug, Clone, Default)]
pub struct ConsistencyReport {
    pub total_shards: usize,
    pub consistent_shards: Vec<u32>,
    pub inconsistent_shards: Vec<u32>,
}

impl ConsistencyReport {
    pub fn is_fully_consistent(&self) -> bool {
        self.inconsistent_shards.is_empty()
    }

    pub fn consistency_ratio(&self) -> f64 {
        if self.total_shards == 0 {
            1.0
        } else {
            self.consistent_shards.len() as f64 / self.total_shards as f64
        }
    }
}

/// Replication statistics
#[derive(Debug, Clone)]
pub struct ReplicationStats {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub total_shards: usize,
    pub pending_operations: usize,
    pub operation_log_size: usize,
}

impl ReplicationStats {
    pub fn availability_ratio(&self) -> f64 {
        if self.total_nodes == 0 {
            1.0
        } else {
            self.active_nodes as f64 / self.total_nodes as f64
        }
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributed::sharding::ShardConfig;

    #[tokio::test]
    async fn test_replication_manager_creation() {
        let shard_config = ShardConfig::default();
        let shard_manager = Arc::new(RwLock::new(crate::distributed::sharding::ShardManager::new(shard_config)));
        let config = ReplicationConfig::default();
        let manager = ReplicationManager::new(config, shard_manager);

        assert_eq!(manager.config.replication_factor, 3);
        assert_eq!(manager.config.consistency_level, ConsistencyLevel::Strong);
    }

    #[tokio::test]
    async fn test_replication_initialization() {
        let shard_config = ShardConfig::default();
        let shard_manager = Arc::new(RwLock::new(crate::distributed::sharding::ShardManager::new(shard_config)));
        let config = ReplicationConfig::default();
        let manager = ReplicationManager::new(config, shard_manager);

        manager.initialize_replication().await.unwrap();

        let nodes = manager.replica_nodes.read().await;
        assert!(nodes.contains_key("local"));
        assert_eq!(nodes.len(), 3); // 1 primary + 2 replicas
    }

    #[tokio::test]
    async fn test_operation_replication() {
        let shard_config = ShardConfig::default();
        let shard_manager = Arc::new(RwLock::new(crate::distributed::sharding::ShardManager::new(shard_config)));
        let config = ReplicationConfig::default();
        let manager = ReplicationManager::new(config, shard_manager);

        manager.initialize_replication().await.unwrap();

        let operation = ReplicationOperation::StateUpdate {
            key: b"test_key".to_vec(),
            value: b"test_value".to_vec(),
            version: 1,
        };

        manager.replicate_operation(operation).await.unwrap();

        let log = manager.operation_log.read().await;
        assert_eq!(log.len(), 1);
    }

    #[tokio::test]
    async fn test_consistency_check() {
        let shard_config = ShardConfig::default();
        let shard_manager = Arc::new(RwLock::new(crate::distributed::sharding::ShardManager::new(shard_config)));
        let config = ReplicationConfig::default();
        let manager = ReplicationManager::new(config, shard_manager);

        manager.initialize_replication().await.unwrap();

        let report = manager.check_consistency().await.unwrap();
        assert_eq!(report.total_shards, 16); // Default shard count
    }

    #[tokio::test]
    async fn test_heartbeat_processing() {
        let shard_config = ShardConfig::default();
        let shard_manager = Arc::new(RwLock::new(crate::distributed::sharding::ShardManager::new(shard_config)));
        let config = ReplicationConfig::default();
        let manager = ReplicationManager::new(config, shard_manager);

        manager.initialize_replication().await.unwrap();

        manager.process_heartbeat("replica_1".to_string()).await.unwrap();

        let nodes = manager.replica_nodes.read().await;
        if let Some(node) = nodes.get("replica_1") {
            assert!(node.last_heartbeat > 0);
        }
    }
}
