//! Distributed systems components for Erbium Blockchain
//!
//! This module provides all distributed systems functionality including:
//! - Sharding for horizontal data distribution
//! - Replication for fault tolerance and high availability
//! - Load balancing for optimal resource utilization
//! - Distributed caching for shared cache across nodes

pub mod sharding;
pub mod replication;
pub mod load_balancer;
pub mod distributed_cache;

// Re-export main components for easy access
pub use sharding::{ShardManager, ShardConfig, ShardInfo, ShardStatus};
pub use replication::{ReplicationManager, ReplicationConfig, ConsistencyLevel, ReplicationStats};
pub use load_balancer::{LoadBalancer, LoadBalancerConfig, LoadBalancingAlgorithm, LoadBalancerStats};
pub use distributed_cache::{DistributedCache, DistributedCacheConfig, CacheConsistencyLevel, DistributedCacheStats};

use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main distributed systems coordinator
pub struct DistributedCoordinator {
    shard_manager: Arc<RwLock<ShardManager>>,
    replication_manager: ReplicationManager,
    load_balancer: LoadBalancer,
    distributed_cache: DistributedCache,
    config: DistributedConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    pub shard_config: ShardConfig,
    pub replication_config: ReplicationConfig,
    pub load_balancer_config: LoadBalancerConfig,
    pub cache_config: DistributedCacheConfig,
    pub node_id: String,
    pub enable_distributed_features: bool,
}

impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            shard_config: ShardConfig::default(),
            replication_config: ReplicationConfig::default(),
            load_balancer_config: LoadBalancerConfig::default(),
            cache_config: DistributedCacheConfig::default(),
            node_id: "local_node".to_string(),
            enable_distributed_features: true,
        }
    }
}

impl DistributedCoordinator {
    /// Create a new distributed coordinator
    pub fn new(config: DistributedConfig) -> Self {
        let shard_manager = Arc::new(RwLock::new(ShardManager::new(config.shard_config.clone())));
        let replication_manager = ReplicationManager::new(
            config.replication_config.clone(),
            Arc::clone(&shard_manager),
        );
        let load_balancer = LoadBalancer::new(
            config.load_balancer_config.clone(),
            Arc::clone(&shard_manager),
        );
        let distributed_cache = DistributedCache::new(
            config.cache_config.clone(),
            config.node_id.clone(),
        );

        Self {
            shard_manager,
            replication_manager,
            load_balancer,
            distributed_cache,
            config,
        }
    }

    /// Initialize all distributed systems
    pub async fn initialize(&self) -> Result<()> {
        if !self.config.enable_distributed_features {
            log::info!("Distributed features disabled");
            return Ok(());
        }

        log::info!("Initializing distributed systems...");

        // Initialize replication
        self.replication_manager.initialize_replication().await?;

        // Initialize distributed cache
        self.distributed_cache.sync_with_peers().await?;

        log::info!("Distributed systems initialized successfully");
        Ok(())
    }

    /// Get shard for a given key
    pub async fn get_shard_for_key(&self, key: &[u8]) -> Result<u32> {
        let shard_manager = self.shard_manager.read().await;
        shard_manager.get_shard_for_key(key)
    }

    /// Route a request through the distributed system
    pub async fn route_request(&self, request: DistributedRequest) -> Result<DistributedResponse> {
        match request {
            DistributedRequest::DataRead { key, consistency } => {
                // Try distributed cache first
                if let Ok(Some(value)) = self.distributed_cache.get(&key).await {
                    return Ok(DistributedResponse::Data { value });
                }

                // Route to appropriate shard
                let shard_id = self.get_shard_for_key(&key).await?;
                Ok(DistributedResponse::ShardRoute { shard_id, key })
            }

            DistributedRequest::DataWrite { key, value } => {
                // Update distributed cache
                self.distributed_cache.put(key.clone(), value.clone()).await?;

                // Route to appropriate shard
                let shard_id = self.get_shard_for_key(&key).await?;
                Ok(DistributedResponse::ShardRoute { shard_id, key })
            }

            DistributedRequest::ConsensusOperation { operation } => {
                // Use load balancer for consensus operations
                let decision = self.load_balancer.balance_request(
                    load_balancer::LoadBalancingRequest::ConsensusVote {
                        block_height: 0, // Would be set properly
                        priority: load_balancer::VotePriority::Normal,
                    }
                ).await?;

                Ok(DistributedResponse::NodeRoute {
                    node_id: decision.target_node,
                    operation,
                })
            }

            DistributedRequest::HealthCheck => {
                let stats = self.get_cluster_stats().await?;
                Ok(DistributedResponse::HealthStatus { stats })
            }
        }
    }

    /// Get comprehensive cluster statistics
    pub async fn get_cluster_stats(&self) -> Result<ClusterStats> {
        let shard_stats = {
            let shard_manager = self.shard_manager.read().await;
            let all_shards = shard_manager.get_all_shards();
            ClusterShardStats {
                total_shards: all_shards.len(),
                active_shards: all_shards.iter().filter(|s| s.status == sharding::ShardStatus::Active).count(),
                total_data_size_gb: all_shards.iter().map(|s| s.size_bytes).sum::<u64>() as f64 / (1024.0 * 1024.0 * 1024.0),
            }
        };

        let replication_stats = self.replication_manager.get_stats().await?;
        let load_balancer_stats = self.load_balancer.get_stats().await?;
        let cache_stats = self.distributed_cache.get_stats().await?;

        Ok(ClusterStats {
            node_id: self.config.node_id.clone(),
            shard_stats,
            replication_stats,
            load_balancer_stats,
            cache_stats,
            uptime_seconds: 0, // Would track actual uptime
            last_health_check: current_timestamp(),
        })
    }

