//! Block sequence integrity verification for multi-region blockchain
//!
//! This module ensures that block sequences are maintained consistently
//! across all regions and nodes, detecting gaps, duplicates, and ordering
//! violations that could compromise blockchain integrity.

use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Sequence integrity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceIntegrityConfig {
    pub enable_integrity_checks: bool,
    pub max_sequence_gap: u64,
    pub integrity_check_interval_seconds: u64,
    pub max_out_of_order_tolerance: u64,
    pub enable_sequence_repair: bool,
    pub integrity_alert_threshold: f64,
}

impl Default for SequenceIntegrityConfig {
    fn default() -> Self {
        Self {
            enable_integrity_checks: true,
            max_sequence_gap: 10,
            integrity_check_interval_seconds: 60,
            max_out_of_order_tolerance: 3,
            enable_sequence_repair: false,
            integrity_alert_threshold: 95.0,
        }
    }
}

/// Block sequence entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSequenceEntry {
    pub height: u64,
    pub hash: Vec<u8>,
    pub timestamp: u64,
    pub region_id: String,
    pub validator_id: String,
    pub previous_hash: Vec<u8>,
}

/// Sequence integrity violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceViolation {
    pub violation_type: ViolationType,
    pub block_height: u64,
    pub description: String,
    pub affected_regions: Vec<String>,
    pub severity: ViolationSeverity,
    pub detected_at: u64,
    pub suggested_action: String,
}

/// Types of sequence violations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViolationType {
    GapInSequence,
    DuplicateBlock,
    OutOfOrderBlock,
    HashMismatch,
    TimestampAnomaly,
    RegionDivergence,
}

