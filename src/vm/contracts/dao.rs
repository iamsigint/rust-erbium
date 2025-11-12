use crate::core::types::{Address, Hash};
use crate::utils::error::{Result, BlockchainError};
use crate::vm::storage::ContractStorage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub creator: Address,
    pub title: String,
    pub description: String,
    pub voting_start: u64,
    pub voting_end: u64,
    pub for_votes: u64,
    pub against_votes: u64,
    pub executed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAOContract {
    pub address: Address,
    pub name: String,
    pub token_address: Address,
    pub voting_delay: u64,
    pub voting_period: u64,
    pub proposal_threshold: u64,
    pub quorum_votes: u64,
    storage: ContractStorage,
    proposals: HashMap<u64, Proposal>,
    next_proposal_id: u64,
    votes: HashMap<(u64, Address), bool>, // (proposal_id, voter) -> vote (true = for, false = against)
}

impl DAOContract {
    pub fn new(
        address: Address,
        name: String,
        token_address: Address,
        voting_delay: u64,
        voting_period: u64,
        proposal_threshold: u64,
        quorum_votes: u64,
    ) -> Result<Self> {
        let mut contract = Self {
            address,
            name,
            token_address,
            voting_delay,
            voting_period,
            proposal_threshold,
            quorum_votes,
            storage: ContractStorage::new()?,
            proposals: HashMap::new(),
            next_proposal_id: 1,
            votes: HashMap::new(),
        };
        
        contract.storage.initialize_contract(&contract.address)?;
        Ok(contract)
    }
    
    pub fn create_proposal(
        &mut self,
        creator: &Address,
        title: String,
        description: String,
        token_balance: u64,
    ) -> Result<u64> {
        if token_balance < self.proposal_threshold {
            return Err(BlockchainError::VM("Insufficient tokens to create proposal".to_string()));
        }
        
        let proposal_id = self.next_proposal_id;
        let current_time = chrono::Utc::now().timestamp_millis() as u64;
        
        let proposal = Proposal {
            id: proposal_id,
            creator: creator.clone(),
            title,
            description,
            voting_start: current_time + self.voting_delay,
            voting_end: current_time + self.voting_delay + self.voting_period,
            for_votes: 0,
            against_votes: 0,
            executed: false,
        };
        
        self.proposals.insert(proposal_id, proposal);
        self.next_proposal_id += 1;
        
        self.store_proposal(proposal_id)?;
        self.emit_proposal_created_event(proposal_id, creator)?;
        
        Ok(proposal_id)
    }
    
    pub fn vote(
        &mut self,
        proposal_id: u64,
        voter: &Address,
        support: bool,
        voting_power: u64,
    ) -> Result<bool> {
        let proposal = self.proposals.get_mut(&proposal_id)
            .ok_or_else(|| BlockchainError::VM("Proposal not found".to_string()))?;
        
        let current_time = chrono::Utc::now().timestamp_millis() as u64;
        
        if current_time < proposal.voting_start || current_time > proposal.voting_end {
            return Err(BlockchainError::VM("Voting period not active".to_string()));
        }
        
        if self.votes.contains_key(&(proposal_id, voter.clone())) {
            return Err(BlockchainError::VM("Already voted".to_string()));
        }
        
        self.votes.insert((proposal_id, voter.clone()), support);
        
        if support {
            proposal.for_votes += voting_power;
        } else {
            proposal.against_votes += voting_power;
        }
        
        self.store_proposal(proposal_id)?;
        self.store_vote(proposal_id, voter, support, voting_power)?;
        self.emit_vote_cast_event(proposal_id, voter, support, voting_power)?;
        
        Ok(true)
    }
    
    pub fn execute_proposal(&mut self, proposal_id: u64) -> Result<bool> {
        let proposal = self.proposals.get_mut(&proposal_id)
            .ok_or_else(|| BlockchainError::VM("Proposal not found".to_string()))?;
        
        if proposal.executed {
            return Err(BlockchainError::VM("Proposal already executed".to_string()));
        }
        
        let current_time = chrono::Utc::now().timestamp_millis() as u64;
        
        if current_time <= proposal.voting_end {
            return Err(BlockchainError::VM("Voting period not ended".to_string()));
        }
        
        let total_votes = proposal.for_votes + proposal.against_votes;
        if total_votes < self.quorum_votes {
            return Err(BlockchainError::VM("Quorum not reached".to_string()));
        }
        
        if proposal.for_votes <= proposal.against_votes {
            return Err(BlockchainError::VM("Proposal did not pass".to_string()));
        }
        
        proposal.executed = true;
        
        self.store_proposal(proposal_id)?;
        self.emit_proposal_executed_event(proposal_id)?;
        
        Ok(true)
    }
    
    pub fn get_proposal(&self, proposal_id: u64) -> Result<Option<Proposal>> {
        Ok(self.proposals.get(&proposal_id).cloned())
    }
    
    pub fn get_vote(&self, proposal_id: u64, voter: &Address) -> Result<Option<bool>> {
        Ok(self.votes.get(&(proposal_id, voter.clone())).copied())
    }
    
    pub fn get_proposals_count(&self) -> u64 {
        self.next_proposal_id - 1
    }
    
