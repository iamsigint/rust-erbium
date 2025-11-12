// src/benchmarking/metrics_collector.rs

//! Performance Metrics Collector
//!
//! This module collects system performance metrics during benchmarking,
//! including CPU usage, memory usage, network I/O, and blockchain-specific metrics.

use crate::benchmarking::PerformanceMetrics;
use crate::utils::error::{Result, BlockchainError};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{System, SystemExt, ProcessExt, CpuExt};

/// Metrics collector for performance benchmarking
pub struct MetricsCollector {
    system: System,
    snapshots: Vec<PerformanceMetrics>,
    start_time: SystemTime,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            system,
            snapshots: Vec::new(),
            start_time: SystemTime::now(),
        }
    }

    /// Collect a performance metrics snapshot
    pub async fn collect_snapshot(&mut self) -> Result<()> {
        self.system.refresh_all();

        let memory_mb = self.system.used_memory() as f64 / 1024.0 / 1024.0;
        let cpu_percent = self.system.global_cpu_info().cpu_usage() as f64;

        // In a real blockchain system, these would come from actual metrics
        let current_tps = self.estimate_current_tps().await;
        let avg_latency_ms = self.estimate_avg_latency().await;

        // Simulate network and queue metrics
        let active_connections = 42; // Mock value
        let queue_depth = 15; // Mock value

        let metrics = PerformanceMetrics {
            current_tps,
            avg_latency_ms,
            memory_mb,
            cpu_percent,
            active_connections,
            queue_depth,
            timestamp: self.current_timestamp(),
        };

        self.snapshots.push(metrics);

        log::debug!("Collected metrics snapshot: TPS={:.1}, Latency={:.1}ms, Memory={:.1}MB, CPU={:.1}%",
                   current_tps, avg_latency_ms, memory_mb, cpu_percent);

        Ok(())
    }

    /// Get all collected snapshots
    pub fn get_snapshots(&self) -> &[PerformanceMetrics] {
        &self.snapshots
    }

    /// Get final aggregated metrics
    pub async fn get_final_metrics(&self) -> Result<PerformanceMetrics> {
        if self.snapshots.is_empty() {
            return Ok(PerformanceMetrics {
                current_tps: 0.0,
                avg_latency_ms: 0.0,
                memory_mb: 0.0,
                cpu_percent: 0.0,
                active_connections: 0,
                queue_depth: 0,
                timestamp: self.current_timestamp(),
            });
        }

        // Calculate averages
        let len = self.snapshots.len() as f64;
        let avg_tps = self.snapshots.iter().map(|m| m.current_tps).sum::<f64>() / len;
        let avg_latency = self.snapshots.iter().map(|m| m.avg_latency_ms).sum::<f64>() / len;
        let avg_memory = self.snapshots.iter().map(|m| m.memory_mb).sum::<f64>() / len;
        let avg_cpu = self.snapshots.iter().map(|m| m.cpu_percent).sum::<f64>() / len;
        let avg_connections = (self.snapshots.iter().map(|m| m.active_connections).sum::<u64>() as f64 / len) as u64;
        let avg_queue = (self.snapshots.iter().map(|m| m.queue_depth).sum::<u64>() as f64 / len) as u64;

        Ok(PerformanceMetrics {
            current_tps: avg_tps,
            avg_latency_ms: avg_latency,
            memory_mb: avg_memory,
            cpu_percent: avg_cpu,
            active_connections: avg_connections,
            queue_depth: avg_queue,
            timestamp: self.current_timestamp(),
        })
    }

    /// Get memory usage statistics
    pub fn get_memory_stats(&self) -> MemoryStats {
        if self.snapshots.is_empty() {
            return MemoryStats::default();
        }

        let memory_values: Vec<f64> = self.snapshots.iter().map(|m| m.memory_mb).collect();

        MemoryStats {
            min_mb: memory_values.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            max_mb: memory_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
            avg_mb: memory_values.iter().sum::<f64>() / memory_values.len() as f64,
            current_mb: *memory_values.last().unwrap_or(&0.0),
        }
    }

    /// Get CPU usage statistics
    pub fn get_cpu_stats(&self) -> CpuStats {
        if self.snapshots.is_empty() {
            return CpuStats::default();
        }

        let cpu_values: Vec<f64> = self.snapshots.iter().map(|m| m.cpu_percent).collect();

        CpuStats {
            min_percent: cpu_values.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            max_percent: cpu_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
            avg_percent: cpu_values.iter().sum::<f64>() / cpu_values.len() as f64,
            current_percent: *cpu_values.last().unwrap_or(&0.0),
        }
    }

    /// Get TPS statistics
    pub fn get_tps_stats(&self) -> TpsStats {
        if self.snapshots.is_empty() {
            return TpsStats::default();
        }

        let tps_values: Vec<f64> = self.snapshots.iter().map(|m| m.current_tps).collect();

        TpsStats {
            min_tps: tps_values.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            max_tps: tps_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
            avg_tps: tps_values.iter().sum::<f64>() / tps_values.len() as f64,
            current_tps: *tps_values.last().unwrap_or(&0.0),
        }
    }

    /// Get latency statistics
    pub fn get_latency_stats(&self) -> LatencyStats {
        if self.snapshots.is_empty() {
            return LatencyStats::default();
        }

        let latency_values: Vec<f64> = self.snapshots.iter().map(|m| m.avg_latency_ms).collect();

        LatencyStats {
            min_ms: latency_values.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
            max_ms: latency_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
            avg_ms: latency_values.iter().sum::<f64>() / latency_values.len() as f64,
            current_ms: *latency_values.last().unwrap_or(&0.0),
        }
    }

    /// Export metrics to JSON
    pub fn export_to_json(&self) -> Result<String> {
        let export_data = serde_json::json!({
            "metadata": {
                "start_time": self.start_time.duration_since(UNIX_EPOCH).unwrap().as_secs(),
                "total_snapshots": self.snapshots.len(),
                "collection_duration_seconds": self.snapshots.last()
                    .map(|m| m.timestamp.saturating_sub(self.current_timestamp()))
                    .unwrap_or(0),
            },
            "snapshots": self.snapshots,
            "summary": {
                "memory": self.get_memory_stats(),
                "cpu": self.get_cpu_stats(),
                "tps": self.get_tps_stats(),
                "latency": self.get_latency_stats(),
            }
        });

        serde_json::to_string_pretty(&export_data)
            .map_err(|e| BlockchainError::Serialization(format!("Failed to serialize metrics: {}", e)))
    }

    /// Estimate current TPS (mock implementation)
    async fn estimate_current_tps(&self) -> f64 {
        // In a real implementation, this would query the blockchain's transaction pool
        // and recent block data to calculate actual TPS
        1250.0 + (rand::random::<f64>() - 0.5) * 100.0 // Mock value with some variance
    }

    /// Estimate average latency (mock implementation)
    async fn estimate_avg_latency(&self) -> f64 {
        // In a real implementation, this would measure actual transaction confirmation times
        45.0 + (rand::random::<f64>() - 0.5) * 10.0 // Mock value with some variance
    }

    /// Get current timestamp
    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// Memory usage statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryStats {
    pub min_mb: f64,
    pub max_mb: f64,
    pub avg_mb: f64,
    pub current_mb: f64,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            min_mb: 0.0,
            max_mb: 0.0,
            avg_mb: 0.0,
            current_mb: 0.0,
        }
    }
}

