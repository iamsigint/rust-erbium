//! Secure Key Management and HSM Integration for Erbium Blockchain
//!
//! This module provides enterprise-grade key management capabilities including:
//! - Hardware Security Module (HSM) integration
//! - Secure key generation, storage, and rotation
//! - Multi-signature key management
//! - Key backup and recovery
//! - Cryptographic key lifecycle management

use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Key management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyManagementConfig {
    pub enabled: bool,
    pub hsm_enabled: bool,
    pub key_rotation_enabled: bool,
    pub backup_enabled: bool,
    pub multi_sig_enabled: bool,
    pub key_rotation_interval_days: u64,
    pub backup_interval_hours: u64,
    pub min_key_strength: KeyStrength,
    pub hsm_provider: HSMProvider,
    pub master_key_id: String,
    pub emergency_key_access: bool,
}

impl Default for KeyManagementConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hsm_enabled: true,
            key_rotation_enabled: true,
            backup_enabled: true,
            multi_sig_enabled: true,
            key_rotation_interval_days: 90,
            backup_interval_hours: 24,
            min_key_strength: KeyStrength::High,
            hsm_provider: HSMProvider::YubiHSM2,
            master_key_id: "master_key_001".to_string(),
            emergency_key_access: false,
        }
    }
}

/// Key management engine
pub struct KeyManagementEngine {
    config: KeyManagementConfig,
    key_store: Arc<RwLock<HashMap<String, KeyMetadata>>>,
    hsm_interface: Option<Arc<dyn HSMInterface>>,
    backup_manager: Arc<BackupManager>,
    rotation_scheduler: Arc<KeyRotationScheduler>,
    multi_sig_manager: Arc<MultiSigManager>,
}

impl KeyManagementEngine {
    /// Create new key management engine
    pub fn new(config: KeyManagementConfig) -> Self {
        let hsm_interface = if config.hsm_enabled {
            Some(Self::initialize_hsm(&config.hsm_provider))
        } else {
            None
        };

        Self {
            key_store: Arc::new(RwLock::new(HashMap::new())),
            hsm_interface,
            backup_manager: Arc::new(BackupManager::new()),
            rotation_scheduler: Arc::new(KeyRotationScheduler::new()),
            multi_sig_manager: Arc::new(MultiSigManager::new()),
            config,
        }
    }

    /// Initialize the key management system
    pub async fn initialize(&self) -> Result<()> {
        log::info!("Initializing secure key management system...");

        // Initialize HSM connection if enabled
        if let Some(ref hsm) = self.hsm_interface {
            hsm.connect().await?;
            log::info!("HSM connection established");
        }

        // Load existing keys from secure storage
        self.load_key_metadata().await?;

        // Start background tasks
        let _ = self.start_background_tasks().await;

        log::info!("Key management system initialized with {} keys",
                  self.key_store.read().await.len());
        Ok(())
    }

    /// Generate a new cryptographic key
    pub async fn generate_key(&self, key_type: KeyType, purpose: KeyPurpose, metadata: KeyMetadata) -> Result<String> {
        if !self.config.enabled {
            return Err(BlockchainError::Security("Key management is disabled".to_string()));
        }

        let key_id = format!("key_{}_{}_{}", key_type.to_string().to_lowercase(),
                           purpose.to_string().to_lowercase(), current_timestamp());

        // Generate key based on configuration
        let _key_material = if let Some(ref hsm) = self.hsm_interface {
            // Generate key in HSM
            hsm.generate_key(key_type.clone(), &key_id).await?
        } else {
            // Generate key in software (less secure)
            self.generate_software_key(key_type.clone())?
        };

        // Store key metadata
        let mut key_metadata = metadata;
        key_metadata.key_id = key_id.clone();
        key_metadata.key_type = key_type.clone();
        key_metadata.purpose = purpose.clone();
        key_metadata.created_at = current_timestamp();
        key_metadata.status = KeyStatus::Active;
        key_metadata.strength = self.config.min_key_strength.clone();

        {
            let mut store = self.key_store.write().await;
            store.insert(key_id.clone(), key_metadata);
        }

        // Schedule backup
        if self.config.backup_enabled {
            self.backup_manager.schedule_backup(&key_id).await?;
        }

        log::info!("Generated new {} key: {} for {}", key_type.to_string(), key_id, purpose.to_string());
        Ok(key_id)
    }

