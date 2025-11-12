//! DAO (Decentralized Autonomous Organization) contract implementation

use crate::core::types::Address;
use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// DAO contract with governance features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAOContract {
    pub name: String,
    pub description: String,
    pub members: HashMap<Address, MemberInfo>,
    pub proposals: Vec<DAOProposal>,
    pub governance_token: Option<Address>, // Address of governance token contract
    pub quorum_percentage: u8, // Minimum percentage of total votes needed
    pub voting_period: u64, // Voting period in blocks
    pub proposal_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberInfo {
    pub address: Address,
    pub voting_power: u64,
    pub joined_at: u64, // Block number when joined
    pub reputation: u32, // Reputation score
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAOProposal {
    pub id: u64,
    pub proposer: Address,
    pub description: String,
    pub proposal_type: ProposalType,
    pub votes_for: u64,
    pub votes_against: u64,
    pub status: ProposalStatus,
    pub created_at: u64, // Block number
    pub voting_ends_at: u64, // Block number
    pub executed_at: Option<u64>, // Block number
    pub voters: HashMap<Address, bool>, // address -> approved
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalType {
    General,
    ParameterChange,
    ContractUpgrade,
    TreasuryAllocation,
    MemberAddition,
    MemberRemoval,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
    Cancelled,
}

impl DAOContract {
    /// Create a new DAO contract
    pub fn new(
        name: String,
        description: String,
        governance_token: Option<Address>,
        quorum_percentage: u8,
        voting_period: u64,
        founder: Address,
        current_block: u64,
    ) -> Self {
        let mut members = HashMap::new();
        members.insert(founder.clone(), MemberInfo {
            address: founder,
            voting_power: 100, // Initial voting power for founder
            joined_at: current_block,
            reputation: 100,
        });

        Self {
            name,
            description,
            members,
            proposals: Vec::new(),
            governance_token,
            quorum_percentage,
            voting_period,
            proposal_count: 0,
        }
    }

    /// Add a new member to the DAO
    pub fn add_member(&mut self, member: Address, voting_power: u64, current_block: u64) -> Result<()> {
        if self.members.contains_key(&member) {
            return Err(BlockchainError::Governance("Member already exists".to_string()));
        }

        self.members.insert(member.clone(), MemberInfo {
            address: member,
            voting_power,
            joined_at: current_block,
            reputation: 50, // Default reputation
        });

        Ok(())
    }

    /// Remove a member from the DAO
    pub fn remove_member(&mut self, member: &Address) -> Result<()> {
        if !self.members.contains_key(member) {
            return Err(BlockchainError::Governance("Member not found".to_string()));
        }

        self.members.remove(member);
        Ok(())
    }

    /// Create a new proposal
    pub fn create_proposal(
        &mut self,
        proposer: &Address,
        description: String,
        proposal_type: ProposalType,
        current_block: u64,
    ) -> Result<u64> {
        // Check if proposer is a member
        if !self.members.contains_key(proposer) {
            return Err(BlockchainError::Governance("Only members can create proposals".to_string()));
        }

        self.proposal_count += 1;
        let proposal_id = self.proposal_count;

        let proposal = DAOProposal {
            id: proposal_id,
            proposer: proposer.clone(),
            description,
            proposal_type,
            votes_for: 0,
            votes_against: 0,
            status: ProposalStatus::Active,
            created_at: current_block,
            voting_ends_at: current_block + self.voting_period,
            executed_at: None,
            voters: HashMap::new(),
        };

        self.proposals.push(proposal);
        Ok(proposal_id)
    }

    /// Vote on a proposal
    pub fn vote(&mut self, voter: &Address, proposal_id: u64, approve: bool, current_block: u64) -> Result<()> {
        // Check if voter is a member
        let member_info = self.members.get(voter)
            .ok_or_else(|| BlockchainError::Governance("Not a DAO member".to_string()))?;

        // Find the proposal
        let proposal = self.proposals.iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| BlockchainError::Governance("Proposal not found".to_string()))?;

        // Check if proposal is active
        if proposal.status != ProposalStatus::Active {
            return Err(BlockchainError::Governance("Proposal not active".to_string()));
        }

        // Check if voting period has ended
        if current_block > proposal.voting_ends_at {
            return Err(BlockchainError::Governance("Voting period has ended".to_string()));
        }

        // Check if member already voted
        if proposal.voters.contains_key(voter) {
            return Err(BlockchainError::Governance("Member already voted".to_string()));
        }

        // Record the vote
        proposal.voters.insert(voter.clone(), approve);

        if approve {
            proposal.votes_for += member_info.voting_power;
        } else {
            proposal.votes_against += member_info.voting_power;
        }

        Ok(())
    }

    /// Execute a proposal after voting period
    pub fn execute_proposal(&mut self, proposal_id: u64, current_block: u64) -> Result<()> {
        let proposal = self.proposals.iter_mut()
            .find(|p| p.id == proposal_id)
            .ok_or_else(|| BlockchainError::Governance("Proposal not found".to_string()))?;

        // Check if proposal is active
        if proposal.status != ProposalStatus::Active {
            return Err(BlockchainError::Governance("Proposal not active".to_string()));
        }

        // Check if voting period has ended
        if current_block <= proposal.voting_ends_at {
            return Err(BlockchainError::Governance("Voting period not ended".to_string()));
        }

        // Calculate total possible votes
        let total_voting_power: u64 = self.members.values().map(|m| m.voting_power).sum();
        let total_votes = proposal.votes_for + proposal.votes_against;

        // Check quorum
        let quorum_threshold = (total_voting_power * self.quorum_percentage as u64) / 100;
        if total_votes < quorum_threshold {
            proposal.status = ProposalStatus::Rejected;
            return Err(BlockchainError::Governance("Quorum not reached".to_string()));
        }

        // Determine outcome
        if proposal.votes_for > proposal.votes_against {
            proposal.status = ProposalStatus::Passed;
            proposal.executed_at = Some(current_block);
        } else {
            proposal.status = ProposalStatus::Rejected;
        }

        Ok(())
    }

    /// Get proposal details
    pub fn get_proposal(&self, proposal_id: u64) -> Option<&DAOProposal> {
        self.proposals.iter().find(|p| p.id == proposal_id)
    }

    /// Get all proposals
    pub fn get_proposals(&self) -> &[DAOProposal] {
        &self.proposals
    }

    /// Get member information
    pub fn get_member(&self, address: &Address) -> Option<&MemberInfo> {
        self.members.get(address)
    }

    /// Get all members
    pub fn get_members(&self) -> &HashMap<Address, MemberInfo> {
        &self.members
    }

    /// Update member reputation
    pub fn update_reputation(&mut self, member: &Address, reputation_change: i32) -> Result<()> {
        if let Some(member_info) = self.members.get_mut(member) {
            let new_reputation = (member_info.reputation as i32 + reputation_change).max(0) as u32;
            member_info.reputation = new_reputation;
            Ok(())
        } else {
            Err(BlockchainError::Governance("Member not found".to_string()))
        }
    }

    /// Get total voting power
    pub fn total_voting_power(&self) -> u64 {
        self.members.values().map(|m| m.voting_power).sum()
    }

    /// Check if an address is a member
    pub fn is_member(&self, address: &Address) -> bool {
        self.members.contains_key(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Address;

    fn create_test_address(id: u8) -> Address {
        Address::new(format!("0x{:040x}", id)).unwrap()
    }

    #[test]
    fn test_dao_creation() {
        let founder = create_test_address(1);
        let dao = DAOContract::new(
            "Test DAO".to_string(),
            "A test DAO".to_string(),
            None,
            20, // 20% quorum
            100, // 100 blocks voting period
            founder.clone(),
            1, // current block
        );

        assert_eq!(dao.name, "Test DAO");
        assert_eq!(dao.members.len(), 1);
        assert!(dao.is_member(&founder));
        assert_eq!(dao.total_voting_power(), 100);
    }

    #[test]
    fn test_dao_member_management() {
        let founder = create_test_address(1);
        let new_member = create_test_address(2);
        let mut dao = DAOContract::new(
            "Test DAO".to_string(),
            "A test DAO".to_string(),
            None,
            20,
            100,
            founder.clone(),
            1,
        );

        // Add new member
        dao.add_member(new_member.clone(), 50, 2).unwrap();
        assert!(dao.is_member(&new_member));
        assert_eq!(dao.total_voting_power(), 150);

        // Remove member
        dao.remove_member(&new_member).unwrap();
        assert!(!dao.is_member(&new_member));
        assert_eq!(dao.total_voting_power(), 100);
    }

    #[test]
    fn test_dao_proposal_creation() {
        let founder = create_test_address(1);
        let mut dao = DAOContract::new(
            "Test DAO".to_string(),
            "A test DAO".to_string(),
            None,
            20,
            100,
            founder.clone(),
            1,
        );

        // Create proposal
        let proposal_id = dao.create_proposal(
            &founder,
            "Test proposal".to_string(),
            ProposalType::General,
            1,
        ).unwrap();

        assert_eq!(proposal_id, 1);
        let proposal = dao.get_proposal(1).unwrap();
        assert_eq!(proposal.description, "Test proposal");
        assert_eq!(proposal.status, ProposalStatus::Active);
        assert_eq!(proposal.voting_ends_at, 101); // 1 + 100
    }

    #[test]
    fn test_dao_voting() {
        let founder = create_test_address(1);
        let member2 = create_test_address(2);
        let mut dao = DAOContract::new(
            "Test DAO".to_string(),
            "A test DAO".to_string(),
            None,
            20,
            100,
            founder.clone(),
            1,
        );

        // Add second member
        dao.add_member(member2.clone(), 50, 2).unwrap();

        // Create proposal
        let proposal_id = dao.create_proposal(
            &founder,
            "Test proposal".to_string(),
            ProposalType::General,
            2,
        ).unwrap();

        // Vote on proposal
        dao.vote(&founder, proposal_id, true, 3).unwrap(); // 100 votes for
        dao.vote(&member2, proposal_id, false, 4).unwrap(); // 50 votes against

        let proposal = dao.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.votes_for, 100);
        assert_eq!(proposal.votes_against, 50);
    }

    #[test]
    fn test_dao_proposal_execution() {
        let founder = create_test_address(1);
        let member2 = create_test_address(2);
        let mut dao = DAOContract::new(
            "Test DAO".to_string(),
            "A test DAO".to_string(),
            None,
            20, // 20% quorum (30 votes needed for 150 total)
            100,
            founder.clone(),
            1,
        );

        // Add second member
        dao.add_member(member2.clone(), 50, 2).unwrap();

        // Create proposal
        let proposal_id = dao.create_proposal(
            &founder,
            "Test proposal".to_string(),
            ProposalType::General,
            2,
        ).unwrap();

        // Vote - should meet quorum (150 total votes, 20% = 30, we have 150 votes)
        dao.vote(&founder, proposal_id, true, 3).unwrap();
        dao.vote(&member2, proposal_id, false, 4).unwrap();

        // Try to execute before voting period ends (should fail)
        let result = dao.execute_proposal(proposal_id, 50);
        assert!(result.is_err());

        // Execute after voting period (block 103 > 2 + 100 = 102)
        dao.execute_proposal(proposal_id, 103).unwrap();

        let proposal = dao.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Passed);
        assert_eq!(proposal.executed_at, Some(103));
    }

    #[test]
    fn test_dao_quorum_not_reached() {
        let founder = create_test_address(1);
        let member2 = create_test_address(2);
        let mut dao = DAOContract::new(
            "Test DAO".to_string(),
            "A test DAO".to_string(),
            None,
            80, // 80% quorum (120 votes needed)
            100,
            founder.clone(),
            1,
        );

        // Add second member
        dao.add_member(member2.clone(), 50, 2).unwrap();

        // Create proposal
        let proposal_id = dao.create_proposal(
            &founder,
            "Test proposal".to_string(),
            ProposalType::General,
            2,
        ).unwrap();

        // Only founder votes (100 votes, but need 120 for 80% quorum)
        dao.vote(&founder, proposal_id, true, 3).unwrap();

        // Execute after voting period
        let result = dao.execute_proposal(proposal_id, 103);
        assert!(result.is_err()); // Should fail due to quorum not reached

        let proposal = dao.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Rejected);
    }

    #[test]
    fn test_dao_reputation_update() {
        let founder = create_test_address(1);
        let mut dao = DAOContract::new(
            "Test DAO".to_string(),
            "A test DAO".to_string(),
            None,
            20,
            100,
            founder.clone(),
            1,
        );

        // Initial reputation
        assert_eq!(dao.get_member(&founder).unwrap().reputation, 100);

        // Update reputation
        dao.update_reputation(&founder, 25).unwrap();
        assert_eq!(dao.get_member(&founder).unwrap().reputation, 125);

        // Decrease reputation (but not below 0)
        dao.update_reputation(&founder, -200).unwrap();
        assert_eq!(dao.get_member(&founder).unwrap().reputation, 0);
    }
}
