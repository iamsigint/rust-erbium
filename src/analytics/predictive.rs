//! Predictive analytics with trend analysis
//!
//! This module provides advanced predictive analytics capabilities including
//! time series forecasting, trend analysis, and predictive modeling for
//! blockchain performance metrics.

use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Predictive analytics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveConfig {
    pub enable_predictions: bool,
    pub model_update_interval_seconds: u64,
    pub prediction_horizon_hours: usize,
    pub historical_window_days: usize,
    pub confidence_interval_percentile: f64,
    pub seasonal_analysis_enabled: bool,
    pub anomaly_aware_predictions: bool,
}

impl Default for PredictiveConfig {
    fn default() -> Self {
        Self {
            enable_predictions: true,
            model_update_interval_seconds: 3600, // 1 hour
            prediction_horizon_hours: 24,
            historical_window_days: 7,
            confidence_interval_percentile: 95.0,
            seasonal_analysis_enabled: true,
            anomaly_aware_predictions: true,
        }
    }
}

/// Prediction result with confidence intervals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    pub metric_name: String,
    pub timestamp: u64,
    pub predicted_value: f64,
    pub confidence_lower: f64,
    pub confidence_upper: f64,
    pub trend_direction: TrendDirection,
    pub seasonal_adjustment: f64,
    pub accuracy_score: f64,
}

/// Trend analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub metric_name: String,
    pub trend_direction: TrendDirection,
    pub trend_strength: f64, // 0-1, higher is stronger trend
    pub slope: f64,          // Rate of change
    pub seasonality_detected: bool,
    pub seasonal_period_hours: Option<usize>,
    pub volatility: f64,
    pub prediction_accuracy: f64,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    StronglyIncreasing,
    Increasing,
    Stable,
    Decreasing,
    StronglyDecreasing,
    Volatile,
}

/// Predictive model for time series forecasting
#[derive(Debug, Clone)]
pub struct PredictionModel {
    pub metric_name: String,
    pub coefficients: Vec<f64>,
    pub intercept: f64,
    pub seasonal_components: HashMap<usize, f64>,
    pub last_updated: u64,
    pub accuracy_history: VecDeque<f64>,
    pub training_samples: usize,
}

/// Predictive analytics engine
pub struct PredictiveAnalytics {
    config: PredictiveConfig,
    models: Arc<RwLock<HashMap<String, PredictionModel>>>,
    historical_data: Arc<RwLock<HashMap<String, VecDeque<(u64, f64)>>>>,
    trend_cache: Arc<RwLock<HashMap<String, TrendAnalysis>>>,
    is_running: Arc<RwLock<bool>>,
}

impl PredictiveAnalytics {
    /// Create a new predictive analytics engine
    pub fn new(config: PredictiveConfig) -> Self {
        Self {
            config,
            models: Arc::new(RwLock::new(HashMap::new())),
            historical_data: Arc::new(RwLock::new(HashMap::new())),
            trend_cache: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the predictive analytics engine
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(BlockchainError::Analytics(
                "Predictive analytics already running".to_string(),
            ));
        }

        *is_running = true;

        log::info!("Predictive analytics engine started");
        Ok(())
    }

    /// Update model with new metrics data
    pub async fn update_model(
        &self,
        metrics: &crate::analytics::monitoring::ClusterMetrics,
    ) -> Result<()> {
        if !self.config.enable_predictions {
            return Ok(());
        }

        // Extract key metrics for prediction
        let metrics_to_predict = vec![
            ("cpu_usage_percent", metrics.cpu_usage_percent),
            ("memory_usage_percent", metrics.memory_usage_percent),
            ("network_bandwidth_mbps", metrics.network_bandwidth_mbps),
            ("transactions_per_second", metrics.transactions_per_second),
            (
                "average_network_latency_ms",
                metrics.average_network_latency_ms,
            ),
            ("error_rate", metrics.error_rate),
            ("consensus_delay_ms", metrics.consensus_delay_ms),
        ];

        for (metric_name, value) in metrics_to_predict {
            self.add_data_point(metric_name, metrics.timestamp, value)
                .await?;
            self.update_trend_analysis(metric_name).await?;
        }

        // Retrain models periodically
        let current_time = current_timestamp();
        let should_retrain = {
            let models = self.models.read().await;
            models.values().any(|model| {
                current_time - model.last_updated >= self.config.model_update_interval_seconds
            })
        };

        if should_retrain {
            self.retrain_models().await?;
        }

        Ok(())
    }