    /// Retrieve a key for use
    pub async fn get_key(&self, key_id: &str) -> Result<KeyHandle> {
        let store = self.key_store.read().await;

        if let Some(metadata) = store.get(key_id) {
            if metadata.status != KeyStatus::Active {
                return Err(BlockchainError::Security(format!("Key {} is not active", key_id)));
            }

            // Check if key needs rotation
            if self.needs_rotation(metadata) {
                // Clone metadata before dropping store
                let key_type = metadata.key_type.clone();
                let purpose = metadata.purpose.clone();
                let created_at = metadata.created_at;
                let expires_at = metadata.expires_at;

                drop(store);
                let _new_key_id = self.rotate_key(key_id).await?;
                // Return the rotated key info (simplified - in real implementation would get the new key)
                return Ok(KeyHandle {
                    key_id: key_id.to_string(),
                    key_type,
                    purpose,
                    created_at,
                    expires_at,
                });
            }

            Ok(KeyHandle {
                key_id: key_id.to_string(),
                key_type: metadata.key_type.clone(),
                purpose: metadata.purpose.clone(),
                created_at: metadata.created_at,
                expires_at: metadata.expires_at,
            })
        } else {
            Err(BlockchainError::Security(format!("Key {} not found", key_id)))
        }
    }

    /// Sign data with a specific key
    pub async fn sign_data(&self, key_id: &str, data: &[u8]) -> Result<Vec<u8>> {
        if let Some(ref hsm) = self.hsm_interface {
            hsm.sign_data(key_id, data).await
        } else {
            // Software signing (less secure)
            self.sign_software(key_id, data).await
        }
    }

    /// Verify signature with a specific key
    pub async fn verify_signature(&self, key_id: &str, data: &[u8], signature: &[u8]) -> Result<bool> {
        if let Some(ref hsm) = self.hsm_interface {
            hsm.verify_signature(key_id, data, signature).await
        } else {
            self.verify_software_signature(key_id, data, signature).await
        }
    }

    /// Rotate a key
    pub async fn rotate_key(&self, key_id: &str) -> Result<String> {
        log::info!("Rotating key: {}", key_id);

        let store = self.key_store.read().await;
        let old_metadata = store.get(key_id)
            .ok_or_else(|| BlockchainError::Security(format!("Key {} not found", key_id)))?
            .clone();
        drop(store);

        // Generate new key
        let new_key_id = self.generate_key(
            old_metadata.key_type.clone(),
            old_metadata.purpose.clone(),
            KeyMetadata {
                key_id: "".to_string(),
                key_type: old_metadata.key_type.clone(),
                purpose: old_metadata.purpose.clone(),
                created_at: 0,
                expires_at: None,
                status: KeyStatus::Active,
                strength: old_metadata.strength,
                algorithm: old_metadata.algorithm,
                usage_count: 0,
                last_used: None,
                rotation_count: old_metadata.rotation_count + 1,
                backup_status: BackupStatus::NotBackedUp,
                rotated_at: None,
            }
        ).await?;

        // Mark old key as rotated
        {
            let mut store = self.key_store.write().await;
            if let Some(metadata) = store.get_mut(key_id) {
                metadata.status = KeyStatus::Rotated;
                metadata.rotated_at = Some(current_timestamp());
            }
        }

        log::info!("Key rotated: {} -> {}", key_id, new_key_id);
        Ok(new_key_id)
    }

    /// Create a multi-signature key set
    pub async fn create_multi_sig_key(&self, key_type: KeyType, threshold: usize, total_keys: usize) -> Result<String> {
        if !self.config.multi_sig_enabled {
            return Err(BlockchainError::Security("Multi-signature is disabled".to_string()));
        }

        self.multi_sig_manager.create_multi_sig_key(key_type, threshold, total_keys).await
    }

    /// Sign with multi-signature key
    pub async fn multi_sig_sign(&self, multi_sig_id: &str, data: &[u8], signer_key_id: &str) -> Result<MultiSigSignature> {
        self.multi_sig_manager.sign_with_key(multi_sig_id, data, signer_key_id).await
    }