/// CPU usage statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CpuStats {
    pub min_percent: f64,
    pub max_percent: f64,
    pub avg_percent: f64,
    pub current_percent: f64,
}

impl Default for CpuStats {
    fn default() -> Self {
        Self {
            min_percent: 0.0,
            max_percent: 0.0,
            avg_percent: 0.0,
            current_percent: 0.0,
        }
    }
}

/// TPS statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TpsStats {
    pub min_tps: f64,
    pub max_tps: f64,
    pub avg_tps: f64,
    pub current_tps: f64,
}

impl Default for TpsStats {
    fn default() -> Self {
        Self {
            min_tps: 0.0,
            max_tps: 0.0,
            avg_tps: 0.0,
            current_tps: 0.0,
        }
    }
}

/// Latency statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LatencyStats {
    pub min_ms: f64,
    pub max_ms: f64,
    pub avg_ms: f64,
    pub current_ms: f64,
}

impl Default for LatencyStats {
    fn default() -> Self {
        Self {
            min_ms: 0.0,
            max_ms: 0.0,
            avg_ms: 0.0,
            current_ms: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert!(collector.snapshots.is_empty());
    }

    #[tokio::test]
    async fn test_collect_snapshot() {
        let mut collector = MetricsCollector::new();

        collector.collect_snapshot().await.unwrap();
        assert_eq!(collector.snapshots.len(), 1);

        let metrics = &collector.snapshots[0];
        assert!(metrics.memory_mb >= 0.0);
        assert!(metrics.cpu_percent >= 0.0);
        assert!(metrics.timestamp > 0);
    }

    #[tokio::test]
    async fn test_memory_stats() {
        let mut collector = MetricsCollector::new();

        // Collect a few snapshots
        for _ in 0..3 {
            collector.collect_snapshot().await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let stats = collector.get_memory_stats();
        assert!(stats.avg_mb >= 0.0);
        assert!(stats.min_mb <= stats.max_mb);
    }

    #[tokio::test]
    async fn test_json_export() {
        let mut collector = MetricsCollector::new();

        collector.collect_snapshot().await.unwrap();

        let json = collector.export_to_json().unwrap();
        assert!(json.contains("metadata"));
        assert!(json.contains("snapshots"));
        assert!(json.contains("summary"));
    }
}
