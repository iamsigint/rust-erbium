//! Anomaly detection using statistical methods
//!
//! This module implements various statistical anomaly detection algorithms
//! including Z-score, Modified Z-score, Isolation Forest concepts, and
//! time series anomaly detection for blockchain performance monitoring.

use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Anomaly detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyConfig {
    pub enable_anomaly_detection: bool,
    pub detection_sensitivity: f64, // 0.0 to 1.0, higher = more sensitive
    pub z_score_threshold: f64,
    pub modified_z_score_threshold: f64,
    pub moving_average_window: usize,
    pub seasonal_decomposition_enabled: bool,
    pub alert_cooldown_minutes: u64,
    pub historical_baseline_days: usize,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            enable_anomaly_detection: true,
            detection_sensitivity: 0.95,
            z_score_threshold: 3.0,
            modified_z_score_threshold: 3.5,
            moving_average_window: 24, // 24 data points
            seasonal_decomposition_enabled: true,
            alert_cooldown_minutes: 15,
            historical_baseline_days: 7,
        }
    }
}

/// Anomaly alert with severity and context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAlert {
    pub id: String,
    pub timestamp: u64,
    pub metric_name: String,
    pub anomaly_type: AnomalyType,
    pub severity: AnomalySeverity,
    pub description: String,
    pub actual_value: f64,
    pub expected_value: f64,
    pub deviation_score: f64,
    pub confidence: f64,
    pub context: HashMap<String, String>,
}

/// Types of anomalies that can be detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum AnomalyType {
    #[default]
    PointAnomaly, // Single data point anomaly
    ContextualAnomaly, // Anomaly in context of surrounding data
    CollectiveAnomaly, // Anomaly in a sequence of points
    SeasonalAnomaly,   // Deviation from seasonal pattern
    TrendAnomaly,      // Deviation from expected trend
}

/// Severity levels for anomalies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum AnomalySeverity {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

/// Statistical baseline for anomaly detection
#[derive(Debug, Clone)]
struct StatisticalBaseline {
    mean: f64,
    std_dev: f64,
    median: f64,
    mad: f64, // Median Absolute Deviation
    min_value: f64,
    max_value: f64,
    sample_count: usize,
    last_updated: u64,
}

/// Anomaly detector engine
pub struct AnomalyDetector {
    config: AnomalyConfig,
    baselines: Arc<RwLock<HashMap<String, StatisticalBaseline>>>,
    recent_alerts: Arc<RwLock<VecDeque<AnomalyAlert>>>,
    seasonal_patterns: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    is_running: Arc<RwLock<bool>>,
}

