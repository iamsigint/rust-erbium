use crate::bridges::core::bridge_manager::BridgeManager;
use crate::core::chain::PersistentBlockchain;
use crate::core::erbium_engine::ErbiumEngine;
use crate::network::p2p_network::{P2PConfig, P2PNetwork};
use crate::node::node_metrics::NodeMetrics;
use crate::utils::error::{BlockchainError, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub struct NodeManager {
    blockchain: Option<Arc<RwLock<crate::core::chain::PersistentBlockchain>>>,
    consensus: Option<Arc<RwLock<crate::consensus::pos::ProofOfStake>>>,
    bridge_manager: Option<Arc<RwLock<BridgeManager>>>,
    erbium_engine: Option<Arc<ErbiumEngine>>,
    p2p_network: Option<Arc<RwLock<P2PNetwork>>>,
    metrics: Option<NodeMetrics>,
    is_running: bool,
    rest_server_handle: Option<JoinHandle<()>>,
    rpc_server_handle: Option<JoinHandle<()>>,
    block_production_handle: Option<JoinHandle<()>>,
    bridge_monitor_handle: Option<JoinHandle<()>>,
    network_handle: Option<JoinHandle<()>>,
    shutdown_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl NodeManager {
    pub async fn new() -> Result<Self> {
        log::info!("Initializing Erbium Node Manager...");

        let blockchain = match PersistentBlockchain::new("./erbium-data").await {
            Ok(bc) => Some(Arc::new(RwLock::new(bc))),
            Err(e) => {
                log::warn!(
                    "Failed to initialize persistent blockchain: {}. Blockchain not available",
                    e
                );
                None
            }
        };

        let consensus_config = crate::consensus::ConsensusConfig::default();
        let mut consensus = crate::consensus::pos::ProofOfStake::new(consensus_config);
        
        // Initialize genesis validators with stake
        if let Err(e) = Self::initialize_genesis_validators(&mut consensus).await {
            log::warn!("Failed to initialize genesis validators: {}", e);
        }
        
        let consensus = Some(Arc::new(RwLock::new(consensus)));

        // Initialize Erbium Engine
        let erbium_engine = match ErbiumEngine::new("./erbium-data").await {
            Ok(engine) => Some(Arc::new(engine)),
            Err(e) => {
                log::warn!("Failed to initialize Erbium Engine: {}", e);
                None
            }
        };

        // Initialize P2P Network (requires blockchain)
        let p2p_network = if let Some(blockchain_ref) = &blockchain {
            let p2p_config = P2PConfig::default();
            Some(Arc::new(RwLock::new(P2PNetwork::new(
                p2p_config,
                blockchain_ref.clone(),
            ))))
        } else {
            log::warn!("Blockchain not available, P2P network not initialized");
            None
        };

        // Initialize bridge manager
        let bridge_manager = Some(Arc::new(RwLock::new(BridgeManager::new())));

        // Initialize metrics
        let metrics = match NodeMetrics::new() {
            Ok(m) => Some(m),
            Err(e) => {
                log::warn!("Failed to initialize metrics: {}", e);
                None
            }
        };

        Ok(Self {
            blockchain,
            consensus,
            bridge_manager,
            erbium_engine,
            p2p_network,
            metrics,
            is_running: false,
            rest_server_handle: None,
            rpc_server_handle: None,
            block_production_handle: None,
            bridge_monitor_handle: None,
            network_handle: None,
            shutdown_sender: None,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.is_running {
            return Err(BlockchainError::Network(
                "Node is already running".to_string(),
            ));
        }

        self.is_running = true;
        log::info!("Starting Erbium Node...");

        self.start_rest_api().await?;
        self.start_rpc_api().await?;
        self.start_metrics_server().await?;

        if self.consensus.is_some() && self.blockchain.is_some() {
            self.start_block_production().await?;
            self.start_staking_services().await?;
            self.start_bridge_monitoring().await?;
        } else {
            log::warn!("Blockchain or consensus not available, starting without block production");
        }

        // Start P2P Network if available
        self.start_p2p_network().await?;

        log::info!("Erbium Node started successfully");
        Ok(())
    }

    async fn start_rest_api(&mut self) -> Result<()> {
        use crate::api::rest::RestServer;

        let mut rest_server = RestServer::new(8080)?;

        // Inject dependencies from the NodeManager
        if let Some(blockchain) = &self.blockchain {
            rest_server = rest_server.with_persistent_blockchain(blockchain.clone());
        }

        if let Some(bridge_manager) = &self.bridge_manager {
            rest_server = rest_server.with_bridge_manager(bridge_manager.clone());
        }

        if let Some(p2p_network) = &self.p2p_network {
            rest_server = rest_server.with_p2p_network(p2p_network.clone());
        }

        let handle = tokio::spawn(async move {
            log::info!("Starting REST API server on port 8080");
            if let Err(e) = rest_server.start().await {
                log::error!("REST API server error: {}", e);
            }
        });

        self.rest_server_handle = Some(handle);
        Ok(())
    }

    async fn start_rpc_api(&mut self) -> Result<()> {
        // TODO: Implement RPC API with PersistentBlockchain support
        log::warn!("RPC API not yet implemented for PersistentBlockchain");
        Ok(())
    }

    async fn start_block_production(&mut self) -> Result<()> {
        let _blockchain = self.blockchain.as_ref().unwrap().clone();
        let consensus = self.consensus.as_ref().unwrap().clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let consensus_guard = consensus.read().await;
                        if let Ok(selected_validator) = consensus_guard.select_next_validator() {
                            log::info!("Selected validator for next block: {}", selected_validator.as_str());
                        }
                    }
                    // This allows the task to be cancelled
                    _ = tokio::task::yield_now() => {
                        // Yield to allow cancellation
                    }
                }
            }
        });

        self.block_production_handle = Some(handle);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if !self.is_running {
            return Err(BlockchainError::Network("Node is not running".to_string()));
        }

        log::info!("Stopping Erbium Node...");
        self.is_running = false;

        // Send shutdown signal to RPC server first
        if let Some(shutdown_tx) = self.shutdown_sender.take() {
            let _ = shutdown_tx.send(());
        }

        // Give a moment for graceful shutdown
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Abort all running tasks with timeout
        let stop_futures = vec![
            ("REST API", self.rest_server_handle.take()),
            ("RPC API", self.rpc_server_handle.take()),
            ("Block production", self.block_production_handle.take()),
            ("Bridge monitoring", self.bridge_monitor_handle.take()),
            ("P2P Network", self.network_handle.take()),
        ];

        for (name, handle_opt) in stop_futures {
            if let Some(handle) = handle_opt {
                handle.abort();
                match tokio::time::timeout(tokio::time::Duration::from_secs(2), handle).await {
                    Ok(_) => log::debug!("{} stopped gracefully", name),
                    Err(_) => log::warn!("{} forced to stop after timeout", name),
                }
            }
        }

        log::info!("All node services stopped successfully");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn has_blockchain(&self) -> bool {
        self.blockchain.is_some()
    }

    pub fn has_consensus(&self) -> bool {
        self.consensus.is_some()
    }

    pub fn has_bridge_manager(&self) -> bool {
        self.bridge_manager.is_some()
    }

    pub fn has_p2p_network(&self) -> bool {
        self.p2p_network.is_some()
    }

    pub fn has_erbium_engine(&self) -> bool {
        self.erbium_engine.is_some()
    }

    pub fn get_erbium_engine(&self) -> Option<&ErbiumEngine> {
        self.erbium_engine.as_ref().map(|v| v.as_ref())
    }

    async fn start_metrics_server(&mut self) -> Result<()> {
        if let Some(metrics) = &self.metrics {
            // SECURITY: No localhost defaults - requires explicit configuration
            if let Err(e) = metrics.start_server("0.0.0.0:0") {
                // Bind to all interfaces, random port
                log::warn!("Failed to start metrics server: {}", e);
                return Err(BlockchainError::Other(format!(
                    "Failed to start metrics server: {}",
                    e
                )));
            }
            log::info!("Metrics server started on 0.0.0.0:0 (random port - configure explicitly in production)");
        } else {
            log::warn!("Metrics not initialized, skipping metrics server");
        }
        Ok(())
    }

    async fn start_bridge_monitoring(&mut self) -> Result<()> {
        let bridge_manager = self.bridge_manager.as_ref().unwrap().clone();

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // Check every minute

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let _bridge_guard = bridge_manager.write().await;

                        // Process any pending bridge operations
                        // This would be expanded with actual bridge monitoring logic
                        log::debug!("Bridge monitoring tick");
                    }
                    _ = tokio::task::yield_now() => {
                        // Yield to allow cancellation
                    }
                }
            }
        });

        self.bridge_monitor_handle = Some(handle);
        log::info!("Bridge monitoring started");

        Ok(())
    }

    async fn start_staking_services(&mut self) -> Result<()> {
        let consensus = self.consensus.as_ref().unwrap().clone();

        let _handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let mut consensus_guard = consensus.write().await;

                        // Process staking rewards
                        if let Err(e) = consensus_guard.process_staking_rewards() {
                            log::error!("Failed to process staking rewards: {}", e);
                        }

                        // Check for validator slashing conditions
                        if let Err(e) = consensus_guard.check_validator_slashing() {
                            log::error!("Failed to check validator slashing: {}", e);
                        }
                    }
                    _ = tokio::task::yield_now() => {
                        // Yield to allow cancellation
                    }
                }
            }
        });

        // Store the staking handle (we'd need to add this to the struct)
        // For now, we'll let it run independently
        log::info!("Staking services started");

        Ok(())
    }

    async fn start_p2p_network(&mut self) -> Result<()> {
        if self.p2p_network.is_none() {
            log::warn!("P2P network not available, cannot start network layer");
            return Ok(());
        }

        let p2p_network = self.p2p_network.as_ref().unwrap().clone();

        let handle = tokio::spawn(async move {
            log::info!("Starting P2P network layer");

            if let Err(e) = p2p_network.write().await.start().await {
                log::error!("Failed to start P2P network: {}", e);
                return;
            }

            log::info!("P2P network started, broadcasting transactions activated");

            // Keep the network alive
            tokio::signal::ctrl_c().await.unwrap();
            log::info!("Shutting down P2P network...");
        });

        self.network_handle = Some(handle);
        log::info!("P2P network initialization complete");
        Ok(())
    }

    /// Initialize genesis validators with their initial stake
    async fn initialize_genesis_validators(
        consensus: &mut crate::consensus::pos::ProofOfStake,
    ) -> Result<()> {
        use std::fs;
        use std::path::Path;
        use crate::core::types::Address;

        let genesis_config_path = "config/genesis/allocations.toml";
        
        if !Path::new(genesis_config_path).exists() {
            log::warn!("Genesis config not found at {}", genesis_config_path);
            return Ok(());
        }

        let config_content = fs::read_to_string(genesis_config_path).map_err(|e| {
            BlockchainError::Storage(format!("Failed to read genesis config: {}", e))
        })?;

        let genesis_config: crate::core::chain::GenesisConfig =
            toml::from_str(&config_content).map_err(|e| {
                BlockchainError::Storage(format!("Failed to parse genesis config: {}", e))
            })?;

        // Process initial validators
        for validator in &genesis_config.genesis.initial_validators {
            let address = Address::new(validator.address.clone()).map_err(|e| {
                BlockchainError::InvalidTransaction(format!(
                    "Invalid validator address {}: {}",
                    validator.address, e
                ))
            })?;

            let stake: u64 = validator.stake.parse().map_err(|e| {
                BlockchainError::InvalidTransaction(format!(
                    "Invalid validator stake {}: {}",
                    validator.stake, e
                ))
            })?;

            // Add validator stake to consensus
            consensus.add_stake(address.clone(), stake)?;
            
            log::info!(
                "Initialized genesis validator: {} with {} ERB staked",
                validator.address,
                stake
            );
        }

        if !genesis_config.genesis.initial_validators.is_empty() {
            log::info!(
                "Successfully initialized {} genesis validators",
                genesis_config.genesis.initial_validators.len()
            );
        }

        Ok(())
    }
}
