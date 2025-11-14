use crate::utils::error::{Result, BlockchainError};
use crate::bridges::core::bridge_manager::BridgeManager;
use crate::node::node_metrics::NodeMetrics;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub struct NodeManager {
    blockchain: Option<Arc<RwLock<crate::core::chain::Blockchain>>>,
    consensus: Option<Arc<RwLock<crate::consensus::pos::ProofOfStake>>>,
    bridge_manager: Option<Arc<RwLock<BridgeManager>>>,
    metrics: Option<NodeMetrics>,
    is_running: bool,
    rest_server_handle: Option<JoinHandle<()>>,
    rpc_server_handle: Option<JoinHandle<()>>,
    block_production_handle: Option<JoinHandle<()>>,
    bridge_monitor_handle: Option<JoinHandle<()>>,
    shutdown_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl NodeManager {
    pub async fn new() -> Result<Self> {
        log::info!("Initializing Erbium Node Manager...");

        let blockchain = match crate::core::chain::Blockchain::new_with_database("./erbium-data").await {
            Ok(bc) => Some(Arc::new(RwLock::new(bc))),
            Err(e) => {
                log::warn!("Failed to initialize blockchain with database: {}. Falling back to in-memory", e);
                // Fallback to in-memory blockchain
                match crate::core::chain::Blockchain::new() {
                    Ok(bc) => Some(Arc::new(RwLock::new(bc))),
                    Err(e2) => {
                        log::error!("Failed to initialize blockchain even in-memory: {}", e2);
                        None
                    }
                }
            }
        };

        let consensus_config = crate::consensus::ConsensusConfig::default();
        let consensus = crate::consensus::pos::ProofOfStake::new(consensus_config);
        let consensus = Some(Arc::new(RwLock::new(consensus)));

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
            metrics,
            is_running: false,
            rest_server_handle: None,
            rpc_server_handle: None,
            block_production_handle: None,
            bridge_monitor_handle: None,
            shutdown_sender: None,
        })
    }
    
    pub async fn start(&mut self) -> Result<()> {
        if self.is_running {
            return Err(BlockchainError::Network("Node is already running".to_string()));
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

        log::info!("Erbium Node started successfully");
        Ok(())
    }
    
    async fn start_rest_api(&mut self) -> Result<()> {
        // Temporarily disable REST API server for v1.0.0 release
        log::warn!("REST API server disabled for v1.0.0 release");
        Ok(())
    }
    
    async fn start_rpc_api(&mut self) -> Result<()> {
        // Temporarily disable RPC API server for v1.0.0 release
        log::warn!("RPC API server disabled for v1.0.0 release");
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
        ];
        
        for (name, handle_opt) in stop_futures {
            if let Some(handle) = handle_opt {
                handle.abort();
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(2),
                    handle
                ).await {
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
    
    async fn start_metrics_server(&mut self) -> Result<()> {
        if let Some(metrics) = &self.metrics {
            // Iniciar o servidor de métricas na porta 9090
            if let Err(e) = metrics.start_server("127.0.0.1:9090") {
                log::warn!("Failed to start metrics server: {}", e);
                return Err(BlockchainError::Other(format!("Failed to start metrics server: {}", e)));
            }
            log::info!("Metrics server started on port 9090");
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
}
