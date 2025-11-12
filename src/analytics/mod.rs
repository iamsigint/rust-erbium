//! Advanced Analytics for Erbium Blockchain
//!
//! This module provides comprehensive analytics capabilities for the blockchain:
//! - Real-time cluster performance monitoring
//! - Predictive analytics with trend analysis
//! - Anomaly detection using statistical methods
//! - Cluster health scoring and optimization recommendations

pub mod monitoring;
pub mod predictive;
pub mod anomaly_detection;
pub mod health_scoring;

// Re-export main components
pub use monitoring::{ClusterMonitor, MonitoringConfig, ClusterMetrics};
pub use predictive::{PredictiveAnalytics, TrendAnalysis, PredictionModel};
pub use anomaly_detection::{AnomalyDetector, AnomalyConfig, AnomalyAlert};
pub use health_scoring::{HealthScorer, HealthScore, OptimizationRecommendation};

use crate::utils::error::Result;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

/// Main analytics coordinator
pub struct AnalyticsEngine {
    monitor: Arc<ClusterMonitor>,
    predictor: Arc<PredictiveAnalytics>,
    anomaly_detector: Arc<AnomalyDetector>,
    health_scorer: Arc<HealthScorer>,
    config: AnalyticsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    pub enable_real_time_monitoring: bool,
    pub monitoring_interval_seconds: u64,
    pub prediction_horizon_hours: usize,
    pub anomaly_detection_sensitivity: f64,
    pub health_scoring_enabled: bool,
    pub alert_thresholds: AlertThresholds,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enable_real_time_monitoring: true,
            monitoring_interval_seconds: 30,
            prediction_horizon_hours: 24,
            anomaly_detection_sensitivity: 0.95,
            health_scoring_enabled: true,
            alert_thresholds: AlertThresholds::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub cpu_usage_critical: f64,
    pub memory_usage_critical: f64,
    pub network_latency_critical_ms: f64,
    pub error_rate_critical: f64,
    pub consensus_delay_critical_ms: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            cpu_usage_critical: 90.0,
            memory_usage_critical: 95.0,
            network_latency_critical_ms: 1000.0,
            error_rate_critical: 0.05,
            consensus_delay_critical_ms: 5000.0,
        }
    }
}

impl AnalyticsEngine {
    /// Create a new analytics engine
    pub fn new(config: AnalyticsConfig) -> Self {
        let monitor = Arc::new(ClusterMonitor::new(monitoring::MonitoringConfig::default()));
        let predictor = Arc::new(PredictiveAnalytics::new(predictive::PredictiveConfig::default()));
        let anomaly_detector = Arc::new(AnomalyDetector::new(anomaly_detection::AnomalyConfig::default()));
        let health_scorer = Arc::new(HealthScorer::new(health_scoring::HealthConfig::default()));

        Self {
            monitor,
            predictor,
            anomaly_detector,
            health_scorer,
            config,
        }
    }

    /// Start the analytics engine
    pub async fn start(&self) -> Result<()> {
        log::info!("Starting advanced analytics engine...");

        if self.config.enable_real_time_monitoring {
            self.monitor.start_monitoring().await?;
        }

        self.anomaly_detector.start_detection().await?;
        self.health_scorer.start_scoring().await?;

        log::info!("Analytics engine started successfully");
        Ok(())
    }

    /// Get comprehensive cluster analytics
    pub async fn get_cluster_analytics(&self) -> Result<ClusterAnalytics> {
        let current_metrics = self.monitor.get_cluster_aggregated_metrics().await?;
        let predictions = self.predictor.get_predictions(self.config.prediction_horizon_hours).await?;
        let anomalies = self.anomaly_detector.get_recent_anomalies().await?;
        let health_score = self.health_scorer.get_current_health_score().await?;
        let recommendations = self.health_scorer.get_optimization_recommendations().await?;

        Ok(ClusterAnalytics {
            timestamp: current_timestamp(),
            current_metrics,
            predictions,
            anomalies,
            health_score,
            recommendations,
        })
    }

