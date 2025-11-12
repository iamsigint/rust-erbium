//! Intelligent load balancing system for distributed blockchain operations
//!
//! This module provides intelligent load balancing across multiple nodes,
//! optimizing resource utilization and response times.

use crate::utils::error::{Result, BlockchainError};
use crate::distributed::sharding::{ShardManager, ShardConfig};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, BTreeMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// Load balancing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub algorithm: LoadBalancingAlgorithm,
    pub health_check_interval_seconds: u64,
    pub max_connections_per_node: usize,
    pub load_threshold_high: f64, // When to trigger load shedding (0.0-1.0)
    pub load_threshold_low: f64,  // When to accept more load
    pub enable_auto_scaling: bool,
    pub scaling_cooldown_seconds: u64,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            algorithm: LoadBalancingAlgorithm::WeightedRoundRobin,
            health_check_interval_seconds: 30,
            max_connections_per_node: 1000,
            load_threshold_high: 0.8,
            load_threshold_low: 0.3,
            enable_auto_scaling: true,
            scaling_cooldown_seconds: 300, // 5 minutes
        }
    }
}

/// Load balancing algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingAlgorithm {
    /// Round-robin distribution
    RoundRobin,
    /// Weighted round-robin based on node capacity
    WeightedRoundRobin,
    /// Least connections algorithm
    LeastConnections,
    /// Resource-based balancing (CPU, memory, network)
    ResourceBased,
    /// Geographic load balancing
    Geographic,
}

/// Node load metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeLoadMetrics {
    pub node_id: String,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub network_bandwidth_mbps: f64,
    pub active_connections: usize,
    pub queue_depth: usize,
    pub response_time_ms: f64,
    pub last_updated: u64,
    pub health_status: NodeHealthStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Offline,
}

/// Load balancing request types
#[derive(Debug, Clone)]
pub enum LoadBalancingRequest {
    Transaction {
        data: Vec<u8>,
        estimated_complexity: u8, // 1-10 scale
    },
    Query {
        key: Vec<u8>,
        read_consistency: ReadConsistency,
    },
    ShardOperation {
        shard_id: u32,
        operation_type: ShardOperationType,
    },
    ConsensusVote {
        block_height: u64,
        priority: VotePriority,
    },
}

#[derive(Debug, Clone)]
pub enum ReadConsistency {
    Strong,
    Eventual,
    Quorum,
}

#[derive(Debug, Clone)]
pub enum ShardOperationType {
    Read,
    Write,
    Rebalance,
    Repair,
}

#[derive(Debug, Clone)]
pub enum VotePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Load balancing decision
#[derive(Debug, Clone)]
pub struct LoadBalancingDecision {
    pub target_node: String,
    pub estimated_latency_ms: f64,
    pub load_after_request: f64,
    pub alternative_nodes: Vec<String>,
}

/// Main load balancer
pub struct LoadBalancer {
    config: LoadBalancerConfig,
    shard_manager: Arc<RwLock<ShardManager>>,
    node_metrics: Arc<RwLock<HashMap<String, NodeLoadMetrics>>>,
    request_queue: Arc<RwLock<VecDeque<QueuedRequest>>>,
    round_robin_index: Arc<RwLock<usize>>,
    last_scaling_event: Arc<RwLock<u64>>,
}

#[derive(Debug, Clone)]
struct QueuedRequest {
    request: LoadBalancingRequest,
    enqueue_time: u64,
    priority: u8,
    source_node: String,
}

impl LoadBalancer {
    pub fn new(
        config: LoadBalancerConfig,
        shard_manager: Arc<RwLock<ShardManager>>,
    ) -> Self {
        Self {
            config,
            shard_manager,
            node_metrics: Arc::new(RwLock::new(HashMap::new())),
            request_queue: Arc::new(RwLock::new(VecDeque::new())),
            round_robin_index: Arc::new(RwLock::new(0)),
            last_scaling_event: Arc::new(RwLock::new(0)),
        }
    }

