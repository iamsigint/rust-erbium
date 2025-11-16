//! Real-time cluster performance monitoring
//!
//! This module provides comprehensive monitoring of cluster performance metrics
//! including CPU, memory, network, consensus, and storage metrics across all nodes.

use crate::analytics::{AlertSeverity, AlertType};
use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Cluster monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub collection_interval_seconds: u64,
    pub metrics_retention_hours: u64,
    pub enable_detailed_logging: bool,
    pub alert_on_thresholds: bool,
    pub node_health_check_interval_seconds: u64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            collection_interval_seconds: 30,
            metrics_retention_hours: 24,
            enable_detailed_logging: true,
            alert_on_thresholds: true,
            node_health_check_interval_seconds: 60,
        }
    }
}

/// Comprehensive cluster metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMetrics {
    pub timestamp: u64,
    pub node_id: String,
    pub region: String,

    // System metrics
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub network_bandwidth_mbps: f64,

    // Network metrics
    pub active_connections: usize,
    pub average_network_latency_ms: f64,
    pub packets_dropped_percent: f64,

    // Consensus metrics
    pub consensus_rounds_per_second: f64,
    pub average_block_time_ms: f64,
    pub validator_participation_percent: f64,
    pub consensus_delay_ms: f64,

    // Storage metrics
    pub storage_operations_per_second: f64,
    pub cache_hit_ratio: f64,
    pub merkle_tree_size_mb: f64,

    // Business metrics
    pub transactions_per_second: f64,
    pub average_transaction_fee: f64,
    pub queue_depth: usize,
    pub error_rate: f64,

    // Security metrics
    pub failed_auth_attempts: u64,
    pub anomaly_score: f64,
    pub security_events_count: u64,

    // Distributed system metrics
    pub shard_balance_score: f64,
    pub replication_lag_seconds: f64,
    pub load_balancer_efficiency: f64,
}

impl ClusterMetrics {
    /// Calculate overall system health score (0-100)
    pub fn system_health_score(&self) -> f64 {
        let cpu_score = 100.0 - self.cpu_usage_percent;
        let memory_score = 100.0 - self.memory_usage_percent;
        let network_score = 100.0 - (self.average_network_latency_ms / 10.0).min(100.0);
        let error_score = 100.0 - (self.error_rate * 1000.0).min(100.0);

        (cpu_score + memory_score + network_score + error_score) / 4.0
    }

    /// Check if metrics indicate critical issues
    pub fn has_critical_issues(&self) -> bool {
        self.cpu_usage_percent > 95.0
            || self.memory_usage_percent > 98.0
            || self.error_rate > 0.1
            || self.consensus_delay_ms > 10000.0
    }
}

/// Node health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeHealthStatus {
    Healthy,
    Degraded,
    Critical,
    Offline,
}

/// Individual node metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub node_id: String,
    pub metrics: ClusterMetrics,
    pub health_status: NodeHealthStatus,
    pub last_seen: u64,
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub enabled: bool,
    pub cooldown_seconds: u64,
    pub escalation_levels: Vec<AlertLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertLevel {
    pub threshold: f64,
    pub message: String,
    pub action: AlertAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertAction {
    Log,
    Email,
    Slack,
    PagerDuty,
    Shutdown,
}

/// Active alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveAlert {
    pub id: String,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub message: String,
    pub node_id: String,
    pub value: f64,
    pub threshold: f64,
    pub created_at: u64,
    pub last_triggered: u64,
    pub trigger_count: u64,
}

/// Performance baseline for anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    pub metric_name: String,
    pub mean: f64,
    pub std_dev: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub sample_count: u64,
    pub last_updated: u64,
}

