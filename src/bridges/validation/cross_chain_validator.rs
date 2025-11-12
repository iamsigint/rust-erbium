//! Cross-Chain Transaction Validation
//!
//! This module implements validation of cross-chain transactions,
//! ensuring consistency and security across different blockchain networks.

use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cross-chain transaction validator
pub struct CrossChainValidator {
    /// Known bridge contracts and their configurations
    bridge_configs: Arc<RwLock<HashMap<String, BridgeConfig>>>,
    /// Transaction validation cache
    validation_cache: Arc<RwLock<HashMap<String, ValidationResult>>>,
    /// Validation rules
    validation_rules: ValidationRules,
}

/// Bridge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub bridge_id: String,
    pub source_chain: String,
    pub target_chain: String,
    pub contract_address: String,
    pub min_amount: u128,
    pub max_amount: u128,
    pub daily_limit: u128,
    pub paused: bool,
}

/// Validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
    pub require_multisig: bool,
    pub min_confirmations: u32,
    pub max_transaction_age: u64, // seconds
    pub allowed_tokens: Vec<String>,
    pub blocked_addresses: Vec<String>,
}

/// Cross-chain transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainTransaction {
    pub tx_hash: String,
    pub bridge_id: String,
    pub source_chain: String,
    pub target_chain: String,
    pub sender: String,
    pub recipient: String,
    pub token: String,
    pub amount: u128,
    pub fee: u128,
    pub nonce: u64,
    pub timestamp: u64,
    pub signatures: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub tx_hash: String,
    pub is_valid: bool,
    pub confidence_score: f64, // 0.0 to 1.0
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub timestamp: u64,
    pub validator_version: String,
}

/// Validation error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    InvalidAmount,
    InsufficientConfirmations,
    InvalidSignature,
    BlockedAddress,
    UnsupportedToken,
    TransactionTooOld,
    BridgePaused,
    DailyLimitExceeded,
    InvalidNonce,
    DuplicateTransaction,
}

/// Validation metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    pub total_validated: u64,
    pub total_rejected: u64,
    pub average_confidence: f64,
    pub validation_time_avg: f64,
    pub error_breakdown: HashMap<String, u64>,
}

impl CrossChainValidator {
    /// Create a new cross-chain validator
    pub fn new(validation_rules: ValidationRules) -> Self {
        Self {
            bridge_configs: Arc::new(RwLock::new(HashMap::new())),
            validation_cache: Arc::new(RwLock::new(HashMap::new())),
            validation_rules,
        }
    }

    /// Add or update bridge configuration
    pub async fn update_bridge_config(&self, config: BridgeConfig) -> Result<()> {
        let mut configs = self.bridge_configs.write().await;
        configs.insert(config.bridge_id.clone(), config);
        Ok(())
    }

    /// Validate a cross-chain transaction
    pub async fn validate_transaction(
        &self,
        transaction: &CrossChainTransaction,
    ) -> Result<ValidationResult> {
        let start_time = std::time::Instant::now();

        // Check cache first
        let cache_key = format!("{}_{}", transaction.tx_hash, transaction.bridge_id);
        if let Some(cached_result) = self.validation_cache.read().await.get(&cache_key) {
            // Return cached result if it's recent (within 5 minutes)
            if current_timestamp() - cached_result.timestamp < 300 {
                return Ok(cached_result.clone());
            }
        }

        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut confidence_score = 1.0;

        // 1. Basic validation
        if let Err(e) = self.validate_basic_properties(transaction).await {
            errors.push(format!("Basic validation failed: {:?}", e));
            confidence_score *= 0.1;
        }

        // 2. Bridge-specific validation
        if let Err(e) = self.validate_bridge_rules(transaction).await {
            errors.push(format!("Bridge validation failed: {:?}", e));
            confidence_score *= 0.3;
        }

        // 3. Security validation
        if let Err(e) = self.validate_security(transaction).await {
            errors.push(format!("Security validation failed: {:?}", e));
            confidence_score *= 0.5;
        }

        // 4. Cross-chain consistency validation
        if let Err(e) = self.validate_cross_chain_consistency(transaction).await {
            errors.push(format!("Cross-chain validation failed: {:?}", e));
            confidence_score *= 0.7;
        }

        // 5. Signature validation
        if self.validation_rules.require_multisig {
            match self.validate_signatures(transaction).await {
                Ok(_) => {
                    // Signatures are valid, confidence remains high
                }
                Err(e) => {
                    errors.push(format!("Signature validation failed: {:?}", e));
                    confidence_score *= 0.2;
                }
            }
        }

        // 6. Amount and limit validation
        if let Err(e) = self.validate_amounts(transaction).await {
            errors.push(format!("Amount validation failed: {:?}", e));
            confidence_score *= 0.8;
        }

        // 7. Generate warnings for potential issues
        self.generate_warnings(transaction, &mut warnings).await;

        let is_valid = errors.is_empty();
        let validation_time = start_time.elapsed().as_millis() as f64;

        let result = ValidationResult {
            tx_hash: transaction.tx_hash.clone(),
            is_valid,
            confidence_score,
            errors,
            warnings,
            timestamp: current_timestamp(),
            validator_version: env!("CARGO_PKG_VERSION").to_string(),
        };

        // Cache the result
        let mut cache = self.validation_cache.write().await;
        cache.insert(cache_key, result.clone());

        log::info!(
            "Validated cross-chain transaction {}: valid={}, confidence={:.3}, time={:.2}ms",
            transaction.tx_hash,
            is_valid,
            confidence_score,
            validation_time
        );

        Ok(result)
    }