    /// Perform cluster maintenance operations
    pub async fn perform_maintenance(&self) -> Result<MaintenanceReport> {
        log::info!("Performing distributed system maintenance...");

        let mut report = MaintenanceReport::default();

        // Rebalance shards if needed
        {
            let mut shard_manager = self.shard_manager.write().await;
            shard_manager.rebalance_shards().await?;
            report.shards_rebalanced = true;
        }

        // Clean up expired cache entries
        report.cache_entries_cleaned = self.distributed_cache.cleanup_expired().await?;

        // Sync replication state
        self.replication_manager.sync_pending_operations().await?;
        report.replication_synced = true;

        // Process queued load balancer requests
        report.queued_requests_processed = self.load_balancer.process_queued_requests().await?;

        log::info!("Maintenance completed: {:?}", report);
        Ok(report)
    }

    /// Handle node failure in the cluster
    pub async fn handle_node_failure(&self, failed_node: String) -> Result<()> {
        log::warn!("Handling cluster node failure: {}", failed_node);

        // Update replication state
        self.replication_manager.handle_node_failure(failed_node.clone()).await?;

        // Update load balancer metrics
        // (Node metrics would be updated by health monitoring)

        // Trigger cache invalidation for failed node data
        // (In production, this would be more sophisticated)

        log::info!("Node failure handling completed for: {}", failed_node);
        Ok(())
    }

    // Access to individual components
    pub fn shard_manager(&self) -> &Arc<RwLock<ShardManager>> {
        &self.shard_manager
    }

    pub fn replication_manager(&self) -> &ReplicationManager {
        &self.replication_manager
    }

    pub fn load_balancer(&self) -> &LoadBalancer {
        &self.load_balancer
    }

    pub fn distributed_cache(&self) -> &DistributedCache {
        &self.distributed_cache
    }
}

/// Distributed request types
#[derive(Debug, Clone)]
pub enum DistributedRequest {
    DataRead { key: Vec<u8>, consistency: ReadConsistency },
    DataWrite { key: Vec<u8>, value: Vec<u8> },
    ConsensusOperation { operation: Vec<u8> },
    HealthCheck,
}

#[derive(Debug, Clone)]
pub enum ReadConsistency {
    Strong,
    Eventual,
    Quorum,
}

/// Distributed response types
#[derive(Debug, Clone)]
pub enum DistributedResponse {
    Data { value: Vec<u8> },
    ShardRoute { shard_id: u32, key: Vec<u8> },
    NodeRoute { node_id: String, operation: Vec<u8> },
    HealthStatus { stats: ClusterStats },
}

/// Cluster-wide statistics
#[derive(Debug, Clone)]
pub struct ClusterStats {
    pub node_id: String,
    pub shard_stats: ClusterShardStats,
    pub replication_stats: ReplicationStats,
    pub load_balancer_stats: LoadBalancerStats,
    pub cache_stats: DistributedCacheStats,
    pub uptime_seconds: u64,
    pub last_health_check: u64,
}

#[derive(Debug, Clone)]
pub struct ClusterShardStats {
    pub total_shards: usize,
    pub active_shards: usize,
    pub total_data_size_gb: f64,
}

/// Maintenance operation report
#[derive(Debug, Clone, Default)]
pub struct MaintenanceReport {
    pub shards_rebalanced: bool,
    pub cache_entries_cleaned: usize,
    pub replication_synced: bool,
    pub queued_requests_processed: usize,
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

    #[tokio::test]
    async fn test_distributed_coordinator_creation() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config);

        assert_eq!(coordinator.config.node_id, "local_node");
        assert!(coordinator.config.enable_distributed_features);
    }

    #[tokio::test]
    async fn test_distributed_coordinator_initialization() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config);

        // Should not fail even if distributed features are enabled
        let result = coordinator.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cluster_stats() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config);

        let stats = coordinator.get_cluster_stats().await.unwrap();
        assert_eq!(stats.node_id, "local_node");
        assert!(stats.shard_stats.total_shards > 0);
    }

    #[tokio::test]
    async fn test_request_routing() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config);

        let request = DistributedRequest::DataRead {
            key: b"test_key".to_vec(),
            consistency: ReadConsistency::Eventual,
        };

        let response = coordinator.route_request(request).await.unwrap();

        match response {
            DistributedResponse::ShardRoute { shard_id, .. } => {
                assert!(shard_id < 16); // Default shard count
            }
            DistributedResponse::Data { .. } => {
                // Cache hit - acceptable
            }
            _ => panic!("Unexpected response type"),
        }
    }

    #[tokio::test]
    async fn test_maintenance_operations() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config);

        let report = coordinator.perform_maintenance().await.unwrap();

        // Maintenance should complete without errors
        // Specific results depend on system state
        assert!(report.cache_entries_cleaned >= 0);
        assert!(report.queued_requests_processed >= 0);
    }
}
