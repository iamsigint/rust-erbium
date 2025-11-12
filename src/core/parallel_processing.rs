// src/core/parallel_processing.rs

//! Parallel Processing for Blockchain Transactions
//!
//! This module implements parallel processing of transactions and blocks
//! to improve blockchain throughput and performance.

use crate::utils::error::{Result, BlockchainError};
use crate::core::transaction::Transaction;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::task;
use std::collections::HashMap;

/// Parallel processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    /// Maximum number of parallel workers
    pub max_workers: usize,
    /// Batch size for parallel processing
    pub batch_size: usize,
    /// Enable CPU affinity for workers
    pub enable_cpu_affinity: bool,
    /// Worker queue size
    pub queue_size: usize,
    /// Enable adaptive batching
    pub adaptive_batching: bool,
    /// Minimum batch size
    pub min_batch_size: usize,
    /// Maximum batch size
    pub max_batch_size: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            max_workers: num_cpus::get(),
            batch_size: 100,
            enable_cpu_affinity: false,
            queue_size: 1000,
            adaptive_batching: true,
            min_batch_size: 10,
            max_batch_size: 1000,
        }
    }
}

/// Parallel processing statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub total_transactions_processed: u64,
    pub total_blocks_processed: u64,
    pub average_processing_time_ms: f64,
    pub peak_tps: f64,
    pub active_workers: usize,
    pub queue_depth: usize,
    pub failed_transactions: u64,
}

/// Transaction processor with parallel execution
pub struct ParallelProcessor {
    config: ParallelConfig,
    stats: Arc<RwLock<ProcessingStats>>,
    semaphore: Arc<Semaphore>,
    workers: Vec<task::JoinHandle<()>>,
}

