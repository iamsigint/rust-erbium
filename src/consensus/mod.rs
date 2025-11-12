pub mod pos;
pub mod slashing;
pub mod validator;
pub mod rewards;
pub mod randomness;
pub mod cross_region;
pub mod sequence_integrity;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::utils::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub block_time: u64, // in seconds
    pub epoch_length: u64, // blocks per epoch
    pub max_validators: u64,
    pub min_stake: u64,
    pub slashing_percentage: u8, // 100% for zero tolerance

    // Multi-node scalability enhancements
    pub enable_cross_region_consensus: bool,
    pub regions: Vec<String>,
    pub region_quorum_requirement: f64, // Percentage of regions needed for consensus
    pub block_sequence_integrity_check: bool,
    pub max_consensus_delay_ms: u64,
    pub adaptive_block_time: bool,
    pub emergency_consensus_mode: bool,

    // Governance integration
    pub governance_override_enabled: bool,
    pub proposal_consensus_threshold: f64,
    pub emergency_governance_quorum: f64,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            block_time: 30,
            epoch_length: 7200, // ~24 hours with 30s blocks
            max_validators: 100,
            min_stake: 10_000, // 10,000 ERB
            slashing_percentage: 100,

            // Multi-node scalability defaults
            enable_cross_region_consensus: true,
            regions: vec!["us-east".to_string(), "us-west".to_string(), "eu-central".to_string(), "ap-southeast".to_string()],
            region_quorum_requirement: 0.75, // 75% of regions
            block_sequence_integrity_check: true,
            max_consensus_delay_ms: 5000,
            adaptive_block_time: true,
            emergency_consensus_mode: false,

            // Governance defaults
            governance_override_enabled: true,
            proposal_consensus_threshold: 0.67, // 2/3 majority
            emergency_governance_quorum: 0.8, // 80% for emergency decisions
        }
    }
}

/// Multi-region consensus state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossRegionConsensusState {
    pub region_states: HashMap<String, RegionConsensusState>,
    pub global_consensus_height: u64,
    pub last_global_block_timestamp: u64,
    pub region_quorum_achieved: bool,
    pub emergency_mode_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConsensusState {
    pub region_id: String,
    pub local_height: u64,
    pub validator_count: usize,
    pub active_validators: usize,
    pub last_block_timestamp: u64,
    pub consensus_delay_ms: u64,
    pub health_score: f64,
}

/// Block sequence integrity tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSequenceIntegrity {
    pub expected_sequence: Vec<u64>,
    pub actual_sequence: Vec<u64>,
    pub gaps_detected: Vec<u64>,
    pub duplicates_detected: Vec<u64>,
    pub integrity_score: f64,
    pub last_check_timestamp: u64,
}

/// Enhanced consensus manager for multi-node blockchain
#[allow(dead_code)]
pub struct MultiNodeConsensusManager {
    config: ConsensusConfig,
    pos_consensus: pos::ProofOfStake,
    cross_region_state: CrossRegionConsensusState,
    sequence_integrity: BlockSequenceIntegrity,
    governance_integration: GovernanceConsensusIntegration,
}

impl MultiNodeConsensusManager {
    /// Create a new multi-node consensus manager
    pub fn new(config: ConsensusConfig) -> Self {
        let pos_consensus = pos::ProofOfStake::new(config.clone());

        let mut region_states = HashMap::new();
        for region in &config.regions {
            region_states.insert(region.clone(), RegionConsensusState {
                region_id: region.clone(),
                local_height: 0,
                validator_count: 0,
                active_validators: 0,
                last_block_timestamp: 0,
                consensus_delay_ms: 0,
                health_score: 100.0,
            });
        }

        let cross_region_state = CrossRegionConsensusState {
            region_states,
            global_consensus_height: 0,
            last_global_block_timestamp: 0,
            region_quorum_achieved: false,
            emergency_mode_active: false,
        };

        let sequence_integrity = BlockSequenceIntegrity {
            expected_sequence: Vec::new(),
            actual_sequence: Vec::new(),
            gaps_detected: Vec::new(),
            duplicates_detected: Vec::new(),
            integrity_score: 100.0,
            last_check_timestamp: current_timestamp(),
        };

        let governance_integration = GovernanceConsensusIntegration::new(config.clone());

        Self {
            config,
            pos_consensus,
            cross_region_state,
            sequence_integrity,
            governance_integration,
        }
    }

    /// Process cross-region consensus for a new block
    pub async fn process_cross_region_consensus(&mut self, block_height: u64, region_id: &str) -> Result<bool> {
        // Update region state
        if let Some(region_state) = self.cross_region_state.region_states.get_mut(region_id) {
            region_state.local_height = block_height;
            region_state.last_block_timestamp = current_timestamp();
        }

        // Check if global consensus can be achieved
        let region_quorum_count = (self.config.regions.len() as f64 * self.config.region_quorum_requirement) as usize;
        let regions_at_height = self.cross_region_state.region_states.values()
            .filter(|state| state.local_height >= block_height)
            .count();

        let quorum_achieved = regions_at_height >= region_quorum_count;

        if quorum_achieved && block_height > self.cross_region_state.global_consensus_height {
            self.cross_region_state.global_consensus_height = block_height;
            self.cross_region_state.last_global_block_timestamp = current_timestamp();
            self.cross_region_state.region_quorum_achieved = true;

            // Update block sequence integrity
            self.update_sequence_integrity(block_height).await?;

            log::info!("Global consensus achieved at height {} with {} regions", block_height, regions_at_height);
        }

        Ok(quorum_achieved)
    }

