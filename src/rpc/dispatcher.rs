//! JSON-RPC 2.0 Dispatcher
//!
//! Routes JSON-RPC method calls to appropriate handler functions.
//! This dispatcher maintains a clean separation between RPC protocol
//! and business logic by delegating to functional modules.

// Allow unused code until RPC server is fully implemented
#![allow(dead_code)]
#![allow(unused_variables)]

use serde_json::{json, Value};
use std::collections::HashMap;

// Include all functional route modules
mod accounts { include!("../routes/accounts.rs"); }
mod blocks { include!("../routes/blocks.rs"); }
mod health { include!("../routes/health.rs"); }
mod mempool { include!("../routes/mempool.rs"); }
mod peers { include!("../routes/peers.rs"); }

// Method registry mapping string methods to handler functions
pub fn get_method_handlers() -> HashMap<&'static str, fn(Value) -> Result<Value, String>> {
    let mut handlers = HashMap::new();

    // Account methods
    handlers.insert("getBalance", accounts::get_balance as fn(Value) -> Result<Value, String>);
    handlers.insert("getAccountInfo", accounts::get_account_info as fn(Value) -> Result<Value, String>);
    handlers.insert("getAccountStats", accounts::get_account_stats as fn(Value) -> Result<Value, String>);

    // Block methods
    handlers.insert("getBlock", blocks::get_block as fn(Value) -> Result<Value, String>);
    handlers.insert("getLatestBlock", blocks::get_latest_block as fn(Value) -> Result<Value, String>);
    handlers.insert("getBlockStats", blocks::get_block_stats as fn(Value) -> Result<Value, String>);

    // Health methods
    handlers.insert("health", health::health_check as fn(Value) -> Result<Value, String>);
    handlers.insert("ping", health::ping as fn(Value) -> Result<Value, String>);

    // Mempool methods
    handlers.insert("getMempoolStats", mempool::get_mempool_stats as fn(Value) -> Result<Value, String>);
    handlers.insert("getMempoolTransactions", mempool::get_mempool_transactions as fn(Value) -> Result<Value, String>);

    // Peer/Network methods
    handlers.insert("getPeerInfo", peers::get_peer_info as fn(Value) -> Result<Value, String>);
    handlers.insert("getNetworkStats", peers::get_network_stats as fn(Value) -> Result<Value, String>);

    handlers
}

/// Main dispatcher function
///
/// Takes a JSON-RPC method name and parameters, routes to appropriate handler,
/// and returns the result in JSON-RPC 2.0 format.
pub async fn dispatch(method: &str, params: Value) -> Result<Value, String> {
    let handlers = get_method_handlers();

    match handlers.get(method) {
        Some(handler) => handler(params),
        None => Err(format!("Method '{}' not found", method)),
    }
}

/// Helper function to create JSON-RPC success response
pub fn success_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "result": result,
        "id": id
    })
}

/// Helper function to create JSON-RPC error response
pub fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "error": {
            "code": code,
            "message": message
        },
        "id": id
    })
}

/// JSON-RPC 2.0 standard error codes
pub mod error_codes {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL_ERROR: i64 = -32603;
}

/// Validate JSON-RPC 2.0 request format
pub fn validate_request(request: &Value) -> Result<(), String> {
    // Check if it's a valid JSON-RPC 2.0 request object
    if !request.is_object() {
        return Err("Request must be a JSON object".to_string());
    }

    let request = request.as_object().unwrap();

    // Check jsonrpc version
    if let Some(version) = request.get("jsonrpc") {
        if version != "2.0" {
            return Err("Only JSON-RPC 2.0 is supported".to_string());
        }
    } else {
        return Err("Missing 'jsonrpc' field".to_string());
    }

    // Check method
    if request.get("method").is_none() {
        return Err("Missing 'method' field".to_string());
    }

    Ok(())
}
