//! Cross-region consensus for multi-region blockchain deployment
//!
//! This module implements consensus mechanisms that work across multiple
//! geographic regions, ensuring global consistency while maintaining
//! regional autonomy and performance.

use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cross-region consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossRegionConfig {
    pub region_id: String,
    pub region_priority: u32,
    pub max_region_latency_ms: u64,
    pub region_weight: f64,
    pub enable_region_failover: bool,
    pub consensus_timeout_ms: u64,
    pub region_health_check_interval_seconds: u64,
}

impl Default for CrossRegionConfig {
    fn default() -> Self {
        Self {
            region_id: "unknown".to_string(),
            region_priority: 1,
            max_region_latency_ms: 1000,
            region_weight: 1.0,
            enable_region_failover: true,
            consensus_timeout_ms: 5000,
            region_health_check_interval_seconds: 30,
        }
    }
}

/// Cross-region consensus message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrossRegionMessage {
    BlockProposal {
        block_height: u64,
        block_hash: Vec<u8>,
        proposer_region: String,
        timestamp: u64,
    },
    BlockVote {
        block_height: u64,
        block_hash: Vec<u8>,
        voter_region: String,
        vote: bool,
        timestamp: u64,
    },
    RegionStatus {
        region_id: String,
        health_score: f64,
        active_validators: usize,
        last_block_height: u64,
        timestamp: u64,
    },
    EmergencyConsensus {
        reason: String,
        affected_regions: Vec<String>,
        emergency_measures: Vec<String>,
        timestamp: u64,
    },
}

