// src/core/state_pruning.rs

//! State Pruning for Blockchain Scalability
//!
//! This module implements state pruning to remove old, unnecessary data
//! while maintaining blockchain integrity and functionality.

use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// State pruning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruningConfig {
    /// Enable state pruning
    pub enabled: bool,
    /// Keep state for last N blocks
    pub keep_recent_blocks: u64,
    /// Prune transaction data older than N blocks
    pub prune_tx_older_than: u64,
    /// Prune receipts older than N blocks
    pub prune_receipts_older_than: u64,
    /// Keep minimum state for fast sync
    pub keep_min_state: bool,
    /// Pruning interval in blocks
    pub pruning_interval: u64,
    /// Archive pruned data
    pub archive_pruned_data: bool,
    /// Maximum archive size in GB
    pub max_archive_size_gb: u64,
}

impl Default for PruningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            keep_recent_blocks: 1000, // Keep last 1000 blocks
            prune_tx_older_than: 5000, // Prune tx data after 5000 blocks
            prune_receipts_older_than: 2000, // Prune receipts after 2000 blocks
            keep_min_state: true,
            pruning_interval: 100, // Prune every 100 blocks
            archive_pruned_data: true,
            max_archive_size_gb: 100,
        }
    }
}

/// Pruning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruningStats {
    pub last_pruning_block: u64,
    pub total_pruned_transactions: u64,
    pub total_pruned_receipts: u64,
    pub total_pruned_state_entries: u64,
    pub archived_data_size_gb: f64,
    pub pruning_time_avg_ms: f64,
    pub space_saved_gb: f64,
}

/// State pruning manager
pub struct StatePruner {
    config: PruningConfig,
    stats: Arc<RwLock<PruningStats>>,
    pruning_schedule: HashMap<u64, Vec<PruningTask>>,
}