/// Severity levels for violations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Sequence integrity verifier
pub struct SequenceIntegrityVerifier {
    config: SequenceIntegrityConfig,
    canonical_sequence: Arc<RwLock<VecDeque<BlockSequenceEntry>>>,
    region_sequences: Arc<RwLock<HashMap<String, VecDeque<BlockSequenceEntry>>>>,
    violations: Arc<RwLock<VecDeque<SequenceViolation>>>,
    integrity_metrics: Arc<RwLock<IntegrityMetrics>>,
    is_running: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityMetrics {
    pub total_blocks_verified: u64,
    pub integrity_score: f64,
    pub gaps_detected: u64,
    pub duplicates_detected: u64,
    pub out_of_order_blocks: u64,
    pub hash_mismatches: u64,
    pub last_integrity_check: u64,
    pub average_verification_time_ms: f64,
}

impl SequenceIntegrityVerifier {
    /// Create a new sequence integrity verifier
    pub fn new(config: SequenceIntegrityConfig) -> Self {
        Self {
            config,
            canonical_sequence: Arc::new(RwLock::new(VecDeque::with_capacity(10000))),
            region_sequences: Arc::new(RwLock::new(HashMap::new())),
            violations: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            integrity_metrics: Arc::new(RwLock::new(IntegrityMetrics {
                total_blocks_verified: 0,
                integrity_score: 100.0,
                gaps_detected: 0,
                duplicates_detected: 0,
                out_of_order_blocks: 0,
                hash_mismatches: 0,
                last_integrity_check: current_timestamp(),
                average_verification_time_ms: 0.0,
            })),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start integrity verification
    pub async fn start_verification(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(BlockchainError::Consensus(
                "Sequence integrity verification already running".to_string(),
            ));
        }

        *is_running = true;

        log::info!("Block sequence integrity verification started");
        Ok(())
    }

    /// Verify and add a block to the sequence
    pub async fn verify_and_add_block(&self, entry: BlockSequenceEntry) -> Result<bool> {
        if !self.config.enable_integrity_checks {
            return Ok(true);
        }

        let start_time = std::time::Instant::now();

        // Perform comprehensive integrity checks
        let integrity_checks_passed = self.perform_integrity_checks(&entry).await?;

        if integrity_checks_passed {
            // Add to canonical sequence
            let mut canonical = self.canonical_sequence.write().await;
            canonical.push_back(entry.clone());

            // Maintain sequence size
            while canonical.len() > 10000 {
                canonical.pop_front();
            }

            // Add to region-specific sequence
            let mut regions = self.region_sequences.write().await;
            let region_sequence = regions
                .entry(entry.region_id.clone())
                .or_insert_with(VecDeque::new);
            region_sequence.push_back(entry);

            // Maintain region sequence size
            while region_sequence.len() > 1000 {
                region_sequence.pop_front();
            }

            // Update metrics
            let mut metrics = self.integrity_metrics.write().await;
            metrics.total_blocks_verified += 1;
            let verification_time = start_time.elapsed().as_millis() as f64;
            metrics.average_verification_time_ms =
                (metrics.average_verification_time_ms + verification_time) / 2.0;

            Ok(true)
        } else {
            // Log violation
            log::warn!(
                "Block sequence integrity check failed for block {}",
                entry.height
            );
            Ok(false)
        }
    }

    /// Perform comprehensive integrity checks
    async fn perform_integrity_checks(&self, entry: &BlockSequenceEntry) -> Result<bool> {
        let canonical = self.canonical_sequence.read().await;
        let regions = self.region_sequences.read().await;

        // Check 1: Sequence continuity
        if !self.check_sequence_continuity(&canonical, entry).await? {
            return Ok(false);
        }

        // Check 2: Hash chain integrity
        if !self.check_hash_chain_integrity(&canonical, entry).await? {
            return Ok(false);
        }

        // Check 3: Timestamp ordering
        if !self.check_timestamp_ordering(&canonical, entry).await? {
            return Ok(false);
        }

        // Check 4: Cross-region consistency
        if !self.check_cross_region_consistency(&regions, entry).await? {
            return Ok(false);
        }

        // Check 5: Duplicate detection
        if !self
            .check_for_duplicates(&canonical, &regions, entry)
            .await?
        {
            return Ok(false);
        }

        Ok(true)
    }

    /// Check sequence continuity (no gaps)
    async fn check_sequence_continuity(
        &self,
        canonical: &VecDeque<BlockSequenceEntry>,
        entry: &BlockSequenceEntry,
    ) -> Result<bool> {
        if canonical.is_empty() && entry.height == 1 {
            return Ok(true);
        }

        if let Some(last_block) = canonical.back() {
            let expected_height = last_block.height + 1;

            if entry.height == expected_height {
                Ok(true)
            } else if entry.height > expected_height {
                // Gap detected
                let gap_size = entry.height - expected_height;
                if gap_size <= self.config.max_sequence_gap {
                    // Allow small gaps (might be filled later)
                    log::warn!(
                        "Small sequence gap detected: {} blocks at height {}",
                        gap_size,
                        entry.height
                    );
                    Ok(true)
                } else {
                    // Large gap - violation
                    self.record_violation(SequenceViolation {
                        violation_type: ViolationType::GapInSequence,
                        block_height: entry.height,
                        description: format!("Large sequence gap of {} blocks detected", gap_size),
                        affected_regions: vec![entry.region_id.clone()],
                        severity: ViolationSeverity::High,
                        detected_at: current_timestamp(),
                        suggested_action: "Investigate network connectivity and consensus issues"
                            .to_string(),
                    })
                    .await;

                    let mut metrics = self.integrity_metrics.write().await;
                    metrics.gaps_detected += 1;

                    Ok(false)
                }
            } else {
                // Out of order block
                self.record_violation(SequenceViolation {
                    violation_type: ViolationType::OutOfOrderBlock,
                    block_height: entry.height,
                    description: format!(
                        "Block received out of order. Expected height: {}, Got: {}",
                        expected_height, entry.height
                    ),
                    affected_regions: vec![entry.region_id.clone()],
                    severity: ViolationSeverity::Medium,
                    detected_at: current_timestamp(),
                    suggested_action: "Check network latency and synchronization".to_string(),
                })
                .await;

                let mut metrics = self.integrity_metrics.write().await;
                metrics.out_of_order_blocks += 1;

                Ok(false)
            }
        } else {
            // First block should be height 1
            Ok(entry.height == 1)
        }
    }

    /// Check hash chain integrity
    async fn check_hash_chain_integrity(
        &self,
        canonical: &VecDeque<BlockSequenceEntry>,
        entry: &BlockSequenceEntry,
    ) -> Result<bool> {
        if canonical.is_empty() && entry.height == 1 {
            // Genesis block - no previous hash to check
            return Ok(true);
        }

        if let Some(last_block) = canonical.back() {
            if entry.previous_hash == self.calculate_block_hash(last_block) {
                Ok(true)
            } else {
                // Hash mismatch
                self.record_violation(SequenceViolation {
                    violation_type: ViolationType::HashMismatch,
                    block_height: entry.height,
                    description: format!("Hash chain integrity violated at block {}", entry.height),
                    affected_regions: vec![entry.region_id.clone()],
                    severity: ViolationSeverity::Critical,
                    detected_at: current_timestamp(),
                    suggested_action: "Immediate investigation required - potential chain fork"
                        .to_string(),
                })
                .await;

                let mut metrics = self.integrity_metrics.write().await;
                metrics.hash_mismatches += 1;

                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    /// Check timestamp ordering
    async fn check_timestamp_ordering(
        &self,
        canonical: &VecDeque<BlockSequenceEntry>,
        entry: &BlockSequenceEntry,
    ) -> Result<bool> {
        if let Some(last_block) = canonical.back() {
            if entry.timestamp >= last_block.timestamp {
                Ok(true)
            } else {
                // Timestamp anomaly
                let time_diff = last_block.timestamp - entry.timestamp;
                self.record_violation(SequenceViolation {
                    violation_type: ViolationType::TimestampAnomaly,
                    block_height: entry.height,
                    description: format!(
                        "Block timestamp {}ms earlier than previous block",
                        time_diff
                    ),
                    affected_regions: vec![entry.region_id.clone()],
                    severity: ViolationSeverity::Medium,
                    detected_at: current_timestamp(),
                    suggested_action: "Check system clock synchronization".to_string(),
                })
                .await;

                Ok(false)
            }
        } else {
            Ok(true)
        }
    }

    /// Check cross-region consistency
    async fn check_cross_region_consistency(
        &self,
        regions: &HashMap<String, VecDeque<BlockSequenceEntry>>,
        entry: &BlockSequenceEntry,
    ) -> Result<bool> {
        let mut region_heights = Vec::new();

        for (region_id, sequence) in regions.iter() {
            if let Some(latest) = sequence.back() {
                region_heights.push((region_id.clone(), latest.height));
            }
        }

        if region_heights.is_empty() {
            return Ok(true);
        }

        // Check if this region is too far behind others
        let max_height = region_heights.iter().map(|(_, h)| *h).max().unwrap();
        let min_height = region_heights.iter().map(|(_, h)| *h).min().unwrap();

        if max_height - min_height > self.config.max_out_of_order_tolerance {
            self.record_violation(SequenceViolation {
                violation_type: ViolationType::RegionDivergence,
                block_height: entry.height,
                description: format!(
                    "Region divergence detected. Max height: {}, Min height: {}",
                    max_height, min_height
                ),
                affected_regions: region_heights.into_iter().map(|(id, _)| id).collect(),
                severity: ViolationSeverity::High,
                detected_at: current_timestamp(),
                suggested_action: "Check network connectivity between regions".to_string(),
            })
            .await;

            return Ok(false);
        }

        Ok(true)
    }

    /// Check for duplicate blocks
    async fn check_for_duplicates(
        &self,
        canonical: &VecDeque<BlockSequenceEntry>,
        regions: &HashMap<String, VecDeque<BlockSequenceEntry>>,
        entry: &BlockSequenceEntry,
    ) -> Result<bool> {
        let block_hash = self.calculate_block_hash(entry);

        // Check canonical sequence
        for existing in canonical.iter() {
            if self.calculate_block_hash(existing) == block_hash && existing.height != entry.height
            {
                self.record_violation(SequenceViolation {
                    violation_type: ViolationType::DuplicateBlock,
                    block_height: entry.height,
                    description: format!(
                        "Duplicate block detected with same hash as block {}",
                        existing.height
                    ),
                    affected_regions: vec![entry.region_id.clone()],
                    severity: ViolationSeverity::Critical,
                    detected_at: current_timestamp(),
                    suggested_action: "Investigate potential replay attack".to_string(),
                })
                .await;

                let mut metrics = self.integrity_metrics.write().await;
                metrics.duplicates_detected += 1;

                return Ok(false);
            }
        }

        // Check region sequences
        for (region_id, sequence) in regions.iter() {
            for existing in sequence.iter() {
                if self.calculate_block_hash(existing) == block_hash
                    && existing.height != entry.height
                {
                    self.record_violation(SequenceViolation {
                        violation_type: ViolationType::DuplicateBlock,
                        block_height: entry.height,
                        description: format!(
                            "Duplicate block detected in region {} with same hash as block {}",
                            region_id, existing.height
                        ),
                        affected_regions: vec![entry.region_id.clone(), region_id.clone()],
                        severity: ViolationSeverity::Critical,
                        detected_at: current_timestamp(),
                        suggested_action: "Cross-region investigation required".to_string(),
                    })
                    .await;

                    let mut metrics = self.integrity_metrics.write().await;
                    metrics.duplicates_detected += 1;

                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Calculate overall integrity score
    pub async fn calculate_integrity_score(&self) -> Result<f64> {
        let metrics = self.integrity_metrics.read().await;
        let _violations = self.violations.read().await;

        if metrics.total_blocks_verified == 0 {
            return Ok(100.0);
        }

        // Calculate penalty based on violations
        let total_violations = metrics.gaps_detected
            + metrics.duplicates_detected
            + metrics.out_of_order_blocks
            + metrics.hash_mismatches;

        let violation_rate = total_violations as f64 / metrics.total_blocks_verified as f64;

        // Integrity score starts at 100 and decreases with violations
        let integrity_score = (1.0 - violation_rate.min(1.0)) * 100.0;

        Ok(integrity_score)
    }

    /// Get recent violations
    pub async fn get_recent_violations(&self, limit: usize) -> Result<Vec<SequenceViolation>> {
        let violations = self.violations.read().await;
        Ok(violations.iter().rev().take(limit).cloned().collect())
    }

    /// Get integrity metrics
    pub async fn get_integrity_metrics(&self) -> Result<IntegrityMetrics> {
        let metrics = self.integrity_metrics.read().await;
        Ok(metrics.clone())
    }

    /// Attempt to repair sequence integrity (if enabled)
    pub async fn attempt_sequence_repair(&self) -> Result<bool> {
        if !self.config.enable_sequence_repair {
            return Ok(false);
        }

        // Implementation would include sequence repair logic
        // For now, just return false
        log::info!("Sequence repair attempted but not implemented");
        Ok(false)
    }

    // Internal helper methods
    fn calculate_block_hash(&self, entry: &BlockSequenceEntry) -> Vec<u8> {
        // Simplified hash calculation - in production, use proper block hashing
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        entry.height.hash(&mut hasher);
        entry.timestamp.hash(&mut hasher);
        entry.region_id.hash(&mut hasher);
        entry.validator_id.hash(&mut hasher);
        hasher.write(&entry.previous_hash);

        let hash_value = hasher.finish();
        hash_value.to_be_bytes().to_vec()
    }

    async fn record_violation(&self, violation: SequenceViolation) {
        let mut violations = self.violations.write().await;
        violations.push_back(violation);

        // Maintain violation history size
        while violations.len() > 1000 {
            violations.pop_front();
        }
    }
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
    async fn test_sequence_integrity_verifier_creation() {
        let config = SequenceIntegrityConfig::default();
        let verifier = SequenceIntegrityVerifier::new(config);

        assert!(verifier.config.enable_integrity_checks);
    }

    #[tokio::test]
    async fn test_block_sequence_verification() {
        let config = SequenceIntegrityConfig::default();
        let verifier = SequenceIntegrityVerifier::new(config);

        verifier.start_verification().await.unwrap();

        let timestamp1 = current_timestamp();

        // Add first block
        let block1 = BlockSequenceEntry {
            height: 1,
            hash: vec![1, 2, 3],
            timestamp: timestamp1,
            region_id: "region-1".to_string(),
            validator_id: "validator-1".to_string(),
            previous_hash: vec![0; 32], // Genesis previous hash
        };

        let result1 = verifier.verify_and_add_block(block1.clone()).await.unwrap();
        assert!(result1);

        // Add second block
        let block2 = BlockSequenceEntry {
            height: 2,
            hash: vec![4, 5, 6],
            timestamp: timestamp1 + 30,
            region_id: "region-1".to_string(),
            validator_id: "validator-1".to_string(),
            previous_hash: verifier.calculate_block_hash(&block1),
        };

        let result2 = verifier.verify_and_add_block(block2).await.unwrap();
        assert!(result2);

        // Check integrity score
        let score = verifier.calculate_integrity_score().await.unwrap();
        assert!(score > 95.0); // Should be high for valid sequence
    }

    #[tokio::test]
    async fn test_gap_detection() {
        let config = SequenceIntegrityConfig::default();
        let verifier = SequenceIntegrityVerifier::new(config);

        verifier.start_verification().await.unwrap();

        // Add block 1
        let block1 = BlockSequenceEntry {
            height: 1,
            hash: vec![1, 2, 3],
            timestamp: current_timestamp(),
            region_id: "region-1".to_string(),
            validator_id: "validator-1".to_string(),
            previous_hash: vec![0; 32],
        };

        verifier.verify_and_add_block(block1).await.unwrap();

        // Try to add block 5 (gap of 3, but within max_sequence_gap of 10)
        // This should pass sequence continuity but fail hash check
        let block5 = BlockSequenceEntry {
            height: 5,
            hash: vec![7, 8, 9],
            timestamp: current_timestamp() + 120,
            region_id: "region-1".to_string(),
            validator_id: "validator-1".to_string(),
            previous_hash: vec![0; 32], // Wrong previous hash
        };

        let result = verifier.verify_and_add_block(block5).await.unwrap();
        // Should fail due to hash mismatch
        assert!(!result);

        // Check that violation was recorded
        let violations = verifier.get_recent_violations(10).await.unwrap();
        assert!(!violations.is_empty());
        assert_eq!(violations[0].violation_type, ViolationType::HashMismatch);
    }

    #[tokio::test]
    async fn test_large_gap_detection() {
        let mut config = SequenceIntegrityConfig::default();
        config.max_sequence_gap = 2; // Small gap tolerance
        let verifier = SequenceIntegrityVerifier::new(config);

        verifier.start_verification().await.unwrap();

        // Add block 1
        let block1 = BlockSequenceEntry {
            height: 1,
            hash: vec![1, 2, 3],
            timestamp: current_timestamp(),
            region_id: "region-1".to_string(),
            validator_id: "validator-1".to_string(),
            previous_hash: vec![0; 32],
        };

        verifier.verify_and_add_block(block1.clone()).await.unwrap();

        // Try to add block 5 (gap of 3, exceeds max_sequence_gap of 2)
        let block5 = BlockSequenceEntry {
            height: 5,
            hash: vec![7, 8, 9],
            timestamp: current_timestamp() + 120,
            region_id: "region-1".to_string(),
            validator_id: "validator-1".to_string(),
            previous_hash: verifier.calculate_block_hash(&block1), // Correct previous hash
        };

        let result = verifier.verify_and_add_block(block5).await.unwrap();
        // Should fail due to large gap
        assert!(!result);

        // Check that violation was recorded
        let violations = verifier.get_recent_violations(10).await.unwrap();
        assert!(!violations.is_empty());
        assert_eq!(violations[0].violation_type, ViolationType::GapInSequence);
    }
}
