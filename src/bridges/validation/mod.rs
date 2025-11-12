// src/bridges/validation/mod.rs
pub mod cross_chain_validator;

pub use cross_chain_validator::{CrossChainValidator, ValidationRules, BridgeConfig, CrossChainTransaction, ValidationResult, ValidationMetrics};
