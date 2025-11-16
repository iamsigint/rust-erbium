//! JSON-RPC 2.0 API Server
//!
//! Provides a single `/rpc` endpoint that handles JSON-RPC 2.0 requests.
//! This implements blockchain-standard RPC like Bitcoin, Ethereum, Solana.
//!
//! ⚠️  PLACEHOLDER IMPLEMENTATION:
//! Server is not fully active until axum/web framework dependencies are added.
//! Currently provides the structural foundation for future implementation.

// Allow unused code until RPC server implementation is complete
#![allow(dead_code)]
#![allow(unused_variables)]

use serde_json::Value;

mod dispatcher;

use dispatcher::{dispatch, success_response, error_response, validate_request, error_codes};

/// Shared application state
pub struct AppState {
    // Add shared state here if needed (blockchain, mempool, etc.)
}

// Create the JSON-RPC router - placeholder until axum dependency is added
// TODO: Uncomment and add axum to Cargo.toml when ready to activate HTTP server
/*
pub fn router() -> axum::Router<()> {
    axum::Router::new().route("/rpc", axum::routing::post(rpc_handler))
}
*/

// JSON-RPC 2.0 handler for POST /rpc
// TODO: Uncomment when axum dependency is added
/*
async fn rpc_handler(
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    // Handle both single requests and batch requests
    if payload.is_array() {
        // Batch request
        let mut responses = Vec::new();

        if let Some(array) = payload.as_array() {
            for request in array {
                let response = handle_single_request(request.clone()).await;
                responses.push(response);
            }
        }

        (axum::http::StatusCode::OK, axum::Json(responses))

    } else {
        // Single request
        let response = handle_single_request(payload).await;
        (axum::http::StatusCode::OK, axum::Json(vec![response]))
    }
}
*/

/// Handle a single JSON-RPC request
async fn handle_single_request(request: Value) -> Value {
    // Extract request ID for response
    let request_id = request.get("id").cloned().unwrap_or(Value::Null);

    // Validate request format
    if let Err(err_msg) = validate_request(&request) {
        return error_response(
            request_id.clone(),
            error_codes::INVALID_REQUEST,
            err_msg,
        );
    }

    // Extract method and params
    let method = request.get("method")
        .and_then(|m| m.as_str())
        .unwrap_or("");

    let params = request.get("params")
        .cloned()
        .unwrap_or(Value::Null);

    // Dispatch to appropriate handler
    match dispatch(method, params).await {
        Ok(result) => success_response(request_id, result),
        Err(err_msg) => {
            // Determine error code based on error message
            let error_code = if err_msg.contains("not found") {
                error_codes::METHOD_NOT_FOUND
            } else {
                error_codes::INTERNAL_ERROR
            };

            error_response(request_id, error_code, err_msg)
        }
    }
}