    /// Process new metrics data
    pub async fn process_metrics(&self, metrics: ClusterMetrics) -> Result<()> {
        // Update monitoring
        self.monitor.record_metrics(metrics.clone()).await?;

        // Check for anomalies
        self.anomaly_detector.analyze_metrics(&metrics).await?;

        // Update health scoring
        self.health_scorer.update_health_score(&metrics).await?;

        // Generate predictions if enough data
        if self.monitor.has_sufficient_data().await? {
            self.predictor.update_model(&metrics).await?;
        }

        Ok(())
    }

    /// Get real-time alerts
    pub async fn get_alerts(&self) -> Result<Vec<AnalyticsAlert>> {
        let mut alerts = Vec::new();

        // Check for critical metrics
        if let Ok(metrics) = self.monitor.get_cluster_aggregated_metrics().await {
            if metrics.cpu_usage_percent > self.config.alert_thresholds.cpu_usage_critical {
                alerts.push(AnalyticsAlert::Critical {
                    alert_type: AlertType::HighCpuUsage,
                    message: format!("CPU usage at {:.1}% exceeds critical threshold", metrics.cpu_usage_percent),
                    severity: AlertSeverity::Critical,
                    timestamp: current_timestamp(),
                });
            }

            if metrics.memory_usage_percent > self.config.alert_thresholds.memory_usage_critical {
                alerts.push(AnalyticsAlert::Critical {
                    alert_type: AlertType::HighMemoryUsage,
                    message: format!("Memory usage at {:.1}% exceeds critical threshold", metrics.memory_usage_percent),
                    severity: AlertSeverity::Critical,
                    timestamp: current_timestamp(),
                });
            }

            if metrics.average_network_latency_ms > self.config.alert_thresholds.network_latency_critical_ms {
                alerts.push(AnalyticsAlert::Warning {
                    alert_type: AlertType::HighNetworkLatency,
                    message: format!("Network latency at {:.0}ms exceeds threshold", metrics.average_network_latency_ms),
                    severity: AlertSeverity::Warning,
                    timestamp: current_timestamp(),
                });
            }
        }

        // Add anomaly alerts
        let anomalies = self.anomaly_detector.get_recent_anomalies().await?;
        for anomaly in anomalies {
            if anomaly.severity == anomaly_detection::AnomalySeverity::Critical {
                alerts.push(AnalyticsAlert::Critical {
                    alert_type: AlertType::AnomalyDetected,
                    message: format!("Critical anomaly detected: {}", anomaly.description),
                    severity: AlertSeverity::Critical,
                    timestamp: anomaly.timestamp,
                });
            }
        }

        Ok(alerts)
    }
}

/// Comprehensive cluster analytics data
#[derive(Debug, Clone)]
pub struct ClusterAnalytics {
    pub timestamp: u64,
    pub current_metrics: ClusterMetrics,
    pub predictions: Vec<predictive::PredictionResult>,
    pub anomalies: Vec<AnomalyAlert>,
    pub health_score: HealthScore,
    pub recommendations: Vec<OptimizationRecommendation>,
}

/// Analytics alerts
#[derive(Debug, Clone)]
pub enum AnalyticsAlert {
    Critical {
        alert_type: AlertType,
        message: String,
        severity: AlertSeverity,
        timestamp: u64,
    },
    Warning {
        alert_type: AlertType,
        message: String,
        severity: AlertSeverity,
        timestamp: u64,
    },
    Info {
        alert_type: AlertType,
        message: String,
        severity: AlertSeverity,
        timestamp: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    HighCpuUsage,
    HighMemoryUsage,
    HighNetworkLatency,
    HighErrorRate,
    ConsensusDelay,
    AnomalyDetected,
    HealthScoreDrop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
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
    async fn test_analytics_engine_creation() {
        let config = AnalyticsConfig::default();
        let engine = AnalyticsEngine::new(config);

        assert!(engine.config.enable_real_time_monitoring);
        assert_eq!(engine.config.monitoring_interval_seconds, 30);
    }

    #[tokio::test]
    async fn test_analytics_engine_start() {
        let config = AnalyticsConfig::default();
        let engine = AnalyticsEngine::new(config);

        let result = engine.start().await;
        assert!(result.is_ok());
    }
}