    /// Validate basic transaction properties
    async fn validate_basic_properties(&self, tx: &CrossChainTransaction) -> Result<()> {
        // Check transaction age
        let age = current_timestamp() - tx.timestamp;
        if age > self.validation_rules.max_transaction_age {
            return Err(BlockchainError::Bridge("Transaction too old".to_string()));
        }

        // Check for blocked addresses
        if self.validation_rules.blocked_addresses.contains(&tx.sender) ||
           self.validation_rules.blocked_addresses.contains(&tx.recipient) {
            return Err(BlockchainError::Bridge("Blocked address".to_string()));
        }

        // Check supported tokens
        if !self.validation_rules.allowed_tokens.is_empty() &&
           !self.validation_rules.allowed_tokens.contains(&tx.token) {
            return Err(BlockchainError::Bridge("Unsupported token".to_string()));
        }

        Ok(())
    }

    /// Validate bridge-specific rules
    async fn validate_bridge_rules(&self, tx: &CrossChainTransaction) -> Result<()> {
        let configs = self.bridge_configs.read().await;

        if let Some(config) = configs.get(&tx.bridge_id) {
            // Check if bridge is paused
            if config.paused {
                return Err(BlockchainError::Bridge("Bridge is paused".to_string()));
            }

            // Check amount limits
            if tx.amount < config.min_amount || tx.amount > config.max_amount {
                return Err(BlockchainError::Bridge("Amount out of bridge limits".to_string()));
            }

            // Check daily limits (simplified - would need persistent storage)
            // This is a placeholder for actual daily limit checking
        } else {
            return Err(BlockchainError::Bridge("Unknown bridge".to_string()));
        }

        Ok(())
    }

    /// Validate security properties
    async fn validate_security(&self, tx: &CrossChainTransaction) -> Result<()> {
        // Check for duplicate transactions (simplified)
        // In production, this would check a persistent store
        let cache = self.validation_cache.read().await;
        let duplicate_count = cache.values()
            .filter(|result| result.tx_hash != tx.tx_hash && result.is_valid)
            .filter(|result| {
                // Check if another valid transaction has same nonce from same sender
                // This is simplified - actual implementation would be more sophisticated
                false // Placeholder
            })
            .count();

        if duplicate_count > 0 {
            return Err(BlockchainError::Bridge("Potential duplicate transaction".to_string()));
        }

        Ok(())
    }

    /// Validate cross-chain consistency
    async fn validate_cross_chain_consistency(&self, tx: &CrossChainTransaction) -> Result<()> {
        // Validate that source and target chains are compatible
        if tx.source_chain == tx.target_chain {
            return Err(BlockchainError::Bridge("Source and target chains must be different".to_string()));
        }

        // Validate chain-specific rules
        match tx.source_chain.as_str() {
            "bitcoin" => {
                // Bitcoin-specific validation
                if tx.amount < 1000 { // 1000 sats minimum
                    return Err(BlockchainError::Bridge("Amount too small for Bitcoin".to_string()));
                }
            }
            "ethereum" => {
                // Ethereum-specific validation
                if tx.token == "ETH" && tx.amount < 1000000000000000 { // 0.001 ETH
                    return Err(BlockchainError::Bridge("Amount too small for Ethereum".to_string()));
                }
            }
            _ => {
                // Generic validation
            }
        }

        Ok(())
    }

    /// Validate multisig signatures
    async fn validate_signatures(&self, tx: &CrossChainTransaction) -> Result<()> {
        // Simplified signature validation
        // In production, this would verify cryptographic signatures

        if tx.signatures.len() < 2 {
            return Err(BlockchainError::Bridge("Insufficient signatures".to_string()));
        }

        // Check for duplicate signatures
        let mut unique_sigs = std::collections::HashSet::new();
        for sig in &tx.signatures {
            if !unique_sigs.insert(sig) {
                return Err(BlockchainError::Bridge("Duplicate signature".to_string()));
            }
        }

        Ok(())
    }

    /// Validate amounts and limits
    async fn validate_amounts(&self, tx: &CrossChainTransaction) -> Result<()> {
        if tx.amount == 0 {
            return Err(BlockchainError::Bridge("Zero amount not allowed".to_string()));
        }

        // Check fee is reasonable (not more than 10% of amount)
        if tx.fee > tx.amount / 10 {
            return Err(BlockchainError::Bridge("Fee too high".to_string()));
        }

        Ok(())
    }

