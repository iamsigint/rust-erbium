use erbium_blockchain::node::manager::NodeManager;
use erbium_blockchain::utils::logger;
use clap::Parser;
use tokio::signal;

#[derive(Parser)]
#[command(name = "erbium-node")]
#[command(about = "Erbium Blockchain Node")]
struct Args {
    /// Data directory for blockchain storage
    #[arg(long, default_value = "./erbium-data")]
    data_dir: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logging
    if let Err(e) = logger::setup_logger() {
        eprintln!("Failed to setup logger: {}", e);
        return Ok(());
    }

    log::info!("Starting Erbium Blockchain Node v{}", env!("CARGO_PKG_VERSION"));
    log::info!("Data directory: {}", args.data_dir);

    // Create and start the node manager
    let mut node_manager = match NodeManager::new().await {
        Ok(nm) => nm,
        Err(e) => {
            log::error!("Failed to create node manager: {}", e);
            return Ok(());
        }
    };

    match node_manager.start().await {
        Ok(_) => {},
        Err(e) => {
            log::error!("Failed to start node: {}", e);
            return Ok(());
        }
    };

    log::info!("Erbium Node started successfully");
    log::info!("Press Ctrl+C to stop...");

    // Wait for shutdown signal
    if let Err(e) = signal::ctrl_c().await {
        log::error!("Failed to listen for shutdown signal: {}", e);
    }

    log::info!("Shutdown signal received, stopping node...");

    // Gracefully stop the node
    match node_manager.stop().await {
        Ok(_) => {},
        Err(e) => log::error!("Error stopping node: {}", e)
    };

    log::info!("Erbium Node stopped successfully");
    Ok(())
}
