// src/governance/mod.rs

// Declare submodules
pub mod dao;
pub mod voting;
pub mod treasury;
pub mod proposals;

 // Keep Result for potential future use
use serde::{Deserialize, Serialize};

// Define GovernanceConfig here (assuming this is the primary definition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub voting_delay: u64,        // blocks before voting starts
    pub voting_period: u64,       // blocks voting is active
    pub proposal_threshold: u64,  // min tokens needed to propose
    pub quorum_votes: u64,        // min votes needed for quorum
    pub timelock_delay: u64,      // blocks before executed proposal can be executed
}

impl Default for GovernanceConfig {
    fn default() -> Self {
        Self {
            voting_delay: 1,      // 1 block
            voting_period: 17280, // ~1 day with 15s blocks (adjust based on your block time)
            proposal_threshold: 10_000, // Example: 10,000 governance tokens
            quorum_votes: 400_000, // Example: 4% of 10M total supply
            timelock_delay: 0,    // No timelock by default
        }
    }
}

// Re-export core types for easier imports from outside the governance module
// Corrected paths for ProposalAction and ProposalStatus based on compiler hints
pub use dao::{DAO, DAOInfo, Member};
pub use voting::{VotingSystem, Vote, VotingSession};
pub use treasury::{Treasury, TreasuryTransaction};
pub use proposals::{ProposalManager, Proposal, ProposalAction, ProposalStatus}; // <-- Fetched from proposals module