    /// Get predictions for specified horizon
    pub async fn get_predictions(&self, hours_ahead: usize) -> Result<Vec<PredictionResult>> {
        let models = self.models.read().await;
        let mut predictions = Vec::new();

        for (metric_name, _model) in models.iter() {
            if let Ok(prediction) = self.predict_metric(metric_name, hours_ahead).await {
                predictions.push(prediction);
            }
        }

        Ok(predictions)
    }

    /// Get trend analysis for a specific metric
    pub async fn get_trend_analysis(&self, metric_name: &str) -> Result<Option<TrendAnalysis>> {
        let trend_cache = self.trend_cache.read().await;
        Ok(trend_cache.get(metric_name).cloned())
    }

    /// Get all trend analyses
    pub async fn get_all_trends(&self) -> Result<Vec<TrendAnalysis>> {
        let trend_cache = self.trend_cache.read().await;
        Ok(trend_cache.values().cloned().collect())
    }

    /// Predict future resource requirements
    pub async fn predict_resource_requirements(
        &self,
        hours_ahead: usize,
    ) -> Result<ResourcePrediction> {
        let predictions = self.get_predictions(hours_ahead).await?;

        let mut cpu_prediction = 0.0;
        let mut memory_prediction = 0.0;
        let mut network_prediction = 0.0;
        let mut storage_prediction = 0.0;

        for prediction in &predictions {
            match prediction.metric_name.as_str() {
                "cpu_usage_percent" => cpu_prediction = prediction.predicted_value,
                "memory_usage_percent" => memory_prediction = prediction.predicted_value,
                "network_bandwidth_mbps" => network_prediction = prediction.predicted_value,
                "storage_operations_per_second" => storage_prediction = prediction.predicted_value,
                _ => {}
            }
        }

        // Calculate scaling recommendations
        let _current_cpu = predictions
            .iter()
            .find(|p| p.metric_name == "cpu_usage_percent")
            .map(|p| p.predicted_value)
            .unwrap_or(50.0);

        let scaling_needed = if cpu_prediction > 80.0 {
            ScalingRecommendation::ScaleUp {
                additional_nodes: ((cpu_prediction - 80.0) / 20.0).ceil() as usize,
                reason: "High predicted CPU usage".to_string(),
            }
        } else if cpu_prediction < 30.0 {
            ScalingRecommendation::ScaleDown {
                nodes_to_remove: ((30.0 - cpu_prediction) / 10.0).ceil() as usize,
                reason: "Low predicted CPU usage".to_string(),
            }
        } else {
            ScalingRecommendation::MaintainCurrent
        };

        Ok(ResourcePrediction {
            timestamp: current_timestamp(),
            prediction_horizon_hours: hours_ahead,
            predicted_cpu_usage: cpu_prediction,
            predicted_memory_usage: memory_prediction,
            predicted_network_usage: network_prediction,
            predicted_storage_ops: storage_prediction,
            scaling_recommendation: scaling_needed,
            confidence_score: 0.85, // Would be calculated based on model accuracy
        })
    }

    // Internal methods
    async fn add_data_point(&self, metric_name: &str, timestamp: u64, value: f64) -> Result<()> {
        let mut historical_data = self.historical_data.write().await;

        let data_queue = historical_data
            .entry(metric_name.to_string())
            .or_insert_with(|| {
                VecDeque::with_capacity(self.config.historical_window_days * 24)
                // Hourly data points
            });

        data_queue.push_back((timestamp, value));

        // Maintain window size
        let max_points = self.config.historical_window_days * 24;
        while data_queue.len() > max_points {
            data_queue.pop_front();
        }

        Ok(())
    }

