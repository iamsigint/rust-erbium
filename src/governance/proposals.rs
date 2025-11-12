use crate::core::types::Address;
use crate::utils::error::{Result, BlockchainError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub creator: Address,
    pub title: String,
    pub description: String,
    pub actions: Vec<ProposalAction>,
    pub created_at: u64,
    pub status: ProposalStatus,
    pub executed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    Pending,
    Active,
    Succeeded,
    Defeated,
    Executed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProposalManager {
    proposals: HashMap<u64, Proposal>,
    next_proposal_id: u64,
}

impl ProposalManager {
    pub fn new() -> Self {
        Self {
            proposals: HashMap::new(),
            next_proposal_id: 1,
        }
    }
    
    pub fn create_proposal(
        &mut self,
        creator: &Address,
        title: String,
        description: String,
        actions: Vec<ProposalAction>,
    ) -> Result<u64> {
        let id = self.next_proposal_id;
        
        let proposal = Proposal {
            id,
            creator: creator.clone(),
            title,
            description,
            actions,
            created_at: chrono::Utc::now().timestamp_millis() as u64,
            status: ProposalStatus::Pending,
            executed_at: None,
        };
        
        self.proposals.insert(id, proposal);
        self.next_proposal_id += 1;
        
        Ok(id)
    }
    
    pub fn add_proposal(
        &mut self,
        id: u64,
        creator: &Address,
        actions: Vec<ProposalAction>,
    ) -> Result<()> {
        if self.proposals.contains_key(&id) {
            return Err(BlockchainError::Governance("Proposal ID already exists".to_string()));
        }
        
        let proposal = Proposal {
            id,
            creator: creator.clone(),
            title: format!("Proposal {}", id),
            description: format!("Description for proposal {}", id),
            actions,
            created_at: chrono::Utc::now().timestamp_millis() as u64,
            status: ProposalStatus::Pending,
            executed_at: None,
        };
        
        self.proposals.insert(id, proposal);
        self.next_proposal_id = self.next_proposal_id.max(id + 1);
        
        Ok(())
    }
    
    pub fn update_proposal_status(&mut self, id: u64, status: ProposalStatus) -> Result<()> {
        let proposal = self.proposals.get_mut(&id)
            .ok_or_else(|| BlockchainError::Governance("Proposal not found".to_string()))?;
        
        let status_clone = status.clone();
        proposal.status = status;
        
        if status_clone == ProposalStatus::Executed {
            proposal.executed_at = Some(chrono::Utc::now().timestamp_millis() as u64);
        }
        
        Ok(())
    }
    
    pub fn mark_executed(&mut self, id: u64) -> Result<()> {
        self.update_proposal_status(id, ProposalStatus::Executed)
    }
    
    pub fn get_proposal(&self, id: u64) -> Option<&Proposal> {
        self.proposals.get(&id)
    }
    
    pub fn get_proposal_mut(&mut self, id: u64) -> Option<&mut Proposal> {
        self.proposals.get_mut(&id)
    }
    
    pub fn get_proposal_actions(&self, id: u64) -> Result<Option<Vec<ProposalAction>>> {
        Ok(self.proposals.get(&id).map(|p| p.actions.clone()))
    }
    
    pub fn get_proposal_status(&self, id: u64) -> Result<Option<ProposalStatus>> {
        Ok(self.proposals.get(&id).map(|p| p.status.clone()))
    }
    
    pub fn get_all_proposals(&self) -> Vec<&Proposal> {
        self.proposals.values().collect()
    }
    
    pub fn get_active_proposals(&self) -> Vec<&Proposal> {
        self.proposals.values()
            .filter(|p| matches!(p.status, ProposalStatus::Pending | ProposalStatus::Active))
            .collect()
    }
    
    pub fn get_proposals_by_creator(&self, creator: &Address) -> Vec<&Proposal> {
        self.proposals.values()
            .filter(|p| &p.creator == creator)
            .collect()
    }
    
    pub fn get_next_proposal_id(&self) -> u64 {
        self.next_proposal_id
    }
    
    pub fn cleanup_old_proposals(&mut self, older_than: u64) -> Result<usize> {
        let current_time = chrono::Utc::now().timestamp_millis() as u64;
        let mut removed_count = 0;
        
        self.proposals.retain(|_, proposal| {
            let should_retain = current_time - proposal.created_at < older_than;
            if !should_retain {
                removed_count += 1;
            }
            should_retain
        });
        
        Ok(removed_count)
    }
}

// Add missing types that are referenced
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalAction {
    TransferFunds { to: Address, amount: u64 },
    UpdateParameter { key: String, value: String },
    AddMember { address: Address, tokens: u64 },
    UpgradeContract { new_code: Vec<u8> },
}