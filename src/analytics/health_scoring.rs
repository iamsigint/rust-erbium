//! Cluster health scoring and optimization recommendations
//!
//! This module provides comprehensive health scoring for blockchain clusters
//! and generates actionable optimization recommendations based on performance
//! metrics, predictive analytics, and anomaly detection results.

use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Health scoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    pub enable_health_scoring: bool,
    pub scoring_weights: HealthWeights,
    pub health_thresholds: HealthThresholds,
    pub recommendation_engine_enabled: bool,
    pub auto_optimization_enabled: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enable_health_scoring: true,
            scoring_weights: HealthWeights::default(),
            health_thresholds: HealthThresholds::default(),
            recommendation_engine_enabled: true,
            auto_optimization_enabled: false, // Disabled by default for safety
        }
    }
}

/// Weights for different health components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthWeights {
    pub system_performance_weight: f64,
    pub consensus_health_weight: f64,
    pub network_stability_weight: f64,
    pub security_score_weight: f64,
    pub resource_efficiency_weight: f64,
    pub anomaly_penalty_weight: f64,
}

impl Default for HealthWeights {
    fn default() -> Self {
        Self {
            system_performance_weight: 0.25,
            consensus_health_weight: 0.25,
            network_stability_weight: 0.20,
            security_score_weight: 0.15,
            resource_efficiency_weight: 0.10,
            anomaly_penalty_weight: 0.05,
        }
    }
}

/// Health score thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthThresholds {
    pub excellent_min_score: f64,
    pub good_min_score: f64,
    pub fair_min_score: f64,
    pub poor_max_score: f64,
    pub critical_max_score: f64,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            excellent_min_score: 90.0,
            good_min_score: 75.0,
            fair_min_score: 60.0,
            poor_max_score: 45.0,
            critical_max_score: 30.0,
        }
    }
}

/// Overall health score with breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthScore {
    pub overall_score: f64,
    pub grade: HealthGrade,
    pub component_scores: HealthComponents,
    pub timestamp: u64,
    pub trend: HealthTrend,
    pub risk_factors: Vec<String>,
}

/// Health grade classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthGrade {
    Excellent,
    Good,
    Fair,
    Poor,
    Critical,
}

/// Individual health component scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthComponents {
    pub system_performance: f64,
    pub consensus_health: f64,
    pub network_stability: f64,
    pub security_score: f64,
    pub resource_efficiency: f64,
    pub anomaly_penalty: f64,
}

/// Health trend over time
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthTrend {
    Improving,
    Stable,
    Declining,
    Volatile,
}

/// Optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub id: String,
    pub category: RecommendationCategory,
    pub priority: RecommendationPriority,
    pub title: String,
    pub description: String,
    pub impact_score: f64, // Expected improvement (0-100)
    pub confidence: f64,   // Confidence in recommendation (0-1)
    pub implementation_effort: ImplementationEffort,
    pub affected_components: Vec<String>,
    pub prerequisites: Vec<String>,
    pub estimated_time_savings: Option<u64>, // seconds
    pub cost_benefit_ratio: f64,
}

/// Recommendation categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationCategory {
    Performance,
    Security,
    Scalability,
    Reliability,
    CostOptimization,
    Maintenance,
}

/// Recommendation priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationPriority {
    Critical,
    High,
    Medium,
    Low,
}

/// Implementation effort levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImplementationEffort {
    Minimal,
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Health scorer engine
pub struct HealthScorer {
    config: HealthConfig,
    previous_scores: Arc<RwLock<Vec<HealthScore>>>,
    recommendations: Arc<RwLock<Vec<OptimizationRecommendation>>>,
    is_running: Arc<RwLock<bool>>,
}

