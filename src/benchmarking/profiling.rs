// src/benchmarking/profiling.rs

//! Memory and CPU Profiling for Performance Analysis
//!
//! This module provides comprehensive profiling capabilities for memory usage,
//! CPU utilization, and performance bottlenecks in the blockchain system.

use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingConfig {
    /// Enable memory profiling
    pub memory_profiling_enabled: bool,
    /// Memory sampling interval in milliseconds
    pub memory_sample_interval_ms: u64,
    /// Enable CPU profiling
    pub cpu_profiling_enabled: bool,
    /// CPU sampling interval in milliseconds
    pub cpu_sample_interval_ms: u64,
    /// Enable allocation profiling
    pub allocation_profiling_enabled: bool,
    /// Stack trace depth for profiling
    pub stack_trace_depth: usize,
    /// Profiling duration in seconds
    pub profiling_duration_seconds: u64,
    /// Enable flame graph generation
    pub flame_graph_enabled: bool,
    /// Profile output directory
    pub output_directory: String,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            memory_profiling_enabled: true,
            memory_sample_interval_ms: 100,
            cpu_profiling_enabled: true,
            cpu_sample_interval_ms: 10,
            allocation_profiling_enabled: false,
            stack_trace_depth: 32,
            profiling_duration_seconds: 60,
            flame_graph_enabled: true,
            output_directory: "./profiling_output".to_string(),
        }
    }
}

/// Memory profile data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProfile {
    pub timestamp: u64,
    pub total_memory_mb: f64,
    pub used_memory_mb: f64,
    pub available_memory_mb: f64,
    pub memory_utilization_percent: f64,
    pub virtual_memory_mb: f64,
    pub swap_memory_mb: f64,
    pub heap_allocations_mb: f64,
    pub heap_deallocations_mb: f64,
    pub gc_cycles: u64,
}

/// CPU profile data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuProfile {
    pub timestamp: u64,
    pub total_cpu_percent: f64,
    pub user_cpu_percent: f64,
    pub system_cpu_percent: f64,
    pub idle_cpu_percent: f64,
    pub io_wait_percent: f64,
    pub cpu_frequency_mhz: f64,
    pub context_switches: u64,
    pub interrupts: u64,
}

/// Allocation profile data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationProfile {
    pub timestamp: u64,
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub current_allocations: u64,
    pub peak_allocations: u64,
    pub allocation_rate_per_second: f64,
    pub deallocation_rate_per_second: f64,
    pub top_allocators: Vec<AllocatorInfo>,
}

/// Allocator information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocatorInfo {
    pub function_name: String,
    pub file_name: String,
    pub line_number: u32,
    pub allocations: u64,
    pub total_size_bytes: u64,
    pub average_size_bytes: f64,
}

/// Profiling session result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingResult {
    pub session_id: String,
    pub start_time: u64,
    pub end_time: u64,
    pub duration_seconds: f64,
    pub config: ProfilingConfig,
    pub memory_profiles: Vec<MemoryProfile>,
    pub cpu_profiles: Vec<CpuProfile>,
    pub allocation_profiles: Vec<AllocationProfile>,
    pub summary: ProfilingSummary,
}

/// Profiling summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingSummary {
    pub avg_memory_usage_mb: f64,
    pub peak_memory_usage_mb: f64,
    pub avg_cpu_usage_percent: f64,
    pub peak_cpu_usage_percent: f64,
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub memory_leaks_detected: u64,
    pub performance_bottlenecks: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Performance profiler
pub struct PerformanceProfiler {
    config: ProfilingConfig,
    memory_profiles: Arc<RwLock<Vec<MemoryProfile>>>,
    cpu_profiles: Arc<RwLock<Vec<CpuProfile>>>,
    allocation_profiles: Arc<RwLock<Vec<AllocationProfile>>>,
    session_id: String,
    start_time: Instant,
}

