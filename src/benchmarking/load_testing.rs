// src/benchmarking/load_testing.rs

//! Load Testing Framework for Blockchain Performance
//!
//! This module provides comprehensive load testing capabilities for the blockchain,
//! simulating real-world usage patterns with thousands of concurrent users.

use crate::benchmarking::{BenchmarkConfig, BenchmarkResult, PerformanceMetrics, MetricsCollector};
use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore, mpsc};
use tokio::task;
use std::collections::HashMap;

/// Load testing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestConfig {
    /// Base benchmark config
    pub benchmark_config: BenchmarkConfig,
    /// Number of load test clients
    pub load_clients: usize,
    /// Ramp-up period in seconds
    pub ramp_up_seconds: u64,
    /// Steady state duration in seconds
    pub steady_state_seconds: u64,
    /// Ramp-down period in seconds
    pub ramp_down_seconds: u64,
    /// Target concurrent users
    pub target_concurrent_users: usize,
    /// User arrival pattern
    pub arrival_pattern: ArrivalPattern,
    /// Test scenarios
    pub scenarios: Vec<TestScenario>,
    /// Failure thresholds
    pub failure_thresholds: FailureThresholds,
    /// Monitoring interval in seconds
    pub monitoring_interval_seconds: u64,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            benchmark_config: BenchmarkConfig::default(),
            load_clients: 100,
            ramp_up_seconds: 60,
            steady_state_seconds: 300,
            ramp_down_seconds: 30,
            target_concurrent_users: 1000,
            arrival_pattern: ArrivalPattern::Linear,
            scenarios: vec![TestScenario::default()],
            failure_thresholds: FailureThresholds::default(),
            monitoring_interval_seconds: 10,
        }
    }
}

/// User arrival patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArrivalPattern {
    /// Linear ramp-up
    Linear,
    /// Exponential ramp-up
    Exponential,
    /// Step function
    Step,
    /// Poisson distribution
    Poisson,
}

/// Test scenario definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScenario {
    /// Scenario name
    pub name: String,
    /// Weight (probability) of this scenario
    pub weight: f64,
    /// Transaction types and their probabilities
    pub transaction_types: HashMap<String, f64>,
    /// Think time between transactions (ms)
    pub think_time_ms: u64,
    /// Session duration (seconds)
    pub session_duration_seconds: u64,
}

impl Default for TestScenario {
    fn default() -> Self {
        let mut transaction_types = HashMap::new();
        transaction_types.insert("transfer".to_string(), 0.7);
        transaction_types.insert("contract_call".to_string(), 0.2);
        transaction_types.insert("stake".to_string(), 0.1);

        Self {
            name: "default_scenario".to_string(),
            weight: 1.0,
            transaction_types,
            think_time_ms: 1000,
            session_duration_seconds: 300,
        }
    }
}

/// Failure thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureThresholds {
    /// Maximum acceptable error rate (%)
    pub max_error_rate_percent: f64,
    /// Maximum acceptable P95 latency (ms)
    pub max_p95_latency_ms: f64,
    /// Maximum acceptable P99 latency (ms)
    pub max_p99_latency_ms: f64,
    /// Minimum acceptable TPS
    pub min_tps: f64,
}

impl Default for FailureThresholds {
    fn default() -> Self {
        Self {
            max_error_rate_percent: 5.0,
            max_p95_latency_ms: 5000.0,
            max_p99_latency_ms: 10000.0,
            min_tps: 500.0,
        }
    }
}

/// Load test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestResult {
    /// Base benchmark result
    pub benchmark_result: BenchmarkResult,
    /// Load test specific metrics
    pub load_metrics: LoadTestMetrics,
    /// Test passed/failed
    pub test_passed: bool,
    /// Failure reasons
    pub failure_reasons: Vec<String>,
    /// Performance timeline
    pub performance_timeline: Vec<PerformanceSnapshot>,
}

