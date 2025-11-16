// src/governance/dao.rs

use super::{
    GovernanceConfig, ProposalAction, ProposalManager, ProposalStatus, Treasury, VotingSystem,
};
use crate::core::types::Address;
use crate::utils::error::{BlockchainError, Result};
use crate::vm::contracts::{DAOContract, ERC20Contract};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAO {
    pub address: Address,
    pub name: String,
    pub token: ERC20Contract,
    pub governance: DAOContract,
    pub voting_system: VotingSystem,
    pub treasury: Treasury,
    pub proposal_manager: ProposalManager,
    pub members: HashMap<Address, Member>,
    pub config: GovernanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub address: Address,
    pub joined_at: u64,
    pub voting_power: u64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAOInfo {
    pub address: Address,
    pub name: String,
    pub total_members: usize,
    pub active_members: usize,
    pub total_supply: u64,
    pub proposal_threshold: u64,
    pub treasury_balance: u64,
}

impl DAO {
    pub fn new(
        name: String,
        token_name: String,
        token_symbol: String,
        initial_supply: u64,
        config: GovernanceConfig,
    ) -> Result<Self> {
        let name_bytes = name.as_bytes();
        let truncated = &name_bytes[..std::cmp::min(name_bytes.len(), 20)];
        let dao_address_str = format!("0x{}", hex::encode(truncated));
        let dao_address = Address::new(dao_address_str)?;

        let founder = dao_address.clone(); // Use DAO address as founder for now
        let token = ERC20Contract::new(
            token_name,
            token_symbol,
            18,
            initial_supply,
            founder.clone(),
        );

        let governance = DAOContract::new(
            name.clone(),
            format!("DAO governance for {}", name),
            Some(dao_address.clone()), // governance token
            20,                        // quorum percentage
            100,                       // voting period in blocks
            founder,
            0, // current block
        );

        let voting_system = VotingSystem::new(dao_address.clone());
        let treasury = Treasury::new(dao_address.clone())?;
        let proposal_manager = ProposalManager::new();

        let dao = Self {
            address: dao_address,
            name,
            token,
            governance,
            voting_system,
            treasury,
            proposal_manager,
            members: HashMap::new(),
            config,
        };

        Ok(dao)
    }

    pub fn create_proposal(
        &mut self,
        creator: &Address,
        title: String,
        description: String,
        actions: Vec<ProposalAction>,
    ) -> Result<u64> {
        let voting_power = self.get_member_voting_power(creator)?;

        if voting_power < self.config.proposal_threshold {
            return Err(BlockchainError::Governance(format!(
                "Insufficient voting power ({}) to create proposal (threshold: {})",
                voting_power, self.config.proposal_threshold
            )));
        }

        let proposal_id = self.proposal_manager.create_proposal(
            creator,
            title.clone(),
            description.clone(),
            actions,
        )?;
        let _gov_proposal_id = self
            .governance
            .create_proposal(
                creator,
                format!("{}: {}", title, description),
                crate::vm::dao::ProposalType::General,
                0, // current block
            )
            .unwrap_or(0);
        self.voting_system.initialize_voting_session(proposal_id)?;

        Ok(proposal_id)
    }

    pub fn vote_on_proposal(
        &mut self,
        proposal_id: u64,
        voter: &Address,
        support: bool,
    ) -> Result<bool> {
        let voting_power = self.get_member_voting_power(voter)?;

        if voting_power == 0 {
            return Err(BlockchainError::Governance(
                "Voter has no voting power".to_string(),
            ));
        }

        self.governance.vote(voter, proposal_id, support, 0)?;

        // Record the vote in our voting system
        self.voting_system
            .record_vote(proposal_id, voter, support, voting_power)?;

        Ok(true)
    }

    pub fn execute_proposal(&mut self, proposal_id: u64, _executor: &Address) -> Result<bool> {
        let status = self
            .get_proposal_status(proposal_id)?
            .ok_or_else(|| BlockchainError::Governance("Proposal not found".to_string()))?;

        match status {
            ProposalStatus::Succeeded | ProposalStatus::Active => { /* continue */ }
            _ => {
                return Err(BlockchainError::Governance(format!(
                    "Proposal cannot be executed in state: {:?}",
                    status
                )))
            }
        }

        self.governance.execute_proposal(proposal_id, 0)?;

        // Execute proposal actions
        self.execute_proposal_actions(proposal_id)?;
        self.proposal_manager.mark_executed(proposal_id)?;

        Ok(true)
    }

    pub fn _internal_add_member(
        &mut self,
        new_member_addr: Address,
        initial_tokens: u64,
    ) -> Result<()> {
        if self.members.contains_key(&new_member_addr) {
            return Err(BlockchainError::Governance(
                "Member already exists".to_string(),
            ));
        }

        // Mock treasury transfer - not implemented
        log::info!(
            "Mock treasury transfer: {} tokens to {}",
            initial_tokens,
            new_member_addr
        );

        let member = Member {
            address: new_member_addr.clone(),
            joined_at: chrono::Utc::now().timestamp_millis() as u64,
            voting_power: initial_tokens,
            is_active: true,
        };
        self.members.insert(new_member_addr, member);
        Ok(())
    }

    pub fn _internal_remove_member(&mut self, member_address: &Address) -> Result<()> {
        if let Some(member) = self.members.get_mut(member_address) {
            member.is_active = false;
            member.voting_power = 0;
            Ok(())
        } else {
            Err(BlockchainError::Governance("Member not found".to_string()))
        }
    }

    pub fn get_proposal_status(&self, proposal_id: u64) -> Result<Option<ProposalStatus>> {
        self.proposal_manager.get_proposal_status(proposal_id)
    }

    pub fn get_member_info(&self, address: &Address) -> Option<&Member> {
        self.members.get(address)
    }

    pub fn get_total_members(&self) -> usize {
        self.members.len()
    }

    pub fn get_active_members(&self) -> Vec<&Member> {
        self.members.values().filter(|m| m.is_active).collect()
    }

    pub fn get_member_voting_power(&self, address: &Address) -> Result<u64> {
        Ok(self.token.balance_of(address))
    }

    pub fn get_dao_info(&self) -> Result<DAOInfo> {
        Ok(DAOInfo {
            address: self.address.clone(),
            name: self.name.clone(),
            total_members: self.get_total_members(),
            active_members: self.get_active_members().len(),
            total_supply: self.token.total_supply, // Field, not method
            proposal_threshold: self.config.proposal_threshold,
            treasury_balance: 0, // Mock treasury balance
        })
    }

    pub fn get_voting_results(&self, proposal_id: u64) -> Option<(u64, u64, usize)> {
        self.voting_system.get_voting_results(proposal_id)
    }

    pub fn get_treasury_balance(&self) -> Result<u64> {
        Ok(0) // Mock treasury balance
    }

    fn execute_proposal_actions(&mut self, proposal_id: u64) -> Result<()> {
        if let Some(actions) = self.proposal_manager.get_proposal_actions(proposal_id)? {
            for action in actions {
                log::info!(
                    "Executing action for proposal {}: {:?}",
                    proposal_id,
                    action
                );
                match action {
                    ProposalAction::TransferFunds { to, amount } => {
                        // Mock treasury transfer
                        log::info!("Mock treasury transfer: {} to {}", amount, to);
                    }
                    ProposalAction::UpdateParameter { key, value } => {
                        self.update_governance_parameter(&key, value)?;
                    }
                    ProposalAction::AddMember { address, tokens } => {
                        self._internal_add_member(address, tokens)?;
                    }
                    ProposalAction::UpgradeContract { new_code: _ } => {
                        log::warn!(
                            "Proposal {}: Contract upgrade action not implemented",
                            proposal_id
                        );
                    }
                }
            }
        } else {
            log::warn!("No actions found for proposal {}", proposal_id);
        }

        Ok(())
    }

    fn update_governance_parameter(&mut self, key: &str, value_str: String) -> Result<()> {
        match key {
            "voting_delay" => {
                let value: u64 = value_str.parse().map_err(|e| {
                    BlockchainError::Governance(format!(
                        "Invalid voting delay value '{}': {}",
                        value_str, e
                    ))
                })?;
                self.config.voting_delay = value;
            }
            "voting_period" => {
                let value: u64 = value_str.parse().map_err(|e| {
                    BlockchainError::Governance(format!(
                        "Invalid voting period value '{}': {}",
                        value_str, e
                    ))
                })?;
                self.config.voting_period = value;
            }
            "proposal_threshold" => {
                let value: u64 = value_str.parse().map_err(|e| {
                    BlockchainError::Governance(format!(
                        "Invalid proposal threshold value '{}': {}",
                        value_str, e
                    ))
                })?;
                self.config.proposal_threshold = value;
            }
            "quorum_votes" => {
                let value: u64 = value_str.parse().map_err(|e| {
                    BlockchainError::Governance(format!(
                        "Invalid quorum votes value '{}': {}",
                        value_str, e
                    ))
                })?;
                self.config.quorum_votes = value;
            }
            "timelock_delay" => {
                let value: u64 = value_str.parse().map_err(|e| {
                    BlockchainError::Governance(format!(
                        "Invalid timelock delay value '{}': {}",
                        value_str, e
                    ))
                })?;
                self.config.timelock_delay = value;
            }
            _ => {
                return Err(BlockchainError::Governance(format!(
                    "Unknown governance parameter key: {}",
                    key
                )))
            }
        }
        log::info!("Updated governance parameter '{}' to '{}'", key, value_str);
        Ok(())
    }
}