    fn store_proposal(&mut self, proposal_id: u64) -> Result<()> {
        let proposal = self.proposals.get(&proposal_id)
            .ok_or_else(|| BlockchainError::VM("Proposal not found".to_string()))?;
        
        let proposal_data = bincode::serialize(proposal)
            .map_err(|e| BlockchainError::Serialization(e.to_string()))?;
        
        let key = Self::get_proposal_key(proposal_id);
        self.storage.store(&self.address, key, proposal_data)?;
        
        Ok(())
    }
    
    fn store_vote(
        &mut self,
        proposal_id: u64,
        voter: &Address,
        support: bool,
        voting_power: u64,
    ) -> Result<()> {
        let vote_data = (support, voting_power);
        let serialized_vote = bincode::serialize(&vote_data)
            .map_err(|e| BlockchainError::Serialization(e.to_string()))?;
        
        let key = Self::get_vote_key(proposal_id, voter);
        self.storage.store(&self.address, key, serialized_vote)?;
        
        Ok(())
    }
    
    fn emit_proposal_created_event(&self, proposal_id: u64, creator: &Address) -> Result<()> {
        log::debug!("DAO Proposal Created: {} by {}", proposal_id, creator.as_str());
        Ok(())
    }
    
    fn emit_vote_cast_event(
        &self,
        proposal_id: u64,
        voter: &Address,
        support: bool,
        voting_power: u64,
    ) -> Result<()> {
        log::debug!("DAO Vote Cast: {} voted {} with {} power on proposal {}", 
                   voter.as_str(), if support { "FOR" } else { "AGAINST" }, voting_power, proposal_id);
        Ok(())
    }
    
    fn emit_proposal_executed_event(&self, proposal_id: u64) -> Result<()> {
        log::debug!("DAO Proposal Executed: {}", proposal_id);
        Ok(())
    }
    
    fn get_proposal_key(proposal_id: u64) -> Vec<u8> {
        let mut key = b"proposal_".to_vec();
        key.extend_from_slice(&proposal_id.to_be_bytes());
        key
    }
    
    fn get_vote_key(proposal_id: u64, voter: &Address) -> Vec<u8> {
        let mut key = b"vote_".to_vec();
        key.extend_from_slice(&proposal_id.to_be_bytes());
        key.extend_from_slice(b"_");
        key.extend_from_slice(voter.as_str().as_bytes());
        key
    }
    
    pub fn get_storage_root(&self) -> Result<Hash> {
        self.storage.calculate_storage_root(&self.address)
    }
}

impl crate::vm::SmartContract for DAOContract {
    fn execute(&mut self, function: &str, args: &[u8]) -> Result<Vec<u8>> {
        match function {
            "createProposal" => {
                if args.len() < 40 {
                    return Err(BlockchainError::VM("Invalid createProposal arguments".to_string()));
                }
                let title_len = args[0] as usize;
                let description_len = args[1] as usize;
                
                if args.len() < 2 + title_len + description_len + 20 {
                    return Err(BlockchainError::VM("Invalid proposal data".to_string()));
                }
                
                let title = String::from_utf8_lossy(&args[2..2 + title_len]).to_string();
                let description = String::from_utf8_lossy(&args[2 + title_len..2 + title_len + description_len]).to_string();
                
                let creator_bytes = &args[2 + title_len + description_len..2 + title_len + description_len + 20];
                let creator_str = String::from_utf8_lossy(creator_bytes).to_string();
                let creator = Address::new(creator_str).map_err(BlockchainError::Address)?;
                
                let token_balance = u64::from_be_bytes(args[2 + title_len + description_len + 20..2 + title_len + description_len + 28].try_into().unwrap());
                
                let proposal_id = self.create_proposal(&creator, title, description, token_balance)?;
                Ok(proposal_id.to_be_bytes().to_vec())
            }
            "vote" => {
                if args.len() < 29 {
                    return Err(BlockchainError::VM("Invalid vote arguments".to_string()));
                }
                let proposal_id = u64::from_be_bytes(args[0..8].try_into().unwrap());
                let support = args[8] != 0;
                
                let voter_bytes = &args[9..29];
                let voter_str = String::from_utf8_lossy(voter_bytes).to_string();
                let voter = Address::new(voter_str).map_err(BlockchainError::Address)?;
                
                let voting_power = u64::from_be_bytes(args[29..37].try_into().unwrap());
                
                let success = self.vote(proposal_id, &voter, support, voting_power)?;
                Ok(vec![if success { 1 } else { 0 }])
            }
            "getProposal" => {
                if args.len() < 8 {
                    return Err(BlockchainError::VM("Invalid getProposal arguments".to_string()));
                }
                let proposal_id = u64::from_be_bytes(args[0..8].try_into().unwrap());
                
                if let Some(proposal) = self.get_proposal(proposal_id)? {
                    let proposal_data = bincode::serialize(&proposal)
                        .map_err(|e| BlockchainError::Serialization(e.to_string()))?;
                    Ok(proposal_data)
                } else {
                    Ok(vec![])
                }
            }
            _ => Err(BlockchainError::VM(format!("Unknown function: {}", function))),
        }
    }
    
    fn get_storage_root(&self) -> Result<Hash> {
        self.get_storage_root()
    }
    
    fn get_address(&self) -> &Address {
        &self.address
    }
}