    async fn predict_metric(
        &self,
        metric_name: &str,
        hours_ahead: usize,
    ) -> Result<PredictionResult> {
        let models = self.models.read().await;
        let historical_data = self.historical_data.read().await;

        let model = models.get(metric_name).ok_or_else(|| {
            BlockchainError::Analytics(format!("No model available for {}", metric_name))
        })?;

        let data = historical_data.get(metric_name).ok_or_else(|| {
            BlockchainError::Analytics(format!("No historical data for {}", metric_name))
        })?;

        if data.len() < 10 {
            return Err(BlockchainError::Analytics(
                "Insufficient historical data for prediction".to_string(),
            ));
        }

        // Use multiple forecasting methods and ensemble them
        let arima_prediction = self.arima_predict(data, hours_ahead)?;
        let exponential_smoothing = self.exponential_smoothing_predict(data, hours_ahead)?;
        let linear_regression = self.linear_regression_predict(data, hours_ahead)?;

        // Ensemble prediction (weighted average)
        let weights = [0.4, 0.3, 0.3]; // ARIMA gets highest weight
        let predictions = [arima_prediction, exponential_smoothing, linear_regression];
        let ensemble_prediction = predictions
            .iter()
            .zip(weights.iter())
            .map(|(pred, weight)| pred * weight)
            .sum::<f64>();

        // Calculate confidence interval
        let variance = predictions
            .iter()
            .map(|pred| (pred - ensemble_prediction).powi(2))
            .sum::<f64>()
            / predictions.len() as f64;
        let std_dev = variance.sqrt();
        let confidence_margin = 1.96 * std_dev; // 95% confidence interval

        // Determine trend direction
        let trend_direction = if ensemble_prediction > data.back().unwrap().1 * 1.05 {
            TrendDirection::Increasing
        } else if ensemble_prediction < data.back().unwrap().1 * 0.95 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        // Calculate accuracy score based on recent predictions
        let accuracy_score = if !model.accuracy_history.is_empty() {
            model.accuracy_history.iter().sum::<f64>() / model.accuracy_history.len() as f64
        } else {
            0.8 // Default accuracy
        };

        Ok(PredictionResult {
            metric_name: metric_name.to_string(),
            timestamp: current_timestamp() + (hours_ahead as u64 * 3600),
            predicted_value: ensemble_prediction,
            confidence_lower: (ensemble_prediction - confidence_margin).max(0.0),
            confidence_upper: ensemble_prediction + confidence_margin,
            trend_direction,
            seasonal_adjustment: 0.0, // Would be calculated with seasonal decomposition
            accuracy_score,
        })
    }