/// Cluster monitoring system
pub struct ClusterMonitor {
    config: MonitoringConfig,
    _alert_config: AlertConfig,
    metrics_history: Arc<RwLock<HashMap<String, VecDeque<ClusterMetrics>>>>,
    node_health: Arc<RwLock<HashMap<String, NodeHealthStatus>>>,
    _active_alerts: Arc<RwLock<HashMap<String, ActiveAlert>>>,
    _performance_baselines: Arc<RwLock<HashMap<String, PerformanceBaseline>>>,
    is_monitoring: Arc<RwLock<bool>>,
    monitoring_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    _alert_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl ClusterMonitor {
    /// Create a new cluster monitor
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            _alert_config: AlertConfig {
                enabled: true,
                cooldown_seconds: 300,
                escalation_levels: vec![
                    AlertLevel {
                        threshold: 80.0,
                        message: "High resource usage detected".to_string(),
                        action: AlertAction::Log,
                    },
                    AlertLevel {
                        threshold: 95.0,
                        message: "Critical resource usage - immediate action required".to_string(),
                        action: AlertAction::PagerDuty,
                    },
                ],
            },
            metrics_history: Arc::new(RwLock::new(HashMap::new())),
            node_health: Arc::new(RwLock::new(HashMap::new())),
            _active_alerts: Arc::new(RwLock::new(HashMap::new())),
            _performance_baselines: Arc::new(RwLock::new(HashMap::new())),
            is_monitoring: Arc::new(RwLock::new(false)),
            monitoring_task: Arc::new(RwLock::new(None)),
            _alert_task: Arc::new(RwLock::new(None)),
        }
    }

    /// Start real-time monitoring
    pub async fn start_monitoring(&self) -> Result<()> {
        let mut is_monitoring = self.is_monitoring.write().await;
        if *is_monitoring {
            return Err(BlockchainError::Analytics(
                "Monitoring already running".to_string(),
            ));
        }

        *is_monitoring = true;
        drop(is_monitoring);

        let config = self.config.clone();
        let metrics_history = Arc::clone(&self.metrics_history);
        let node_health = Arc::clone(&self.node_health);
        let is_monitoring_flag = Arc::clone(&self.is_monitoring);

        let task = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(config.collection_interval_seconds));

            loop {
                interval.tick().await;

                let monitoring_active = *is_monitoring_flag.read().await;
                if !monitoring_active {
                    break;
                }

                // Collect metrics from all nodes (in production, this would query actual nodes)
                if let Err(e) = Self::collect_cluster_metrics(&metrics_history, &node_health).await
                {
                    log::error!("Failed to collect cluster metrics: {:?}", e);
                }

                // Clean up old metrics
                Self::cleanup_old_metrics(&metrics_history, config.metrics_retention_hours).await;

                // Update node health status
                Self::update_node_health_status(&node_health, &metrics_history).await;
            }
        });

        let mut monitoring_task = self.monitoring_task.write().await;
        *monitoring_task = Some(task);

        log::info!("Real-time cluster monitoring started");
        Ok(())
    }

    /// Stop monitoring
    pub async fn stop_monitoring(&self) -> Result<()> {
        let mut is_monitoring = self.is_monitoring.write().await;
        *is_monitoring = false;
        drop(is_monitoring);

        if let Some(task) = self.monitoring_task.write().await.take() {
            task.abort();
        }

        log::info!("Cluster monitoring stopped");
        Ok(())
    }

    /// Record metrics for a specific node
    pub async fn record_metrics(&self, metrics: ClusterMetrics) -> Result<()> {
        let mut history = self.metrics_history.write().await;

        let node_history = history.entry(metrics.node_id.clone()).or_insert_with(|| {
            VecDeque::with_capacity(
                (self.config.metrics_retention_hours * 3600
                    / self.config.collection_interval_seconds) as usize,
            )
        });

        node_history.push_back(metrics);

        // Maintain capacity
        let max_entries = (self.config.metrics_retention_hours * 3600
            / self.config.collection_interval_seconds) as usize;
        while node_history.len() > max_entries {
            node_history.pop_front();
        }

        Ok(())
    }

    /// Get current metrics for all nodes
    pub async fn get_current_metrics(&self) -> Result<Vec<NodeMetrics>> {
        let history = self.metrics_history.read().await;
        let node_health = self.node_health.read().await;

        let mut result = Vec::new();

        for (node_id, metrics_queue) in history.iter() {
            if let Some(latest_metrics) = metrics_queue.back() {
                let health_status = node_health
                    .get(node_id)
                    .cloned()
                    .unwrap_or(NodeHealthStatus::Offline);

                result.push(NodeMetrics {
                    node_id: node_id.clone(),
                    metrics: latest_metrics.clone(),
                    health_status,
                    last_seen: latest_metrics.timestamp,
                });
            }
        }

        Ok(result)
    }

    /// Get aggregated cluster metrics
    pub async fn get_cluster_aggregated_metrics(&self) -> Result<ClusterMetrics> {
        let node_metrics = self.get_current_metrics().await?;

        if node_metrics.is_empty() {
            return Err(BlockchainError::Analytics(
                "No metrics available".to_string(),
            ));
        }

        let mut aggregated = ClusterMetrics {
            timestamp: current_timestamp(),
            node_id: "cluster_aggregate".to_string(),
            region: "multi-region".to_string(),
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            disk_usage_percent: 0.0,
            network_bandwidth_mbps: 0.0,
            active_connections: 0,
            average_network_latency_ms: 0.0,
            packets_dropped_percent: 0.0,
            consensus_rounds_per_second: 0.0,
            average_block_time_ms: 0.0,
            validator_participation_percent: 0.0,
            consensus_delay_ms: 0.0,
            storage_operations_per_second: 0.0,
            cache_hit_ratio: 0.0,
            merkle_tree_size_mb: 0.0,
            transactions_per_second: 0.0,
            average_transaction_fee: 0.0,
            queue_depth: 0,
            error_rate: 0.0,
            failed_auth_attempts: 0,
            anomaly_score: 0.0,
            security_events_count: 0,
            shard_balance_score: 0.0,
            replication_lag_seconds: 0.0,
            load_balancer_efficiency: 0.0,
        };

        let node_count = node_metrics.len() as f64;

        for node_metric in &node_metrics {
            let m = &node_metric.metrics;
            aggregated.cpu_usage_percent += m.cpu_usage_percent;
            aggregated.memory_usage_percent += m.memory_usage_percent;
            aggregated.disk_usage_percent += m.disk_usage_percent;
            aggregated.network_bandwidth_mbps += m.network_bandwidth_mbps;
            aggregated.active_connections += m.active_connections;
            aggregated.average_network_latency_ms += m.average_network_latency_ms;
            aggregated.packets_dropped_percent += m.packets_dropped_percent;
            aggregated.consensus_rounds_per_second += m.consensus_rounds_per_second;
            aggregated.average_block_time_ms += m.average_block_time_ms;
            aggregated.validator_participation_percent += m.validator_participation_percent;
            aggregated.consensus_delay_ms += m.consensus_delay_ms;
            aggregated.storage_operations_per_second += m.storage_operations_per_second;
            aggregated.cache_hit_ratio += m.cache_hit_ratio;
            aggregated.merkle_tree_size_mb += m.merkle_tree_size_mb;
            aggregated.transactions_per_second += m.transactions_per_second;
            aggregated.average_transaction_fee += m.average_transaction_fee;
            aggregated.queue_depth += m.queue_depth;
            aggregated.error_rate += m.error_rate;
            aggregated.failed_auth_attempts += m.failed_auth_attempts;
            aggregated.anomaly_score += m.anomaly_score;
            aggregated.security_events_count += m.security_events_count;
            aggregated.shard_balance_score += m.shard_balance_score;
            aggregated.replication_lag_seconds += m.replication_lag_seconds;
            aggregated.load_balancer_efficiency += m.load_balancer_efficiency;
        }

        // Calculate averages
        aggregated.cpu_usage_percent /= node_count;
        aggregated.memory_usage_percent /= node_count;
        aggregated.disk_usage_percent /= node_count;
        aggregated.network_bandwidth_mbps /= node_count;
        aggregated.average_network_latency_ms /= node_count;
        aggregated.packets_dropped_percent /= node_count;
        aggregated.consensus_rounds_per_second /= node_count;
        aggregated.average_block_time_ms /= node_count;
        aggregated.validator_participation_percent /= node_count;
        aggregated.consensus_delay_ms /= node_count;
        aggregated.storage_operations_per_second /= node_count;
        aggregated.cache_hit_ratio /= node_count;
        aggregated.merkle_tree_size_mb /= node_count;
        aggregated.transactions_per_second /= node_count;
        aggregated.average_transaction_fee /= node_count;
        aggregated.error_rate /= node_count;
        aggregated.anomaly_score /= node_count;
        aggregated.shard_balance_score /= node_count;
        aggregated.replication_lag_seconds /= node_count;
        aggregated.load_balancer_efficiency /= node_count;

        Ok(aggregated)
    }

    /// Get metrics history for a specific node
    pub async fn get_node_metrics_history(
        &self,
        node_id: &str,
        hours: u64,
    ) -> Result<Vec<ClusterMetrics>> {
        let history = self.metrics_history.read().await;

        if let Some(node_history) = history.get(node_id) {
            let cutoff_time = current_timestamp() - (hours * 3600);
            let metrics: Vec<ClusterMetrics> = node_history
                .iter()
                .filter(|m| m.timestamp >= cutoff_time)
                .cloned()
                .collect();
            Ok(metrics)
        } else {
            Ok(Vec::new())
        }
    }

    /// Check if we have sufficient data for analysis
    pub async fn has_sufficient_data(&self) -> Result<bool> {
        let history = self.metrics_history.read().await;
        let total_metrics: usize = history.values().map(|q| q.len()).sum();
        Ok(total_metrics >= 100) // Require at least 100 data points
    }

    /// Get cluster health overview
    pub async fn get_cluster_health_overview(&self) -> Result<ClusterHealthOverview> {
        let node_metrics = self.get_current_metrics().await?;
        let _node_health = self.node_health.read().await;

        let total_nodes = node_metrics.len();
        let healthy_nodes = node_metrics
            .iter()
            .filter(|n| n.health_status == NodeHealthStatus::Healthy)
            .count();
        let degraded_nodes = node_metrics
            .iter()
            .filter(|n| n.health_status == NodeHealthStatus::Degraded)
            .count();
        let critical_nodes = node_metrics
            .iter()
            .filter(|n| n.health_status == NodeHealthStatus::Critical)
            .count();
        let offline_nodes = node_metrics
            .iter()
            .filter(|n| n.health_status == NodeHealthStatus::Offline)
            .count();

        let average_health_score = if !node_metrics.is_empty() {
            node_metrics
                .iter()
                .map(|n| n.metrics.system_health_score())
                .sum::<f64>()
                / node_metrics.len() as f64
        } else {
            0.0
        };

        Ok(ClusterHealthOverview {
            total_nodes,
            healthy_nodes,
            degraded_nodes,
            critical_nodes,
            offline_nodes,
            average_health_score,
            last_updated: current_timestamp(),
        })
    }

    // Internal methods
    async fn collect_cluster_metrics(
        metrics_history: &Arc<RwLock<HashMap<String, VecDeque<ClusterMetrics>>>>,
        _node_health: &Arc<RwLock<HashMap<String, NodeHealthStatus>>>,
    ) -> Result<()> {
        // In production, this would collect real metrics from nodes
        // For now, generate mock data for demonstration

        let nodes = ["node-1", "node-2", "node-3", "node-4"];
        let regions = ["us-east", "us-west", "eu-central", "ap-southeast"];

        for (i, node_id) in nodes.iter().enumerate() {
            let metrics = ClusterMetrics {
                timestamp: current_timestamp(),
                node_id: node_id.to_string(),
                region: regions[i % regions.len()].to_string(),
                cpu_usage_percent: 45.0 + (rand::random::<f64>() * 30.0),
                memory_usage_percent: 60.0 + (rand::random::<f64>() * 25.0),
                disk_usage_percent: 70.0 + (rand::random::<f64>() * 15.0),
                network_bandwidth_mbps: 500.0 + (rand::random::<f64>() * 300.0),
                active_connections: 1000 + (rand::random::<usize>() % 2000),
                average_network_latency_ms: 50.0 + (rand::random::<f64>() * 100.0),
                packets_dropped_percent: rand::random::<f64>() * 2.0,
                consensus_rounds_per_second: 10.0 + (rand::random::<f64>() * 5.0),
                average_block_time_ms: 3000.0 + (rand::random::<f64>() * 1000.0),
                validator_participation_percent: 85.0 + (rand::random::<f64>() * 10.0),
                consensus_delay_ms: rand::random::<f64>() * 2000.0,
                storage_operations_per_second: 1000.0 + (rand::random::<f64>() * 500.0),
                cache_hit_ratio: 0.8 + (rand::random::<f64>() * 0.15),
                merkle_tree_size_mb: 1000.0 + (rand::random::<f64>() * 500.0),
                transactions_per_second: 500.0 + (rand::random::<f64>() * 300.0),
                average_transaction_fee: 0.001 + (rand::random::<f64>() * 0.01),
                queue_depth: (rand::random::<usize>() % 1000),
                error_rate: rand::random::<f64>() * 0.02,
                failed_auth_attempts: rand::random::<u64>() % 10,
                anomaly_score: rand::random::<f64>() * 0.3,
                security_events_count: rand::random::<u64>() % 5,
                shard_balance_score: 0.7 + (rand::random::<f64>() * 0.3),
                replication_lag_seconds: rand::random::<f64>() * 5.0,
                load_balancer_efficiency: 0.85 + (rand::random::<f64>() * 0.1),
            };

            let mut history = metrics_history.write().await;
            let node_history = history
                .entry(node_id.to_string())
                .or_insert_with(VecDeque::new);
            node_history.push_back(metrics);

            // Keep only recent metrics
            while node_history.len() > 2880 {
                // 24 hours at 30-second intervals
                node_history.pop_front();
            }
        }

        Ok(())
    }

    async fn cleanup_old_metrics(
        metrics_history: &Arc<RwLock<HashMap<String, VecDeque<ClusterMetrics>>>>,
        retention_hours: u64,
    ) {
        let cutoff_time = current_timestamp() - (retention_hours * 3600);
        let mut history = metrics_history.write().await;

        for node_history in history.values_mut() {
            while let Some(oldest) = node_history.front() {
                if oldest.timestamp < cutoff_time {
                    node_history.pop_front();
                } else {
                    break;
                }
            }
        }
    }

    async fn update_node_health_status(
        node_health: &Arc<RwLock<HashMap<String, NodeHealthStatus>>>,
        metrics_history: &Arc<RwLock<HashMap<String, VecDeque<ClusterMetrics>>>>,
    ) {
        let history = metrics_history.read().await;
        let mut health = node_health.write().await;

        for (node_id, metrics_queue) in history.iter() {
            let status = if let Some(latest) = metrics_queue.back() {
                let health_score = latest.system_health_score();
                let time_since_last_metric = current_timestamp() - latest.timestamp;

                if time_since_last_metric > 300 {
                    // 5 minutes
                    NodeHealthStatus::Offline
                } else if latest.has_critical_issues() || health_score < 30.0 {
                    NodeHealthStatus::Critical
                } else if health_score < 60.0 {
                    NodeHealthStatus::Degraded
                } else {
                    NodeHealthStatus::Healthy
                }
            } else {
                NodeHealthStatus::Offline
            };

            health.insert(node_id.clone(), status);
        }
    }
}

