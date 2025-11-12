// src/bridges/chains/polkadot/adapter.rs
use super::types::*;
use jsonrpsee::{
    core::client::ClientT,
    http_client::{HttpClient, HttpClientBuilder},
    rpc_params,
};
use serde_json::Value;
use std::collections::HashMap;

/// Polkadot blockchain adapter for bridge operations
pub struct PolkadotAdapter {
    client: HttpClient,
    runtime_version: Option<RuntimeVersion>,
    metadata: Option<Metadata>,
    last_processed_block: u64,
    watched_modules: HashMap<String, Vec<String>>, // module -> methods to watch
}

impl PolkadotAdapter {
    /// Create a new Polkadot adapter instance
    pub async fn new(config: PolkadotConfig) -> Result<Self, PolkadotError> {
        let client = HttpClientBuilder::default()
            .build(&config.rpc_endpoint)
            .map_err(|e| PolkadotError::Connection(e.to_string()))?;
        
        // Verify connection
        let _version: RuntimeVersion = client.request("chain_getRuntimeVersion", rpc_params![])
            .await
            .map_err(|e| PolkadotError::Connection(e.to_string()))?;
        
        let mut adapter = Self {
            client,
            runtime_version: None,
            metadata: None,
            last_processed_block: 0,
            watched_modules: HashMap::new(),
        };
        
        // Initialize runtime version and metadata
        adapter.refresh_runtime_version().await?;
        adapter.refresh_metadata().await?;
        
        Ok(adapter)
    }
    
    /// Watch specific module and methods for bridge operations
    pub fn watch_module(&mut self, module: String, methods: Vec<String>) {
        self.watched_modules.insert(module.clone(), methods.clone());
        log::info!("Now watching module: {} with methods: {:?}", module, methods);
    }
    
    /// Stop watching a module
    pub fn unwatch_module(&mut self, module: &str) -> Result<(), PolkadotError> {
        self.watched_modules.remove(module)
            .ok_or_else(|| PolkadotError::InvalidParameters(format!("Module not being watched: {}", module)))?;
        
        log::info!("Stopped watching module: {}", module);
        Ok(())
    }
    
    /// Refresh runtime version information
    pub async fn refresh_runtime_version(&mut self) -> Result<(), PolkadotError> {
        self.runtime_version = Some(
            self.client.request("chain_getRuntimeVersion", rpc_params![])
                .await?
        );
        log::debug!("Refreshed runtime version");
        Ok(())
    }
    
    /// Refresh chain metadata
    pub async fn refresh_metadata(&mut self) -> Result<(), PolkadotError> {
        let metadata_bytes: String = self.client.request("state_getMetadata", rpc_params![])
            .await?;
        
        // Decode metadata (simplified - in production use scale-info)
        self.metadata = Some(Metadata {
            raw: hex::decode(&metadata_bytes)?,
            modules: Vec::new(), // Would parse metadata properly using scale-info
            version: 14, // Default version
        });
        
        log::info!("Refreshed Polkadot metadata");
        Ok(())
    }
    
    /// Get current block number
    pub async fn get_current_block(&self) -> Result<u64, PolkadotError> {
        let block_hash: Option<String> = self.client.request("chain_getBlockHash", rpc_params![])
            .await?;
        
        if let Some(hash) = block_hash {
            let header: Value = self.client.request("chain_getHeader", rpc_params![hash])
                .await?;
            
            if let Some(number) = header["number"].as_str() {
                return Ok(u64::from_str_radix(number.trim_start_matches("0x"), 16)?);
            }
        }
        
        Err(PolkadotError::BlockNotFound)
    }
    
    /// Process new blocks and return relevant extrinsics
    pub async fn process_new_blocks(&mut self) -> Result<Vec<PolkadotExtrinsic>, PolkadotError> {
        let current_block = self.get_current_block().await?;
        
        if current_block <= self.last_processed_block {
            return Ok(Vec::new());
        }
        
        let mut new_extrinsics = Vec::new();
        
        // Process blocks from last processed to current
        for block_number in (self.last_processed_block + 1)..=current_block {
            let block_hash = self.get_block_hash(block_number).await?;
            let extrinsics = self.get_block_extrinsics(&block_hash).await?;
            
            for extrinsic in extrinsics {
                if self.is_relevant_extrinsic(&extrinsic) {
                    new_extrinsics.push(extrinsic);
                }
            }
        }
        
        self.last_processed_block = current_block;
        
        log::debug!("Processed {} new Polkadot blocks, found {} extrinsics", 
                   current_block - self.last_processed_block, 
                   new_extrinsics.len());
        
        Ok(new_extrinsics)
    }
    
    /// Submit an extrinsic to the chain
    pub async fn submit_extrinsic(
        &self,
        module: &str,
        method: &str,
        params: Vec<Value>,
        _signer: &str, // In production, use proper key management
    ) -> Result<String, PolkadotError> {
        // Construct the extrinsic call
        let _call_object = serde_json::json!({
            "module": module,
            "method": method,
            "params": params
        });
        
        // This would be properly signed and submitted using the author_submitExtrinsic RPC
        // For now, it's a placeholder implementation
        
        log::warn!("Extrinsic submission not fully implemented - placeholder");
        
        // In production, this would:
        // 1. Construct the call using the metadata
        // 2. Sign the extrinsic with the provided signer
        // 3. Submit via author_submitExtrinsic
        // 4. Return the extrinsic hash
        
        Ok(format!("0x{:064x}", rand::random::<u64>())) // Placeholder hash
    }
    