impl ParallelProcessor {
    /// Create a new parallel processor
    pub fn new(config: ParallelConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_workers));

        Self {
            config,
            stats: Arc::new(RwLock::new(ProcessingStats {
                total_transactions_processed: 0,
                total_blocks_processed: 0,
                average_processing_time_ms: 0.0,
                peak_tps: 0.0,
                active_workers: 0,
                queue_depth: 0,
                failed_transactions: 0,
            })),
            semaphore,
            workers: Vec::new(),
        }
    }

    /// Process a batch of transactions in parallel
    pub async fn process_transaction_batch(
        &self,
        transactions: Vec<Transaction>,
    ) -> Result<BatchProcessingResult> {
        if transactions.is_empty() {
            return Ok(BatchProcessingResult::default());
        }

        let start_time = std::time::Instant::now();
        let batch_size = transactions.len();

        log::debug!("Processing batch of {} transactions", batch_size);

        // Split transactions into smaller chunks for parallel processing
        let chunks = self.split_into_chunks(transactions, self.config.batch_size);

        // Process chunks in parallel
        let mut handles = Vec::new();
        let semaphore = Arc::clone(&self.semaphore);

        for chunk in chunks {
            let permit = semaphore.acquire().await
                .map_err(|e| BlockchainError::Processing(format!("Failed to acquire semaphore: {}", e)))?;

            let stats = Arc::clone(&self.stats);

            let handle = task::spawn(async move {
                let result = Self::process_chunk(chunk).await;
                drop(permit); // Release semaphore

                // Update stats
                let mut stats_lock = stats.write().await;
                stats_lock.total_transactions_processed += result.processed as u64;
                stats_lock.failed_transactions += result.failed as u64;

                result
            });

            handles.push(handle);
        }

        // Collect results
        let mut total_processed = 0;
        let mut total_failed = 0;
        let mut valid_transactions = Vec::new();

        for handle in handles {
            match handle.await {
                Ok(chunk_result) => {
                    total_processed += chunk_result.processed;
                    total_failed += chunk_result.failed;
                    valid_transactions.extend(chunk_result.valid_transactions);
                }
                Err(e) => {
                    log::error!("Task join error: {}", e);
                    total_failed += 1;
                }
            }
        }

        let processing_time = start_time.elapsed();

        // Update overall stats
        let mut stats = self.stats.write().await;
        stats.average_processing_time_ms = (stats.average_processing_time_ms + processing_time.as_millis() as f64) / 2.0;

        let tps = total_processed as f64 / processing_time.as_secs_f64();
        if tps > stats.peak_tps {
            stats.peak_tps = tps;
        }

        log::debug!(
            "Batch processed: {} tx in {:.2}ms ({:.1} TPS)",
            total_processed,
            processing_time.as_millis(),
            tps
        );

        Ok(BatchProcessingResult {
            processed: total_processed,
            failed: total_failed,
            valid_transactions,
            processing_time_ms: processing_time.as_millis() as f64,
            tps,
        })
    }

    /// Process a block with parallel validation
    pub async fn process_block_parallel(
        &self,
        transactions: Vec<Transaction>,
    ) -> Result<BlockProcessingResult> {
        let start_time = std::time::Instant::now();

        // First pass: Basic validation in parallel
        let basic_result = self.process_transaction_batch(transactions.clone()).await?;

        // Second pass: State-dependent validation (sequential for now)
        let state_validation_result = self.validate_state_dependencies(basic_result.valid_transactions).await?;

        // Third pass: Execute transactions in parallel where possible
        let execution_result = self.execute_transactions_parallel(state_validation_result.valid_transactions).await?;

        let total_time = start_time.elapsed();

        // Update block stats
        let mut stats = self.stats.write().await;
        stats.total_blocks_processed += 1;

        Ok(BlockProcessingResult {
            total_transactions: transactions.len(),
            valid_transactions: execution_result.processed,
            failed_transactions: execution_result.failed,
            gas_used: execution_result.gas_used,
            processing_time_ms: total_time.as_millis() as f64,
            tps: execution_result.processed as f64 / total_time.as_secs_f64(),
        })
    }

    /// Split transactions into chunks for parallel processing
    fn split_into_chunks(&self, transactions: Vec<Transaction>, chunk_size: usize) -> Vec<Vec<Transaction>> {
        transactions
            .chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Process a chunk of transactions
    async fn process_chunk(chunk: Vec<Transaction>) -> ChunkResult {
        let mut processed = 0;
        let mut failed = 0;
        let mut valid_transactions = Vec::new();

        for tx in chunk {
            // Perform basic validation
            match tx.validate_basic() {
                Ok(_) => {
                    processed += 1;
                    valid_transactions.push(tx);
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }

        ChunkResult {
            processed,
            failed,
            valid_transactions,
        }
    }

    /// Validate state dependencies (simplified - would need actual state)
    async fn validate_state_dependencies(&self, transactions: Vec<Transaction>) -> Result<ValidationResult> {
        // In a real implementation, this would check:
        // - Nonce ordering
        // - Account balances
        // - State conflicts

        // For now, assume all are valid
        Ok(ValidationResult {
            valid_transactions: transactions,
            invalid_transactions: Vec::new(),
        })
    }

    /// Execute transactions in parallel where possible
    async fn execute_transactions_parallel(&self, transactions: Vec<Transaction>) -> Result<ExecutionResult> {
        if transactions.is_empty() {
            return Ok(ExecutionResult::default());
        }

        // Group transactions by type for parallel execution
        let mut transfer_txs = Vec::new();
        let mut contract_txs = Vec::new();
        let mut other_txs = Vec::new();

        for tx in transactions {
            match tx.transaction_type() {
                "transfer" => transfer_txs.push(tx),
                "contract_call" => contract_txs.push(tx),
                _ => other_txs.push(tx),
            }
        }

        // Process different types in parallel
        let (transfer_result, contract_result, other_result) = tokio::join!(
            self.process_transfer_batch(transfer_txs),
            self.process_contract_batch(contract_txs),
            self.process_other_batch(other_txs)
        );

        let total_processed = transfer_result?.processed + contract_result?.processed + other_result?.processed;
        let total_failed = transfer_result?.failed + contract_result?.failed + other_result?.failed;

        Ok(ExecutionResult {
            processed: total_processed,
            failed: total_failed,
            gas_used: 0, // Would calculate actual gas usage
        })
    }

    /// Process transfer transactions (can be highly parallel)
    async fn process_transfer_batch(&self, transactions: Vec<Transaction>) -> Result<ExecutionResult> {
        if transactions.is_empty() {
            return Ok(ExecutionResult::default());
        }

        let semaphore = Arc::clone(&self.semaphore);
        let mut handles = Vec::new();

        for tx in transactions {
            let permit = semaphore.acquire().await
                .map_err(|e| BlockchainError::Processing(format!("Failed to acquire semaphore: {}", e)))?;

            let handle = task::spawn(async move {
                // Simulate transfer execution
                tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
                drop(permit);
                Ok(())
            });

            handles.push(handle);
        }

        let mut processed = 0;
        let mut failed = 0;

        for handle in handles {
            match handle.await {
                Ok(Ok(_)) => processed += 1,
                _ => failed += 1,
            }
        }

        Ok(ExecutionResult {
            processed,
            failed,
            gas_used: processed * 21000, // Standard transfer gas
        })
    }

    /// Process contract transactions (more complex, some parallelism possible)
    async fn process_contract_batch(&self, transactions: Vec<Transaction>) -> Result<ExecutionResult> {
        // Contract calls may have dependencies, so process with limited parallelism
        let mut processed = 0;
        let mut failed = 0;

        for tx in transactions {
            // Simulate contract execution (more expensive)
            tokio::time::sleep(tokio::time::Duration::from_micros(500)).await;
            processed += 1;
        }

        Ok(ExecutionResult {
            processed,
            failed,
            gas_used: processed * 50000, // Higher gas for contracts
        })
    }

    /// Process other transaction types
    async fn process_other_batch(&self, transactions: Vec<Transaction>) -> Result<ExecutionResult> {
        let mut processed = 0;
        let mut failed = 0;

        for tx in transactions {
            // Simulate other execution
            tokio::time::sleep(tokio::time::Duration::from_micros(200)).await;
            processed += 1;
        }

        Ok(ExecutionResult {
            processed,
            failed,
            gas_used: processed * 30000,
        })
    }

    /// Get current processing statistics
    pub async fn get_stats(&self) -> ProcessingStats {
        self.stats.read().await.clone()
    }

    /// Adjust batch size based on performance (adaptive batching)
    pub async fn adjust_batch_size(&mut self, recent_tps: f64) {
        if !self.config.adaptive_batching {
            return;
        }

        let target_tps = 1000.0; // Configurable target
        let efficiency = recent_tps / target_tps;

        if efficiency > 1.2 {
            // Performing well, can increase batch size
            self.config.batch_size = (self.config.batch_size as f64 * 1.1).min(self.config.max_batch_size as f64) as usize;
        } else if efficiency < 0.8 {
            // Underperforming, reduce batch size
            self.config.batch_size = (self.config.batch_size as f64 * 0.9).max(self.config.min_batch_size as f64) as usize;
        }

        log::debug!("Adjusted batch size to {} based on TPS {:.1}", self.config.batch_size, recent_tps);
    }

    /// Get optimal worker count based on system resources
    pub fn get_optimal_worker_count(&self) -> usize {
        let cpu_count = num_cpus::get();

        // Reserve 2 cores for system operations
        let available_cores = cpu_count.saturating_sub(2);

        // Use 75% of available cores for transaction processing
        (available_cores as f64 * 0.75).max(1.0) as usize
    }
}

/// Result of processing a transaction batch
#[derive(Debug, Clone)]
pub struct BatchProcessingResult {
    pub processed: usize,
    pub failed: usize,
    pub valid_transactions: Vec<Transaction>,
    pub processing_time_ms: f64,
    pub tps: f64,
}

impl Default for BatchProcessingResult {
    fn default() -> Self {
        Self {
            processed: 0,
            failed: 0,
            valid_transactions: Vec::new(),
            processing_time_ms: 0.0,
            tps: 0.0,
        }
    }
}

/// Result of processing a block
#[derive(Debug, Clone)]
pub struct BlockProcessingResult {
    pub total_transactions: usize,
    pub valid_transactions: usize,
    pub failed_transactions: usize,
    pub gas_used: u64,
    pub processing_time_ms: f64,
    pub tps: f64,
}

/// Result of processing a chunk
struct ChunkResult {
    processed: usize,
    failed: usize,
    valid_transactions: Vec<Transaction>,
}

/// Result of validation
struct ValidationResult {
    valid_transactions: Vec<Transaction>,
    invalid_transactions: Vec<Transaction>,
}

/// Result of execution
#[derive(Debug, Clone)]
struct ExecutionResult {
    processed: usize,
    failed: usize,
    gas_used: u64,
}

impl Default for ExecutionResult {
    fn default() -> Self {
        Self {
            processed: 0,
            failed: 0,
            gas_used: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Address;

    #[test]
    fn test_parallel_config() {
        let config = ParallelConfig::default();
        assert_eq!(config.max_workers, num_cpus::get());
        assert_eq!(config.batch_size, 100);
    }

    #[test]
    fn test_parallel_processor_creation() {
        let config = ParallelConfig::default();
        let processor = ParallelProcessor::new(config);
        assert_eq!(processor.config.max_workers, num_cpus::get());
    }

    #[test]
    fn test_optimal_worker_count() {
        let config = ParallelConfig::default();
        let processor = ParallelProcessor::new(config);

        let optimal = processor.get_optimal_worker_count();
        assert!(optimal >= 1);
        assert!(optimal <= num_cpus::get());
    }

    #[tokio::test]
    async fn test_empty_batch_processing() {
        let config = ParallelConfig::default();
        let processor = ParallelProcessor::new(config);

        let result = processor.process_transaction_batch(vec![]).await.unwrap();
        assert_eq!(result.processed, 0);
        assert_eq!(result.failed, 0);
    }

    #[tokio::test]
    async fn test_batch_processing() {
        let config = ParallelConfig {
            max_workers: 2,
            batch_size: 10,
            ..Default::default()
        };
        let processor = ParallelProcessor::new(config);

        // Create some test transactions
        let mut transactions = Vec::new();
        for i in 0..5 {
            let from = Address::new(format!("0x{:040x}", i)).unwrap();
            let to = Address::new(format!("0x{:040x}", i + 1000)).unwrap();
            let tx = Transaction::new_transfer(from, to, 1000, 10, i as u64);
            transactions.push(tx);
        }

        let result = processor.process_transaction_batch(transactions).await.unwrap();
        assert_eq!(result.processed, 5);
        assert_eq!(result.failed, 0);
        assert_eq!(result.valid_transactions.len(), 5);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let config = ParallelConfig::default();
        let processor = ParallelProcessor::new(config);

        let stats = processor.get_stats().await;
        assert_eq!(stats.total_transactions_processed, 0);
        assert_eq!(stats.total_blocks_processed, 0);
    }
}
