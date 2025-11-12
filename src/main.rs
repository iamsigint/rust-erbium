use erbium_blockchain::node::{cli::Cli, manager::NodeManager};
use erbium_blockchain::utils::logger;
use clap::Parser;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize logging
    logger::setup_logger()?;

    log::info!("Starting Erbium Blockchain Node v{}", env!("CARGO_PKG_VERSION"));
    log::info!("Network: {}, Data Directory: {}", cli.network, cli.data_dir);

    // Create and start the node manager
    let mut node_manager = NodeManager::new().await?;
    node_manager.start().await?;

    log::info!("Erbium Node started successfully");
    log::info!("REST API: http://127.0.0.1:{}", cli.rest_port);
    log::info!("RPC API: http://127.0.0.1:{}", cli.rpc_port);
    log::info!("Metrics: http://127.0.0.1:9090/metrics");

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    log::info!("Received shutdown signal, stopping node...");

    // Gracefully stop the node
    node_manager.stop().await?;

    log::info!("Erbium Node stopped successfully");
    Ok(())
}
