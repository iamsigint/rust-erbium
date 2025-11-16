use erbium_blockchain::core::chain::Blockchain;
use std::fs;
use std::path::Path;

// Initialize basic logging
fn init_logging() {
    env_logger::init();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    println!("Erbium Genesis System Test");
    println!("==========================\n");

    // Check if configuration file exists
    let genesis_config_path = "config/genesis/allocations.toml";
    println!("1. Checking genesis configuration file...");
    if Path::new(genesis_config_path).exists() {
        println!("   File found: {}", genesis_config_path);
        match fs::read_to_string(genesis_config_path) {
            Ok(content) => {
                println!("   File content (first 200 chars):");
                println!("   {}", &content.chars().take(200).collect::<String>());
                println!();
            }
            Err(e) => {
                println!("   Error reading file: {}", e);
            }
        }
    } else {
        println!("   File NOT found: {}", genesis_config_path);
        println!("   Current directory: {:?}", std::env::current_dir()?);
        println!("   Listing config/ directory:");
        if let Ok(entries) = std::fs::read_dir("config") {
            for entry in entries {
                if let Ok(entry) = entry {
                    println!("      - {}", entry.file_name().to_string_lossy());
                }
            }
        }
        println!();
    }

    // Create blockchain (this will load genesis allocations)
    println!("2. Creating blockchain with genesis allocations...");
    let blockchain = Blockchain::new()?;
    println!(
        "   Blockchain created with {} blocks",
        blockchain.get_block_height()
    );

    // Check genesis block
    if let Some(genesis_block) = blockchain.get_latest_block() {
        println!(
            "   Genesis block contains {} transactions",
            genesis_block.transactions.len()
        );

        if !genesis_block.transactions.is_empty() {
            println!("   Genesis allocations found:");
            for (i, tx) in genesis_block.transactions.iter().enumerate() {
                println!("      {}. {} ERB to {}", i + 1, tx.amount, tx.to.as_str());
            }
        } else {
            println!("   No genesis allocation transactions found");
        }
    }

    // Check total supply
    let total_supply = blockchain.state.get_total_supply();
    println!("\n3. Network total supply: {} ERB", total_supply);

    // Check development faucet
    let faucet_addr = erbium_blockchain::core::types::Address::new_unchecked(
        "0x0000000000000000000000000000000000000001".to_string(),
    );
    let faucet_balance = blockchain.state.get_balance(&faucet_addr)?;
    println!("4. Development faucet: {} ERB", faucet_balance);

    // Demonstrate ERB units system
    println!("\n5. ERB Units System:");
    use erbium_blockchain::core::units::{constants, ErbAmount, Unit};

    println!("   Available units:");
    println!("      - 1 ion = 0.00000001 ERB (minimum unit)");
    println!("      - 1 μERB = 0.000001 ERB (microERB)");
    println!("      - 1 mERB = 0.001 ERB (milliERB)");
    println!("      - 1 cERB = 0.01 ERB (centiERB)");
    println!("      - 1 dERB = 0.1 ERB (deciERB)");
    println!("      - 1 ERB = 1.0 ERB (main unit)");
    println!("      - 1 kERB = 1,000 ERB (kiloERB)");
    println!("      - 1 M-ERB = 1,000,000 ERB (megaERB)");

    println!("\n   Conversion examples:");
    let examples = vec![
        ErbAmount::from_erb(1.5),
        ErbAmount::from_erb(0.1),
        ErbAmount::from_erb(0.001),
        ErbAmount::from_ions(150),
        ErbAmount::from_ions(1),
    ];

    for example in examples {
        println!(
            "      - {} = {} = {}",
            example.format(Unit::ERB),
            example.format(Unit::Ion),
            example.format_auto()
        );
    }

    // Check configured genesis allocations
    println!("\n6. Checking configured genesis allocations:");

    // Try to read configuration to verify addresses
    if let Ok(config_content) = fs::read_to_string("config/genesis/allocations.toml") {
        if let Ok(genesis_config) =
            toml::from_str::<erbium_blockchain::core::chain::GenesisConfig>(&config_content)
        {
            println!("   Configured genesis allocations:");
            for (i, allocation) in genesis_config.genesis.initial_balances.iter().enumerate() {
                let allocation_type = match i {
                    0 => "Personal Reserve",
                    1 => "Development Fund",
                    2 => "Ecosystem Fund",
                    3 => "Treasury",
                    _ => "Other",
                };

                let amount_ions = allocation.amount.parse::<u128>().unwrap_or(0);
                let amount = ErbAmount::from_ions(amount_ions);
                println!(
                    "      {}. {}: {} ({})",
                    i + 1,
                    allocation_type,
                    amount.format_auto(),
                    allocation.address
                );

                // Check if address is valid (not placeholder)
                if allocation.address == "0x0000000000000000000000000000000000000000" {
                    println!("         WARNING: PLACEHOLDER ADDRESS - REPLACE BEFORE PRODUCTION!");
                } else if let Ok(address) =
                    erbium_blockchain::core::types::Address::new(allocation.address.clone())
                {
                    if let Ok(balance) = blockchain.state.get_balance(&address) {
                        let balance_amount = ErbAmount::from_ions(balance as u128);
                        println!("         Current balance: {}", balance_amount.format_auto());
                    } else {
                        println!("         Error checking balance");
                    }
                } else {
                    println!("         Invalid address");
                }
            }

            let total_allocated: u128 = genesis_config
                .genesis
                .initial_balances
                .iter()
                .map(|a| a.amount.parse::<u128>().unwrap_or(0))
                .sum();
            let total_amount = ErbAmount::from_ions(total_allocated);
            println!("   Total pre-allocated: {}", total_amount.format_auto());
        }
    }

    // Demonstrate constants
    println!("\n7. Important constants:");
    println!("   - Maximum supply: {} ERB", ErbAmount::MAX_SUPPLY_ERB);
    println!("   - Maximum supply: {} ions", ErbAmount::MAX_SUPPLY_IONS);
    println!(
        "   - Personal reserve: {}",
        constants::PERSONAL_RESERVE.format_auto()
    );
    println!(
        "   - Development fund: {}",
        constants::DEV_FUND.format_auto()
    );
    println!("   - Ecosystem fund: {}", constants::ECO_FUND.format_auto());
    println!("   - Treasury: {}", constants::TREASURY.format_auto());

    println!("\nTest completed!");
    println!("\nSummary:");
    println!("   - Blocks in chain: {}", blockchain.get_block_height());
    println!("   - Total supply: {} ERB", total_supply);
    println!(
        "   - Genesis transactions: {}",
        blockchain
            .get_latest_block()
            .map(|b| b.transactions.len())
            .unwrap_or(0)
    );

    Ok(())
}
