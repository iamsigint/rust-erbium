//! Transaction Template System (Q1 2025 Implementation)
//! Inspired by PSBT but innovated for Erbium's multi-layer architecture

use crate::core::{Address, BlockchainError, Hash, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transaction Template State
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TemplateState {
    Draft,     // Initial proposal
    Reviewing, // Stakeholders reviewing
    Approved,  // Template approved
    Executing, // Being executed
    Complete,  // Transaction completed
}

/// Erbium Transaction Template (PSBT-inspired but innovative)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionTemplate {
    /// Unique template ID
    pub id: Hash,
    /// Template metadata
    pub metadata: TemplateMetadata,
    /// Proposer information
    pub proposer: TemplateParticipant,
    /// Required stakeholders
    pub required_participants: Vec<TemplateParticipant>,
    /// Current approvals
    pub approvals: Vec<TemplateApproval>,
    /// Template state
    pub state: TemplateState,
    /// Expiration timestamp
    pub expires_at: u64,
    /// Template version
    pub version: u32,

    /// Transaction data (multi-layer compatible)
    pub transaction_data: TransactionDataTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    pub title: String,
    pub description: String,
    pub category: String,
    pub priority: TemplatePriority,
    pub estimated_value: u64,
    pub required_approvals: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplatePriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParticipant {
    pub address: Address,
    pub role: ParticipantRole,
    pub permissions: Vec<Permission>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParticipantRole {
    Proposer,
    Reviewer,
    Approver,
    Executor,
    Observer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    CanEdit,
    CanApprove,
    CanExecute,
    CanView,
    CanComment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateApproval {
    pub participant: Address,
    pub role: ParticipantRole,
    pub timestamp: u64,
    pub signature: Vec<u8>, // Multi-signature support
    pub comments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDataTemplate {
    /// Core transaction parameters
    pub transfers: Vec<TransferTemplate>,
    pub contract_calls: Vec<ContractCallTemplate>,
    /// Cross-chain operations
    pub cross_chain_transfers: Vec<CrossChainTransferTemplate>,
    /// Layer-2 operations
    pub layer2_operations: Vec<Layer2OperationTemplate>,
    /// Advanced features
    pub conditional_executions: Vec<ConditionalExecutionTemplate>,
    pub ai_oracle_calls: Vec<AiOracleCallTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferTemplate {
    pub from: AddressConstraint,
    pub to: AddressConstraint,
    pub amount: AmountConstraint,
    pub token: Option<TokenConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCallTemplate {
    pub contract_address: Address,
    pub method: String,
    pub parameters: Vec<ParameterConstraint>,
    pub value: ValueConstraint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainTransferTemplate {
    pub source_chain: String,
    pub target_chain: String,
    pub amount: AmountConstraint,
    pub recipient: AddressConstraint,
    pub bridge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer2OperationTemplate {
    pub operation_type: Layer2OperationType,
    pub channel_id: Hash,
    pub participants: Vec<AddressConstraint>,
    pub parameters: HashMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Layer2OperationType {
    OpenChannel,
    UpdateChannel,
    CloseChannel,
    DisputeChannel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalExecutionTemplate {
    pub condition_type: ConditionType,
    pub parameters: HashMap<String, Vec<u8>>,
    pub execution_branch: Box<TransactionDataTemplate>,
    pub fallback_branch: Option<Box<TransactionDataTemplate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    PriceOracle,
    TimeBased,
    MultiSigApproval,
    AiPrediction,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiOracleCallTemplate {
    pub oracle_address: Address,
    pub request_type: String,
    pub parameters: HashMap<String, Vec<u8>>,
    pub callback_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressConstraint {
    pub allowed_addresses: Vec<Address>,
    pub excluded_addresses: Vec<Address>,
    pub role_requirement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmountConstraint {
    pub min_amount: Option<u64>,
    pub max_amount: Option<u64>,
    pub exact_amount: Option<u64>,
    pub percentage_of_total: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConstraint {
    pub token_address: Address,
    pub token_symbol: String,
    pub decimals: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueConstraint {
    Exact(u64),
    Range(u64, u64),
    Unconstrained,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterConstraint {
    pub name: String,
    pub constraint_type: ParameterType,
    pub values: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    Address(AddressConstraint),
    Amount(AmountConstraint),
    String(Vec<String>),
    Bytes(Vec<Vec<u8>>),
    Boolean,
    Custom(String),
}

impl TransactionTemplate {
    /// Create new transaction template
    pub fn new(title: String, description: String, proposer: TemplateParticipant) -> Self {
        let id = Hash::new(&format!("template_{}_{}", title, proposer.name).as_bytes());

        Self {
            id,
            metadata: TemplateMetadata {
                title,
                description,
                category: "General".to_string(),
                priority: TemplatePriority::Medium,
                estimated_value: 0,
                required_approvals: 1,
            },
            proposer,
            required_participants: Vec::new(),
            approvals: Vec::new(),
            state: TemplateState::Draft,
            expires_at: current_timestamp() + 30 * 24 * 60 * 60, // 30 days
            version: 1,
            transaction_data: TransactionDataTemplate {
                transfers: Vec::new(),
                contract_calls: Vec::new(),
                cross_chain_transfers: Vec::new(),
                layer2_operations: Vec::new(),
                conditional_executions: Vec::new(),
                ai_oracle_calls: Vec::new(),
            },
        }
    }

    /// Add required participant
    pub fn add_participant(&mut self, participant: TemplateParticipant) {
        self.required_participants.push(participant);
    }

    /// Approve template
    pub fn approve(
        &mut self,
        approver: Address,
        signature: Vec<u8>,
        comments: Option<String>,
    ) -> Result<()> {
        // Check if approver is authorized
        let participant = self
            .required_participants
            .iter()
            .find(|p| p.address == approver && p.permissions.contains(&Permission::CanApprove))
            .ok_or_else(|| BlockchainError::Validator("Unauthorized approver".to_string()))?;

        // Check if already approved
        if self.approvals.iter().any(|a| a.participant == approver) {
            return Err(BlockchainError::Validator("Already approved".to_string()));
        }

        // Add approval
        self.approvals.push(TemplateApproval {
            participant: approver,
            role: participant.role.clone(),
            timestamp: current_timestamp(),
            signature,
            comments,
        });

        // Check if all required approvals are met
        if self.approvals.len() >= self.metadata.required_approvals {
            self.state = TemplateState::Approved;
        }

        Ok(())
    }

    /// Execute approved template
    pub fn execute(&mut self, executor: Address) -> Result<()> {
        if self.state != TemplateState::Approved {
            return Err(BlockchainError::Validator(
                "Template not approved".to_string(),
            ));
        }

        let _participant = self
            .required_participants
            .iter()
            .find(|p| p.address == executor && p.permissions.contains(&Permission::CanExecute))
            .ok_or_else(|| BlockchainError::Validator("Unauthorized executor".to_string()))?;

        self.state = TemplateState::Executing;

        Ok(())
    }

    /// Complete template execution
    pub fn complete(&mut self) -> Result<()> {
        if self.state != TemplateState::Executing {
            return Err(BlockchainError::Validator(
                "Template not executing".to_string(),
            ));
        }

        self.state = TemplateState::Complete;
        Ok(())
    }

    /// Check if template is expired
    pub fn is_expired(&self) -> bool {
        current_timestamp() > self.expires_at
    }

    /// Validate template constraints
    pub fn validate_constraints(&self) -> Result<bool> {
        // Validate transfer constraints
        for transfer in &self.transaction_data.transfers {
            if !transfer.validate()? {
                return Ok(false);
            }
        }

        // Validate contract call constraints
        for call in &self.transaction_data.contract_calls {
            if !call.validate()? {
                return Ok(false);
            }
        }

        // Validate cross-chain operations
        for cc_transfer in &self.transaction_data.cross_chain_transfers {
            if !cc_transfer.validate()? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Calculate template complexity score (for gas estimation)
    pub fn complexity_score(&self) -> u64 {
        let mut score = 100; // Base score

        // Transfers
        score += self.transaction_data.transfers.len() as u64 * 50;

        // Contract calls
        score += self.transaction_data.contract_calls.len() as u64 * 100;

        // Cross-chain operations
        score += self.transaction_data.cross_chain_transfers.len() as u64 * 200;

        // Layer-2 operations
        score += self.transaction_data.layer2_operations.len() as u64 * 150;

        // AI oracle calls
        score += self.transaction_data.ai_oracle_calls.len() as u64 * 500;

        // Conditional executions
        score += self.transaction_data.conditional_executions.len() as u64 * 300;

        score
    }
}

impl TransferTemplate {
    fn validate(&self) -> Result<bool> {
        // Basic validation - addresses and amounts
        if self.from.allowed_addresses.is_empty() {
            return Ok(false);
        }

        if self.to.allowed_addresses.is_empty() {
            return Ok(false);
        }

        // Amount validation
        if let Some(exact) = self.amount.exact_amount {
            if exact == 0 {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl ContractCallTemplate {
    fn validate(&self) -> Result<bool> {
        // Validate contract address format
        if self.contract_address.as_str().is_empty() {
            return Ok(false);
        }

        // Validate method name
        if self.method.is_empty() {
            return Ok(false);
        }

        Ok(true)
    }
}

impl CrossChainTransferTemplate {
    fn validate(&self) -> Result<bool> {
        if self.source_chain.is_empty() || self.target_chain.is_empty() {
            return Ok(false);
        }

        if self.source_chain == self.target_chain {
            return Ok(false); // Must be cross-chain
        }

        Ok(true)
    }
}

/// Template Manager for collaborative transaction creation
pub struct TemplateManager {
    templates: HashMap<Hash, TransactionTemplate>,
    template_index: HashMap<Address, Vec<Hash>>, // Address -> Template IDs
}

impl TemplateManager {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
            template_index: HashMap::new(),
        }
    }

    /// Create new template
    pub fn create_template(
        &mut self,
        title: String,
        description: String,
        proposer: TemplateParticipant,
    ) -> Hash {
        let template = TransactionTemplate::new(title, description, proposer.clone());
        let id = template.id;

        let proposer_address = proposer.address.clone();
        self.templates.insert(id, template);
        self.template_index
            .entry(proposer_address)
            .or_insert(Vec::new())
            .push(id);

        id
    }

    /// Get template by ID
    pub fn get_template(&self, id: &Hash) -> Option<&TransactionTemplate> {
        self.templates.get(id)
    }

    /// Get templates by address
    pub fn get_templates_by_address(&self, address: &Address) -> Vec<&TransactionTemplate> {
        self.template_index
            .get(address)
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|id| self.templates.get(id))
            .collect()
    }

    /// Approve template
    pub fn approve_template(
        &mut self,
        template_id: &Hash,
        approver: Address,
        signature: Vec<u8>,
        comments: Option<String>,
    ) -> Result<()> {
        let template = self
            .templates
            .get_mut(template_id)
            .ok_or_else(|| BlockchainError::Validator("Template not found".to_string()))?;

        template.approve(approver, signature, comments)
    }

    /// Execute template
    pub fn execute_template(&mut self, template_id: &Hash, executor: Address) -> Result<()> {
        let template = self
            .templates
            .get_mut(template_id)
            .ok_or_else(|| BlockchainError::Validator("Template not found".to_string()))?;

        template.execute(executor)
    }

    /// Complete template
    pub fn complete_template(&mut self, template_id: &Hash) -> Result<()> {
        let template = self
            .templates
            .get_mut(template_id)
            .ok_or_else(|| BlockchainError::Validator("Template not found".to_string()))?;

        template.complete()
    }

    /// Clean expired templates
    pub fn clean_expired(&mut self) -> usize {
        let current_time = current_timestamp();
        let expired_ids: Vec<Hash> = self
            .templates
            .iter()
            .filter(|(_, template)| current_time > template.expires_at)
            .map(|(id, _)| *id)
            .collect();

        for id in &expired_ids {
            self.templates.remove(id);
            // Clean from index
            for templates in self.template_index.values_mut() {
                templates.retain(|template_id| template_id != id);
            }
        }

        expired_ids.len()
    }

    /// Get statistics
    pub fn statistics(&self) -> TemplateStatistics {
        let mut draft = 0;
        let mut approved = 0;
        let mut executing = 0;
        let mut complete = 0;

        for template in self.templates.values() {
            match template.state {
                TemplateState::Draft => draft += 1,
                TemplateState::Approved => approved += 1,
                TemplateState::Executing => executing += 1,
                TemplateState::Complete => complete += 1,
                _ => {}
            }
        }

        TemplateStatistics {
            total_templates: self.templates.len(),
            draft,
            approved,
            executing,
            complete,
            expired: 0, // TODO: Calculate expired count separately
        }
    }
}

/// Template Statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateStatistics {
    pub total_templates: usize,
    pub draft: usize,
    pub approved: usize,
    pub executing: usize,
    pub complete: usize,
    pub expired: usize,
}

/// Simple Transaction Template Manager stub for ErbiumEngine
#[derive(Debug)]
pub struct TransactionTemplateManager {
    templates: HashMap<Hash, TransactionTemplate>,
}

impl TransactionTemplateManager {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    pub fn get_template(&self, id: &Hash) -> Option<&TransactionTemplate> {
        self.templates.get(id)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn execute_template(&self, _template_id: &Hash, _executor: Address) -> Result<()> {
        // TODO: Implement template execution logic
        Ok(())
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

    #[test]
    fn test_template_creation() {
        let proposer = TemplateParticipant {
            address: Address::new_unchecked(
                "0x0000000000000000000000000000000000000001".to_string(),
            ),
            role: ParticipantRole::Proposer,
            permissions: vec![Permission::CanEdit, Permission::CanApprove],
            name: "Alice".to_string(),
        };

        let template = TransactionTemplate::new(
            "Test Transfer".to_string(),
            "Simple transfer template".to_string(),
            proposer,
        );

        assert_eq!(template.state, TemplateState::Draft);
        assert_eq!(template.metadata.title, "Test Transfer");
    }

    #[test]
    fn test_template_approval_workflow() {
        let proposer = TemplateParticipant {
            address: Address::new_unchecked(
                "0x0000000000000000000000000000000000000001".to_string(),
            ),
            role: ParticipantRole::Proposer,
            permissions: vec![Permission::CanEdit],
            name: "Alice".to_string(),
        };

        let approver = TemplateParticipant {
            address: Address::new_unchecked(
                "0x0000000000000000000000000000000000000002".to_string(),
            ),
            role: ParticipantRole::Approver,
            permissions: vec![Permission::CanApprove],
            name: "Bob".to_string(),
        };

        let mut template = TransactionTemplate::new(
            "Test Transfer".to_string(),
            "Simple transfer template".to_string(),
            proposer,
        );

        template.add_participant(approver.clone());
        template.metadata.required_approvals = 1;

        // Approve template
        template
            .approve(
                approver.address,
                vec![1, 2, 3],
                Some("Approved".to_string()),
            )
            .unwrap();

        assert_eq!(template.state, TemplateState::Approved);
        assert_eq!(template.approvals.len(), 1);
    }

    #[test]
    fn test_template_manager() {
        let proposer = TemplateParticipant {
            address: Address::new_unchecked(
                "0x0000000000000000000000000000000000000001".to_string(),
            ),
            role: ParticipantRole::Proposer,
            permissions: vec![Permission::CanEdit],
            name: "Alice".to_string(),
        };

        let mut manager = TemplateManager::new();

        let template_id = manager.create_template(
            "Test Template".to_string(),
            "Template for testing".to_string(),
            proposer.clone(),
        );

        let template = manager.get_template(&template_id).unwrap();
        assert_eq!(template.metadata.title, "Test Template");

        let user_templates = manager.get_templates_by_address(&proposer.address);
        assert_eq!(user_templates.len(), 1);
    }
}