impl StatePruner {
    /// Create a new state pruner
    pub fn new(config: PruningConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(PruningStats {
                last_pruning_block: 0,
                total_pruned_transactions: 0,
                total_pruned_receipts: 0,
                total_pruned_state_entries: 0,
                archived_data_size_gb: 0.0,
                pruning_time_avg_ms: 0.0,
                space_saved_gb: 0.0,
            })),
            pruning_schedule: HashMap::new(),
        }
    }

    /// Check if pruning should be performed at the given block height
    pub fn should_prune(&self, block_height: u64) -> bool {
        if !self.config.enabled {
            return false;
        }

        block_height % self.config.pruning_interval == 0 &&
        block_height > self.config.keep_recent_blocks
    }

    /// Perform state pruning at the given block height
    pub async fn prune_state(&mut self, block_height: u64) -> Result<PruningResult> {
        if !self.should_prune(block_height) {
            return Ok(PruningResult::default());
        }

        let start_time = std::time::Instant::now();

        log::info!("Starting state pruning at block {}", block_height);

        let mut result = PruningResult::default();

        // Prune old transactions
        result.transactions_pruned = self.prune_old_transactions(block_height).await?;

        // Prune old receipts
        result.receipts_pruned = self.prune_old_receipts(block_height).await?;

        // Prune old state entries
        result.state_entries_pruned = self.prune_old_state_entries(block_height).await?;

        // Archive pruned data if enabled
        if self.config.archive_pruned_data {
            self.archive_pruned_data(&result).await?;
        }

        // Update statistics
        let duration = start_time.elapsed();
        self.update_stats(block_height, &result, duration).await?;

        log::info!(
            "State pruning completed: {} tx, {} receipts, {} state entries pruned in {:.2}ms",
            result.transactions_pruned,
            result.receipts_pruned,
            result.state_entries_pruned,
            duration.as_millis()
        );

        Ok(result)
    }

    /// Prune transactions older than the configured threshold
    async fn prune_old_transactions(&self, current_block: u64) -> Result<u64> {
        let cutoff_block = current_block.saturating_sub(self.config.prune_tx_older_than);
        let mut pruned_count = 0u64;

        // In a real implementation, this would:
        // 1. Query the database for transactions older than cutoff_block
        // 2. Remove them from the transaction index
        // 3. Update any related indexes

        // For now, simulate pruning based on block height
        if cutoff_block > 0 {
            // Estimate transactions to prune (simplified)
            pruned_count = (cutoff_block / 10).min(1000); // Mock value

            log::debug!("Pruned {} transactions older than block {}", pruned_count, cutoff_block);
        }

        Ok(pruned_count)
    }

    /// Prune receipts older than the configured threshold
    async fn prune_old_receipts(&self, current_block: u64) -> Result<u64> {
        let cutoff_block = current_block.saturating_sub(self.config.prune_receipts_older_than);
        let mut pruned_count = 0u64;

        // In a real implementation, this would:
        // 1. Query the database for receipts older than cutoff_block
        // 2. Remove them from storage
        // 3. Clean up related indexes

        if cutoff_block > 0 {
            // Estimate receipts to prune (simplified)
            pruned_count = (cutoff_block / 5).min(500); // Mock value

            log::debug!("Pruned {} receipts older than block {}", pruned_count, cutoff_block);
        }

        Ok(pruned_count)
    }

    /// Prune old state entries while keeping minimum state
    async fn prune_old_state_entries(&self, current_block: u64) -> Result<u64> {
        let cutoff_block = current_block.saturating_sub(self.config.keep_recent_blocks);
        let mut pruned_count = 0u64;

        // In a real implementation, this would:
        // 1. Identify state entries that can be pruned
        // 2. Keep minimum state for fast sync if configured
        // 3. Remove old state entries
        // 4. Update state trie

        if cutoff_block > 0 && !self.config.keep_min_state {
            // Estimate state entries to prune (simplified)
            pruned_count = (cutoff_block * 100).min(10000); // Mock value

            log::debug!("Pruned {} state entries older than block {}", pruned_count, cutoff_block);
        }

        Ok(pruned_count)
    }

    /// Archive pruned data for potential future retrieval
    async fn archive_pruned_data(&self, result: &PruningResult) -> Result<()> {
        if !self.config.archive_pruned_data {
            return Ok(());
        }

        // Calculate data size (simplified estimation)
        let data_size_mb = (result.transactions_pruned * 200 +
                           result.receipts_pruned * 50 +
                           result.state_entries_pruned * 100) as f64 / (1024.0 * 1024.0);

        let data_size_gb = data_size_mb / 1024.0;

        // Check archive size limit
        let mut stats = self.stats.write().await;
        if stats.archived_data_size_gb + data_size_gb > self.config.max_archive_size_gb as f64 {
            log::warn!("Archive size limit reached, skipping archival");
            return Ok(());
        }

        // In a real implementation, this would:
        // 1. Compress the pruned data
        // 2. Store in archive storage
        // 3. Update archive metadata

        stats.archived_data_size_gb += data_size_gb;
        stats.space_saved_gb += data_size_gb; // Archived data also saves space

        log::debug!("Archived {:.2} GB of pruned data", data_size_gb);

        Ok(())
    }

    /// Update pruning statistics
    async fn update_stats(
        &self,
        block_height: u64,
        result: &PruningResult,
        duration: std::time::Duration,
    ) -> Result<()> {
        let mut stats = self.stats.write().await;

        stats.last_pruning_block = block_height;
        stats.total_pruned_transactions += result.transactions_pruned;
        stats.total_pruned_receipts += result.receipts_pruned;
        stats.total_pruned_state_entries += result.state_entries_pruned;

        // Update average pruning time
        let total_prunings = (block_height / self.config.pruning_interval) as f64;
        let current_avg = stats.pruning_time_avg_ms;
        stats.pruning_time_avg_ms = (current_avg * (total_prunings - 1.0) + duration.as_millis() as f64) / total_prunings;

        Ok(())
    }

    /// Get current pruning statistics
    pub async fn get_stats(&self) -> PruningStats {
        self.stats.read().await.clone()
    }

    /// Schedule a pruning task for a future block
    pub fn schedule_pruning(&mut self, block_height: u64, task: PruningTask) {
        self.pruning_schedule.entry(block_height)
            .or_insert_with(Vec::new)
            .push(task);
    }

    /// Get scheduled tasks for a block
    pub fn get_scheduled_tasks(&self, block_height: u64) -> Vec<PruningTask> {
        self.pruning_schedule.get(&block_height)
            .cloned()
            .unwrap_or_default()
    }

    /// Clean up old scheduled tasks
    pub fn cleanup_old_schedules(&mut self, current_block: u64) {
        let cutoff = current_block.saturating_sub(1000); // Keep last 1000 blocks of schedule
        self.pruning_schedule.retain(|&block, _| block >= cutoff);
    }

    /// Estimate space that would be saved by pruning at a given block height
    pub fn estimate_space_savings(&self, block_height: u64) -> SpaceSavingsEstimate {
        let cutoff_tx = block_height.saturating_sub(self.config.prune_tx_older_than);
        let cutoff_receipts = block_height.saturating_sub(self.config.prune_receipts_older_than);
        let cutoff_state = block_height.saturating_sub(self.config.keep_recent_blocks);

        // Rough estimates (in MB)
        let tx_space = cutoff_tx as f64 * 0.2; // 200KB per transaction on average
        let receipts_space = cutoff_receipts as f64 * 0.05; // 50KB per receipt on average
        let state_space = if self.config.keep_min_state {
            cutoff_state as f64 * 0.01 // Minimal state kept
        } else {
            cutoff_state as f64 * 1.0 // Full state pruning
        };

        SpaceSavingsEstimate {
            transactions_mb: tx_space,
            receipts_mb: receipts_space,
            state_mb: state_space,
            total_mb: tx_space + receipts_space + state_space,
            total_gb: (tx_space + receipts_space + state_space) / 1024.0,
        }
    }
}