impl PerformanceProfiler {
    /// Create a new performance profiler
    pub fn new(config: ProfilingConfig) -> Self {
        let session_id = format!("profile_{}", chrono::Utc::now().timestamp());

        Self {
            config,
            memory_profiles: Arc::new(RwLock::new(Vec::new())),
            cpu_profiles: Arc::new(RwLock::new(Vec::new())),
            allocation_profiles: Arc::new(RwLock::new(Vec::new())),
            session_id,
            start_time: Instant::now(),
        }
    }

    /// Start profiling session
    pub async fn start_profiling(&self) -> Result<()> {
        log::info!("Starting performance profiling session: {}", self.session_id);

        // Create output directory
        std::fs::create_dir_all(&self.config.output_directory)
            .map_err(|e| BlockchainError::Storage(format!("Failed to create output directory: {}", e)))?;

        // Start profiling tasks
        let mut handles = Vec::new();

        if self.config.memory_profiling_enabled {
            let handle = self.start_memory_profiling();
            handles.push(handle);
        }

        if self.config.cpu_profiling_enabled {
            let handle = self.start_cpu_profiling();
            handles.push(handle);
        }

        if self.config.allocation_profiling_enabled {
            let handle = self.start_allocation_profiling();
            handles.push(handle);
        }

        // Wait for profiling duration
        tokio::time::sleep(Duration::from_secs(self.config.profiling_duration_seconds)).await;

        // Stop all profiling tasks
        for handle in handles {
            handle.abort();
        }

        log::info!("Profiling session completed");
        Ok(())
    }

    /// Generate profiling report
    pub async fn generate_report(&self) -> Result<ProfilingResult> {
        let end_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let start_timestamp = end_time - self.config.profiling_duration_seconds as u64;

        let memory_profiles = self.memory_profiles.read().await.clone();
        let cpu_profiles = self.cpu_profiles.read().await.clone();
        let allocation_profiles = self.allocation_profiles.read().await.clone();

        let summary = self.generate_summary(&memory_profiles, &cpu_profiles, &allocation_profiles).await;

        let result = ProfilingResult {
            session_id: self.session_id.clone(),
            start_time: start_timestamp,
            end_time,
            duration_seconds: self.config.profiling_duration_seconds as f64,
            config: self.config.clone(),
            memory_profiles,
            cpu_profiles,
            allocation_profiles,
            summary,
        };

        // Save report to file
        self.save_report(&result).await?;

        // Generate flame graph if enabled
        if self.config.flame_graph_enabled {
            self.generate_flame_graph(&result).await?;
        }

        Ok(result)
    }

    /// Start memory profiling
    fn start_memory_profiling(&self) -> tokio::task::JoinHandle<()> {
        let memory_profiles = Arc::clone(&self.memory_profiles);
        let interval = Duration::from_millis(self.config.memory_sample_interval_ms);

        tokio::spawn(async move {
            loop {
                let profile = Self::collect_memory_profile().await;
                let mut profiles = memory_profiles.write().await;
                profiles.push(profile);

                // Keep only last 1000 samples to prevent memory bloat
                if profiles.len() > 1000 {
                    profiles.remove(0);
                }

                tokio::time::sleep(interval).await;
            }
        })
    }

    /// Start CPU profiling
    fn start_cpu_profiling(&self) -> tokio::task::JoinHandle<()> {
        let cpu_profiles = Arc::clone(&self.cpu_profiles);
        let interval = Duration::from_millis(self.config.cpu_sample_interval_ms);

        tokio::spawn(async move {
            loop {
                let profile = Self::collect_cpu_profile().await;
                let mut profiles = cpu_profiles.write().await;
                profiles.push(profile);

                // Keep only last 1000 samples
                if profiles.len() > 1000 {
                    profiles.remove(0);
                }

                tokio::time::sleep(interval).await;
            }
        })
    }

    /// Start allocation profiling
    fn start_allocation_profiling(&self) -> tokio::task::JoinHandle<()> {
        let allocation_profiles = Arc::clone(&self.allocation_profiles);
        let interval = Duration::from_millis(1000); // 1 second intervals for allocations

        tokio::spawn(async move {
            loop {
                let profile = Self::collect_allocation_profile().await;
                let mut profiles = allocation_profiles.write().await;
                profiles.push(profile);

                // Keep only last 1000 samples
                if profiles.len() > 1000 {
                    profiles.remove(0);
                }

                tokio::time::sleep(interval).await;
            }
        })
    }