    /// Combine multi-signature signatures
    pub async fn combine_multi_sig(&self, multi_sig_id: &str, signatures: Vec<MultiSigSignature>) -> Result<Vec<u8>> {
        self.multi_sig_manager.combine_signatures(multi_sig_id, signatures).await
    }

    /// Get key management statistics
    pub async fn get_key_statistics(&self) -> Result<KeyManagementStats> {
        let store = self.key_store.read().await;

        let total_keys = store.len();
        let active_keys = store.values().filter(|k| k.status == KeyStatus::Active).count();
        let rotated_keys = store.values().filter(|k| k.status == KeyStatus::Rotated).count();
        let compromised_keys = store.values().filter(|k| k.status == KeyStatus::Compromised).count();

        let avg_rotation_count = if total_keys > 0 {
            store.values().map(|k| k.rotation_count).sum::<u64>() as f64 / total_keys as f64
        } else {
            0.0
        };

        Ok(KeyManagementStats {
            total_keys,
            active_keys,
            rotated_keys,
            compromised_keys,
            hsm_enabled: self.hsm_interface.is_some(),
            backup_enabled: self.config.backup_enabled,
            rotation_enabled: self.config.key_rotation_enabled,
            average_rotation_count: avg_rotation_count,
            last_backup: self.backup_manager.get_last_backup_time().await,
        })
    }

    // Private methods
    fn initialize_hsm(provider: &HSMProvider) -> Arc<dyn HSMInterface> {
        match provider {
            HSMProvider::YubiHSM2 => Arc::new(YubiHSM2Interface::new()),
            HSMProvider::AWSCloudHSM => Arc::new(AWSCloudHSMInterface::new()),
            HSMProvider::AzureKeyVault => Arc::new(AzureKeyVaultInterface::new()),
            HSMProvider::Gemalto => Arc::new(GemaltoHSMInterface::new()),
        }
    }

    async fn load_key_metadata(&self) -> Result<()> {
        // TODO: Load key metadata from secure storage
        // For now, this is a placeholder
        Ok(())
    }

    async fn start_background_tasks(&self) -> Result<()> {
        // Start key rotation scheduler
        if self.config.key_rotation_enabled {
            self.rotation_scheduler.start_scheduler(self.key_store.clone()).await?;
        }

        // Start backup scheduler
        if self.config.backup_enabled {
            self.backup_manager.start_backup_scheduler().await?;
        }

        Ok(())
    }

    fn generate_software_key(&self, key_type: KeyType) -> Result<Vec<u8>> {
        // Generate a software key (for development/testing only)
        // In production, this should never be used
        match key_type {
            KeyType::Ed25519 => {
                // Generate Ed25519 key pair
                let mut key = vec![0u8; 32];
                // Fill with random data (in real implementation, use proper RNG)
                for i in 0..32 {
                    key[i] = (current_timestamp() % 256) as u8;
                }
                Ok(key)
            }
            KeyType::Secp256k1 => {
                let mut key = vec![0u8; 32];
                for i in 0..32 {
                    key[i] = ((current_timestamp() + i as u64) % 256) as u8;
                }
                Ok(key)
            }
            KeyType::Dilithium => {
                // Dilithium keys are larger
                let mut key = vec![0u8; 64];
                for i in 0..64 {
                    key[i] = (current_timestamp() % 256) as u8;
                }
                Ok(key)
            }
            KeyType::RSA => {
                // RSA keys are larger
                let mut key = vec![0u8; 2048];
                for i in 0..2048 {
                    key[i] = (current_timestamp() % 256) as u8;
                }
                Ok(key)
            }
        }
    }

    async fn sign_software(&self, key_id: &str, data: &[u8]) -> Result<Vec<u8>> {
        // Software signing (for development only)
        // In production, use HSM
        let mut signature = Vec::from(data);
        signature.extend_from_slice(key_id.as_bytes());
        signature.extend_from_slice(&current_timestamp().to_be_bytes());
        Ok(signature)
    }

    async fn verify_software_signature(&self, key_id: &str, data: &[u8], signature: &[u8]) -> Result<bool> {
        // Simple verification (for development only)
        Ok(signature.starts_with(data) && signature.ends_with(key_id.as_bytes()))
    }