impl HealthScorer {
    /// Create a new health scorer
    pub fn new(config: HealthConfig) -> Self {
        Self {
            config,
            previous_scores: Arc::new(RwLock::new(Vec::with_capacity(100))),
            recommendations: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the health scoring engine
    pub async fn start_scoring(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(BlockchainError::Analytics(
                "Health scoring already running".to_string(),
            ));
        }

        *is_running = true;

        log::info!("Health scoring engine started");
        Ok(())
    }

    /// Update health score based on current metrics
    pub async fn update_health_score(
        &self,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> Result<()> {
        if !self.config.enable_health_scoring {
            return Ok(());
        }

        let component_scores = self.calculate_component_scores(metrics).await?;
        let overall_score = self.calculate_overall_score(&component_scores);
        let grade = self.determine_health_grade(overall_score);
        let trend = self.calculate_health_trend(overall_score).await;
        let risk_factors = self.identify_risk_factors(&component_scores, metrics);

        let health_score = HealthScore {
            overall_score,
            grade,
            component_scores,
            timestamp: current_timestamp(),
            trend,
            risk_factors,
        };

        // Store score history
        let mut scores = self.previous_scores.write().await;
        scores.push(health_score.clone());

        // Keep only recent scores
        while scores.len() > 100 {
            scores.remove(0);
        }

        // Generate recommendations if enabled
        if self.config.recommendation_engine_enabled {
            self.generate_recommendations(&health_score, metrics)
                .await?;
        }

        Ok(())
    }

    /// Get current health score
    pub async fn get_current_health_score(&self) -> Result<HealthScore> {
        let scores = self.previous_scores.read().await;
        scores
            .last()
            .cloned()
            .ok_or_else(|| BlockchainError::Analytics("No health scores available".to_string()))
    }

    /// Get health score history
    pub async fn get_health_history(&self, hours: u64) -> Result<Vec<HealthScore>> {
        let scores = self.previous_scores.read().await;
        let cutoff_time = current_timestamp() - (hours * 3600);

        let history: Vec<HealthScore> = scores
            .iter()
            .filter(|score| score.timestamp >= cutoff_time)
            .cloned()
            .collect();

        Ok(history)
    }

    /// Get optimization recommendations
    pub async fn get_optimization_recommendations(
        &self,
    ) -> Result<Vec<OptimizationRecommendation>> {
        let recommendations = self.recommendations.read().await;
        Ok(recommendations.clone())
    }

    /// Get recommendations by priority
    pub async fn get_recommendations_by_priority(
        &self,
        priority: RecommendationPriority,
    ) -> Result<Vec<OptimizationRecommendation>> {
        let recommendations = self.recommendations.read().await;
        let filtered: Vec<OptimizationRecommendation> = recommendations
            .iter()
            .filter(|rec| rec.priority == priority)
            .cloned()
            .collect();
        Ok(filtered)
    }

    /// Get cluster health overview
    pub async fn get_cluster_health_overview(&self) -> Result<ClusterHealthOverview> {
        let current_score = self.get_current_health_score().await?;
        let history = self.get_health_history(24).await?; // Last 24 hours

        let avg_score_24h = if history.is_empty() {
            current_score.overall_score
        } else {
            history.iter().map(|s| s.overall_score).sum::<f64>() / history.len() as f64
        };

        let score_volatility = if history.len() > 1 {
            let variance = history
                .iter()
                .map(|s| (s.overall_score - avg_score_24h).powi(2))
                .sum::<f64>()
                / history.len() as f64;
            variance.sqrt()
        } else {
            0.0
        };

        let critical_recommendations = self
            .get_recommendations_by_priority(RecommendationPriority::Critical)
            .await?;
        let high_recommendations = self
            .get_recommendations_by_priority(RecommendationPriority::High)
            .await?;

        Ok(ClusterHealthOverview {
            current_health_score: current_score.overall_score,
            health_grade: current_score.grade,
            average_score_24h: avg_score_24h,
            score_volatility,
            health_trend: current_score.trend,
            critical_issues_count: critical_recommendations.len(),
            high_priority_actions: high_recommendations.len(),
            risk_factors: current_score.risk_factors,
            last_updated: current_score.timestamp,
        })
    }

    // Internal scoring methods
    async fn calculate_component_scores(
        &self,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> Result<HealthComponents> {
        // System Performance Score (CPU, Memory, Disk)
        let system_performance = self.calculate_system_performance_score(metrics);

        // Consensus Health Score
        let consensus_health = self.calculate_consensus_health_score(metrics);

        // Network Stability Score
        let network_stability = self.calculate_network_stability_score(metrics);

        // Security Score
        let security_score = self.calculate_security_score(metrics);

        // Resource Efficiency Score
        let resource_efficiency = self.calculate_resource_efficiency_score(metrics);

        // Anomaly Penalty (lower score = more anomalies)
        let anomaly_penalty = (100.0 - (metrics.anomaly_score * 100.0)).max(0.0);

        Ok(HealthComponents {
            system_performance,
            consensus_health,
            network_stability,
            security_score,
            resource_efficiency,
            anomaly_penalty,
        })
    }

    fn calculate_system_performance_score(
        &self,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> f64 {
        let cpu_score = 100.0 - metrics.cpu_usage_percent;
        let memory_score = 100.0 - metrics.memory_usage_percent;
        let disk_score = 100.0 - metrics.disk_usage_percent;

        // Weighted average with emphasis on CPU and memory
        (cpu_score * 0.5) + (memory_score * 0.3) + (disk_score * 0.2)
    }

    fn calculate_consensus_health_score(
        &self,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> f64 {
        let participation_score = metrics.validator_participation_percent;
        let delay_penalty = (5000.0 - metrics.consensus_delay_ms).max(0.0) / 50.0; // Max 100 points for <5s delay
        let rounds_score = metrics.consensus_rounds_per_second.min(20.0) * 5.0; // Max 100 for 20+ rounds/sec

        (participation_score * 0.4) + (delay_penalty.min(100.0) * 0.4) + (rounds_score * 0.2)
    }

    fn calculate_network_stability_score(
        &self,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> f64 {
        let latency_score = (200.0 - metrics.average_network_latency_ms).max(0.0) / 2.0; // Max 100 for <200ms
        let bandwidth_score = metrics.network_bandwidth_mbps.min(1000.0) / 10.0; // Max 100 for 1Gbps+
        let packet_loss_score = 100.0 - (metrics.packets_dropped_percent * 1000.0); // Penalty for packet loss

        (latency_score.min(100.0) * 0.4)
            + (bandwidth_score * 0.3)
            + (packet_loss_score.max(0.0) * 0.3)
    }

    fn calculate_security_score(
        &self,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> f64 {
        let auth_attempts_penalty = (metrics.failed_auth_attempts as f64 * 10.0).min(50.0); // Max 50 point penalty
        let error_rate_penalty = (metrics.error_rate * 500.0).min(30.0); // Max 30 point penalty
        let security_events_penalty = (metrics.security_events_count as f64 * 5.0).min(20.0); // Max 20 point penalty

        100.0 - auth_attempts_penalty - error_rate_penalty - security_events_penalty
    }

    fn calculate_resource_efficiency_score(
        &self,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> f64 {
        let throughput_efficiency =
            metrics.transactions_per_second / metrics.cpu_usage_percent.max(1.0);
        let cache_efficiency = metrics.cache_hit_ratio * 100.0;
        let queue_efficiency = (1000.0 - metrics.queue_depth as f64).max(0.0) / 10.0; // Max 100 for queue < 1000

        // Normalize and weight
        let throughput_score = (throughput_efficiency.min(10.0) * 10.0).min(100.0);
        (throughput_score * 0.4) + (cache_efficiency * 0.4) + (queue_efficiency * 0.2)
    }

    fn calculate_overall_score(&self, components: &HealthComponents) -> f64 {
        let weights = &self.config.scoring_weights;

        (components.system_performance * weights.system_performance_weight)
            + (components.consensus_health * weights.consensus_health_weight)
            + (components.network_stability * weights.network_stability_weight)
            + (components.security_score * weights.security_score_weight)
            + (components.resource_efficiency * weights.resource_efficiency_weight)
            + (components.anomaly_penalty * weights.anomaly_penalty_weight)
    }

    fn determine_health_grade(&self, score: f64) -> HealthGrade {
        let thresholds = &self.config.health_thresholds;

        if score >= thresholds.excellent_min_score {
            HealthGrade::Excellent
        } else if score >= thresholds.good_min_score {
            HealthGrade::Good
        } else if score >= thresholds.fair_min_score {
            HealthGrade::Fair
        } else if score >= thresholds.critical_max_score {
            HealthGrade::Poor
        } else {
            HealthGrade::Critical
        }
    }

    async fn calculate_health_trend(&self, _current_score: f64) -> HealthTrend {
        let scores = self.previous_scores.read().await;

        if scores.len() < 5 {
            return HealthTrend::Stable;
        }

        let recent_scores: Vec<f64> = scores
            .iter()
            .rev()
            .take(10)
            .map(|s| s.overall_score)
            .collect();
        let avg_recent = recent_scores.iter().sum::<f64>() / recent_scores.len() as f64;
        let avg_older = if scores.len() > 10 {
            let older_scores: Vec<f64> = scores
                .iter()
                .rev()
                .skip(10)
                .take(10)
                .map(|s| s.overall_score)
                .collect();
            older_scores.iter().sum::<f64>() / older_scores.len() as f64
        } else {
            avg_recent
        };

        let change = avg_recent - avg_older;
        let volatility = recent_scores
            .iter()
            .map(|s| (s - avg_recent).abs())
            .sum::<f64>()
            / recent_scores.len() as f64;

        if volatility > 10.0 {
            HealthTrend::Volatile
        } else if change > 5.0 {
            HealthTrend::Improving
        } else if change < -5.0 {
            HealthTrend::Declining
        } else {
            HealthTrend::Stable
        }
    }

    fn identify_risk_factors(
        &self,
        components: &HealthComponents,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> Vec<String> {
        let mut risks: Vec<String> = Vec::new();

        if components.system_performance < 60.0 {
            risks.push("High system resource utilization".to_string());
        }

        if components.consensus_health < 70.0 {
            risks.push("Consensus performance degradation".to_string());
        }

        if components.network_stability < 65.0 {
            risks.push("Network stability issues".to_string());
        }

        if components.security_score < 75.0 {
            risks.push("Security vulnerabilities detected".to_string());
        }

        if components.anomaly_penalty < 80.0 {
            risks.push("Frequent anomaly detection".to_string());
        }

        if metrics.cpu_usage_percent > 90.0 {
            risks.push("Critical CPU usage".to_string());
        }

        if metrics.memory_usage_percent > 95.0 {
            risks.push("Critical memory usage".to_string());
        }

        if metrics.consensus_delay_ms > 30000.0 {
            risks.push("Severe consensus delays".to_string());
        }

        risks
    }

    async fn generate_recommendations(
        &self,
        health_score: &HealthScore,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> Result<()> {
        let mut recommendations = Vec::new();

        // Generate recommendations based on health components
        self.generate_performance_recommendations(
            &mut recommendations,
            &health_score.component_scores,
            metrics,
        );
        self.generate_security_recommendations(
            &mut recommendations,
            &health_score.component_scores,
            metrics,
        );
        self.generate_scalability_recommendations(
            &mut recommendations,
            &health_score.component_scores,
            metrics,
        );
        self.generate_maintenance_recommendations(&mut recommendations, health_score);

        // Sort by impact and priority
        recommendations.sort_by(|a, b| {
            (b.impact_score * b.confidence)
                .partial_cmp(&(a.impact_score * a.confidence))
                .unwrap()
        });

        // Keep top recommendations
        recommendations.truncate(20);

        // Store recommendations
        *self.recommendations.write().await = recommendations;

        Ok(())
    }

    fn generate_performance_recommendations(
        &self,
        recommendations: &mut Vec<OptimizationRecommendation>,
        components: &HealthComponents,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) {
        if components.system_performance < 70.0 {
            if metrics.cpu_usage_percent > 80.0 {
                recommendations.push(OptimizationRecommendation {
                    id: "cpu_optimization".to_string(),
                    category: RecommendationCategory::Performance,
                    priority: RecommendationPriority::High,
                    title: "Optimize CPU Usage".to_string(),
                    description: "Implement CPU optimization techniques including query optimization, caching improvements, and load distribution".to_string(),
                    impact_score: 25.0,
                    confidence: 0.85,
                    implementation_effort: ImplementationEffort::Medium,
                    affected_components: vec!["CPU".to_string(), "Performance".to_string()],
                    prerequisites: vec!["Performance monitoring access".to_string()],
                    estimated_time_savings: Some(3600), // 1 hour
                    cost_benefit_ratio: 15.0,
                });
            }

            if metrics.memory_usage_percent > 85.0 {
                recommendations.push(OptimizationRecommendation {
                    id: "memory_optimization".to_string(),
                    category: RecommendationCategory::Performance,
                    priority: RecommendationPriority::High,
                    title: "Optimize Memory Usage".to_string(),
                    description: "Reduce memory footprint through garbage collection tuning, cache size optimization, and memory leak fixes".to_string(),
                    impact_score: 30.0,
                    confidence: 0.90,
                    implementation_effort: ImplementationEffort::Medium,
                    affected_components: vec!["Memory".to_string(), "Performance".to_string()],
                    prerequisites: vec!["Memory profiling tools".to_string()],
                    estimated_time_savings: Some(7200), // 2 hours
                    cost_benefit_ratio: 20.0,
                });
            }
        }

        if components.resource_efficiency < 60.0 {
            recommendations.push(OptimizationRecommendation {
                id: "cache_optimization".to_string(),
                category: RecommendationCategory::Performance,
                priority: RecommendationPriority::Medium,
                title: "Improve Cache Efficiency".to_string(),
                description: "Optimize cache hit ratios through better cache key design and eviction policies".to_string(),
                impact_score: 20.0,
                confidence: 0.75,
                implementation_effort: ImplementationEffort::Low,
                affected_components: vec!["Cache".to_string(), "Performance".to_string()],
                prerequisites: vec!["Cache metrics access".to_string()],
                estimated_time_savings: Some(1800), // 30 minutes
                cost_benefit_ratio: 25.0,
            });
        }
    }

    fn generate_security_recommendations(
        &self,
        recommendations: &mut Vec<OptimizationRecommendation>,
        components: &HealthComponents,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) {
        if components.security_score < 80.0 {
            if metrics.failed_auth_attempts > 5 {
                recommendations.push(OptimizationRecommendation {
                    id: "auth_security".to_string(),
                    category: RecommendationCategory::Security,
                    priority: RecommendationPriority::Critical,
                    title: "Strengthen Authentication Security".to_string(),
                    description: "Implement multi-factor authentication, rate limiting, and suspicious activity detection".to_string(),
                    impact_score: 40.0,
                    confidence: 0.95,
                    implementation_effort: ImplementationEffort::High,
                    affected_components: vec!["Security".to_string(), "Authentication".to_string()],
                    prerequisites: vec!["Security audit completed".to_string()],
                    estimated_time_savings: None,
                    cost_benefit_ratio: 50.0,
                });
            }

            if metrics.error_rate > 0.05 {
                recommendations.push(OptimizationRecommendation {
                    id: "error_handling".to_string(),
                    category: RecommendationCategory::Security,
                    priority: RecommendationPriority::High,
                    title: "Improve Error Handling".to_string(),
                    description: "Enhance error handling to prevent information leakage and improve system resilience".to_string(),
                    impact_score: 15.0,
                    confidence: 0.80,
                    implementation_effort: ImplementationEffort::Medium,
                    affected_components: vec!["Error Handling".to_string(), "Security".to_string()],
                    prerequisites: vec!["Error monitoring system".to_string()],
                    estimated_time_savings: Some(3600), // 1 hour
                    cost_benefit_ratio: 12.0,
                });
            }
        }
    }

    fn generate_scalability_recommendations(
        &self,
        recommendations: &mut Vec<OptimizationRecommendation>,
        components: &HealthComponents,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) {
        if components.consensus_health < 75.0 && metrics.consensus_delay_ms > 5000.0 {
            recommendations.push(OptimizationRecommendation {
                id: "consensus_optimization".to_string(),
                category: RecommendationCategory::Scalability,
                priority: RecommendationPriority::High,
                title: "Optimize Consensus Performance".to_string(),
                description: "Improve consensus algorithm efficiency through parallel processing and optimized message handling".to_string(),
                impact_score: 35.0,
                confidence: 0.85,
                implementation_effort: ImplementationEffort::High,
                affected_components: vec!["Consensus".to_string(), "Performance".to_string()],
                prerequisites: vec!["Consensus protocol analysis".to_string()],
                estimated_time_savings: Some(10800), // 3 hours
                cost_benefit_ratio: 30.0,
            });
        }

        if components.network_stability < 70.0 {
            recommendations.push(OptimizationRecommendation {
                id: "network_optimization".to_string(),
                category: RecommendationCategory::Scalability,
                priority: RecommendationPriority::Medium,
                title: "Optimize Network Performance".to_string(),
                description: "Implement network optimizations including compression, connection pooling, and routing improvements".to_string(),
                impact_score: 25.0,
                confidence: 0.75,
                implementation_effort: ImplementationEffort::Medium,
                affected_components: vec!["Network".to_string(), "Performance".to_string()],
                prerequisites: vec!["Network monitoring tools".to_string()],
                estimated_time_savings: Some(5400), // 1.5 hours
                cost_benefit_ratio: 18.0,
            });
        }
    }

    fn generate_maintenance_recommendations(
        &self,
        recommendations: &mut Vec<OptimizationRecommendation>,
        health_score: &HealthScore,
    ) {
        // Regular maintenance recommendations
        recommendations.push(OptimizationRecommendation {
            id: "regular_maintenance".to_string(),
            category: RecommendationCategory::Maintenance,
            priority: RecommendationPriority::Low,
            title: "Perform Regular System Maintenance".to_string(),
            description: "Execute routine maintenance tasks including log rotation, database optimization, and cache cleanup".to_string(),
            impact_score: 10.0,
            confidence: 0.90,
            implementation_effort: ImplementationEffort::Low,
            affected_components: vec!["System".to_string(), "Maintenance".to_string()],
            prerequisites: vec!["Maintenance schedule".to_string()],
            estimated_time_savings: Some(1800), // 30 minutes
            cost_benefit_ratio: 8.0,
        });

        // Health monitoring improvement
        if health_score.overall_score < 80.0 {
            recommendations.push(OptimizationRecommendation {
                id: "health_monitoring".to_string(),
                category: RecommendationCategory::Maintenance,
                priority: RecommendationPriority::Medium,
                title: "Enhance Health Monitoring".to_string(),
                description: "Improve monitoring coverage and alerting to detect issues earlier and prevent degradation".to_string(),
                impact_score: 15.0,
                confidence: 0.85,
                implementation_effort: ImplementationEffort::Low,
                affected_components: vec!["Monitoring".to_string(), "Health".to_string()],
                prerequisites: vec!["Monitoring system access".to_string()],
                estimated_time_savings: Some(3600), // 1 hour
                cost_benefit_ratio: 12.0,
            });
        }
    }
}

/// Cluster health overview summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealthOverview {
    pub current_health_score: f64,
    pub health_grade: HealthGrade,
    pub average_score_24h: f64,
    pub score_volatility: f64,
    pub health_trend: HealthTrend,
    pub critical_issues_count: usize,
    pub high_priority_actions: usize,
    pub risk_factors: Vec<String>,
    pub last_updated: u64,
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
    async fn test_health_scorer_creation() {
        let config = HealthConfig::default();
        let scorer = HealthScorer::new(config);

        assert!(scorer.config.enable_health_scoring);
        assert_eq!(
            scorer.config.scoring_weights.system_performance_weight,
            0.25
        );
    }

    #[tokio::test]
    async fn test_health_score_calculation() {
        let config = HealthConfig::default();
        let scorer = HealthScorer::new(config);

        let metrics = crate::analytics::monitoring::ClusterMetrics {
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

        scorer.update_health_score(&metrics).await.unwrap();
        let health_score = scorer.get_current_health_score().await.unwrap();

        assert!(health_score.overall_score > 0.0 && health_score.overall_score <= 100.0);
        assert!(matches!(
            health_score.grade,
            HealthGrade::Excellent
                | HealthGrade::Good
                | HealthGrade::Fair
                | HealthGrade::Poor
                | HealthGrade::Critical
        ));
    }

    #[tokio::test]
    async fn test_recommendation_generation() {
        let config = HealthConfig::default();
        let scorer = HealthScorer::new(config);

        // Create metrics that should trigger recommendations
        let metrics = crate::analytics::monitoring::ClusterMetrics {
            timestamp: current_timestamp(),
            node_id: "test-node".to_string(),
            region: "test-region".to_string(),
            cpu_usage_percent: 95.0, // High CPU - should trigger recommendation
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

        scorer.update_health_score(&metrics).await.unwrap();
        let recommendations = scorer.get_optimization_recommendations().await.unwrap();

        assert!(!recommendations.is_empty());
        // Should have CPU optimization recommendation
        let has_cpu_rec = recommendations.iter().any(|r| r.id == "cpu_optimization");
        assert!(has_cpu_rec);
    }
}