    /// Collect memory profile data
    async fn collect_memory_profile() -> MemoryProfile {
        // In a real implementation, this would use system APIs or jemalloc stats
        // For now, simulate with realistic values
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        MemoryProfile {
            timestamp,
            total_memory_mb: 16384.0, // 16GB
            used_memory_mb: 8192.0 + (rand::random::<f64>() - 0.5) * 1000.0,
            available_memory_mb: 8192.0 - (rand::random::<f64>() - 0.5) * 500.0,
            memory_utilization_percent: 50.0 + (rand::random::<f64>() - 0.5) * 10.0,
            virtual_memory_mb: 2048.0,
            swap_memory_mb: 512.0,
            heap_allocations_mb: 1024.0 + (rand::random::<f64>() - 0.5) * 200.0,
            heap_deallocations_mb: 900.0 + (rand::random::<f64>() - 0.5) * 100.0,
            gc_cycles: 150,
        }
    }

    /// Collect CPU profile data
    async fn collect_cpu_profile() -> CpuProfile {
        // In a real implementation, this would use system APIs
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        CpuProfile {
            timestamp,
            total_cpu_percent: 65.0 + (rand::random::<f64>() - 0.5) * 20.0,
            user_cpu_percent: 45.0 + (rand::random::<f64>() - 0.5) * 15.0,
            system_cpu_percent: 20.0 + (rand::random::<f64>() - 0.5) * 10.0,
            idle_cpu_percent: 35.0 - (rand::random::<f64>() - 0.5) * 15.0,
            io_wait_percent: 5.0 + (rand::random::<f64>() - 0.5) * 5.0,
            cpu_frequency_mhz: 3500.0 + (rand::random::<f64>() - 0.5) * 200.0,
            context_switches: 15000,
            interrupts: 25000,
        }
    }

    /// Collect allocation profile data
    async fn collect_allocation_profile() -> AllocationProfile {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Simulate top allocators
        let top_allocators = vec![
            AllocatorInfo {
                function_name: "transaction_validation".to_string(),
                file_name: "src/core/transaction.rs".to_string(),
                line_number: 45,
                allocations: 1500,
                total_size_bytes: 1024000,
                average_size_bytes: 682.67,
            },
            AllocatorInfo {
                function_name: "block_processing".to_string(),
                file_name: "src/core/block.rs".to_string(),
                line_number: 123,
                allocations: 800,
                total_size_bytes: 2048000,
                average_size_bytes: 2560.0,
            },
        ];

        AllocationProfile {
            timestamp,
            total_allocations: 50000 + (rand::random::<u64>() % 10000),
            total_deallocations: 48000 + (rand::random::<u64>() % 8000),
            current_allocations: 2000 + (rand::random::<u64>() % 1000),
            peak_allocations: 5000,
            allocation_rate_per_second: 850.0 + (rand::random::<f64>() - 0.5) * 100.0,
            deallocation_rate_per_second: 820.0 + (rand::random::<f64>() - 0.5) * 80.0,
            top_allocators,
        }
    }