/// Cluster health overview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealthOverview {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub degraded_nodes: usize,
    pub critical_nodes: usize,
    pub offline_nodes: usize,
    pub average_health_score: f64,
    pub last_updated: u64,
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

    #[tokio::test]
    async fn test_cluster_monitor_creation() {
        let config = MonitoringConfig::default();
        let _monitor = ClusterMonitor::new(config);

        assert_eq!(_monitor.config.collection_interval_seconds, 30);
        assert_eq!(_monitor.config.metrics_retention_hours, 24);
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let config = MonitoringConfig::default();
        let monitor = ClusterMonitor::new(config);

        let metrics = ClusterMetrics {
            timestamp: current_timestamp(),
            node_id: "test-node".to_string(),
            region: "test-region".to_string(),
            cpu_usage_percent: 50.0,
            memory_usage_percent: 60.0,
            disk_usage_percent: 70.0,
            network_bandwidth_mbps: 500.0,
            active_connections: 1000,
            average_network_latency_ms: 50.0,
            packets_dropped_percent: 0.1,
            consensus_rounds_per_second: 10.0,
            average_block_time_ms: 3000.0,
            validator_participation_percent: 90.0,
            consensus_delay_ms: 100.0,
            storage_operations_per_second: 1000.0,
            cache_hit_ratio: 0.85,
            merkle_tree_size_mb: 1000.0,
            transactions_per_second: 500.0,
            average_transaction_fee: 0.001,
            queue_depth: 100,
            error_rate: 0.01,
            failed_auth_attempts: 0,
            anomaly_score: 0.1,
            security_events_count: 0,
            shard_balance_score: 0.8,
            replication_lag_seconds: 1.0,
            load_balancer_efficiency: 0.9,
        };

        monitor.record_metrics(metrics).await.unwrap();

        let current = monitor.get_current_metrics().await.unwrap();
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].node_id, "test-node");
    }

    #[tokio::test]
    async fn test_cluster_health_calculation() {
        let config = MonitoringConfig::default();
        let _monitor = ClusterMonitor::new(config);

        let metrics = ClusterMetrics {
            timestamp: current_timestamp(),
            node_id: "test-node".to_string(),
            region: "test-region".to_string(),
            cpu_usage_percent: 50.0,
            memory_usage_percent: 60.0,
            disk_usage_percent: 70.0,
            network_bandwidth_mbps: 500.0,
            active_connections: 1000,
            average_network_latency_ms: 50.0,
            packets_dropped_percent: 0.1,
            consensus_rounds_per_second: 10.0,
            average_block_time_ms: 3000.0,
            validator_participation_percent: 90.0,
            consensus_delay_ms: 100.0,
            storage_operations_per_second: 1000.0,
            cache_hit_ratio: 0.85,
            merkle_tree_size_mb: 1000.0,
            transactions_per_second: 500.0,
            average_transaction_fee: 0.001,
            queue_depth: 100,
            error_rate: 0.01,
            failed_auth_attempts: 0,
            anomaly_score: 0.1,
            security_events_count: 0,
            shard_balance_score: 0.8,
            replication_lag_seconds: 1.0,
            load_balancer_efficiency: 0.9,
        };

        let health_score = metrics.system_health_score();
        assert!(health_score > 0.0 && health_score <= 100.0);

        let has_critical = metrics.has_critical_issues();
        assert!(!has_critical);
    }
}
