// src/benchmarking/mod.rs

//! Performance Benchmarking Suite
//!
//! This module provides comprehensive benchmarking tools for measuring
//! blockchain performance, including TPS, latency, throughput, and scalability metrics.

pub mod tps_benchmark;
pub mod latency_benchmark;
pub mod throughput_benchmark;
pub mod load_generator;
pub mod metrics_collector;
pub mod report_generator;
pub mod load_testing;
pub mod profiling;

pub use tps_benchmark::TPSBenchmark;
pub use latency_benchmark::LatencyBenchmark;
pub use throughput_benchmark::ThroughputBenchmark;
pub use load_generator::LoadGenerator;
pub use metrics_collector::MetricsCollector;
pub use report_generator::BenchmarkReport;

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of concurrent clients
    pub concurrent_clients: usize,
    /// Duration of benchmark in seconds
    pub duration_seconds: u64,
    /// Target TPS for load testing
    pub target_tps: u64,
    /// Warm-up period in seconds
    pub warmup_seconds: u64,
    /// Cool-down period in seconds
    pub cooldown_seconds: u64,
    /// Transaction batch size
    pub batch_size: usize,
    /// Enable detailed metrics collection
    pub detailed_metrics: bool,
    /// Memory profiling enabled
    pub memory_profiling: bool,
    /// CPU profiling enabled
    pub cpu_profiling: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            concurrent_clients: 10,
            duration_seconds: 60,
            target_tps: 1000,
            warmup_seconds: 10,
            cooldown_seconds: 5,
            batch_size: 100,
            detailed_metrics: true,
            memory_profiling: true,
            cpu_profiling: true,
        }
    }
}

/// Benchmark result summary
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Test configuration
    pub config: BenchmarkConfig,
    /// Total transactions processed
    pub total_transactions: u64,
    /// Successful transactions
    pub successful_transactions: u64,
    /// Failed transactions
    pub failed_transactions: u64,
    /// Average TPS achieved
    pub average_tps: f64,
    /// Peak TPS achieved
    pub peak_tps: f64,
    /// Average latency in milliseconds
    pub average_latency_ms: f64,
    /// 95th percentile latency
    pub p95_latency_ms: f64,
    /// 99th percentile latency
    pub p99_latency_ms: f64,
    /// Total duration in seconds
    pub duration_seconds: f64,
    /// Memory usage in MB
    pub memory_usage_mb: f64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Error breakdown
    pub errors: std::collections::HashMap<String, u64>,
}

/// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Current TPS
    pub current_tps: f64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Memory usage
    pub memory_mb: f64,
    /// CPU usage
    pub cpu_percent: f64,
    /// Active connections
    pub active_connections: u64,
    /// Queue depth
    pub queue_depth: u64,
    /// Timestamp
    pub timestamp: u64,
}

impl BenchmarkResult {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_transactions == 0 {
            0.0
        } else {
            (self.successful_transactions as f64 / self.total_transactions as f64) * 100.0
        }
    }

    /// Generate summary string
    pub fn summary(&self) -> String {
        format!(
            "Benchmark Results:\n\
             Total TX: {}\n\
             Successful: {} ({:.2}%)\n\
             Average TPS: {:.2}\n\
             Peak TPS: {:.2}\n\
             Avg Latency: {:.2}ms\n\
             P95 Latency: {:.2}ms\n\
             P99 Latency: {:.2}ms\n\
             Memory Usage: {:.2}MB\n\
             CPU Usage: {:.2}%\n\
             Duration: {:.2}s",
            self.total_transactions,
            self.successful_transactions,
            self.success_rate(),
            self.average_tps,
            self.peak_tps,
            self.average_latency_ms,
            self.p95_latency_ms,
            self.p99_latency_ms,
            self.memory_usage_mb,
            self.cpu_usage_percent,
            self.duration_seconds
        )
    }
}