    /// Query storage at a specific key
    pub async fn get_storage(
        &self,
        storage_key: &str,
        block_hash: Option<String>,
    ) -> Result<Option<Vec<u8>>, PolkadotError> {
        let storage_value: Option<String> = self.client.request(
            "state_getStorage", 
            rpc_params![storage_key, block_hash]
        ).await?;
        
        if let Some(hex_value) = storage_value {
            let bytes = hex::decode(hex_value.trim_start_matches("0x"))?;
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }
    
    /// Get storage with proof
    pub async fn get_storage_with_proof(
        &self,
        storage_key: &str,
        block_hash: Option<String>,
    ) -> Result<StorageResponse, PolkadotError> {
        let storage_value: Option<String> = self.client.request(
            "state_getStorage", 
            rpc_params![storage_key, &block_hash]
        ).await?;
        
        // Proof would be obtained via state_getReadProof
        let proof: Option<Vec<String>> = self.client.request(
            "state_getReadProof",
            rpc_params![vec![storage_key], block_hash.clone()]
        ).await?;
        
        let value_bytes = if let Some(hex_value) = storage_value {
            Some(hex::decode(hex_value.trim_start_matches("0x"))?)
        } else {
            None
        };
        
        let proof_bytes = proof.map(|p| {
            p.into_iter()
                .map(|hex_str| hex::decode(hex_str.trim_start_matches("0x")))
                .collect::<Result<Vec<_>, _>>()
        }).transpose()?;
        
        Ok(StorageResponse {
            key: storage_key.to_string(),
            value: value_bytes,
            proof: proof_bytes,
        })
    }
    
    /// Get events for a specific block
    pub async fn get_events(
        &self,
        block_hash: String,
    ) -> Result<Vec<Event>, PolkadotError> {
        let _events_raw: String = self.client.request(
            "state_getStorage",
            rpc_params!["0x26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9", &block_hash]
        ).await?;

        // Decode events using metadata
        // This would properly decode the events using scale-info
        let events = Vec::new(); // Placeholder

        Ok(events)
    }
    
    /// Get runtime version
    pub fn get_runtime_version(&self) -> Option<&RuntimeVersion> {
        self.runtime_version.as_ref()
    }
    
    /// Get metadata
    pub fn get_metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }
    
    // Private methods
    
    /// Get block hash at specific height
    async fn get_block_hash(&self, block_number: u64) -> Result<String, PolkadotError> {
        let block_hash: Option<String> = self.client.request(
            "chain_getBlockHash", 
            rpc_params![block_number]
        ).await?;
        
        block_hash.ok_or(PolkadotError::BlockNotFound)
    }
    
    /// Get extrinsics from a block
    async fn get_block_extrinsics(
        &self,
        block_hash: &str,
    ) -> Result<Vec<PolkadotExtrinsic>, PolkadotError> {
        let block: Value = self.client.request("chain_getBlock", rpc_params![block_hash])
            .await?;
        
        let mut extrinsics = Vec::new();
        
        if let Some(extrinsics_array) = block["block"]["extrinsics"].as_array() {
            for (index, extrinsic) in extrinsics_array.iter().enumerate() {
                if let Some(parsed_extrinsic) = self.parse_extrinsic(extrinsic, index as u32, block_hash).await? {
                    extrinsics.push(parsed_extrinsic);
                }
            }
        }
        
        Ok(extrinsics)
    }
    
    /// Parse extrinsic from raw data
    async fn parse_extrinsic(
        &self,
        _extrinsic: &Value,
        index: u32,
        block_hash: &str,
    ) -> Result<Option<PolkadotExtrinsic>, PolkadotError> {
        // Simplified extrinsic parsing
        // In production, use proper SCALE decoding with metadata
        
        let extrinsic_hash = format!("{}_{}", block_hash, index);
        
        // This would properly decode the extrinsic using the metadata
        // For now, return a placeholder
        
        let extrinsic_data = PolkadotExtrinsic {
            hash: extrinsic_hash,
            module: "unknown".to_string(),
            method: "unknown".to_string(),
            signer: "unknown".to_string(),
            params: HashMap::new(),
            block_number: 0, // Would be set properly from block data
            success: true, // Would check events to determine success
        };
        
        Ok(Some(extrinsic_data))
    }
    
    /// Check if extrinsic is relevant for bridge monitoring
    fn is_relevant_extrinsic(&self, extrinsic: &PolkadotExtrinsic) -> bool {
        // Check if extrinsic is from a watched module and method
        if let Some(methods) = self.watched_modules.get(&extrinsic.module) {
            return methods.contains(&extrinsic.method);
        }
        
        // Also check for specific bridge-related modules
        matches!(
            (extrinsic.module.as_str(), extrinsic.method.as_str()),
            ("XcmPallet", "send") |
            ("XcmPallet", "teleport_assets") |
            ("XcmPallet", "reserve_transfer_assets") |
            ("Bridge", _) |
            ("Multisig", "as_multi") |
            ("Utility", "batch")
        )
    }
}