    async fn update_trend_analysis(&self, metric_name: &str) -> Result<()> {
        let historical_data = self.historical_data.read().await;

        let data = match historical_data.get(metric_name) {
            Some(data) if data.len() >= 24 => data, // Need at least 24 hours of data
            _ => return Ok(()),                     // Not enough data for trend analysis
        };

        // Calculate linear trend
        let n = data.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_xx = 0.0;

        for (i, (_, value)) in data.iter().enumerate() {
            let x = i as f64;
            let y = *value;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_xx += x * x;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        // Calculate trend strength (R-squared)
        let y_mean = sum_y / n;
        let mut ss_res = 0.0;
        let mut ss_tot = 0.0;

        for (i, (_, actual)) in data.iter().enumerate() {
            let predicted = slope * i as f64 + intercept;
            ss_res += (actual - predicted).powi(2);
            ss_tot += (actual - y_mean).powi(2);
        }

        let r_squared = 1.0 - (ss_res / ss_tot);
        let trend_strength = r_squared.clamp(0.0, 1.0);

        // Determine trend direction
        let trend_direction = if slope > 1.0 && trend_strength > 0.7 {
            TrendDirection::StronglyIncreasing
        } else if slope > 0.5 && trend_strength > 0.5 {
            TrendDirection::Increasing
        } else if slope < -1.0 && trend_strength > 0.7 {
            TrendDirection::StronglyDecreasing
        } else if slope < -0.5 && trend_strength > 0.5 {
            TrendDirection::Decreasing
        } else if trend_strength < 0.3 {
            TrendDirection::Volatile
        } else {
            TrendDirection::Stable
        };

        // Calculate volatility (coefficient of variation)
        let values: Vec<f64> = data.iter().map(|(_, v)| *v).collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let volatility = if mean != 0.0 {
            (variance.sqrt() / mean).abs()
        } else {
            0.0
        };

        // Detect seasonality (simplified)
        let seasonality_detected = self.detect_seasonality(&values);
        let seasonal_period = if seasonality_detected { Some(24) } else { None }; // Daily pattern

        // Get prediction accuracy from model
        let prediction_accuracy = {
            let models = self.models.read().await;
            models
                .get(metric_name)
                .and_then(|model| {
                    if !model.accuracy_history.is_empty() {
                        Some(
                            model.accuracy_history.iter().sum::<f64>()
                                / model.accuracy_history.len() as f64,
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or(0.8)
        };

        let trend_analysis = TrendAnalysis {
            metric_name: metric_name.to_string(),
            trend_direction,
            trend_strength,
            slope,
            seasonality_detected,
            seasonal_period_hours: seasonal_period,
            volatility,
            prediction_accuracy,
        };

        let mut trend_cache = self.trend_cache.write().await;
        trend_cache.insert(metric_name.to_string(), trend_analysis);

        Ok(())
    }

    async fn retrain_models(&self) -> Result<()> {
        let historical_data = self.historical_data.read().await;
        let mut models = self.models.write().await;

        for (metric_name, data) in historical_data.iter() {
            if data.len() < 50 {
                // Need minimum data for training
                continue;
            }

            // Train ARIMA-like model (simplified)
            let coefficients = self.train_arima_coefficients(data);
            let intercept = self.calculate_intercept(data, &coefficients);

            // Calculate seasonal components if enabled
            let seasonal_components = if self.config.seasonal_analysis_enabled {
                self.calculate_seasonal_components(data)
            } else {
                HashMap::new()
            };

            let model = PredictionModel {
                metric_name: metric_name.clone(),
                coefficients,
                intercept,
                seasonal_components,
                last_updated: current_timestamp(),
                accuracy_history: VecDeque::with_capacity(100),
                training_samples: data.len(),
            };

            models.insert(metric_name.clone(), model);
        }

        log::info!("Retrained predictive models for {} metrics", models.len());
        Ok(())
    }

    // Forecasting methods
    fn arima_predict(&self, data: &VecDeque<(u64, f64)>, _hours_ahead: usize) -> Result<f64> {
        if data.len() < 5 {
            return Err(BlockchainError::Analytics(
                "Insufficient data for ARIMA prediction".to_string(),
            ));
        }

        // Simplified ARIMA(1,0,1) prediction
        let values: Vec<f64> = data.iter().rev().take(24).map(|(_, v)| *v).collect(); // Last 24 hours
        let mut prediction = values[0]; // Start with most recent value

        // Apply autoregressive component
        if values.len() >= 2 {
            let ar_coeff = 0.7; // Simplified AR coefficient
            prediction += ar_coeff * (values[0] - values[1]);
        }

        // Apply moving average component
        if values.len() >= 3 {
            let ma_coeff = 0.3; // Simplified MA coefficient
            let residuals: Vec<f64> = values.windows(2).map(|w| w[0] - w[1]).collect();
            if !residuals.is_empty() {
                prediction += ma_coeff * residuals[residuals.len() - 1];
            }
        }

        Ok(prediction.max(0.0))
    }

    fn exponential_smoothing_predict(
        &self,
        data: &VecDeque<(u64, f64)>,
        _hours_ahead: usize,
    ) -> Result<f64> {
        if data.is_empty() {
            return Err(BlockchainError::Analytics(
                "No data for exponential smoothing".to_string(),
            ));
        }

        let values: Vec<f64> = data.iter().map(|(_, v)| *v).collect();
        let alpha = 0.3; // Smoothing factor

        let mut smoothed = values[0];
        for &value in values.iter().skip(1) {
            smoothed = alpha * value + (1.0 - alpha) * smoothed;
        }

        Ok(smoothed.max(0.0))
    }

    fn linear_regression_predict(
        &self,
        data: &VecDeque<(u64, f64)>,
        hours_ahead: usize,
    ) -> Result<f64> {
        if data.len() < 2 {
            return Err(BlockchainError::Analytics(
                "Insufficient data for linear regression".to_string(),
            ));
        }

        let n = data.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_xx = 0.0;

        for (i, (_, value)) in data.iter().enumerate() {
            let x = i as f64;
            let y = *value;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_xx += x * x;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        let future_x = (data.len() + hours_ahead) as f64;
        let prediction = slope * future_x + intercept;

        Ok(prediction.max(0.0))
    }

    // Helper methods
    fn train_arima_coefficients(&self, _data: &VecDeque<(u64, f64)>) -> Vec<f64> {
        // Simplified coefficient training
        vec![0.6, 0.3, 0.1] // AR, I, MA coefficients
    }

    fn calculate_intercept(&self, data: &VecDeque<(u64, f64)>, _coefficients: &[f64]) -> f64 {
        let values: Vec<f64> = data.iter().map(|(_, v)| *v).collect();
        values.iter().sum::<f64>() / values.len() as f64
    }

    fn calculate_seasonal_components(&self, data: &VecDeque<(u64, f64)>) -> HashMap<usize, f64> {
        let mut components = HashMap::new();

        // Simplified seasonal decomposition (daily pattern)
        if data.len() >= 48 {
            // At least 2 days
            for hour in 0..24 {
                let hour_values: Vec<f64> = data
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| i % 24 == hour)
                    .map(|(_, (_, v))| *v)
                    .collect();

                if !hour_values.is_empty() {
                    let avg = hour_values.iter().sum::<f64>() / hour_values.len() as f64;
                    components.insert(hour, avg);
                }
            }
        }

        components
    }

    fn detect_seasonality(&self, values: &[f64]) -> bool {
        if values.len() < 48 {
            // Need at least 2 days
            return false;
        }

        // Simple autocorrelation-based seasonality detection
        let mut autocorr = 0.0;
        let mean = values.iter().sum::<f64>() / values.len() as f64;

        for i in 24..values.len() {
            autocorr += (values[i] - mean) * (values[i - 24] - mean);
        }

        autocorr /= values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;

        if variance > 0.0 {
            let correlation = autocorr / variance;
            correlation > 0.5 // Strong correlation indicates seasonality
        } else {
            false
        }
    }
}

/// Resource prediction with scaling recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePrediction {
    pub timestamp: u64,
    pub prediction_horizon_hours: usize,
    pub predicted_cpu_usage: f64,
    pub predicted_memory_usage: f64,
    pub predicted_network_usage: f64,
    pub predicted_storage_ops: f64,
    pub scaling_recommendation: ScalingRecommendation,
    pub confidence_score: f64,
}

/// Scaling recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingRecommendation {
    ScaleUp {
        additional_nodes: usize,
        reason: String,
    },
    ScaleDown {
        nodes_to_remove: usize,
        reason: String,
    },
    MaintainCurrent,
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
    async fn test_predictive_analytics_creation() {
        let config = PredictiveConfig::default();
        let analytics = PredictiveAnalytics::new(config);

        assert!(analytics.config.enable_predictions);
        assert_eq!(analytics.config.prediction_horizon_hours, 24);
    }

    #[tokio::test]
    async fn test_trend_analysis() {
        let config = PredictiveConfig::default();
        let analytics = PredictiveAnalytics::new(config);

        // Add some test data
        for i in 0..50 {
            analytics
                .add_data_point(
                    "cpu_usage_percent",
                    current_timestamp() - (i * 3600),
                    50.0 + i as f64,
                )
                .await
                .unwrap();
        }

        analytics
            .update_trend_analysis("cpu_usage_percent")
            .await
            .unwrap();

        let trend = analytics
            .get_trend_analysis("cpu_usage_percent")
            .await
            .unwrap();
        assert!(trend.is_some());

        let trend = trend.unwrap();
        assert_eq!(trend.metric_name, "cpu_usage_percent");
        assert!(matches!(
            trend.trend_direction,
            TrendDirection::Increasing | TrendDirection::StronglyIncreasing
        ));
    }

    #[tokio::test]
    async fn test_linear_regression_prediction() {
        let config = PredictiveConfig::default();
        let analytics = PredictiveAnalytics::new(config);

        // Add linear trend data
        for i in 0..24 {
            analytics
                .add_data_point(
                    "test_metric",
                    current_timestamp() - (i * 3600),
                    10.0 + i as f64 * 2.0,
                )
                .await
                .unwrap();
        }

        let prediction = analytics
            .linear_regression_predict(
                &analytics
                    .historical_data
                    .read()
                    .await
                    .get("test_metric")
                    .unwrap()
                    .clone(),
                1,
            )
            .unwrap();

        assert!(prediction > 50.0); // Should predict continuation of upward trend
    }
}