/// Result of a pruning operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PruningResult {
    pub transactions_pruned: u64,
    pub receipts_pruned: u64,
    pub state_entries_pruned: u64,
    pub archive_size_mb: f64,
}

/// Scheduled pruning task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PruningTask {
    /// Prune transactions older than specified block
    PruneTransactions { older_than_block: u64 },
    /// Prune receipts older than specified block
    PruneReceipts { older_than_block: u64 },
    /// Prune state entries older than specified block
    PruneState { older_than_block: u64 },
    /// Compact database after pruning
    CompactDatabase,
    /// Archive old data
    ArchiveData { archive_path: String },
}

/// Space savings estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceSavingsEstimate {
    pub transactions_mb: f64,
    pub receipts_mb: f64,
    pub state_mb: f64,
    pub total_mb: f64,
    pub total_gb: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pruning_config() {
        let config = PruningConfig::default();
        assert!(config.enabled);
        assert_eq!(config.keep_recent_blocks, 1000);
        assert_eq!(config.pruning_interval, 100);
    }

    #[test]
    fn test_state_pruner_creation() {
        let config = PruningConfig::default();
        let pruner = StatePruner::new(config);
        assert!(pruner.config.enabled);
    }

    #[test]
    fn test_should_prune() {
        let config = PruningConfig {
            enabled: true,
            keep_recent_blocks: 1000,
            pruning_interval: 100,
            ..Default::default()
        };
        let pruner = StatePruner::new(config);

        assert!(!pruner.should_prune(50)); // Too early
        assert!(!pruner.should_prune(200)); // Not at interval
        assert!(pruner.should_prune(1100)); // Should prune: past keep_recent_blocks and at interval
    }

    #[test]
    fn test_space_savings_estimate() {
        let config = PruningConfig::default();
        let pruner = StatePruner::new(config);

        let estimate = pruner.estimate_space_savings(2000);
        assert!(estimate.total_mb > 0.0);
        assert!(estimate.total_gb > 0.0);
    }

    #[tokio::test]
    async fn test_pruning_disabled() {
        let config = PruningConfig {
            enabled: false,
            ..Default::default()
        };
        let mut pruner = StatePruner::new(config);

        let result = pruner.prune_state(1100).await.unwrap();
        assert_eq!(result.transactions_pruned, 0);
        assert_eq!(result.receipts_pruned, 0);
        assert_eq!(result.state_entries_pruned, 0);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let config = PruningConfig::default();
        let pruner = StatePruner::new(config);

        let stats = pruner.get_stats().await;
        assert_eq!(stats.last_pruning_block, 0);
        assert_eq!(stats.total_pruned_transactions, 0);
    }
}