    /// Generate warnings for potential issues
    async fn generate_warnings(&self, tx: &CrossChainTransaction, warnings: &mut Vec<String>) {
        // Warning for high-value transactions
        if tx.amount > 1000000000000000000 { // 1000 ETH or equivalent
            warnings.push("High-value transaction".to_string());
        }

        // Warning for new recipients
        // This would check if recipient has received transactions before

        // Warning for unusual timing
        let hour = (tx.timestamp % 86400) / 3600;
        if hour < 6 || hour > 22 { // Outside 6 AM - 10 PM
            warnings.push("Transaction outside normal hours".to_string());
        }
    }

    /// Get validation metrics
    pub async fn get_validation_metrics(&self) -> ValidationMetrics {
        let cache = self.validation_cache.read().await;

        let total_validated = cache.len() as u64;
        let total_rejected = cache.values()
            .filter(|result| !result.is_valid)
            .count() as u64;

        let valid_results: Vec<_> = cache.values()
            .filter(|result| result.is_valid)
            .collect();

        let average_confidence = if valid_results.is_empty() {
            0.0
        } else {
            valid_results.iter()
                .map(|r| r.confidence_score)
                .sum::<f64>() / valid_results.len() as f64
        };

        // Calculate error breakdown
        let mut error_breakdown = HashMap::new();
        for result in cache.values() {
            for error in &result.errors {
                let count = error_breakdown.entry(error.clone()).or_insert(0);
                *count += 1;
            }
        }

        ValidationMetrics {
            total_validated,
            total_rejected,
            average_confidence,
            validation_time_avg: 50.0, // Placeholder - would track actual times
            error_breakdown,
        }
    }

    /// Clear old cached results
    pub async fn cleanup_cache(&self, max_age_seconds: u64) {
        let mut cache = self.validation_cache.write().await;
        let cutoff = current_timestamp() - max_age_seconds;

        cache.retain(|_, result| result.timestamp > cutoff);
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
    fn test_validator_creation() {
        let rules = ValidationRules {
            require_multisig: true,
            min_confirmations: 12,
            max_transaction_age: 3600,
            allowed_tokens: vec!["ETH".to_string(), "BTC".to_string()],
            blocked_addresses: vec!["0xblocked".to_string()],
        };

        let validator = CrossChainValidator::new(rules);
        assert!(validator.validation_rules.require_multisig);
    }

    #[tokio::test]
    async fn test_basic_transaction_validation() {
        let rules = ValidationRules {
            require_multisig: false,
            min_confirmations: 6,
            max_transaction_age: 3600,
            allowed_tokens: vec!["ETH".to_string()],
            blocked_addresses: vec![],
        };

        let validator = CrossChainValidator::new(rules);

        // Add bridge config
        let config = BridgeConfig {
            bridge_id: "eth_bridge".to_string(),
            source_chain: "ethereum".to_string(),
            target_chain: "erbium".to_string(),
            contract_address: "0x123".to_string(),
            min_amount: 1000000000000000, // 0.001 ETH
            max_amount: 100000000000000000000, // 100 ETH
            daily_limit: 1000000000000000000000, // 1000 ETH
            paused: false,
        };

        validator.update_bridge_config(config).await.unwrap();

        // Create test transaction
        let tx = CrossChainTransaction {
            tx_hash: "0xabc123".to_string(),
            bridge_id: "eth_bridge".to_string(),
            source_chain: "ethereum".to_string(),
            target_chain: "erbium".to_string(),
            sender: "0xsender".to_string(),
            recipient: "erbium_address".to_string(),
            token: "ETH".to_string(),
            amount: 1000000000000000000, // 1 ETH
            fee: 10000000000000000, // 0.01 ETH
            nonce: 1,
            timestamp: current_timestamp(),
            signatures: vec![],
            metadata: HashMap::new(),
        };

        let result = validator.validate_transaction(&tx).await.unwrap();
        assert!(result.is_valid);
        assert!(result.confidence_score > 0.8);
    }

    #[tokio::test]
    async fn test_invalid_transaction() {
        let rules = ValidationRules {
            require_multisig: false,
            min_confirmations: 6,
            max_transaction_age: 3600,
            allowed_tokens: vec!["ETH".to_string()],
            blocked_addresses: vec!["0xblocked".to_string()],
        };

        let validator = CrossChainValidator::new(rules);

        let tx = CrossChainTransaction {
            tx_hash: "0xabc123".to_string(),
            bridge_id: "eth_bridge".to_string(),
            source_chain: "ethereum".to_string(),
            target_chain: "erbium".to_string(),
            sender: "0xblocked".to_string(), // Blocked address
            recipient: "erbium_address".to_string(),
            token: "ETH".to_string(),
            amount: 1000000000000000000,
            fee: 10000000000000000,
            nonce: 1,
            timestamp: current_timestamp(),
            signatures: vec![],
            metadata: HashMap::new(),
        };

        let result = validator.validate_transaction(&tx).await.unwrap();
        assert!(!result.is_valid);
        assert!(result.errors.len() > 0);
    }
}