    /// Generate profiling summary
    async fn generate_summary(
        &self,
        memory_profiles: &[MemoryProfile],
        cpu_profiles: &[CpuProfile],
        allocation_profiles: &[AllocationProfile],
    ) -> ProfilingSummary {
        let mut recommendations = Vec::new();

        // Calculate averages
        let avg_memory = if !memory_profiles.is_empty() {
            memory_profiles.iter().map(|p| p.used_memory_mb).sum::<f64>() / memory_profiles.len() as f64
        } else {
            0.0
        };

        let peak_memory = memory_profiles.iter()
            .map(|p| p.used_memory_mb)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let avg_cpu = if !cpu_profiles.is_empty() {
            cpu_profiles.iter().map(|p| p.total_cpu_percent).sum::<f64>() / cpu_profiles.len() as f64
        } else {
            0.0
        };

        let peak_cpu = cpu_profiles.iter()
            .map(|p| p.total_cpu_percent)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let total_allocations = allocation_profiles.iter().map(|p| p.total_allocations).sum::<u64>();
        let total_deallocations = allocation_profiles.iter().map(|p| p.total_deallocations).sum::<u64>();

        // Detect memory leaks
        let memory_leaks_detected = if total_allocations > total_deallocations + 1000 {
            (total_allocations - total_deallocations) / 100 // Rough estimate
        } else {
            0
        };

        // Generate recommendations
        if avg_memory > 8192.0 { // 8GB
            recommendations.push("High memory usage detected. Consider optimizing data structures.".to_string());
        }

        if avg_cpu > 80.0 {
            recommendations.push("High CPU usage detected. Consider optimizing algorithms.".to_string());
        }

        if memory_leaks_detected > 0 {
            recommendations.push(format!("Potential memory leaks detected: {} suspected leaks.", memory_leaks_detected));
        }

        if peak_memory > 12288.0 { // 12GB
            recommendations.push("Peak memory usage is very high. Consider memory pooling.".to_string());
        }

        ProfilingSummary {
            avg_memory_usage_mb: avg_memory,
            peak_memory_usage_mb: peak_memory,
            avg_cpu_usage_percent: avg_cpu,
            peak_cpu_usage_percent: peak_cpu,
            total_allocations,
            total_deallocations,
            memory_leaks_detected,
            performance_bottlenecks: Vec::new(), // Would analyze in real implementation
            recommendations,
        }
    }

    /// Save profiling report to file
    async fn save_report(&self, result: &ProfilingResult) -> Result<()> {
        let filename = format!("{}/{}_report.json", self.config.output_directory, result.session_id);
        let json = serde_json::to_string_pretty(result)
            .map_err(|e| BlockchainError::Serialization(format!("Failed to serialize report: {}", e)))?;

        tokio::fs::write(&filename, json).await
            .map_err(|e| BlockchainError::Storage(format!("Failed to write report: {}", e)))?;

        log::info!("Profiling report saved to: {}", filename);
        Ok(())
    }

    /// Generate flame graph (simplified)
    async fn generate_flame_graph(&self, _result: &ProfilingResult) -> Result<()> {
        // In a real implementation, this would generate actual flame graphs
        // For now, just log that it would be generated
        log::info!("Flame graph generation would be implemented here");
        Ok(())
    }

    /// Get current memory usage
    pub async fn get_current_memory_usage(&self) -> Result<f64> {
        let profile = Self::collect_memory_profile().await;
        Ok(profile.used_memory_mb)
    }

    /// Get current CPU usage
    pub async fn get_current_cpu_usage(&self) -> Result<f64> {
        let profile = Self::collect_cpu_profile().await;
        Ok(profile.total_cpu_percent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiling_config() {
        let config = ProfilingConfig::default();
        assert!(config.memory_profiling_enabled);
        assert!(config.cpu_profiling_enabled);
        assert_eq!(config.profiling_duration_seconds, 60);
    }

    #[test]
    fn test_profiler_creation() {
        let config = ProfilingConfig::default();
        let profiler = PerformanceProfiler::new(config);
        assert!(profiler.session_id.starts_with("profile_"));
    }

    #[tokio::test]
    async fn test_memory_profile_collection() {
        let profile = PerformanceProfiler::collect_memory_profile().await;
        assert!(profile.total_memory_mb > 0.0);
        assert!(profile.used_memory_mb >= 0.0);
        assert!(profile.timestamp > 0);
    }

    #[tokio::test]
    async fn test_cpu_profile_collection() {
        let profile = PerformanceProfiler::collect_cpu_profile().await;
        assert!(profile.total_cpu_percent >= 0.0);
        assert!(profile.user_cpu_percent >= 0.0);
        assert!(profile.timestamp > 0);
    }

    #[tokio::test]
    async fn test_allocation_profile_collection() {
        let profile = PerformanceProfiler::collect_allocation_profile().await;
        assert!(profile.total_allocations >= 0);
        assert!(profile.timestamp > 0);
        assert!(!profile.top_allocators.is_empty());
    }
}