    /// Check block sequence integrity across regions
    pub async fn verify_block_sequence_integrity(&mut self) -> Result<f64> {
        let mut integrity_score = 100.0;
        let mut gaps = Vec::new();
        let mut duplicates = Vec::new();

        // Collect all block heights from all regions
        let mut all_heights = Vec::new();
        for region_state in self.cross_region_state.region_states.values() {
            all_heights.push(region_state.local_height);
        }

        if let Some(&max_height) = all_heights.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)) {
            // Check for gaps in sequence
            for height in 1..=max_height {
                let regions_with_height = self.cross_region_state.region_states.values()
                    .filter(|state| state.local_height >= height)
                    .count();

                if regions_with_height == 0 {
                    gaps.push(height);
                    integrity_score -= 5.0; // Penalty for gaps
                }
            }

            // Check for duplicates (simplified - would need more sophisticated logic)
            let mut height_counts = HashMap::new();
            for &height in &all_heights {
                *height_counts.entry(height).or_insert(0) += 1;
            }

            for (&height, &count) in &height_counts {
                if count > self.config.regions.len() {
                    duplicates.push(height);
                    integrity_score -= 2.0; // Penalty for duplicates
                }
            }
        }

        // Update integrity state
        self.sequence_integrity.gaps_detected = gaps;
        self.sequence_integrity.duplicates_detected = duplicates;
        self.sequence_integrity.integrity_score = if integrity_score > 0.0 { integrity_score } else { 0.0 };
        self.sequence_integrity.last_check_timestamp = current_timestamp();

        Ok(integrity_score)
    }

    /// Handle emergency consensus mode
    pub async fn activate_emergency_consensus(&mut self) -> Result<()> {
        if !self.config.emergency_consensus_mode {
            return Err(crate::utils::error::BlockchainError::Consensus("Emergency consensus not enabled".to_string()));
        }

        self.cross_region_state.emergency_mode_active = true;

        // Reduce consensus requirements for emergency
        // Implementation would include emergency consensus logic

        log::warn!("Emergency consensus mode activated");
        Ok(())
    }

    /// Integrate governance decisions into consensus
    pub async fn process_governance_consensus(&mut self, proposal_id: u64, votes: HashMap<String, bool>) -> Result<bool> {
        self.governance_integration.process_proposal_consensus(proposal_id, votes).await
    }

    /// Get current cross-region consensus status
    pub fn get_cross_region_status(&self) -> &CrossRegionConsensusState {
        &self.cross_region_state
    }

    /// Get block sequence integrity status
    pub fn get_sequence_integrity(&self) -> &BlockSequenceIntegrity {
        &self.sequence_integrity
    }

    // Internal methods
    async fn update_sequence_integrity(&mut self, block_height: u64) -> Result<()> {
        self.sequence_integrity.actual_sequence.push(block_height);

        // Maintain sequence history (last 1000 blocks)
        if self.sequence_integrity.actual_sequence.len() > 1000 {
            self.sequence_integrity.actual_sequence.remove(0);
        }

        // Recalculate integrity score
        self.verify_block_sequence_integrity().await?;

        Ok(())
    }
}

/// Governance integration for consensus decisions
pub struct GovernanceConsensusIntegration {
    config: ConsensusConfig,
    active_proposals: HashMap<u64, GovernanceProposalState>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct GovernanceProposalState {
    proposal_id: u64,
    votes: HashMap<String, bool>,
    consensus_reached: bool,
    consensus_result: Option<bool>,
}

impl GovernanceConsensusIntegration {
    pub fn new(config: ConsensusConfig) -> Self {
        Self {
            config,
            active_proposals: HashMap::new(),
        }
    }

    pub async fn process_proposal_consensus(&mut self, proposal_id: u64, votes: HashMap<String, bool>) -> Result<bool> {
        let state = self.active_proposals.entry(proposal_id).or_insert(GovernanceProposalState {
            proposal_id,
            votes: HashMap::new(),
            consensus_reached: false,
            consensus_result: None,
        });

        // Update votes
        for (voter, vote) in votes {
            state.votes.insert(voter, vote);
        }

        // Check if consensus threshold is met
        let total_votes = state.votes.len() as f64;
        let yes_votes = state.votes.values().filter(|&&v| v).count() as f64;

        if total_votes > 0.0 {
            let approval_rate = yes_votes / total_votes;

            if approval_rate >= self.config.proposal_consensus_threshold {
                state.consensus_reached = true;
                state.consensus_result = Some(true);
                return Ok(true);
            } else if (total_votes - yes_votes) / total_votes >= self.config.proposal_consensus_threshold {
                state.consensus_reached = true;
                state.consensus_result = Some(false);
                return Ok(false);
            }
        }

        Ok(false) // No consensus yet
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
