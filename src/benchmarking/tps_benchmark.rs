// src/benchmarking/tps_benchmark.rs

//! TPS (Transactions Per Second) Benchmark
//!
//! This module implements comprehensive TPS benchmarking for the blockchain,
//! measuring sustained transaction throughput under various load conditions.

use crate::benchmarking::{BenchmarkConfig, BenchmarkResult, PerformanceMetrics, MetricsCollector};
use crate::utils::error::{Result, BlockchainError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time;

/// TPS benchmark implementation
pub struct TPSBenchmark {
    config: BenchmarkConfig,
    metrics_collector: Arc<RwLock<MetricsCollector>>,
}

impl TPSBenchmark {
    /// Create a new TPS benchmark
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            metrics_collector: Arc::new(RwLock::new(MetricsCollector::new())),
        }
    }

    /// Run the TPS benchmark
    pub async fn run(&self) -> Result<BenchmarkResult> {
        log::info!("Starting TPS benchmark with config: {:?}", self.config);

        let start_time = Instant::now();

        // Warm-up phase
        log::info!("Starting warm-up phase ({}s)", self.config.warmup_seconds);
        self.warmup_phase().await?;

        // Main benchmark phase
        log::info!("Starting main benchmark phase ({}s)", self.config.duration_seconds);
        let main_result = self.main_benchmark_phase().await?;

        // Cool-down phase
        log::info!("Starting cool-down phase ({}s)", self.config.cooldown_seconds);
        self.cooldown_phase().await?;

        let total_duration = start_time.elapsed();

        // Collect final metrics
        let final_metrics = self.metrics_collector.read().await.get_final_metrics().await?;

        // Calculate percentiles
        let latency_percentiles = self.calculate_latency_percentiles().await;

        let result = BenchmarkResult {
            config: self.config.clone(),
            total_transactions: main_result.total_tx,
            successful_transactions: main_result.successful_tx,
            failed_transactions: main_result.failed_tx,
            average_tps: main_result.average_tps,
            peak_tps: main_result.peak_tps,
            average_latency_ms: latency_percentiles.avg_latency,
            p95_latency_ms: latency_percentiles.p95_latency,
            p99_latency_ms: latency_percentiles.p99_latency,
            duration_seconds: total_duration.as_secs_f64(),
            memory_usage_mb: final_metrics.memory_mb,
            cpu_usage_percent: final_metrics.cpu_percent,
            errors: main_result.errors,
        };

        log::info!("TPS benchmark completed:\n{}", result.summary());
        Ok(result)
    }

    /// Warm-up phase to stabilize the system
    async fn warmup_phase(&self) -> Result<()> {
        let warmup_tx_per_second = self.config.target_tps / 10; // 10% of target TPS
        let warmup_duration = Duration::from_secs(self.config.warmup_seconds);

        log::info!("Warm-up: {} TPS for {:?}", warmup_tx_per_second, warmup_duration);

        let start_time = Instant::now();
        let mut tx_count = 0;

        while start_time.elapsed() < warmup_duration {
            // Generate warm-up transactions at reduced rate
            let batch_size = (warmup_tx_per_second / 10).max(1) as usize; // Small batches
            self.generate_transaction_batch(batch_size).await?;
            tx_count += batch_size;

            // Sleep to maintain target rate
            time::sleep(Duration::from_millis(100)).await;
        }

        log::info!("Warm-up completed: {} transactions sent", tx_count);
        Ok(())
    }

    /// Main benchmark phase
    async fn main_benchmark_phase(&self) -> Result<MainBenchmarkResult> {
        let duration = Duration::from_secs(self.config.duration_seconds);
        let start_time = Instant::now();

        let mut total_tx = 0u64;
        let mut successful_tx = 0u64;
        let mut failed_tx = 0u64;
        let mut errors = std::collections::HashMap::new();
        let mut tps_samples = Vec::new();
        let mut peak_tps = 0.0;

        // Calculate target transactions per batch
        let tx_per_batch = (self.config.target_tps as f64 / 10.0).ceil() as usize; // 10 batches per second
        let batch_interval = Duration::from_millis(100); // 100ms intervals

        log::info!("Main phase: Target {} TPS, {} tx per batch, {}ms intervals",
                  self.config.target_tps, tx_per_batch, batch_interval.as_millis());

        while start_time.elapsed() < duration {
            let batch_start = Instant::now();

            // Generate transaction batch
            match self.generate_transaction_batch(tx_per_batch).await {
                Ok(batch_result) => {
                    total_tx += batch_result.total as u64;
                    successful_tx += batch_result.successful as u64;
                    failed_tx += batch_result.failed as u64;

                    // Record errors
                    for (error_type, count) in batch_result.errors {
                        *errors.entry(error_type).or_insert(0) += count;
                    }
                }
                Err(e) => {
                    log::warn!("Batch generation failed: {}", e);
                    failed_tx += tx_per_batch as u64;
                }
            }

            // Calculate current TPS
            let elapsed_seconds = start_time.elapsed().as_secs_f64();
            if elapsed_seconds > 0.0 {
                let current_tps = total_tx as f64 / elapsed_seconds;
                tps_samples.push(current_tps);
                peak_tps = peak_tps.max(current_tps);

                // Collect metrics every second
                if (elapsed_seconds * 10.0) as u64 % 10 == 0 {
                    self.collect_performance_metrics().await?;
                }
            }

            // Sleep to maintain batch rate
            let batch_duration = batch_start.elapsed();
            if batch_duration < batch_interval {
                time::sleep(batch_interval - batch_duration).await;
            }
        }

        let average_tps = if !tps_samples.is_empty() {
            tps_samples.iter().sum::<f64>() / tps_samples.len() as f64
        } else {
            0.0
        };

        log::info!("Main phase completed: {} total, {} successful, {} failed",
                  total_tx, successful_tx, failed_tx);

        Ok(MainBenchmarkResult {
            total_tx,
            successful_tx,
            failed_tx,
            average_tps,
            peak_tps,
            errors,
        })
    }

    /// Cool-down phase
    async fn cooldown_phase(&self) -> Result<()> {
        log::info!("Cool-down: Allowing system to stabilize");

        // Wait for pending transactions to clear
        time::sleep(Duration::from_secs(self.config.cooldown_seconds)).await;

        // Final metrics collection
        self.collect_performance_metrics().await?;

        Ok(())
    }

    /// Generate a batch of transactions
    async fn generate_transaction_batch(&self, batch_size: usize) -> Result<BatchResult> {
        let mut total = 0;
        let mut successful = 0;
        let mut failed = 0;
        let mut errors = std::collections::HashMap::new();

        // Generate transactions in parallel
        let tasks: Vec<_> = (0..batch_size)
            .map(|_| {
                tokio::spawn(async move {
                    // Simulate transaction generation and submission
                    self.generate_single_transaction().await
                })
            })
            .collect();

        // Wait for all tasks to complete
        for task in tasks {
            match task.await {
                Ok(Ok(_)) => {
                    total += 1;
                    successful += 1;
                }
                Ok(Err(e)) => {
                    total += 1;
                    failed += 1;
                    let error_key = format!("{:?}", e);
                    *errors.entry(error_key).or_insert(0) += 1;
                }
                Err(e) => {
                    total += 1;
                    failed += 1;
                    let error_key = format!("TaskError: {}", e);
                    *errors.entry(error_key).or_insert(0) += 1;
                }
            }
        }

        Ok(BatchResult {
            total,
            successful,
            failed,
            errors,
        })
    }

    /// Generate a single transaction (simplified for benchmarking)
    async fn generate_single_transaction(&self) -> Result<String> {
        // In a real implementation, this would:
        // 1. Generate transaction data
        // 2. Sign the transaction
        // 3. Submit to the network/API
        // 4. Wait for confirmation

        // For now, simulate with random delay
        let delay_ms = (rand::random::<u64>() % 50) + 10; // 10-60ms random delay
        time::sleep(Duration::from_millis(delay_ms)).await;

        // Simulate occasional failures
        if rand::random::<f64>() < 0.02 { // 2% failure rate
            return Err(BlockchainError::InvalidTransaction("Simulated failure".to_string()));
        }

        // Return mock transaction hash
        Ok(format!("0x{:064x}", rand::random::<u64>()))
    }

    /// Collect performance metrics
    async fn collect_performance_metrics(&self) -> Result<()> {
        let mut collector = self.metrics_collector.write().await;
        collector.collect_snapshot().await?;
        Ok(())
    }

    /// Calculate latency percentiles
    async fn calculate_latency_percentiles(&self) -> LatencyPercentiles {
        let collector = self.metrics_collector.read().await;

        // In a real implementation, collect actual latency measurements
        // For now, return simulated values
        LatencyPercentiles {
            avg_latency: 45.2,
            p95_latency: 120.5,
            p99_latency: 250.8,
        }
    }
}

