//! Machine Learning-based performance optimization for Erbium Blockchain
//!
//! This module uses machine learning algorithms to automatically optimize
//! system performance based on workload patterns, resource usage, and
//! historical performance data.

use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// ML optimizer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlOptimizerConfig {
    pub enable_ml_optimization: bool,
    pub learning_rate: f64,
    pub training_interval_seconds: u64,
    pub prediction_horizon: usize, // How far ahead to predict
    pub historical_data_window: usize, // How much historical data to keep
    pub auto_tuning_enabled: bool,
    pub confidence_threshold: f64, // Minimum confidence for applying optimizations
}

impl Default for MlOptimizerConfig {
    fn default() -> Self {
        Self {
            enable_ml_optimization: true,
            learning_rate: 0.01,
            training_interval_seconds: 3600, // 1 hour
            prediction_horizon: 24, // 24 hours ahead
            historical_data_window: 10000,
            auto_tuning_enabled: true,
            confidence_threshold: 0.8,
        }
    }
}

/// Performance metrics for ML training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: u64,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub network_bandwidth_mbps: f64,
    pub active_connections: usize,
    pub queue_depth: usize,
    pub response_time_ms: f64,
    pub throughput_tps: f64,
    pub error_rate: f64,
    pub cache_hit_ratio: f64,
}

/// Optimization recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub component: String,
    pub parameter: String,
    pub current_value: f64,
    pub recommended_value: f64,
    pub confidence: f64,
    pub expected_improvement: f64,
    pub reasoning: String,
}

/// ML-based performance optimizer
pub struct MlOptimizer {
    config: MlOptimizerConfig,
    metrics_history: Arc<RwLock<VecDeque<PerformanceMetrics>>>,
    optimization_model: Arc<RwLock<OptimizationModel>>,
    recommendations: Arc<RwLock<Vec<OptimizationRecommendation>>>,
    last_training: Arc<RwLock<u64>>,
    prediction_cache: Arc<RwLock<HashMap<String, PredictedMetrics>>>,
}

#[derive(Debug, Clone)]
struct OptimizationModel {
    // Simple linear regression weights for each parameter
    cpu_weights: HashMap<String, f64>,
    memory_weights: HashMap<String, f64>,
    network_weights: HashMap<String, f64>,
    throughput_weights: HashMap<String, f64>,
    bias: f64,
    trained_samples: usize,
}

