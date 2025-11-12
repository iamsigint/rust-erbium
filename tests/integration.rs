// Integration tests for Erbium Blockchain
//
// These tests verify that different components work together correctly.

use erbium_blockchain::*;
use erbium_blockchain::core::{Block, Transaction};
use erbium_blockchain::core::types::Address;
use erbium_blockchain::consensus::{ConsensusConfig, pos::ProofOfStake};
use erbium_blockchain::storage::database::Database;

#[tokio::test]
async fn test_basic_transaction_flow() {
    // Create test addresses
    let sender = Address::new("0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string()).unwrap();
    let receiver = Address::new("0x742d35Cc6634C0532925a3b844Bc454e4438f44f".to_string()).unwrap();

    // Create a transfer transaction
    let transaction = Transaction::new_transfer(
        sender.clone(),
        receiver.clone(),
        1000, // amount
        10,   // fee
        1,    // nonce
    );

    // Validate transaction
    assert!(transaction.validate_basic().is_ok());
    assert_eq!(transaction.transaction_type, crate::core::transaction::TransactionType::Transfer);
    assert_eq!(transaction.amount, 1000);
    assert_eq!(transaction.fee, 10);

    // Test transaction hashing
    let hash1 = transaction.hash();
    let hash2 = transaction.hash();
    assert_eq!(hash1, hash2); // Hash should be deterministic

    println!("✅ Transaction validation and hashing works");
}

#[tokio::test]
async fn test_consensus_basic_flow() {
    // Create consensus configuration
    let config = ConsensusConfig::default();

    // Create PoS consensus
    let mut pos = ProofOfStake::new(config);

    // Create test validator
    let validator = Address::new("0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string()).unwrap();

    // Add stake
    pos.add_stake(validator.clone(), 50000).unwrap();

    // Check stake was added
    assert_eq!(pos.get_validator_stake(&validator), Some(50000));
    assert_eq!(pos.total_stake(), 50000);

    // Test validator selection (should work with stake)
    let selected = pos.select_next_validator().unwrap();
    assert_eq!(selected, validator);

    println!("✅ Basic consensus flow works");
}

#[tokio::test]
async fn test_storage_basic_operations() {
    // Create temporary database
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().to_str().unwrap();

    // Create database
    let mut db = Database::new(db_path).unwrap();

    // Test basic put/get
    let key = b"test_key";
    let value = b"test_value";

    db.put(key, value).unwrap();
    let retrieved = db.get(key).unwrap().unwrap();

    assert_eq!(retrieved, value);

    // Test existence check
    assert!(db.exists(key).unwrap());

    // Test delete
    db.delete(key).unwrap();
    assert!(!db.exists(key).unwrap());

    println!("✅ Basic storage operations work");
}

#[tokio::test]
async fn test_block_creation_and_validation() {
    // Create test addresses
    let validator = Address::new("0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string()).unwrap();

    // Create a block with transactions
    let transactions = vec![
        Transaction::new_transfer(
            Address::new("0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string()).unwrap(),
            Address::new("0x742d35Cc6634C0532925a3b844Bc454e4438f44f".to_string()).unwrap(),
            1000, 10, 1,
        )
    ];

    let block = Block::new(
        1, // block number
        crate::core::types::Hash::new(b"genesis"), // previous hash
        transactions,
        validator.as_str().to_string(),
        1000000, // difficulty
    );

    // Test block properties
    assert_eq!(block.header.number, 1);
    assert_eq!(block.header.version, 1);
    assert_eq!(block.transactions.len(), 1);

    // Test block hashing
    let hash1 = block.hash();
    let hash2 = block.hash();
    assert_eq!(hash1, hash2); // Hash should be deterministic

    println!("✅ Block creation and validation works");
}

#[tokio::test]
async fn test_end_to_end_flow() {
    // This test simulates a basic end-to-end flow:
    // 1. Create transaction
    // 2. Add to block
    // 3. Validate with consensus
    // 4. Store in database

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().to_str().unwrap();

    // Setup components
    let mut db = Database::new(db_path).unwrap();
    let config = ConsensusConfig::default();
    let mut pos = ProofOfStake::new(config);

    // Create validator and add stake
    let validator = Address::new("0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string()).unwrap();
    pos.add_stake(validator.clone(), 100000).unwrap();

    // Generate a real Dilithium keypair for the validator
    let keypair = crate::crypto::dilithium::DilithiumKeypair::generate().unwrap();
    pos.set_validator_public_key(validator.clone(), keypair.public_key).unwrap();

    // Create transaction
    let transaction = Transaction::new_transfer(
        Address::new("0x742d35Cc6634C0532925a3b844Bc454e4438f44e".to_string()).unwrap(),
        Address::new("0x742d35Cc6634C0532925a3b844Bc454e4438f44f".to_string()).unwrap(),
        1000, 10, 1,
    );

    // Create block
    let mut block = Block::new(
        1,
        crate::core::types::Hash::new(b"genesis"),
        vec![transaction],
        validator.as_str().to_string(),
        1000000,
    );

    // Sign the block with the validator's private key
    block.sign(&keypair.secret_key).unwrap();

    // Validate block with consensus
    let is_valid = pos.validate_block(&block, &validator).unwrap();
    assert!(is_valid);

    // Store block in database
    let block_key = format!("block:{}", block.header.number);
    let block_data = bincode::serialize(&block).unwrap();
    db.put(block_key.as_bytes(), &block_data).unwrap();

    // Retrieve and verify
    let stored_data = db.get(block_key.as_bytes()).unwrap().unwrap();
    let stored_block: Block = bincode::deserialize(&stored_data).unwrap();
    assert_eq!(stored_block.header.number, block.header.number);

    println!("✅ End-to-end flow works: transaction → block → consensus validation → storage");
}