    /// Balance a load balancing request
    pub async fn balance_request(&self, request: LoadBalancingRequest) -> Result<LoadBalancingDecision> {
        // Check if request should be queued due to high load
        if self.should_queue_request().await? {
            self.queue_request(request).await?;
            return Err(BlockchainError::LoadBalancing("Request queued due to high load".to_string()));
        }

        // Get candidate nodes based on request type
        let candidate_nodes = self.get_candidate_nodes(&request).await?;

        if candidate_nodes.is_empty() {
            return Err(BlockchainError::LoadBalancing("No available nodes for request".to_string()));
        }

        // Apply load balancing algorithm
        let decision = match self.config.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                self.round_robin_balance(&candidate_nodes, &request).await?
            }
            LoadBalancingAlgorithm::WeightedRoundRobin => {
                self.weighted_round_robin_balance(&candidate_nodes, &request).await?
            }
            LoadBalancingAlgorithm::LeastConnections => {
                self.least_connections_balance(&candidate_nodes, &request).await?
            }
            LoadBalancingAlgorithm::ResourceBased => {
                self.resource_based_balance(&candidate_nodes, &request).await?
            }
            LoadBalancingAlgorithm::Geographic => {
                self.geographic_balance(&candidate_nodes, &request).await?
            }
        };

        // Update load metrics for target node
        self.update_node_load(&decision.target_node, &request).await?;

        Ok(decision)
    }

    /// Update node load metrics
    pub async fn update_node_metrics(&self, node_id: String, metrics: NodeLoadMetrics) -> Result<()> {
        let mut node_metrics = self.node_metrics.write().await;
        node_metrics.insert(node_id, metrics);

        // Check for auto-scaling triggers
        self.check_auto_scaling_triggers().await?;

        Ok(())
    }

    /// Get load balancing statistics
    pub async fn get_stats(&self) -> Result<LoadBalancerStats> {
        let node_metrics = self.node_metrics.read().await;
        let request_queue = self.request_queue.read().await;

        let total_nodes = node_metrics.len();
        let healthy_nodes = node_metrics.values()
            .filter(|m| m.health_status == NodeHealthStatus::Healthy)
            .count();

        let avg_cpu_usage = if !node_metrics.is_empty() {
            node_metrics.values().map(|m| m.cpu_usage_percent).sum::<f64>() / node_metrics.len() as f64
        } else {
            0.0
        };

        let avg_memory_usage = if !node_metrics.is_empty() {
            node_metrics.values().map(|m| m.memory_usage_percent).sum::<f64>() / node_metrics.len() as f64
        } else {
            0.0
        };

        Ok(LoadBalancerStats {
            total_nodes,
            healthy_nodes,
            queued_requests: request_queue.len(),
            average_cpu_usage: avg_cpu_usage,
            average_memory_usage: avg_memory_usage,
            total_active_connections: node_metrics.values().map(|m| m.active_connections).sum(),
        })
    }

    /// Process queued requests when load decreases
    pub async fn process_queued_requests(&self) -> Result<usize> {
        let mut processed = 0;
        let mut request_queue = self.request_queue.write().await;

        // Process requests in priority order
        while let Some(queued) = request_queue.front() {
            if self.should_process_queued_request().await? {
                if let Some(queued_request) = request_queue.pop_front() {
                    // Try to balance the queued request
                    match self.balance_request(queued_request.request).await {
                        Ok(_) => processed += 1,
                        Err(_) => {
                            // If balancing fails, put back in queue
                            request_queue.push_front(queued_request);
                            break;
                        }
                    }
                }
            } else {
                break;
            }
        }

        Ok(processed)
    }

    // Load balancing algorithms
    async fn round_robin_balance(
        &self,
        candidate_nodes: &[String],
        _request: &LoadBalancingRequest,
    ) -> Result<LoadBalancingDecision> {
        let mut index = self.round_robin_index.write().await;
        let target_index = *index % candidate_nodes.len();
        *index += 1;

        let target_node = candidate_nodes[target_index].clone();

        Ok(LoadBalancingDecision {
            target_node,
            estimated_latency_ms: 50.0, // Base estimate
            load_after_request: 0.1, // Small load increase
            alternative_nodes: candidate_nodes.iter()
                .enumerate()
                .filter(|(i, _)| *i != target_index)
                .take(2)
                .map(|(_, node)| node.clone())
                .collect(),
        })
    }

    async fn weighted_round_robin_balance(
        &self,
        candidate_nodes: &[String],
        request: &LoadBalancingRequest,
    ) -> Result<LoadBalancingDecision> {
        let node_metrics = self.node_metrics.read().await;

        // Calculate weights based on available capacity
        let mut node_weights: Vec<(String, f64)> = Vec::new();

        for node_id in candidate_nodes {
            let weight = if let Some(metrics) = node_metrics.get(node_id) {
                // Weight based on inverse of current load
                let load_factor = (metrics.cpu_usage_percent + metrics.memory_usage_percent) / 200.0;
                (1.0 - load_factor).max(0.1) // Minimum weight of 0.1
            } else {
                1.0 // Default weight for unknown nodes
            };

            // Adjust weight based on request type
            let adjusted_weight = self.adjust_weight_for_request(weight, request);
            node_weights.push((node_id.clone(), adjusted_weight));
        }

        // Select node with highest weight
        let (target_node, _) = node_weights.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .ok_or_else(|| BlockchainError::LoadBalancing("No nodes available".to_string()))?;

        Ok(LoadBalancingDecision {
            target_node: target_node.clone(),
            estimated_latency_ms: 30.0,
            load_after_request: 0.05,
            alternative_nodes: node_weights.into_iter()
                .filter(|(node, _)| node != target_node)
                .take(2)
                .map(|(node, _)| node)
                .collect(),
        })
    }

    async fn least_connections_balance(
        &self,
        candidate_nodes: &[String],
        _request: &LoadBalancingRequest,
    ) -> Result<LoadBalancingDecision> {
        let node_metrics = self.node_metrics.read().await;

        let (target_node, _) = candidate_nodes.iter()
            .filter_map(|node_id| {
                node_metrics.get(node_id)
                    .map(|metrics| (node_id.clone(), metrics.active_connections))
            })
            .min_by_key(|(_, connections)| *connections)
            .ok_or_else(|| BlockchainError::LoadBalancing("No nodes with connection metrics".to_string()))?;

        Ok(LoadBalancingDecision {
            target_node,
            estimated_latency_ms: 25.0,
            load_after_request: 0.02,
            alternative_nodes: candidate_nodes.iter()
                .filter(|node| *node != target_node)
                .take(2)
                .cloned()
                .collect(),
        })
    }

    async fn resource_based_balance(
        &self,
        candidate_nodes: &[String],
        request: &LoadBalancingRequest,
    ) -> Result<LoadBalancingDecision> {
        let node_metrics = self.node_metrics.read().await;

        // Score nodes based on multiple resource metrics
        let mut node_scores: Vec<(String, f64)> = Vec::new();

        for node_id in candidate_nodes {
            let score = if let Some(metrics) = node_metrics.get(node_id) {
                // Composite score: lower is better
                let cpu_score = metrics.cpu_usage_percent / 100.0;
                let mem_score = metrics.memory_usage_percent / 100.0;
                let conn_score = metrics.active_connections as f64 / self.config.max_connections_per_node as f64;

                // Weighted average
                (cpu_score * 0.4 + mem_score * 0.4 + conn_score * 0.2).min(1.0)
            } else {
                0.5 // Neutral score for unknown nodes
            };

            node_scores.push((node_id.clone(), score));
        }

        // Select node with lowest score (best resource availability)
        let (target_node, _) = node_scores.into_iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .ok_or_else(|| BlockchainError::LoadBalancing("No suitable nodes found".to_string()))?;

        Ok(LoadBalancingDecision {
            target_node,
            estimated_latency_ms: 20.0,
            load_after_request: 0.01,
            alternative_nodes: candidate_nodes.iter()
                .filter(|node| *node != target_node)
                .take(2)
                .cloned()
                .collect(),
        })
    }

    async fn geographic_balance(
        &self,
        candidate_nodes: &[String],
        _request: &LoadBalancingRequest,
    ) -> Result<LoadBalancingDecision> {
        // For now, fall back to round-robin
        // In production, this would consider geographic location
        self.round_robin_balance(candidate_nodes, _request).await
    }

    // Helper methods
    async fn get_candidate_nodes(&self, request: &LoadBalancingRequest) -> Result<Vec<String>> {
        match request {
            LoadBalancingRequest::Transaction { .. } => {
                // Any healthy node can handle transactions
                self.get_healthy_nodes().await
            }
            LoadBalancingRequest::Query { key, .. } => {
                // Route to shard owner
                let shard_manager = self.shard_manager.read().await;
                let shard_id = shard_manager.get_shard_for_key(key)?;
                let shard_info = shard_manager.get_shard_info(shard_id);

                if let Some(shard) = shard_info {
                    Ok(vec![shard.node_id.clone()])
                } else {
                    Err(BlockchainError::LoadBalancing("Shard not found".to_string()))
                }
            }
            LoadBalancingRequest::ShardOperation { shard_id, .. } => {
                // Route to shard owner and replicas
                let shard_manager = self.shard_manager.read().await;
                let mut nodes = vec![];
                if let Some(shard) = shard_manager.get_shard_info(*shard_id) {
                    nodes.push(shard.node_id.clone());
                }
                nodes.extend(shard_manager.get_replica_nodes(*shard_id));
                Ok(nodes)
            }
            LoadBalancingRequest::ConsensusVote { .. } => {
                // All validator nodes
                self.get_validator_nodes().await
            }
        }
    }

    async fn get_healthy_nodes(&self) -> Result<Vec<String>> {
        let node_metrics = self.node_metrics.read().await;
        Ok(node_metrics.iter()
            .filter(|(_, metrics)| metrics.health_status == NodeHealthStatus::Healthy)
            .map(|(node_id, _)| node_id.clone())
            .collect())
    }

    async fn get_validator_nodes(&self) -> Result<Vec<String>> {
        // For now, return all healthy nodes
        // In production, this would filter for validator nodes only
        self.get_healthy_nodes().await
    }

    async fn should_queue_request(&self) -> Result<bool> {
        let node_metrics = self.node_metrics.read().await;

        // Check if any node is above high load threshold
        for metrics in node_metrics.values() {
            let load_factor = (metrics.cpu_usage_percent + metrics.memory_usage_percent) / 200.0;
            if load_factor > self.config.load_threshold_high {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn should_process_queued_request(&self) -> Result<bool> {
        let node_metrics = self.node_metrics.read().await;

        // Check if any node is below low load threshold
        for metrics in node_metrics.values() {
            let load_factor = (metrics.cpu_usage_percent + metrics.memory_usage_percent) / 200.0;
            if load_factor < self.config.load_threshold_low {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn queue_request(&self, request: LoadBalancingRequest) -> Result<()> {
        let queued_request = QueuedRequest {
            request,
            enqueue_time: current_timestamp(),
            priority: 1, // Default priority
            source_node: "unknown".to_string(), // Would be set by caller
        };

        let mut request_queue = self.request_queue.write().await;
        request_queue.push_back(queued_request);

        Ok(())
    }

    async fn update_node_load(&self, node_id: &str, request: &LoadBalancingRequest) -> Result<()> {
        let mut node_metrics = self.node_metrics.write().await;

        if let Some(metrics) = node_metrics.get_mut(node_id) {
            // Update connection count and queue depth
            metrics.active_connections += 1;

            // Estimate load increase based on request type
            let load_increase = match request {
                LoadBalancingRequest::Transaction { estimated_complexity, .. } => {
                    *estimated_complexity as f64 / 100.0
                }
                LoadBalancingRequest::Query { .. } => 0.01,
                LoadBalancingRequest::ShardOperation { operation_type, .. } => {
                    match operation_type {
                        ShardOperationType::Read => 0.02,
                        ShardOperationType::Write => 0.05,
                        ShardOperationType::Rebalance => 0.1,
                        ShardOperationType::Repair => 0.08,
                    }
                }
                LoadBalancingRequest::ConsensusVote { priority, .. } => {
                    match priority {
                        VotePriority::Low => 0.01,
                        VotePriority::Normal => 0.03,
                        VotePriority::High => 0.05,
                        VotePriority::Critical => 0.08,
                    }
                }
            };

            metrics.cpu_usage_percent = (metrics.cpu_usage_percent + load_increase * 10.0).min(100.0);
            metrics.memory_usage_percent = (metrics.memory_usage_percent + load_increase * 5.0).min(100.0);
        }

        Ok(())
    }

    async fn check_auto_scaling_triggers(&self) -> Result<()> {
        if !self.config.enable_auto_scaling {
            return Ok(());
        }

        let last_scaling = *self.last_scaling_event.read().await;
        let now = current_timestamp();

        if now - last_scaling < self.config.scaling_cooldown_seconds {
            return Ok(); // Still in cooldown
        }

        let node_metrics = self.node_metrics.read().await;
        let high_load_nodes = node_metrics.values()
            .filter(|m| {
                let load_factor = (m.cpu_usage_percent + m.memory_usage_percent) / 200.0;
                load_factor > self.config.load_threshold_high
            })
            .count();

        if high_load_nodes > node_metrics.len() / 2 {
            // More than half the nodes are overloaded
            log::warn!("High load detected on {} nodes, triggering auto-scaling", high_load_nodes);
            *self.last_scaling_event.write().await = now;

            // In production, this would trigger auto-scaling
            // For now, just log the event
        }

        Ok(())
    }

    fn adjust_weight_for_request(&self, base_weight: f64, request: &LoadBalancingRequest) -> f64 {
        match request {
            LoadBalancingRequest::Transaction { estimated_complexity, .. } => {
                // Prefer nodes for complex transactions
                if *estimated_complexity > 7 {
                    base_weight * 1.2 // Boost weight for high-capacity nodes
                } else {
                    base_weight
                }
            }
            LoadBalancingRequest::Query { read_consistency, .. } => {
                match read_consistency {
                    ReadConsistency::Strong => base_weight * 0.8, // Prefer stable nodes
                    ReadConsistency::Eventual => base_weight * 1.1, // Can use more loaded nodes
                    ReadConsistency::Quorum => base_weight * 0.9,
                }
            }
            LoadBalancingRequest::ConsensusVote { priority, .. } => {
                match priority {
                    VotePriority::Critical => base_weight * 1.5, // Highest priority
                    VotePriority::High => base_weight * 1.2,
                    VotePriority::Normal => base_weight,
                    VotePriority::Low => base_weight * 0.8,
                }
            }
            _ => base_weight,
        }
    }
}

/// Load balancer statistics
#[derive(Debug, Clone)]
pub struct LoadBalancerStats {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub queued_requests: usize,
    pub average_cpu_usage: f64,
    pub average_memory_usage: f64,
    pub total_active_connections: usize,
}

impl LoadBalancerStats {
    pub fn cluster_health_score(&self) -> f64 {
        if self.total_nodes == 0 {
            return 0.0;
        }

        let health_ratio = self.healthy_nodes as f64 / self.total_nodes as f64;
        let load_factor = (self.average_cpu_usage + self.average_memory_usage) / 200.0;
        let queue_factor = if self.queued_requests > 100 { 0.5 } else { 1.0 };

        (health_ratio * (1.0 - load_factor) * queue_factor).max(0.0)
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
    async fn test_load_balancer_creation() {
        let shard_config = ShardConfig::default();
        let shard_manager = Arc::new(RwLock::new(crate::distributed::sharding::ShardManager::new(shard_config)));
        let config = LoadBalancerConfig::default();
        let balancer = LoadBalancer::new(config, shard_manager);

        assert_eq!(balancer.config.algorithm, LoadBalancingAlgorithm::WeightedRoundRobin);
    }

    #[tokio::test]
    async fn test_round_robin_balancing() {
        let shard_config = ShardConfig::default();
        let shard_manager = Arc::new(RwLock::new(crate::distributed::sharding::ShardManager::new(shard_config)));
        let config = LoadBalancerConfig::default();
        let balancer = LoadBalancer::new(config, shard_manager);

        // Add some test nodes
        balancer.update_node_metrics("node1".to_string(), NodeLoadMetrics {
            node_id: "node1".to_string(),
            cpu_usage_percent: 50.0,
            memory_usage_percent: 50.0,
            network_bandwidth_mbps: 100.0,
            active_connections: 100,
            queue_depth: 10,
            response_time_ms: 50.0,
            last_updated: current_timestamp(),
            health_status: NodeHealthStatus::Healthy,
        }).await.unwrap();

        balancer.update_node_metrics("node2".to_string(), NodeLoadMetrics {
            node_id: "node2".to_string(),
            cpu_usage_percent: 30.0,
            memory_usage_percent: 40.0,
            network_bandwidth_mbps: 100.0,
            active_connections: 80,
            queue_depth: 5,
            response_time_ms: 40.0,
            last_updated: current_timestamp(),
            health_status: NodeHealthStatus::Healthy,
        }).await.unwrap();

        let request = LoadBalancingRequest::Transaction {
            data: vec![1, 2, 3],
            estimated_complexity: 5,
        };

        let decision = balancer.balance_request(request).await.unwrap();
        assert!(decision.target_node.starts_with("node"));
        assert!(!decision.alternative_nodes.is_empty());
    }

    #[tokio::test]
    async fn test_weighted_round_robin() {
        let shard_config = ShardConfig::default();
        let shard_manager = Arc::new(RwLock::new(crate::distributed::sharding::ShardManager::new(shard_config)));
        let mut config = LoadBalancerConfig::default();
        config.algorithm = LoadBalancingAlgorithm::WeightedRoundRobin;
        let balancer = LoadBalancer::new(config, shard_manager);

        // Add test nodes with different loads
        balancer.update_node_metrics("low_load".to_string(), NodeLoadMetrics {
            node_id: "low_load".to_string(),
            cpu_usage_percent: 20.0,
            memory_usage_percent: 20.0,
            network_bandwidth_mbps: 100.0,
            active_connections: 50,
            queue_depth: 2,
            response_time_ms: 30.0,
            last_updated: current_timestamp(),
            health_status: NodeHealthStatus::Healthy,
        }).await.unwrap();

        balancer.update_node_metrics("high_load".to_string(), NodeLoadMetrics {
            node_id: "high_load".to_string(),
            cpu_usage_percent: 80.0,
            memory_usage_percent: 80.0,
            network_bandwidth_mbps: 100.0,
            active_connections: 200,
            queue_depth: 20,
            response_time_ms: 100.0,
            last_updated: current_timestamp(),
            health_status: NodeHealthStatus::Healthy,
        }).await.unwrap();

        let request = LoadBalancingRequest::Transaction {
            data: vec![1, 2, 3],
            estimated_complexity: 5,
        };

        let decision = balancer.balance_request(request).await.unwrap();
        // Should prefer the low load node
        assert_eq!(decision.target_node, "low_load");
    }

    #[tokio::test]
    async fn test_load_balancer_stats() {
        let shard_config = ShardConfig::default();
        let shard_manager = Arc::new(RwLock::new(crate::distributed::sharding::ShardManager::new(shard_config)));
        let config = LoadBalancerConfig::default();
        let balancer = LoadBalancer::new(config, shard_manager);

        // Add test metrics
        balancer.update_node_metrics("node1".to_string(), NodeLoadMetrics {
            node_id: "node1".to_string(),
            cpu_usage_percent: 60.0,
            memory_usage_percent: 70.0,
            network_bandwidth_mbps: 100.0,
            active_connections: 150,
            queue_depth: 10,
            response_time_ms: 50.0,
            last_updated: current_timestamp(),
            health_status: NodeHealthStatus::Healthy,
        }).await.unwrap();

        let stats = balancer.get_stats().await.unwrap();
        assert_eq!(stats.total_nodes, 1);
        assert_eq!(stats.healthy_nodes, 1);
        assert_eq!(stats.average_cpu_usage, 60.0);
        assert_eq!(stats.average_memory_usage, 70.0);
    }
}
