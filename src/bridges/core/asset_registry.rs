// src/bridges/core/asset_registry.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub asset_id: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub chains: HashMap<String, ChainAssetMapping>, // chain_id -> contract/address
    pub is_wrapped: bool,
    pub original_chain: Option<String>,
    pub max_supply: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainAssetMapping {
    pub contract_address: String, // For EVM chains
    pub asset_id: String,         // For non-EVM chains
    pub is_native: bool,
}

pub struct AssetRegistry {
    assets: HashMap<String, AssetInfo>, // asset_id -> AssetInfo
    chain_assets: HashMap<String, Vec<String>>, // chain_id -> list of asset_ids
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
            chain_assets: HashMap::new(),
        }
    }
    
    pub fn register_asset(&mut self, asset: AssetInfo) -> Result<(), AssetError> {
        // Validate asset data
        self.validate_asset(&asset)?;
        
        // Register asset
        self.assets.insert(asset.asset_id.clone(), asset.clone());
        
        // Update chain mappings
        for chain_id in asset.chains.keys() {
            self.chain_assets
                .entry(chain_id.clone())
                .or_insert_with(Vec::new)
                .push(asset.asset_id.clone());
        }
        
        log::info!("Asset registered: {} ({})", asset.name, asset.asset_id);
        
        Ok(())
    }
    
    pub fn get_asset_info(&self, asset_id: &str) -> Option<&AssetInfo> {
        self.assets.get(asset_id)
    }
    
    pub fn get_chain_assets(&self, chain_id: &str) -> Vec<&AssetInfo> {
        self.chain_assets
            .get(chain_id)
            .map(|asset_ids| {
                asset_ids.iter()
                    .filter_map(|id| self.assets.get(id))
                    .collect()
            })
            .unwrap_or_else(Vec::new)
    }
    
    pub fn register_chain_mapping(
        &mut self,
        asset_id: &str,
        chain_id: String,
        mapping: ChainAssetMapping,
    ) -> Result<(), AssetError> {
        let asset = self.assets.get_mut(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound(asset_id.to_string()))?;
        
        asset.chains.insert(chain_id.clone(), mapping);
        
        // Update chain assets index
        self.chain_assets
            .entry(chain_id)
            .or_insert_with(Vec::new)
            .push(asset_id.to_string());
        
        Ok(())
    }
    
    pub fn create_wrapped_asset(
        &mut self,
        original_asset_id: &str,
        target_chain: &str,
        wrapped_symbol: String,
    ) -> Result<String, AssetError> {
        let original_asset = self.assets.get(original_asset_id)
            .ok_or_else(|| AssetError::AssetNotFound(original_asset_id.to_string()))?;
        
        // Generate wrapped asset ID
        let wrapped_asset_id = format!("wrapped_{}_{}", original_asset.symbol.to_lowercase(), target_chain);
        
        let wrapped_asset = AssetInfo {
            asset_id: wrapped_asset_id.clone(),
            name: format!("Wrapped {} ({})", original_asset.name, target_chain),
            symbol: wrapped_symbol,
            decimals: original_asset.decimals,
            chains: HashMap::new(),
            is_wrapped: true,
            original_chain: Some(original_asset.chains.keys().next()
                .ok_or_else(|| AssetError::InvalidAsset("Original asset has no chain mappings".to_string()))?
                .clone()),
            max_supply: None, // Wrapped assets don't have max supply
        };
        
        self.register_asset(wrapped_asset)?;
        
        log::info!("Created wrapped asset: {} for chain {}", wrapped_asset_id, target_chain);
        
        Ok(wrapped_asset_id)
    }
    
    fn validate_asset(&self, asset: &AssetInfo) -> Result<(), AssetError> {
        if asset.asset_id.is_empty() {
            return Err(AssetError::InvalidAsset("Asset ID cannot be empty".to_string()));
        }
        
        if asset.name.is_empty() {
            return Err(AssetError::InvalidAsset("Asset name cannot be empty".to_string()));
        }
        
        if asset.symbol.is_empty() {
            return Err(AssetError::InvalidAsset("Asset symbol cannot be empty".to_string()));
        }
        
        if asset.chains.is_empty() {
            return Err(AssetError::InvalidAsset("Asset must be available on at least one chain".to_string()));
        }
        
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("Asset not found: {0}")]
    AssetNotFound(String),
    #[error("Invalid asset data: {0}")]
    InvalidAsset(String),
    #[error("Asset already exists: {0}")]
    AssetExists(String),
}