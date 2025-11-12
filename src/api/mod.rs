pub mod rpc;
pub mod rest;
pub mod websocket;
pub mod graphql;

use crate::utils::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub rpc_enabled: bool,
    pub rpc_port: u16,
    pub rest_enabled: bool,
    pub rest_port: u16,
    pub websocket_enabled: bool,
    pub websocket_port: u16,
    pub graphql_enabled: bool,
    pub graphql_port: u16,
    pub max_connections: u32,
    pub rate_limit: u32, // requests per minute
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            rpc_enabled: true,
            rpc_port: 8545,
            rest_enabled: true,
            rest_port: 8080,
            websocket_enabled: true,
            websocket_port: 8546,
            graphql_enabled: false, // Disabled by default
            graphql_port: 8081,
            max_connections: 100,
            rate_limit: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub version: String,
    pub network_id: u64,
    pub block_height: u64,
    pub sync_status: String,
    pub peers_count: usize,
    pub is_mining: bool,
}

pub struct ApiServer {
    config: ApiConfig,
}

impl ApiServer {
    pub fn new(config: ApiConfig) -> Result<Self> {
        Ok(Self {
            config,
        })
    }
    
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting API servers...");
        
        if self.config.rpc_enabled {
            let cfg = crate::node::config::NodeConfig::default();
            let _rpc_server = rpc::RpcServer::new(self.config.rpc_port, cfg)?;
            // Note: RpcServer::start is async but we're not awaiting it properly here
            // In real implementation, you'd spawn tasks for each server
            log::info!("JSON-RPC server configured for port {}", self.config.rpc_port);
        }
        
        if self.config.rest_enabled {
            log::info!("REST API server configured for port {}", self.config.rest_port);
        }
        
        if self.config.websocket_enabled {
            log::info!("WebSocket server configured for port {}", self.config.websocket_port);
        }
        
        if self.config.graphql_enabled {
            log::info!("GraphQL server configured for port {}", self.config.graphql_port);
        }
        
        log::info!("API servers configured successfully (implementation pending)");
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping API servers...");
        log::info!("All API servers stopped");
        Ok(())
    }
    
    pub fn get_node_info(&self) -> NodeInfo {
        NodeInfo {
            version: "0.1.0".to_string(),
            network_id: 1,
            block_height: 0,
            sync_status: "syncing".to_string(),
            peers_count: 0,
            is_mining: false,
        }
    }
}