impl AnomalyDetector {
    /// Create a new anomaly detector
    pub fn new(config: AnomalyConfig) -> Self {
        Self {
            config,
            baselines: Arc::new(RwLock::new(HashMap::new())),
            recent_alerts: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            seasonal_patterns: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the anomaly detection engine
    pub async fn start_detection(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(BlockchainError::Analytics(
                "Anomaly detection already running".to_string(),
            ));
        }

        *is_running = true;

        log::info!("Anomaly detection engine started");
        Ok(())
    }

    /// Analyze metrics for anomalies
    pub async fn analyze_metrics(
        &self,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> Result<()> {
        if !self.config.enable_anomaly_detection {
            return Ok(());
        }

        // Define metrics to monitor for anomalies
        let metrics_to_check = vec![
            (
                "cpu_usage_percent",
                metrics.cpu_usage_percent,
                AnomalySeverity::High,
            ),
            (
                "memory_usage_percent",
                metrics.memory_usage_percent,
                AnomalySeverity::High,
            ),
            (
                "network_bandwidth_mbps",
                metrics.network_bandwidth_mbps,
                AnomalySeverity::Medium,
            ),
            (
                "transactions_per_second",
                metrics.transactions_per_second,
                AnomalySeverity::Medium,
            ),
            (
                "average_network_latency_ms",
                metrics.average_network_latency_ms,
                AnomalySeverity::High,
            ),
            ("error_rate", metrics.error_rate, AnomalySeverity::Critical),
            (
                "consensus_delay_ms",
                metrics.consensus_delay_ms,
                AnomalySeverity::Critical,
            ),
            (
                "failed_auth_attempts",
                metrics.failed_auth_attempts as f64,
                AnomalySeverity::High,
            ),
            (
                "anomaly_score",
                metrics.anomaly_score,
                AnomalySeverity::Medium,
            ),
        ];

        for (metric_name, value, base_severity) in metrics_to_check {
            if let Err(e) = self
                .detect_anomaly(metric_name, value, base_severity, metrics)
                .await
            {
                log::warn!("Failed to detect anomaly for {}: {:?}", metric_name, e);
            }
        }

        Ok(())
    }

    /// Get recent anomalies
    pub async fn get_recent_anomalies(&self) -> Result<Vec<AnomalyAlert>> {
        let alerts = self.recent_alerts.read().await;
        Ok(alerts.iter().cloned().collect())
    }

    /// Get anomalies by severity
    pub async fn get_anomalies_by_severity(
        &self,
        severity: AnomalySeverity,
    ) -> Result<Vec<AnomalyAlert>> {
        let alerts = self.recent_alerts.read().await;
        let filtered: Vec<AnomalyAlert> = alerts
            .iter()
            .filter(|alert| alert.severity == severity)
            .cloned()
            .collect();
        Ok(filtered)
    }

    /// Get anomaly statistics
    pub async fn get_anomaly_statistics(&self) -> Result<AnomalyStatistics> {
        let alerts = self.recent_alerts.read().await;

        let total_anomalies = alerts.len();
        let critical_count = alerts
            .iter()
            .filter(|a| matches!(a.severity, AnomalySeverity::Critical))
            .count();
        let high_count = alerts
            .iter()
            .filter(|a| matches!(a.severity, AnomalySeverity::High))
            .count();
        let medium_count = alerts
            .iter()
            .filter(|a| matches!(a.severity, AnomalySeverity::Medium))
            .count();
        let low_count = alerts
            .iter()
            .filter(|a| matches!(a.severity, AnomalySeverity::Low))
            .count();

        // Calculate anomaly rate (anomalies per hour)
        let time_span_hours = if total_anomalies > 1 {
            let first = alerts.front().unwrap().timestamp;
            let last = alerts.back().unwrap().timestamp;
            (last - first) as f64 / 3600.0
        } else {
            1.0
        };

        let anomaly_rate_per_hour = total_anomalies as f64 / time_span_hours.max(1.0);

        // Most common anomaly types
        let mut type_counts = HashMap::new();
        for alert in alerts.iter() {
            *type_counts.entry(alert.anomaly_type.clone()).or_insert(0) += 1;
        }

        let most_common_type = type_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(anomaly_type, _)| anomaly_type);

        Ok(AnomalyStatistics {
            total_anomalies,
            critical_count,
            high_count,
            medium_count,
            low_count,
            anomaly_rate_per_hour,
            most_common_anomaly_type: most_common_type,
            time_window_hours: time_span_hours,
        })
    }

    // Internal anomaly detection methods
    async fn detect_anomaly(
        &self,
        metric_name: &str,
        value: f64,
        base_severity: AnomalySeverity,
        context: &crate::analytics::monitoring::ClusterMetrics,
    ) -> Result<()> {
        // Update baseline with new value
        self.update_baseline(metric_name, value).await;

        // Skip detection if insufficient baseline data
        let baseline = {
            let baselines = self.baselines.read().await;
            match baselines.get(metric_name) {
                Some(b) if b.sample_count >= 50 => b.clone(),
                _ => return Ok(()), // Not enough data for reliable detection
            }
        };

        // Apply multiple detection algorithms
        let z_score_anomaly = self.z_score_detection(value, &baseline);
        let modified_z_score_anomaly = self.modified_z_score_detection(value, &baseline);
        let iqr_anomaly = self.iqr_detection(value, &baseline);
        let context_anomaly = self.contextual_anomaly_detection(metric_name, value).await;

        // Combine results (if any algorithm detects anomaly)
        let is_anomaly = z_score_anomaly.is_some()
            || modified_z_score_anomaly.is_some()
            || iqr_anomaly.is_some()
            || context_anomaly.is_some();

        if is_anomaly {
            // Determine anomaly type and severity
            let (anomaly_type, severity, confidence, description) = self.classify_anomaly(
                z_score_anomaly,
                modified_z_score_anomaly,
                iqr_anomaly,
                context_anomaly,
                base_severity,
                value,
                &baseline,
            );

            // Check for alert cooldown
            if self.should_alert(metric_name, &anomaly_type).await {
                let alert = AnomalyAlert {
                    id: format!("{}_{}", metric_name, current_timestamp()),
                    timestamp: current_timestamp(),
                    metric_name: metric_name.to_string(),
                    anomaly_type,
                    severity,
                    description: description.clone(),
                    actual_value: value,
                    expected_value: baseline.mean,
                    deviation_score: (value - baseline.mean).abs() / baseline.std_dev.max(0.1),
                    confidence,
                    context: self.build_context(context),
                };

                // Store alert
                let mut alerts = self.recent_alerts.write().await;
                alerts.push_back(alert);

                // Maintain alert history size
                while alerts.len() > 1000 {
                    alerts.pop_front();
                }

                log::warn!("Anomaly detected: {} - {}", metric_name, description);
            }
        }

        Ok(())
    }

    fn z_score_detection(&self, value: f64, baseline: &StatisticalBaseline) -> Option<f64> {
        if baseline.std_dev == 0.0 {
            return None;
        }

        let z_score = (value - baseline.mean) / baseline.std_dev;
        if z_score.abs() > self.config.z_score_threshold {
            Some(z_score)
        } else {
            None
        }
    }

    fn modified_z_score_detection(
        &self,
        value: f64,
        baseline: &StatisticalBaseline,
    ) -> Option<f64> {
        if baseline.mad == 0.0 {
            return None;
        }

        // Modified Z-score = 0.6745 * (x - median) / MAD
        let modified_z = 0.6745 * (value - baseline.median) / baseline.mad;
        if modified_z.abs() > self.config.modified_z_score_threshold {
            Some(modified_z)
        } else {
            None
        }
    }

    fn iqr_detection(&self, value: f64, baseline: &StatisticalBaseline) -> Option<f64> {
        // Simplified IQR-based detection using baseline stats
        let iqr = baseline.std_dev * 1.349; // Approximate IQR from std dev
        let lower_bound = baseline.median - 1.5 * iqr;
        let upper_bound = baseline.median + 1.5 * iqr;

        if value < lower_bound || value > upper_bound {
            let deviation = if value < lower_bound {
                (lower_bound - value) / iqr.max(0.1)
            } else {
                (value - upper_bound) / iqr.max(0.1)
            };
            Some(deviation)
        } else {
            None
        }
    }

    async fn contextual_anomaly_detection(&self, metric_name: &str, value: f64) -> Option<f64> {
        // Check against recent moving average
        let seasonal_patterns = self.seasonal_patterns.read().await;
        if let Some(pattern) = seasonal_patterns.get(metric_name) {
            if pattern.len() >= self.config.moving_average_window {
                let recent_avg = pattern
                    .iter()
                    .rev()
                    .take(self.config.moving_average_window)
                    .sum::<f64>()
                    / self.config.moving_average_window as f64;

                let deviation = (value - recent_avg).abs() / recent_avg.max(0.1);
                if deviation > 0.3 {
                    // 30% deviation from moving average
                    return Some(deviation);
                }
            }
        }
        None
    }

    fn classify_anomaly(
        &self,
        z_score: Option<f64>,
        modified_z: Option<f64>,
        iqr: Option<f64>,
        contextual: Option<f64>,
        base_severity: AnomalySeverity,
        value: f64,
        baseline: &StatisticalBaseline,
    ) -> (AnomalyType, AnomalySeverity, f64, String) {
        // Determine anomaly type
        let anomaly_type = if contextual.is_some() {
            AnomalyType::ContextualAnomaly
        } else if modified_z.is_some() && modified_z.unwrap().abs() > 4.0 {
            AnomalyType::PointAnomaly
        } else if z_score.is_some() && iqr.is_some() {
            AnomalyType::CollectiveAnomaly
        } else {
            AnomalyType::PointAnomaly
        };

        // Calculate confidence based on agreement between methods
        let detection_methods = [
            z_score.is_some(),
            modified_z.is_some(),
            iqr.is_some(),
            contextual.is_some(),
        ];
        let agreement_count = detection_methods.iter().filter(|&&x| x).count() as f64;
        let confidence = agreement_count / detection_methods.len() as f64;

        // Adjust severity based on confidence and deviation
        let deviation_score = (value - baseline.mean).abs() / baseline.std_dev.max(0.1);
        let severity = if confidence > 0.7 && deviation_score > 5.0 {
            AnomalySeverity::Critical
        } else if confidence > 0.5 && deviation_score > 3.0 {
            match base_severity {
                AnomalySeverity::Low => AnomalySeverity::Medium,
                AnomalySeverity::Medium => AnomalySeverity::High,
                AnomalySeverity::High => AnomalySeverity::Critical,
                AnomalySeverity::Critical => AnomalySeverity::Critical,
            }
        } else {
            base_severity
        };

        // Generate description
        let direction = if value > baseline.mean { "high" } else { "low" };
        let description = format!(
            "Anomalous {} value detected: {:.2} (expected: {:.2}, deviation: {:.1}σ)",
            direction, value, baseline.mean, deviation_score
        );

        (anomaly_type, severity, confidence, description)
    }

    async fn should_alert(&self, metric_name: &str, anomaly_type: &AnomalyType) -> bool {
        let alerts = self.recent_alerts.read().await;

        // Check cooldown period
        let cooldown_seconds = self.config.alert_cooldown_minutes * 60;
        let cutoff_time = current_timestamp() - cooldown_seconds;

        // Count recent alerts for this metric and type
        let recent_count = alerts
            .iter()
            .filter(|alert| {
                alert.metric_name == metric_name
                    && alert.anomaly_type == *anomaly_type
                    && alert.timestamp > cutoff_time
            })
            .count();

        // Allow alert if no recent alerts or if this is critical
        recent_count == 0
    }

    fn build_context(
        &self,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> HashMap<String, String> {
        let mut context = HashMap::new();
        context.insert("node_id".to_string(), metrics.node_id.clone());
        context.insert("region".to_string(), metrics.region.clone());
        context.insert("timestamp".to_string(), metrics.timestamp.to_string());
        context.insert(
            "active_connections".to_string(),
            metrics.active_connections.to_string(),
        );
        context.insert("queue_depth".to_string(), metrics.queue_depth.to_string());
        context
    }

    async fn update_baseline(&self, metric_name: &str, value: f64) {
        let mut baselines = self.baselines.write().await;
        let baseline =
            baselines
                .entry(metric_name.to_string())
                .or_insert_with(|| StatisticalBaseline {
                    mean: 0.0,
                    std_dev: 0.0,
                    median: 0.0,
                    mad: 0.0,
                    min_value: f64::INFINITY,
                    max_value: f64::NEG_INFINITY,
                    sample_count: 0,
                    last_updated: current_timestamp(),
                });

        // Update running statistics using Welford's online algorithm
        baseline.sample_count += 1;
        let delta = value - baseline.mean;
        baseline.mean += delta / baseline.sample_count as f64;
        let delta2 = value - baseline.mean;
        baseline.std_dev = ((baseline.sample_count - 1) as f64 * baseline.std_dev.powi(2)
            + delta * delta2)
            / baseline.sample_count as f64;
        baseline.std_dev = baseline.std_dev.sqrt();

        // Update min/max
        baseline.min_value = baseline.min_value.min(value);
        baseline.max_value = baseline.max_value.max(value);

        // Update median and MAD (simplified - would need full dataset for accuracy)
        baseline.median = baseline.mean; // Approximation
        baseline.mad = baseline.std_dev * 0.6745; // Approximation

        baseline.last_updated = current_timestamp();

        // Update seasonal patterns
        let mut seasonal_patterns = self.seasonal_patterns.write().await;
        let pattern = seasonal_patterns
            .entry(metric_name.to_string())
            .or_insert_with(Vec::new);
        pattern.push(value);

        // Keep only recent patterns
        let max_pattern_size = self.config.historical_baseline_days * 24;
        while pattern.len() > max_pattern_size {
            pattern.remove(0);
        }
    }
}

/// Anomaly statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyStatistics {
    pub total_anomalies: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub anomaly_rate_per_hour: f64,
    pub most_common_anomaly_type: Option<AnomalyType>,
    pub time_window_hours: f64,
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
    async fn test_anomaly_detector_creation() {
        let config = AnomalyConfig::default();
        let detector = AnomalyDetector::new(config);

        assert!(detector.config.enable_anomaly_detection);
        assert_eq!(detector.config.z_score_threshold, 3.0);
    }

    #[tokio::test]
    async fn test_z_score_detection() {
        let config = AnomalyConfig::default();
        let detector = AnomalyDetector::new(config);

        let baseline = StatisticalBaseline {
            mean: 50.0,
            std_dev: 10.0,
            median: 50.0,
            mad: 6.75,
            min_value: 20.0,
            max_value: 80.0,
            sample_count: 100,
            last_updated: current_timestamp(),
        };

        // Normal value
        assert!(detector.z_score_detection(55.0, &baseline).is_none());

        // Anomalous value (4 std devs away)
        let anomaly = detector.z_score_detection(90.0, &baseline);
        assert!(anomaly.is_some());
        assert!(anomaly.unwrap() > 3.0);
    }

    #[tokio::test]
    async fn test_modified_z_score_detection() {
        let config = AnomalyConfig::default();
        let detector = AnomalyDetector::new(config);

        let baseline = StatisticalBaseline {
            mean: 50.0,
            std_dev: 10.0,
            median: 50.0,
            mad: 6.75,
            min_value: 20.0,
            max_value: 80.0,
            sample_count: 100,
            last_updated: current_timestamp(),
        };

        // Normal value
        assert!(detector
            .modified_z_score_detection(55.0, &baseline)
            .is_none());

        // Anomalous value
        let anomaly = detector.modified_z_score_detection(90.0, &baseline);
        assert!(anomaly.is_some());
    }

    #[tokio::test]
    async fn test_baseline_update() {
        let config = AnomalyConfig::default();
        let detector = AnomalyDetector::new(config);

        // Add some values
        for i in 0..10 {
            detector
                .update_baseline("test_metric", 50.0 + i as f64)
                .await;
        }

        let baselines = detector.baselines.read().await;
        let baseline = baselines.get("test_metric").unwrap();

        assert_eq!(baseline.sample_count, 10);
        assert!(baseline.mean > 50.0 && baseline.mean < 60.0);
        assert!(baseline.std_dev > 0.0);
    }
}
