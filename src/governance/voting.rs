use crate::core::types::Address;
use crate::utils::error::{BlockchainError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter: Address,
    pub support: bool, // true = for, false = against
    pub voting_power: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingSession {
    pub proposal_id: u64,
    pub votes: HashMap<Address, Vote>,
    pub total_for: u64,
    pub total_against: u64,
    pub total_voters: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingSystem {
    pub address: Address,
    sessions: HashMap<u64, VotingSession>,
}

impl VotingSystem {
    pub fn new(address: Address) -> Self {
        Self {
            address,
            sessions: HashMap::new(),
        }
    }

    pub fn initialize_voting_session(&mut self, proposal_id: u64) -> Result<()> {
        if self.sessions.contains_key(&proposal_id) {
            return Err(BlockchainError::Validator(
                "Voting session already exists".to_string(),
            ));
        }

        let session = VotingSession {
            proposal_id,
            votes: HashMap::new(),
            total_for: 0,
            total_against: 0,
            total_voters: 0,
        };

        self.sessions.insert(proposal_id, session);
        Ok(())
    }

    pub fn record_vote(
        &mut self,
        proposal_id: u64,
        voter: &Address,
        support: bool,
        voting_power: u64,
    ) -> Result<()> {
        let session = self
            .sessions
            .get_mut(&proposal_id)
            .ok_or_else(|| BlockchainError::Validator("Voting session not found".to_string()))?;

        // Check if voter has already voted
        if session.votes.contains_key(voter) {
            return Err(BlockchainError::Validator(
                "Already voted in this session".to_string(),
            ));
        }

        let vote = Vote {
            voter: voter.clone(),
            support,
            voting_power,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        };

        // Update totals
        if support {
            session.total_for += voting_power;
        } else {
            session.total_against += voting_power;
        }
        session.total_voters += 1;

        session.votes.insert(voter.clone(), vote);
        Ok(())
    }

    pub fn get_vote(&self, proposal_id: u64, voter: &Address) -> Option<&Vote> {
        self.sessions
            .get(&proposal_id)
            .and_then(|session| session.votes.get(voter))
    }

    pub fn get_voting_results(&self, proposal_id: u64) -> Option<(u64, u64, usize)> {
        self.sessions.get(&proposal_id).map(|session| {
            (
                session.total_for,
                session.total_against,
                session.total_voters,
            )
        })
    }

    pub fn get_voter_turnout(&self, proposal_id: u64, total_members: usize) -> Option<f64> {
        self.sessions
            .get(&proposal_id)
            .map(|session| (session.total_voters as f64 / total_members as f64) * 100.0)
    }

    pub fn has_voted(&self, proposal_id: u64, voter: &Address) -> bool {
        self.sessions
            .get(&proposal_id)
            .map(|session| session.votes.contains_key(voter))
            .unwrap_or(false)
    }

    pub fn get_session(&self, proposal_id: u64) -> Option<&VotingSession> {
        self.sessions.get(&proposal_id)
    }

    pub fn cleanup_session(&mut self, proposal_id: u64) -> Result<()> {
        self.sessions.remove(&proposal_id);
        Ok(())
    }
}