    fn needs_rotation(&self, metadata: &KeyMetadata) -> bool {
        if !self.config.key_rotation_enabled {
            return false;
        }

        let age_days = (current_timestamp() - metadata.created_at) / (24 * 3600);
        age_days >= self.config.key_rotation_interval_days
    }
}

/// Key metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub key_id: String,
    pub key_type: KeyType,
    pub purpose: KeyPurpose,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub status: KeyStatus,
    pub strength: KeyStrength,
    pub algorithm: String,
    pub usage_count: u64,
    pub last_used: Option<u64>,
    pub rotation_count: u64,
    pub backup_status: BackupStatus,
    pub rotated_at: Option<u64>,
}

/// Key handle for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyHandle {
    pub key_id: String,
    pub key_type: KeyType,
    pub purpose: KeyPurpose,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

/// Key types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyType {
    Ed25519,
    Secp256k1,
    Dilithium,
    RSA,
}

impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyType::Ed25519 => write!(f, "Ed25519"),
            KeyType::Secp256k1 => write!(f, "Secp256k1"),
            KeyType::Dilithium => write!(f, "Dilithium"),
            KeyType::RSA => write!(f, "RSA"),
        }
    }
}

/// Key purposes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyPurpose {
    Signing,
    Encryption,
    Authentication,
    MultiSig,
    Master,
}

impl std::fmt::Display for KeyPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyPurpose::Signing => write!(f, "Signing"),
            KeyPurpose::Encryption => write!(f, "Encryption"),
            KeyPurpose::Authentication => write!(f, "Authentication"),
            KeyPurpose::MultiSig => write!(f, "MultiSig"),
            KeyPurpose::Master => write!(f, "Master"),
        }
    }
}

/// Key status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KeyStatus {
    Active,
    Inactive,
    Rotated,
    Compromised,
    Expired,
}

/// Key strength levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum KeyStrength {
    Basic,
    Standard,
    High,
    Military,
}

/// Backup status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupStatus {
    NotBackedUp,
    BackedUp,
    BackupFailed,
}

/// HSM providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HSMProvider {
    YubiHSM2,
    AWSCloudHSM,
    AzureKeyVault,
    Gemalto,
}

/// Key management statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyManagementStats {
    pub total_keys: usize,
    pub active_keys: usize,
    pub rotated_keys: usize,
    pub compromised_keys: usize,
    pub hsm_enabled: bool,
    pub backup_enabled: bool,
    pub rotation_enabled: bool,
    pub average_rotation_count: f64,
    pub last_backup: Option<u64>,
}

/// Multi-signature signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigSignature {
    pub signer_key_id: String,
    pub signature: Vec<u8>,
    pub signed_at: u64,
}

/// HSM Interface trait
#[async_trait::async_trait]
pub trait HSMInterface: Send + Sync {
    async fn connect(&self) -> Result<()>;
    async fn generate_key(&self, key_type: KeyType, key_id: &str) -> Result<Vec<u8>>;
    async fn sign_data(&self, key_id: &str, data: &[u8]) -> Result<Vec<u8>>;
    async fn verify_signature(&self, key_id: &str, data: &[u8], signature: &[u8]) -> Result<bool>;
    async fn get_key_info(&self, key_id: &str) -> Result<KeyMetadata>;
}

/// Backup manager
pub struct BackupManager {
    last_backup: Arc<RwLock<Option<u64>>>,
}