/// Cross-region consensus manager
pub struct CrossRegionConsensus {
    config: CrossRegionConfig,
    region_states: Arc<RwLock<HashMap<String, RegionState>>>,
    pending_proposals: Arc<RwLock<HashMap<u64, BlockProposalState>>>,
    consensus_queue: Arc<RwLock<VecDeque<CrossRegionMessage>>>,
    is_running: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionState {
    pub region_id: String,
    pub last_seen: u64,
    pub health_score: f64,
    pub active_validators: usize,
    pub last_block_height: u64,
    pub consensus_participation: f64,
    pub average_latency_ms: u64,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct BlockProposalState {
    block_height: u64,
    block_hash: Vec<u8>,
    proposer_region: String,
    votes: HashMap<String, bool>,
    start_time: u64,
    consensus_reached: bool,
    consensus_result: Option<bool>,
}

impl CrossRegionConsensus {
    /// Create a new cross-region consensus manager
    pub fn new(config: CrossRegionConfig) -> Self {
        Self {
            config,
            region_states: Arc::new(RwLock::new(HashMap::new())),
            pending_proposals: Arc::new(RwLock::new(HashMap::new())),
            consensus_queue: Arc::new(RwLock::new(VecDeque::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start cross-region consensus
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(BlockchainError::Consensus("Cross-region consensus already running".to_string()));
        }

        *is_running = true;

        log::info!("Cross-region consensus started for region {}", self.config.region_id);
        Ok(())
    }

    /// Propose a block for cross-region consensus
    pub async fn propose_block(&self, block_height: u64, block_hash: Vec<u8>) -> Result<()> {
        let proposal = CrossRegionMessage::BlockProposal {
            block_height,
            block_hash: block_hash.clone(),
            proposer_region: self.config.region_id.clone(),
            timestamp: current_timestamp(),
        };

        // Add to local pending proposals
        let mut pending = self.pending_proposals.write().await;
        pending.insert(block_height, BlockProposalState {
            block_height,
            block_hash: block_hash.clone(),
            proposer_region: self.config.region_id.clone(),
            votes: HashMap::new(),
            start_time: current_timestamp(),
            consensus_reached: false,
            consensus_result: None,
        });

        // Broadcast proposal to other regions
        self.broadcast_message(proposal).await?;

        log::info!("Proposed block {} for cross-region consensus", block_height);
        Ok(())
    }

    /// Vote on a block proposal
    pub async fn vote_on_block(&self, block_height: u64, block_hash: Vec<u8>, approve: bool) -> Result<()> {
        let vote = CrossRegionMessage::BlockVote {
            block_height,
            block_hash: block_hash.clone(),
            voter_region: self.config.region_id.clone(),
            vote: approve,
            timestamp: current_timestamp(),
        };

        // Record local vote
        let mut pending = self.pending_proposals.write().await;
        if let Some(proposal) = pending.get_mut(&block_height) {
            if proposal.block_hash == block_hash {
                proposal.votes.insert(self.config.region_id.clone(), approve);
            }
        }

        // Broadcast vote
        self.broadcast_message(vote).await?;

        Ok(())
    }

    /// Process incoming cross-region message
    pub async fn process_message(&self, message: CrossRegionMessage) -> Result<()> {
        match message {
            CrossRegionMessage::BlockProposal { block_height, block_hash, proposer_region, timestamp } => {
                self.process_block_proposal(block_height, block_hash, proposer_region, timestamp).await?;
            }
            CrossRegionMessage::BlockVote { block_height, block_hash, voter_region, vote, timestamp } => {
                self.process_block_vote(block_height, block_hash, voter_region, vote, timestamp).await?;
            }
            CrossRegionMessage::RegionStatus { region_id, health_score, active_validators, last_block_height, timestamp } => {
                self.update_region_status(region_id, health_score, active_validators, last_block_height, timestamp).await?;
            }
            CrossRegionMessage::EmergencyConsensus { reason, affected_regions, emergency_measures, timestamp } => {
                self.handle_emergency_consensus(reason, affected_regions, emergency_measures, timestamp).await?;
            }
        }

        Ok(())
    }

    /// Check if consensus is reached for a block
    pub async fn check_consensus(&self, block_height: u64) -> Result<Option<bool>> {
        let pending = self.pending_proposals.read().await;
        let region_states = self.region_states.read().await;

        if let Some(proposal) = pending.get(&block_height) {
            // Count active regions
            let active_regions: Vec<_> = region_states.values()
                .filter(|state| state.is_active)
                .collect();

            if active_regions.is_empty() {
                return Ok(None);
            }

            // Calculate required quorum (weighted by region health and priority)
            let total_weight: f64 = active_regions.iter()
                .map(|state| state.health_score * self.get_region_weight(&state.region_id))
                .sum();

            let approve_weight: f64 = proposal.votes.iter()
                .filter_map(|(region_id, &vote)| {
                    if vote {
                        region_states.get(region_id)
                            .map(|state| state.health_score * self.get_region_weight(region_id))
                    } else {
                        None
                    }
                })
                .sum();

            let reject_weight: f64 = proposal.votes.iter()
                .filter_map(|(region_id, &vote)| {
                    if !vote {
                        region_states.get(region_id)
                            .map(|state| state.health_score * self.get_region_weight(region_id))
                    } else {
                        None
                    }
                })
                .sum();

            // Check for supermajority (2/3 weighted approval)
            let approval_threshold = total_weight * (2.0 / 3.0);

            if approve_weight >= approval_threshold {
                return Ok(Some(true));
            } else if reject_weight >= approval_threshold {
                return Ok(Some(false));
            }

            // Check timeout
            let elapsed = current_timestamp() - proposal.start_time;
            if elapsed > (self.config.consensus_timeout_ms / 1000) {
                // Timeout - decide based on current votes
                return Ok(Some(approve_weight > reject_weight));
            }
        }

        Ok(None)
    }

    /// Get cross-region consensus statistics
    pub async fn get_consensus_stats(&self) -> Result<CrossRegionStats> {
        let region_states = self.region_states.read().await;
        let pending = self.pending_proposals.read().await;

        let active_regions = region_states.values()
            .filter(|state| state.is_active)
            .count();

        let total_regions = region_states.len();
        let average_health = if total_regions > 0 {
            region_states.values()
                .map(|state| state.health_score)
                .sum::<f64>() / total_regions as f64
        } else {
            0.0
        };

        let pending_proposals = pending.len();

        Ok(CrossRegionStats {
            total_regions,
            active_regions,
            average_health_score: average_health,
            pending_proposals,
            consensus_efficiency: self.calculate_consensus_efficiency().await,
            last_updated: current_timestamp(),
        })
    }

    // Internal methods
    async fn process_block_proposal(&self, block_height: u64, block_hash: Vec<u8>, proposer_region: String, timestamp: u64) -> Result<()> {
        // Add to pending proposals if not already present
        let mut pending = self.pending_proposals.write().await;
        pending.entry(block_height).or_insert_with(|| BlockProposalState {
            block_height,
            block_hash: block_hash.clone(),
            proposer_region,
            votes: HashMap::new(),
            start_time: timestamp,
            consensus_reached: false,
            consensus_result: None,
        });

        Ok(())
    }

    async fn process_block_vote(&self, block_height: u64, block_hash: Vec<u8>, voter_region: String, vote: bool, _timestamp: u64) -> Result<()> {
        let mut pending = self.pending_proposals.write().await;
        if let Some(proposal) = pending.get_mut(&block_height) {
            if proposal.block_hash == block_hash {
                proposal.votes.insert(voter_region, vote);
            }
        }

        Ok(())
    }

    async fn update_region_status(&self, region_id: String, health_score: f64, active_validators: usize, last_block_height: u64, timestamp: u64) -> Result<()> {
        let mut region_states = self.region_states.write().await;

        let state = region_states.entry(region_id.clone()).or_insert_with(|| RegionState {
            region_id: region_id.clone(),
            last_seen: 0,
            health_score: 100.0,
            active_validators: 0,
            last_block_height: 0,
            consensus_participation: 1.0,
            average_latency_ms: 0,
            is_active: true,
        });

        state.last_seen = timestamp;
        state.health_score = health_score;
        state.active_validators = active_validators;
        state.last_block_height = last_block_height;
        state.is_active = timestamp > current_timestamp() - 300; // Active if seen in last 5 minutes

        Ok(())
    }

    async fn handle_emergency_consensus(&self, reason: String, affected_regions: Vec<String>, _emergency_measures: Vec<String>, _timestamp: u64) -> Result<()> {
        log::warn!("Emergency consensus triggered: {} for regions {:?}", reason, affected_regions);

        // Implementation would include emergency consensus logic
        // For now, just log the emergency

        Ok(())
    }

    async fn broadcast_message(&self, message: CrossRegionMessage) -> Result<()> {
        // In production, this would broadcast to other regions via network
        // For now, just queue the message
        let mut queue = self.consensus_queue.write().await;
        queue.push_back(message);

        Ok(())
    }

    async fn calculate_consensus_efficiency(&self) -> f64 {
        let pending = self.pending_proposals.read().await;

        if pending.is_empty() {
            return 100.0;
        }

        let mut total_time = 0u64;
        let mut completed_proposals = 0usize;

        for proposal in pending.values() {
            if proposal.consensus_reached {
                total_time += current_timestamp() - proposal.start_time;
                completed_proposals += 1;
            }
        }

        if completed_proposals == 0 {
            return 50.0; // Default efficiency if no completed proposals
        }

        // Efficiency based on average consensus time (lower time = higher efficiency)
        let avg_time = total_time as f64 / completed_proposals as f64;
        let target_time = self.config.consensus_timeout_ms as f64 / 1000.0;

        (1.0 - (avg_time / target_time).min(1.0)) * 100.0
    }

    fn get_region_weight(&self, _region_id: &str) -> f64 {
        // In production, this would be configurable per region
        // For now, return equal weight
        1.0
    }
}

/// Cross-region consensus statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossRegionStats {
    pub total_regions: usize,
    pub active_regions: usize,
    pub average_health_score: f64,
    pub pending_proposals: usize,
    pub consensus_efficiency: f64,
    pub last_updated: u64,
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
    async fn test_cross_region_consensus_creation() {
        let config = CrossRegionConfig::default();
        let consensus = CrossRegionConsensus::new(config);

        assert!(!consensus.config.region_id.is_empty());
    }

    #[tokio::test]
    async fn test_block_proposal() {
        let config = CrossRegionConfig::default();
        let consensus = CrossRegionConsensus::new(config);

        consensus.start().await.unwrap();

        let block_hash = vec![1, 2, 3, 4];
        consensus.propose_block(1, block_hash).await.unwrap();

        let pending = consensus.pending_proposals.read().await;
        assert!(pending.contains_key(&1));
    }

    #[tokio::test]
    async fn test_region_status_update() {
        let config = CrossRegionConfig::default();
        let consensus = CrossRegionConsensus::new(config);

        consensus.update_region_status(
            "test-region".to_string(),
            95.0,
            10,
            100,
            current_timestamp()
        ).await.unwrap();

        let regions = consensus.region_states.read().await;
        assert!(regions.contains_key("test-region"));
    }
}