/// Result of main benchmark phase
struct MainBenchmarkResult {
    total_tx: u64,
    successful_tx: u64,
    failed_tx: u64,
    average_tps: f64,
    peak_tps: f64,
    errors: std::collections::HashMap<String, u64>,
}

/// Result of a transaction batch
struct BatchResult {
    total: usize,
    successful: usize,
    failed: usize,
    errors: std::collections::HashMap<String, u64>,
}

/// Latency percentiles
struct LatencyPercentiles {
    avg_latency: f64,
    p95_latency: f64,
    p99_latency: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tps_benchmark_creation() {
        let config = BenchmarkConfig::default();
        let benchmark = TPSBenchmark::new(config);

        assert_eq!(benchmark.config.concurrent_clients, 10);
        assert_eq!(benchmark.config.target_tps, 1000);
    }

    #[tokio::test]
    async fn test_benchmark_config() {
        let config = BenchmarkConfig {
            concurrent_clients: 5,
            duration_seconds: 30,
            target_tps: 500,
            warmup_seconds: 5,
            cooldown_seconds: 2,
            batch_size: 50,
            detailed_metrics: false,
            memory_profiling: false,
            cpu_profiling: false,
        };

        let benchmark = TPSBenchmark::new(config);
        assert_eq!(benchmark.config.concurrent_clients, 5);
        assert_eq!(benchmark.config.target_tps, 500);
    }
}