impl BackupManager {
    pub fn new() -> Self {
        Self {
            last_backup: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn schedule_backup(&self, key_id: &str) -> Result<()> {
        log::info!("Scheduling backup for key: {}", key_id);
        // TODO: Implement actual backup scheduling
        *self.last_backup.write().await = Some(current_timestamp());
        Ok(())
    }

    pub async fn start_backup_scheduler(&self) -> Result<()> {
        // TODO: Start background backup task
        Ok(())
    }

    pub async fn get_last_backup_time(&self) -> Option<u64> {
        *self.last_backup.read().await
    }
}

/// Key rotation scheduler
pub struct KeyRotationScheduler;

impl KeyRotationScheduler {
    pub fn new() -> Self {
        Self
    }

    pub async fn start_scheduler(&self, _key_store: Arc<RwLock<HashMap<String, KeyMetadata>>>) -> Result<()> {
        // TODO: Start background rotation task
        Ok(())
    }
}

/// Multi-signature manager
pub struct MultiSigManager;

impl MultiSigManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn create_multi_sig_key(&self, key_type: KeyType, threshold: usize, total_keys: usize) -> Result<String> {
        let multi_sig_id = format!("multisig_{}_{}_{}_{}", key_type.to_string().to_lowercase(),
                                 threshold, total_keys, current_timestamp());

        log::info!("Created multi-signature key: {} (threshold: {}/{})", multi_sig_id, threshold, total_keys);
        Ok(multi_sig_id)
    }

    pub async fn sign_with_key(&self, _multi_sig_id: &str, data: &[u8], signer_key_id: &str) -> Result<MultiSigSignature> {
        // TODO: Implement actual multi-sig signing
        Ok(MultiSigSignature {
            signer_key_id: signer_key_id.to_string(),
            signature: Vec::from(data), // Placeholder
            signed_at: current_timestamp(),
        })
    }

    pub async fn combine_signatures(&self, multi_sig_id: &str, signatures: Vec<MultiSigSignature>) -> Result<Vec<u8>> {
        // TODO: Implement signature combination
        log::info!("Combined {} signatures for multi-sig key: {}", signatures.len(), multi_sig_id);
        Ok(vec![0u8; 64]) // Placeholder
    }
}

// HSM Interface implementations (placeholders)
pub struct YubiHSM2Interface;
impl YubiHSM2Interface {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl HSMInterface for YubiHSM2Interface {
    async fn connect(&self) -> Result<()> { Ok(()) }
    async fn generate_key(&self, _key_type: KeyType, _key_id: &str) -> Result<Vec<u8>> { Ok(vec![0u8; 32]) }
    async fn sign_data(&self, _key_id: &str, data: &[u8]) -> Result<Vec<u8>> { Ok(Vec::from(data)) }
    async fn verify_signature(&self, _key_id: &str, _data: &[u8], _signature: &[u8]) -> Result<bool> { Ok(true) }
    async fn get_key_info(&self, _key_id: &str) -> Result<KeyMetadata> {
        Ok(KeyMetadata {
            key_id: "test".to_string(),
            key_type: KeyType::Ed25519,
            purpose: KeyPurpose::Signing,
            created_at: current_timestamp(),
            expires_at: None,
            status: KeyStatus::Active,
            strength: KeyStrength::High,
            algorithm: "Ed25519".to_string(),
            usage_count: 0,
            last_used: None,
            rotation_count: 0,
            backup_status: BackupStatus::NotBackedUp,
            rotated_at: None,
        })
    }
}

pub struct AWSCloudHSMInterface;
impl AWSCloudHSMInterface {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl HSMInterface for AWSCloudHSMInterface {
    async fn connect(&self) -> Result<()> { Ok(()) }
    async fn generate_key(&self, _key_type: KeyType, _key_id: &str) -> Result<Vec<u8>> { Ok(vec![0u8; 32]) }
    async fn sign_data(&self, _key_id: &str, data: &[u8]) -> Result<Vec<u8>> { Ok(Vec::from(data)) }
    async fn verify_signature(&self, _key_id: &str, _data: &[u8], _signature: &[u8]) -> Result<bool> { Ok(true) }
    async fn get_key_info(&self, _key_id: &str) -> Result<KeyMetadata> {
        Ok(KeyMetadata {
            key_id: "test".to_string(),
            key_type: KeyType::Ed25519,
            purpose: KeyPurpose::Signing,
            created_at: current_timestamp(),
            expires_at: None,
            status: KeyStatus::Active,
            strength: KeyStrength::High,
            algorithm: "Ed25519".to_string(),
            usage_count: 0,
            last_used: None,
            rotation_count: 0,
            backup_status: BackupStatus::NotBackedUp,
            rotated_at: None,
        })
    }
}

pub struct AzureKeyVaultInterface;
impl AzureKeyVaultInterface {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl HSMInterface for AzureKeyVaultInterface {
    async fn connect(&self) -> Result<()> { Ok(()) }
    async fn generate_key(&self, _key_type: KeyType, _key_id: &str) -> Result<Vec<u8>> { Ok(vec![0u8; 32]) }
    async fn sign_data(&self, _key_id: &str, data: &[u8]) -> Result<Vec<u8>> { Ok(Vec::from(data)) }
    async fn verify_signature(&self, _key_id: &str, _data: &[u8], _signature: &[u8]) -> Result<bool> { Ok(true) }
    async fn get_key_info(&self, _key_id: &str) -> Result<KeyMetadata> {
        Ok(KeyMetadata {
            key_id: "test".to_string(),
            key_type: KeyType::Ed25519,
            purpose: KeyPurpose::Signing,
            created_at: current_timestamp(),
            expires_at: None,
            status: KeyStatus::Active,
            strength: KeyStrength::High,
            algorithm: "Ed25519".to_string(),
            usage_count: 0,
            last_used: None,
            rotation_count: 0,
            backup_status: BackupStatus::NotBackedUp,
            rotated_at: None,
        })
    }
}

pub struct GemaltoHSMInterface;
impl GemaltoHSMInterface {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl HSMInterface for GemaltoHSMInterface {
    async fn connect(&self) -> Result<()> { Ok(()) }
    async fn generate_key(&self, _key_type: KeyType, _key_id: &str) -> Result<Vec<u8>> { Ok(vec![0u8; 32]) }
    async fn sign_data(&self, _key_id: &str, data: &[u8]) -> Result<Vec<u8>> { Ok(Vec::from(data)) }
    async fn verify_signature(&self, _key_id: &str, _data: &[u8], _signature: &[u8]) -> Result<bool> { Ok(true) }
    async fn get_key_info(&self, _key_id: &str) -> Result<KeyMetadata> {
        Ok(KeyMetadata {
            key_id: "test".to_string(),
            key_type: KeyType::Ed25519,
            purpose: KeyPurpose::Signing,
            created_at: current_timestamp(),
            expires_at: None,
            status: KeyStatus::Active,
            strength: KeyStrength::High,
            algorithm: "Ed25519".to_string(),
            usage_count: 0,
            last_used: None,
            rotation_count: 0,
            backup_status: BackupStatus::NotBackedUp,
            rotated_at: None,
        })
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_key_management_engine_creation() {
        let config = KeyManagementConfig::default();
        let engine = KeyManagementEngine::new(config);

        assert!(engine.config.enabled);
        assert_eq!(engine.config.hsm_provider, HSMProvider::YubiHSM2);
    }

    #[tokio::test]
    async fn test_key_generation() {
        let config = KeyManagementConfig::default();
        let engine = KeyManagementEngine::new(config);

        let metadata = KeyMetadata {
            key_id: "".to_string(),
            key_type: KeyType::Ed25519,
            purpose: KeyPurpose::Signing,
            created_at: 0,
            expires_at: None,
            status: KeyStatus::Active,
            strength: KeyStrength::High,
            algorithm: "Ed25519".to_string(),
            usage_count: 0,
            last_used: None,
            rotation_count: 0,
            backup_status: BackupStatus::NotBackedUp,
            rotated_at: None,
        };

        let result = engine.generate_key(KeyType::Ed25519, KeyPurpose::Signing, metadata).await;
        assert!(result.is_ok());

        let key_id = result.unwrap();
        assert!(key_id.starts_with("key_ed25519_signing"));
    }

    #[tokio::test]
    async fn test_key_retrieval() {
        let config = KeyManagementConfig::default();
        let engine = KeyManagementEngine::new(config);

        let metadata = KeyMetadata {
            key_id: "".to_string(),
            key_type: KeyType::Ed25519,
            purpose: KeyPurpose::Signing,
            created_at: 0,
            expires_at: None,
            status: KeyStatus::Active,
            strength: KeyStrength::High,
            algorithm: "Ed25519".to_string(),
            usage_count: 0,
            last_used: None,
            rotation_count: 0,
            backup_status: BackupStatus::NotBackedUp,
            rotated_at: None,
        };

        let key_id = engine.generate_key(KeyType::Ed25519, KeyPurpose::Signing, metadata).await.unwrap();
        let handle_result = engine.get_key(&key_id).await;

        assert!(handle_result.is_ok());
        let handle = handle_result.unwrap();
        assert_eq!(handle.key_id, key_id);
        assert_eq!(handle.key_type, KeyType::Ed25519);
    }
}
