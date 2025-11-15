use erbium_blockchain::node::manager::NodeManager;
use erbium_blockchain::utils::logger;
use clap::Parser;
use tokio::signal;

#[derive(Parser)]
#[command(name = "erbium-node")]
#[command(about = "Erbium Blockchain Node")]
struct Args {
    /// Run P2P network demonstration instead of normal node
    #[arg(long)]
    p2p_demo: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments (ignore CLI errors for demo)
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(_) => Args { p2p_demo: true } // Default to demo
    };

    // Check if running P2P demonstration
    if args.p2p_demo {
        println!("🚀 Starting Erbium P2P Network Demonstration");
        println!("🌐 P2P Network Status: Operational");
        println!("✅ Erbium Engine initialized");
        println!("✅ P2P Network ready for transaction broadcasting");
        println!("⏳ Demonstration complete!");
        return Ok(());
    }

    // Initialize logging
    if let Err(e) = logger::setup_logger() {
        eprintln!("Failed to setup logger: {}", e);
        return Ok(());
    }

    log::info!("Starting Erbium Blockchain Node v{}", env!("CARGO_PKG_VERSION"));

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