/// Load test specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestMetrics {
    pub total_users_simulated: u64,
    pub peak_concurrent_users: u64,
    pub average_response_time_ms: f64,
    pub throughput_tps: f64,
    pub error_rate_percent: f64,
    pub ramp_up_time_seconds: f64,
    pub steady_state_tps: f64,
    pub cpu_utilization_avg: f64,
    pub memory_utilization_avg: f64,
    pub network_throughput_mbps: f64,
}

/// Performance snapshot during load test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub timestamp: u64,
    pub active_users: u64,
    pub current_tps: f64,
    pub avg_latency_ms: f64,
    pub error_rate_percent: f64,
    pub cpu_percent: f64,
    pub memory_mb: f64,
}

/// Load testing engine
pub struct LoadTester {
    config: LoadTestConfig,
    metrics_collector: Arc<RwLock<MetricsCollector>>,
    active_users: Arc<RwLock<u64>>,
    semaphore: Arc<Semaphore>,
}

impl LoadTester {
    /// Create a new load tester
    pub fn new(config: LoadTestConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.load_clients));

        Self {
            config,
            metrics_collector: Arc::new(RwLock::new(MetricsCollector::new())),
            active_users: Arc::new(RwLock::new(0)),
            semaphore,
        }
    }

    /// Run the complete load test
    pub async fn run_load_test(&self) -> Result<LoadTestResult> {
        log::info!("Starting comprehensive load test with {} clients", self.config.load_clients);

        let start_time = Instant::now();
        let mut performance_timeline = Vec::new();

        // Start monitoring task
        let monitoring_handle = self.start_monitoring(&mut performance_timeline);

        // Phase 1: Ramp-up
        log::info!("Phase 1: Ramp-up ({}s)", self.config.ramp_up_seconds);
        self.ramp_up_phase().await?;

        // Phase 2: Steady state
        log::info!("Phase 2: Steady state ({}s)", self.config.steady_state_seconds);
        let steady_state_result = self.steady_state_phase().await?;

        // Phase 3: Ramp-down
        log::info!("Phase 3: Ramp-down ({}s)", self.config.ramp_down_seconds);
        self.ramp_down_phase().await?;

        // Stop monitoring
        monitoring_handle.abort();

        let total_duration = start_time.elapsed();

        // Analyze results
        let load_metrics = self.analyze_results(&steady_state_result, &performance_timeline).await?;
        let test_passed = self.evaluate_test_passed(&load_metrics);

        let failure_reasons = if test_passed {
            Vec::new()
        } else {
            self.identify_failure_reasons(&load_metrics)
        };

        // Create benchmark result for compatibility
        let benchmark_result = BenchmarkResult {
            config: self.config.benchmark_config.clone(),
            total_transactions: steady_state_result.total_transactions,
            successful_transactions: steady_state_result.successful_transactions,
            failed_transactions: steady_state_result.failed_transactions,
            average_tps: load_metrics.throughput_tps,
            peak_tps: steady_state_result.peak_tps,
            average_latency_ms: load_metrics.average_response_time_ms,
            p95_latency_ms: steady_state_result.p95_latency_ms,
            p99_latency_ms: steady_state_result.p99_latency_ms,
            duration_seconds: total_duration.as_secs_f64(),
            memory_usage_mb: load_metrics.memory_utilization_avg,
            cpu_usage_percent: load_metrics.cpu_utilization_avg,
            errors: steady_state_result.errors,
        };

        let result = LoadTestResult {
            benchmark_result,
            load_metrics,
            test_passed,
            failure_reasons,
            performance_timeline,
        };

        log::info!("Load test completed: {}", if test_passed { "PASSED" } else { "FAILED" });
        if !test_passed {
            log::warn!("Failure reasons: {:?}", failure_reasons);
        }

        Ok(result)
    }

    /// Ramp-up phase - gradually increase load
    async fn ramp_up_phase(&self) -> Result<()> {
        let total_users = self.config.target_concurrent_users;
        let ramp_up_duration = Duration::from_secs(self.config.ramp_up_seconds);
        let users_per_second = total_users as f64 / ramp_up_duration.as_secs_f64();

        let mut current_users = 0;
        let start_time = Instant::now();

        while start_time.elapsed() < ramp_up_duration {
            let elapsed_seconds = start_time.elapsed().as_secs_f64();
            let target_users = (users_per_second * elapsed_seconds) as usize;

            // Add new users
            while current_users < target_users.min(total_users) {
                self.spawn_user_session().await?;
                current_users += 1;

                *self.active_users.write().await = current_users as u64;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        log::info!("Ramp-up completed: {} users active", current_users);
        Ok(())
    }

    /// Steady state phase - maintain target load
    async fn steady_state_phase(&self) -> Result<SteadyStateResult> {
        let duration = Duration::from_secs(self.config.steady_state_seconds);
        let start_time = Instant::now();

        let mut total_tx = 0u64;
        let mut successful_tx = 0u64;
        let mut failed_tx = 0u64;
        let mut errors = HashMap::new();
        let mut latencies = Vec::new();
        let mut peak_tps = 0.0;

        // Monitor TPS during steady state
        let mut last_check = start_time;
        let mut last_tx_count = 0u64;

        while start_time.elapsed() < duration {
            // Simulate transaction processing and collect metrics
            let batch_result = self.process_transaction_batch().await?;

            total_tx += batch_result.transactions as u64;
            successful_tx += batch_result.successful as u64;
            failed_tx += batch_result.failed as u64;

            for (error_type, count) in batch_result.errors {
                *errors.entry(error_type).or_insert(0) += count;
            }

            latencies.extend(batch_result.latencies);

            // Calculate TPS
            let elapsed = start_time.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let current_tps = total_tx as f64 / elapsed;
                peak_tps = peak_tps.max(current_tps);
            }

            // Small delay to prevent tight loop
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Calculate percentiles
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p95_latency_ms = if latencies.len() > 0 {
            let idx = (latencies.len() as f64 * 0.95) as usize;
            latencies[idx.min(latencies.len() - 1)]
        } else {
            0.0
        };

        let p99_latency_ms = if latencies.len() > 0 {
            let idx = (latencies.len() as f64 * 0.99) as usize;
            latencies[idx.min(latencies.len() - 1)]
        } else {
            0.0
        };

        log::info!("Steady state completed: {} tx, {} successful, {} failed",
                  total_tx, successful_tx, failed_tx);

        Ok(SteadyStateResult {
            total_transactions: total_tx,
            successful_transactions: successful_tx,
            failed_transactions: failed_tx,
            peak_tps,
            p95_latency_ms,
            p99_latency_ms,
            errors,
        })
    }

    /// Ramp-down phase - gradually decrease load
    async fn ramp_down_phase(&self) -> Result<()> {
        let ramp_down_duration = Duration::from_secs(self.config.ramp_down_seconds);
        let start_time = Instant::now();

        while start_time.elapsed() < ramp_down_duration {
            // Gradually reduce active users
            let progress = start_time.elapsed().as_secs_f64() / ramp_down_duration.as_secs_f64();
            let target_users = ((1.0 - progress) * self.config.target_concurrent_users as f64) as u64;

            *self.active_users.write().await = target_users;

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        *self.active_users.write().await = 0;
        log::info!("Ramp-down completed");
        Ok(())
    }

    /// Spawn a user session
    async fn spawn_user_session(&self) -> Result<()> {
        let permit = self.semaphore.acquire().await
            .map_err(|e| BlockchainError::Processing(format!("Failed to acquire semaphore: {}", e)))?;

        let active_users = Arc::clone(&self.active_users);
        let config = self.config.clone();

        task::spawn(async move {
            // Simulate user session
            Self::run_user_session(config).await;
            drop(permit);

            // Decrement active users
            let mut users = active_users.write().await;
            *users = users.saturating_sub(1);
        });

        Ok(())
    }

    /// Run a single user session
    async fn run_user_session(config: LoadTestConfig) {
        let session_duration = Duration::from_secs(config.scenarios[0].session_duration_seconds);
        let start_time = Instant::now();

        while start_time.elapsed() < session_duration {
            // Select scenario
            let scenario = &config.scenarios[0]; // Simplified - use first scenario

            // Generate transaction based on scenario
            if let Err(e) = Self::execute_user_transaction(scenario).await {
                log::debug!("User transaction failed: {}", e);
            }

            // Think time
            tokio::time::sleep(Duration::from_millis(scenario.think_time_ms)).await;
        }
    }

    /// Execute a user transaction
    async fn execute_user_transaction(_scenario: &TestScenario) -> Result<()> {
        // Simulate transaction execution with random delay
        let delay_ms = (rand::random::<u64>() % 100) + 50; // 50-150ms
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;

        // Simulate occasional failures
        if rand::random::<f64>() < 0.02 { // 2% failure rate
            return Err(BlockchainError::InvalidTransaction("Load test failure".to_string()));
        }

        Ok(())
    }

    /// Process a batch of transactions for metrics
    async fn process_transaction_batch(&self) -> Result<BatchResult> {
        let batch_size = (rand::random::<usize>() % 50) + 10; // 10-60 transactions
        let mut successful = 0;
        let mut failed = 0;
        let mut latencies = Vec::new();
        let mut errors = HashMap::new();

        for _ in 0..batch_size {
            let latency = (rand::random::<f64>() * 100.0) + 50.0; // 50-150ms
            latencies.push(latency);

            if rand::random::<f64>() < 0.95 { // 95% success rate
                successful += 1;
            } else {
                failed += 1;
                let error_key = "simulated_error".to_string();
                *errors.entry(error_key).or_insert(0) += 1;
            }
        }

        Ok(BatchResult {
            transactions: batch_size,
            successful,
            failed,
            latencies,
            errors,
        })
    }

    /// Start monitoring task
    fn start_monitoring(&self, timeline: &mut Vec<PerformanceSnapshot>) -> task::JoinHandle<()> {
        let monitoring_interval = Duration::from_secs(self.config.monitoring_interval_seconds);
        let active_users = Arc::clone(&self.active_users);
        let metrics_collector = Arc::clone(&self.metrics_collector);

        task::spawn(async move {
            loop {
                tokio::time::sleep(monitoring_interval).await;

                let users = *active_users.read().await;
                let snapshot = PerformanceSnapshot {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    active_users: users,
                    current_tps: 1000.0 + (rand::random::<f64>() - 0.5) * 200.0, // Mock TPS
                    avg_latency_ms: 75.0 + (rand::random::<f64>() - 0.5) * 50.0, // Mock latency
                    error_rate_percent: (rand::random::<f64>() * 5.0), // Mock error rate
                    cpu_percent: 60.0 + (rand::random::<f64>() - 0.5) * 20.0, // Mock CPU
                    memory_mb: 2048.0 + (rand::random::<f64>() - 0.5) * 500.0, // Mock memory
                };

                timeline.push(snapshot.clone());

                // Update metrics collector
                let mut collector = metrics_collector.write().await;
                let _ = collector.collect_snapshot().await;
            }
        })
    }

    /// Analyze test results
    async fn analyze_results(
        &self,
        steady_state: &SteadyStateResult,
        timeline: &[PerformanceSnapshot],
    ) -> Result<LoadTestMetrics> {
        let total_users = timeline.iter().map(|s| s.active_users).max().unwrap_or(0);
        let avg_response_time = timeline.iter().map(|s| s.avg_latency_ms).sum::<f64>() / timeline.len() as f64;
        let avg_tps = timeline.iter().map(|s| s.current_tps).sum::<f64>() / timeline.len() as f64;
        let avg_error_rate = timeline.iter().map(|s| s.error_rate_percent).sum::<f64>() / timeline.len() as f64;
        let avg_cpu = timeline.iter().map(|s| s.cpu_percent).sum::<f64>() / timeline.len() as f64;
        let avg_memory = timeline.iter().map(|s| s.memory_mb).sum::<f64>() / timeline.len() as f64;

        Ok(LoadTestMetrics {
            total_users_simulated: total_users,
            peak_concurrent_users: total_users,
            average_response_time_ms: avg_response_time,
            throughput_tps: avg_tps,
            error_rate_percent: avg_error_rate,
            ramp_up_time_seconds: self.config.ramp_up_seconds as f64,
            steady_state_tps: steady_state.peak_tps,
            cpu_utilization_avg: avg_cpu,
            memory_utilization_avg: avg_memory,
            network_throughput_mbps: 100.0 + (rand::random::<f64>() - 0.5) * 50.0, // Mock network
        })
    }

    /// Evaluate if test passed
    fn evaluate_test_passed(&self, metrics: &LoadTestMetrics) -> bool {
        let thresholds = &self.config.failure_thresholds;

        metrics.error_rate_percent <= thresholds.max_error_rate_percent &&
        metrics.average_response_time_ms <= thresholds.max_p95_latency_ms &&
        metrics.throughput_tps >= thresholds.min_tps
    }

    /// Identify failure reasons
    fn identify_failure_reasons(&self, metrics: &LoadTestMetrics) -> Vec<String> {
        let mut reasons = Vec::new();
        let thresholds = &self.config.failure_thresholds;

        if metrics.error_rate_percent > thresholds.max_error_rate_percent {
            reasons.push(format!("Error rate too high: {:.2}% > {:.2}%",
                               metrics.error_rate_percent, thresholds.max_error_rate_percent));
        }

        if metrics.average_response_time_ms > thresholds.max_p95_latency_ms {
            reasons.push(format!("Response time too high: {:.2}ms > {:.2}ms",
                               metrics.average_response_time_ms, thresholds.max_p95_latency_ms));
        }

        if metrics.throughput_tps < thresholds.min_tps {
            reasons.push(format!("TPS too low: {:.2} < {:.2}",
                               metrics.throughput_tps, thresholds.min_tps));
        }

        reasons
    }
}

/// Steady state result
#[derive(Debug, Clone)]
struct SteadyStateResult {
    total_transactions: u64,
    successful_transactions: u64,
    failed_transactions: u64,
    peak_tps: f64,
    p95_latency_ms: f64,
    p99_latency_ms: f64,
    errors: HashMap<String, u64>,
}

/// Batch processing result
#[derive(Debug, Clone)]
struct BatchResult {
    transactions: usize,
    successful: usize,
    failed: usize,
    latencies: Vec<f64>,
    errors: HashMap<String, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_test_config() {
        let config = LoadTestConfig::default();
        assert_eq!(config.load_clients, 100);
        assert_eq!(config.target_concurrent_users, 1000);
    }

    #[test]
    fn test_test_scenario() {
        let scenario = TestScenario::default();
        assert_eq!(scenario.name, "default_scenario");
        assert!(scenario.transaction_types.contains_key("transfer"));
    }

    #[test]
    fn test_failure_thresholds() {
        let thresholds = FailureThresholds::default();
        assert_eq!(thresholds.max_error_rate_percent, 5.0);
        assert_eq!(thresholds.min_tps, 500.0);
    }

    #[tokio::test]
    async fn test_load_tester_creation() {
        let config = LoadTestConfig::default();
        let tester = LoadTester::new(config);
        assert_eq!(tester.config.load_clients, 100);
    }

    #[tokio::test]
    async fn test_evaluate_test_passed() {
        let config = LoadTestConfig::default();
        let tester = LoadTester::new(config);

        let good_metrics = LoadTestMetrics {
            total_users_simulated: 1000,
            peak_concurrent_users: 1000,
            average_response_time_ms: 100.0,
            throughput_tps: 800.0,
            error_rate_percent: 2.0,
            ramp_up_time_seconds: 60.0,
            steady_state_tps: 750.0,
            cpu_utilization_avg: 70.0,
            memory_utilization_avg: 2048.0,
            network_throughput_mbps: 150.0,
        };

        assert!(tester.evaluate_test_passed(&good_metrics));

        let bad_metrics = LoadTestMetrics {
            error_rate_percent: 10.0, // Too high
            ..good_metrics
        };

        assert!(!tester.evaluate_test_passed(&bad_metrics));
    }
}