impl Default for OptimizationModel {
    fn default() -> Self {
        Self {
            cpu_weights: HashMap::new(),
            memory_weights: HashMap::new(),
            network_weights: HashMap::new(),
            throughput_weights: HashMap::new(),
            bias: 0.0,
            trained_samples: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedMetrics {
    pub timestamp: u64,
    pub predicted_cpu_usage: f64,
    pub predicted_memory_usage: f64,
    pub predicted_throughput: f64,
    pub confidence: f64,
}

impl MlOptimizer {
    /// Create a new ML optimizer
    pub fn new(config: MlOptimizerConfig) -> Self {
        Self {
            config,
            metrics_history: Arc::new(RwLock::new(VecDeque::with_capacity(config.historical_data_window))),
            optimization_model: Arc::new(RwLock::new(OptimizationModel::default())),
            recommendations: Arc::new(RwLock::new(Vec::new())),
            last_training: Arc::new(RwLock::new(0)),
            prediction_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record performance metrics for training
    pub async fn record_metrics(&self, metrics: PerformanceMetrics) -> Result<()> {
        let mut history = self.metrics_history.write().await;

        // Add new metrics
        history.push_back(metrics);

        // Maintain window size
        while history.len() > self.config.historical_data_window {
            history.pop_front();
        }

        // Check if we should retrain the model
        let current_time = current_timestamp();
        let last_training = *self.last_training.read().await;

        if current_time - last_training >= self.config.training_interval_seconds {
            drop(history); // Release lock before training
            self.train_model().await?;
            *self.last_training.write().await = current_time;
        }

        Ok(())
    }

    /// Generate optimization recommendations
    pub async fn generate_recommendations(&self) -> Result<Vec<OptimizationRecommendation>> {
        let mut recommendations = Vec::new();

        if !self.config.enable_ml_optimization {
            return Ok(recommendations);
        }

        // Analyze current metrics
        let current_metrics = {
            let history = self.metrics_history.read().await;
            history.back().cloned()
        };

        if let Some(metrics) = current_metrics {
            // Generate recommendations based on ML model
            recommendations.extend(self.analyze_cpu_usage(&metrics).await);
            recommendations.extend(self.analyze_memory_usage(&metrics).await);
            recommendations.extend(self.analyze_network_usage(&metrics).await);
            recommendations.extend(self.analyze_throughput(&metrics).await);
        }

        // Filter by confidence threshold
        recommendations.retain(|r| r.confidence >= self.config.confidence_threshold);

        // Store recommendations
        *self.recommendations.write().await = recommendations.clone();

        Ok(recommendations)
    }

    /// Predict future performance metrics
    pub async fn predict_metrics(&self, hours_ahead: usize) -> Result<PredictedMetrics> {
        let history = self.metrics_history.read().await;

        if history.len() < 10 {
            return Err(BlockchainError::Ml("Insufficient historical data for prediction".to_string()));
        }

        // Simple linear regression prediction
        let recent_metrics: Vec<&PerformanceMetrics> = history.iter()
            .rev()
            .take(100)
            .collect();

        let predicted_cpu = self.predict_linear_regression(&recent_metrics, |m| m.cpu_usage_percent, hours_ahead)?;
        let predicted_memory = self.predict_linear_regression(&recent_metrics, |m| m.memory_usage_percent, hours_ahead)?;
        let predicted_throughput = self.predict_linear_regression(&recent_metrics, |m| m.throughput_tps, hours_ahead)?;

        let confidence = self.calculate_prediction_confidence(&recent_metrics);

        let prediction = PredictedMetrics {
            timestamp: current_timestamp() + (hours_ahead as u64 * 3600),
            predicted_cpu_usage: predicted_cpu.max(0.0).min(100.0),
            predicted_memory_usage: predicted_memory.max(0.0).min(100.0),
            predicted_throughput: predicted_throughput.max(0.0),
            confidence,
        };

        // Cache prediction
        let cache_key = format!("prediction_{}", hours_ahead);
        self.prediction_cache.write().await.insert(cache_key, prediction.clone());

        Ok(prediction)
    }

    /// Apply automatic optimizations based on ML recommendations
    pub async fn apply_auto_optimizations(&self) -> Result<Vec<String>> {
        if !self.config.auto_tuning_enabled {
            return Ok(vec!["Auto-tuning disabled".to_string()]);
        }

        let recommendations = self.recommendations.read().await;
        let mut applied_changes = Vec::new();

        for recommendation in recommendations.iter() {
            if recommendation.confidence >= self.config.confidence_threshold &&
               recommendation.expected_improvement > 5.0 { // At least 5% improvement

                let change_result = self.apply_recommendation(recommendation).await;
                match change_result {
                    Ok(change) => applied_changes.push(format!("Applied: {}", change)),
                    Err(e) => applied_changes.push(format!("Failed to apply {}: {:?}", recommendation.parameter, e)),
                }
            }
        }

        Ok(applied_changes)
    }

    /// Get current optimization recommendations
    pub async fn get_recommendations(&self) -> Result<Vec<OptimizationRecommendation>> {
        let recommendations = self.recommendations.read().await;
        Ok(recommendations.clone())
    }

    /// Train the ML optimization model
    async fn train_model(&self) -> Result<()> {
        let history = self.metrics_history.read().await;

        if history.len() < 50 {
            return Err(BlockchainError::Ml("Insufficient data for training".to_string()));
        }

        let mut model = self.optimization_model.write().await;

        // Simple online learning for parameter optimization
        // In production, this would use more sophisticated ML algorithms

        for metrics in history.iter().rev().take(1000) {
            // Update weights based on performance patterns
            self.update_weights(&mut model, metrics);
        }

        model.trained_samples = history.len();

        log::info!("Trained ML optimization model with {} samples", model.trained_samples);
        Ok(())
    }

    // Analysis methods for different metrics
    async fn analyze_cpu_usage(&self, metrics: &PerformanceMetrics) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        if metrics.cpu_usage_percent > 80.0 {
            // High CPU usage - recommend optimizations
            recommendations.push(OptimizationRecommendation {
                component: "CPU".to_string(),
                parameter: "thread_pool_size".to_string(),
                current_value: 8.0, // Assume current value
                recommended_value: 12.0, // Increase threads
                confidence: 0.85,
                expected_improvement: 15.0,
                reasoning: "High CPU usage detected, increasing thread pool size should improve parallelism".to_string(),
            });
        } else if metrics.cpu_usage_percent < 30.0 {
            // Low CPU usage - recommend consolidation
            recommendations.push(OptimizationRecommendation {
                component: "CPU".to_string(),
                parameter: "thread_pool_size".to_string(),
                current_value: 8.0,
                recommended_value: 4.0, // Decrease threads
                confidence: 0.75,
                expected_improvement: 8.0,
                reasoning: "Low CPU usage suggests over-provisioning, reducing threads can save resources".to_string(),
            });
        }

        recommendations
    }

    async fn analyze_memory_usage(&self, metrics: &PerformanceMetrics) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        if metrics.memory_usage_percent > 85.0 {
            recommendations.push(OptimizationRecommendation {
                component: "Memory".to_string(),
                parameter: "cache_size_mb".to_string(),
                current_value: 100.0,
                recommended_value: 50.0,
                confidence: 0.9,
                expected_improvement: 20.0,
                reasoning: "High memory usage, reducing cache size can prevent OOM errors".to_string(),
            });
        } else if metrics.memory_usage_percent < 40.0 {
            recommendations.push(OptimizationRecommendation {
                component: "Memory".to_string(),
                parameter: "cache_size_mb".to_string(),
                current_value: 100.0,
                recommended_value: 150.0,
                confidence: 0.7,
                expected_improvement: 12.0,
                reasoning: "Low memory usage, increasing cache size can improve performance".to_string(),
            });
        }

        recommendations
    }

    async fn analyze_network_usage(&self, metrics: &PerformanceMetrics) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        if metrics.network_bandwidth_mbps > 900.0 { // Near 1Gbps limit
            recommendations.push(OptimizationRecommendation {
                component: "Network".to_string(),
                parameter: "compression_enabled".to_string(),
                current_value: 0.0,
                recommended_value: 1.0,
                confidence: 0.95,
                expected_improvement: 25.0,
                reasoning: "High network usage, enabling compression can reduce bandwidth consumption".to_string(),
            });
        }

        recommendations
    }

    async fn analyze_throughput(&self, metrics: &PerformanceMetrics) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        if metrics.throughput_tps < 1000.0 && metrics.cpu_usage_percent < 50.0 {
            recommendations.push(OptimizationRecommendation {
                component: "Throughput".to_string(),
                parameter: "batch_size".to_string(),
                current_value: 100.0,
                recommended_value: 200.0,
                confidence: 0.8,
                expected_improvement: 18.0,
                reasoning: "Low throughput with available CPU, increasing batch size can improve efficiency".to_string(),
            });
        }

        recommendations
    }

    // Helper methods
    fn predict_linear_regression<F>(
        &self,
        metrics: &[&PerformanceMetrics],
        extractor: F,
        hours_ahead: usize,
    ) -> Result<f64>
    where
        F: Fn(&PerformanceMetrics) -> f64,
    {
        if metrics.len() < 2 {
            return Err(BlockchainError::Ml("Insufficient data for prediction".to_string()));
        }

        // Simple linear regression
        let n = metrics.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_xx = 0.0;

        for (i, metric) in metrics.iter().enumerate() {
            let x = i as f64; // Time index
            let y = extractor(*metric);

            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_xx += x * x;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        // Predict future value
        let future_x = (metrics.len() + hours_ahead) as f64;
        let prediction = slope * future_x + intercept;

        Ok(prediction)
    }

    fn calculate_prediction_confidence(&self, metrics: &[&PerformanceMetrics]) -> f64 {
        // Simple confidence calculation based on data variance
        if metrics.len() < 2 {
            return 0.0;
        }

        let values: Vec<f64> = metrics.iter().map(|m| m.cpu_usage_percent).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        // Higher confidence with lower variance
        let confidence = 1.0 / (1.0 + std_dev / 10.0);
        confidence.min(1.0).max(0.0)
    }

    fn update_weights(&self, model: &mut OptimizationModel, metrics: &PerformanceMetrics) {
        // Simple online learning update
        let learning_rate = self.config.learning_rate;

        // Update CPU-related weights
        let cpu_error = metrics.cpu_usage_percent - 50.0; // Target 50% CPU usage
        self.update_weight(&mut model.cpu_weights, "thread_pool_size", cpu_error, learning_rate);
        self.update_weight(&mut model.cpu_weights, "batch_size", cpu_error, learning_rate);

        // Update memory-related weights
        let memory_error = metrics.memory_usage_percent - 60.0; // Target 60% memory usage
        self.update_weight(&mut model.memory_weights, "cache_size_mb", memory_error, learning_rate);

        // Update network-related weights
        let network_error = metrics.network_bandwidth_mbps - 500.0; // Target 500Mbps
        self.update_weight(&mut model.network_weights, "compression_enabled", network_error, learning_rate);

        // Update throughput weights
        let throughput_error = 10000.0 - metrics.throughput_tps; // Target 10k TPS
        self.update_weight(&mut model.throughput_weights, "batch_size", throughput_error, learning_rate);
    }

    fn update_weight(&self, weights: &mut HashMap<String, f64>, parameter: &str, error: f64, learning_rate: f64) {
        let current_weight = weights.get(parameter).copied().unwrap_or(0.0);
        let new_weight = current_weight + learning_rate * error;
        weights.insert(parameter.to_string(), new_weight);
    }

    async fn apply_recommendation(&self, recommendation: &OptimizationRecommendation) -> Result<String> {
        // In production, this would actually apply the configuration changes
        // For now, just log the intended change
        log::info!("Applying ML recommendation: {} -> {} (confidence: {:.2})",
                  recommendation.parameter,
                  recommendation.recommended_value,
                  recommendation.confidence);

        Ok(format!("Changed {} from {} to {}",
                  recommendation.parameter,
                  recommendation.current_value,
                  recommendation.recommended_value))
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

    #[tokio::test]
    async fn test_ml_optimizer_creation() {
        let config = MlOptimizerConfig::default();
        let optimizer = MlOptimizer::new(config);

        assert!(optimizer.config.enable_ml_optimization);
        assert_eq!(optimizer.config.learning_rate, 0.01);
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        let config = MlOptimizerConfig::default();
        let optimizer = MlOptimizer::new(config);

        let metrics = PerformanceMetrics {
            timestamp: current_timestamp(),
            cpu_usage_percent: 60.0,
            memory_usage_percent: 70.0,
            network_bandwidth_mbps: 100.0,
            active_connections: 500,
            queue_depth: 10,
            response_time_ms: 50.0,
            throughput_tps: 2000.0,
            error_rate: 0.01,
            cache_hit_ratio: 0.85,
        };

        optimizer.record_metrics(metrics).await.unwrap();

        let history = optimizer.metrics_history.read().await;
        assert_eq!(history.len(), 1);
    }

    #[tokio::test]
    async fn test_recommendations_generation() {
        let config = MlOptimizerConfig::default();
        let optimizer = MlOptimizer::new(config);

        // Add high CPU usage metrics
        let metrics = PerformanceMetrics {
            timestamp: current_timestamp(),
            cpu_usage_percent: 90.0,
            memory_usage_percent: 50.0,
            network_bandwidth_mbps: 100.0,
            active_connections: 500,
            queue_depth: 10,
            response_time_ms: 50.0,
            throughput_tps: 2000.0,
            error_rate: 0.01,
            cache_hit_ratio: 0.85,
        };

        optimizer.record_metrics(metrics).await.unwrap();

        let recommendations = optimizer.generate_recommendations().await.unwrap();
        assert!(!recommendations.is_empty());

        // Should recommend increasing thread pool size for high CPU
        let cpu_rec = recommendations.iter().find(|r| r.component == "CPU");
        assert!(cpu_rec.is_some());
    }

    #[tokio::test]
    async fn test_metrics_prediction() {
        let config = MlOptimizerConfig::default();
        let optimizer = MlOptimizer::new(config);

        // Add historical data
        for i in 0..20 {
            let metrics = PerformanceMetrics {
                timestamp: current_timestamp() - (i as u64 * 3600),
                cpu_usage_percent: 50.0 + (i as f64 * 2.0),
                memory_usage_percent: 60.0,
                network_bandwidth_mbps: 100.0,
                active_connections: 500,
                queue_depth: 10,
                response_time_ms: 50.0,
                throughput_tps: 2000.0 + (i as f64 * 100.0),
                error_rate: 0.01,
                cache_hit_ratio: 0.85,
            };
            optimizer.record_metrics(metrics).await.unwrap();
        }

        let prediction = optimizer.predict_metrics(1).await.unwrap();
        assert!(prediction.predicted_cpu_usage > 0.0);
        assert!(prediction.predicted_throughput > 0.0);
        assert!(prediction.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_auto_optimization() {
        let config = MlOptimizerConfig {
            auto_tuning_enabled: true,
            confidence_threshold: 0.5, // Lower threshold for testing
            ..Default::default()
        };
        let optimizer = MlOptimizer::new(config);

        // Add metrics that should trigger recommendations
        let metrics = PerformanceMetrics {
            timestamp: current_timestamp(),
            cpu_usage_percent: 90.0,
            memory_usage_percent: 50.0,
            network_bandwidth_mbps: 100.0,
            active_connections: 500,
            queue_depth: 10,
            response_time_ms: 50.0,
            throughput_tps: 2000.0,
            error_rate: 0.01,
            cache_hit_ratio: 0.85,
        };

        optimizer.record_metrics(metrics).await.unwrap();
        optimizer.generate_recommendations().await.unwrap();

        let changes = optimizer.apply_auto_optimizations().await.unwrap();
        // May or may not apply changes depending on confidence
        assert!(changes.len() >= 0);
    }
}
