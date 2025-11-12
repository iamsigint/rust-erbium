// src/api/websocket.rs
use warp::ws::{WebSocket, Ws, Message};
use warp::Filter;
use futures_util::{SinkExt, StreamExt}; // CORRECTION: Added SinkExt
use crate::utils::error::BlockchainError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

#[derive(Clone)]
pub struct WebSocketServer {
    port: u16,
    clients: Arc<RwLock<HashMap<usize, broadcast::Sender<String>>>>,
    next_client_id: Arc<RwLock<usize>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage {
    pub r#type: String,
    pub data: serde_json::Value,
}

impl WebSocketServer {
    pub fn new(port: u16) -> Result<Self, BlockchainError> {
        Ok(Self {
            port,
            clients: Arc::new(RwLock::new(HashMap::new())),
            next_client_id: Arc::new(RwLock::new(0)),
        })
    }
    
    pub async fn start(&self) -> Result<(), BlockchainError> {
        let clients = self.clients.clone();
        let next_client_id = self.next_client_id.clone();
        
        let ws_route = warp::path("ws")
            .and(warp::ws())
            .map(move |ws: Ws| {
                let clients = clients.clone();
                let next_client_id = next_client_id.clone();
                
                ws.on_upgrade(move |socket| {
                    Self::handle_connection(socket, clients, next_client_id)
                })
            });
        
        log::info!("Starting WebSocket server on port {}", self.port);
        warp::serve(ws_route)
            .run(([0, 0, 0, 0], self.port))
            .await;
        
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<(), BlockchainError> {
        // Close all client connections
        let mut clients = self.clients.write().await;
        clients.clear();
        Ok(())
    }
    
    async fn handle_connection(
        ws: WebSocket,
        clients: Arc<RwLock<HashMap<usize, broadcast::Sender<String>>>>,
        next_client_id: Arc<RwLock<usize>>,
    ) {
        let client_id = {
            let mut id_lock = next_client_id.write().await;
            let id = *id_lock;
            *id_lock += 1;
            id
        };
        
        log::info!("WebSocket client connected: {}", client_id);
        
        // Create broadcast channel for this client
        let (tx, mut rx) = broadcast::channel(100);
        
        {
            clients.write().await.insert(client_id, tx);
        }
        
        // Split WebSocket into sender and receiver
        let (mut ws_tx, mut ws_rx) = ws.split();
        
        // Clone clients for the receive task
        let clients_for_recv = clients.clone();
        
        // Task to send messages to WebSocket
        let send_task = tokio::spawn(async move {
            while let Ok(message) = rx.recv().await {
                if ws_tx.send(Message::text(message)).await.is_err() {
                    break;
                }
            }
        });
        
        // Task to receive messages from WebSocket
        let recv_task = tokio::spawn(async move {
            while let Some(result) = ws_rx.next().await {
                match result {
                    Ok(message) => {
                        if let Ok(text) = message.to_str() {
                            Self::handle_message(client_id, text, &clients_for_recv).await;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        
        // Wait for either task to finish
        tokio::select! {
            _ = send_task => {},
            _ = recv_task => {},
        }
        
        // Cleanup
        clients.write().await.remove(&client_id);
        log::info!("WebSocket client disconnected: {}", client_id);
    }
    
    async fn handle_message(
        client_id: usize,
        message: &str,
        clients: &Arc<RwLock<HashMap<usize, broadcast::Sender<String>>>>,
    ) {
        log::debug!("WebSocket message from client {}: {}", client_id, message);
        
        // Parse message
        let ws_message: Result<WsMessage, _> = serde_json::from_str(message);
        if let Ok(ws_message) = ws_message {
            match ws_message.r#type.as_str() {
                "subscribe" => {
                    Self::handle_subscribe(client_id, &ws_message.data, clients).await;
                }
                "unsubscribe" => {
                    Self::handle_unsubscribe(client_id, &ws_message.data, clients).await;
                }
                "ping" => {
                    let pong_message = WsMessage {
                        r#type: "pong".to_string(),
                        data: serde_json::json!({}),
                    };
                    Self::send_to_client(client_id, &pong_message, clients).await;
                }
                _ => {
                    let error = WsMessage {
                        r#type: "error".to_string(),
                        data: serde_json::json!({
                            "message": "Unknown message type"
                        }),
                    };
                    Self::send_to_client(client_id, &error, clients).await;
                }
            }
        } else {
            let error = WsMessage {
                r#type: "error".to_string(),
                data: serde_json::json!({
                    "message": "Invalid message format"
                }),
            };
            Self::send_to_client(client_id, &error, clients).await;
        }
    }
    
    async fn handle_subscribe(
        client_id: usize,
        data: &serde_json::Value,
        clients: &Arc<RwLock<HashMap<usize, broadcast::Sender<String>>>>,
    ) {
        if let Some(topics) = data.get("topics").and_then(|t| t.as_array()) {
            for topic in topics {
                if let Some(topic_str) = topic.as_str() {
                    log::info!("Client {} subscribed to topic: {}", client_id, topic_str);
                    
                    // Send initial data for the topic
                    match topic_str {
                        "blocks" => {
                            Self::send_new_block_notification(client_id, clients).await;
                        }
                        "transactions" => {
                            Self::send_new_transaction_notification(client_id, clients).await;
                        }
                        "validators" => {
                            Self::send_validator_update(client_id, clients).await;
                        }
                        _ => {}
                    }
                }
            }
            
            let response = WsMessage {
                r#type: "subscribed".to_string(),
                data: serde_json::json!({
                    "topics": topics,
                    "message": "Successfully subscribed"
                }),
            };
            Self::send_to_client(client_id, &response, clients).await;
        }
    }
    
    async fn handle_unsubscribe(
        client_id: usize,
        data: &serde_json::Value,
        clients: &Arc<RwLock<HashMap<usize, broadcast::Sender<String>>>>,
    ) {
        if let Some(topics) = data.get("topics").and_then(|t| t.as_array()) {
            for topic in topics {
                if let Some(topic_str) = topic.as_str() {
                    log::info!("Client {} unsubscribed from topic: {}", client_id, topic_str);
                }
            }
            
            let response = WsMessage {
                r#type: "unsubscribed".to_string(),
                data: serde_json::json!({
                    "topics": topics,
                    "message": "Successfully unsubscribed"
                }),
            };
            Self::send_to_client(client_id, &response, clients).await;
        }
    }
    
    async fn send_to_client(
        client_id: usize,
        message: &WsMessage,
        clients: &Arc<RwLock<HashMap<usize, broadcast::Sender<String>>>>,
    ) {
        if let Ok(json) = serde_json::to_string(message) {
            if let Some(client) = clients.read().await.get(&client_id) {
                let _ = client.send(json);
            }
        }
    }
    
    async fn send_to_client_by_type(
        client_id: usize,
        message_type: &str,
        data: serde_json::Value,
        clients: &Arc<RwLock<HashMap<usize, broadcast::Sender<String>>>>,
    ) {
        let message = WsMessage {
            r#type: message_type.to_string(),
            data,
        };
        Self::send_to_client(client_id, &message, clients).await;
    }
    
    // Example notification methods
    async fn send_new_block_notification(
        client_id: usize,
        clients: &Arc<RwLock<HashMap<usize, broadcast::Sender<String>>>>,
    ) {
        let block_data = serde_json::json!({
            "hash": "0xnewblock",
            "height": 1001,
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "transactionCount": 5
        });
        
        Self::send_to_client_by_type(client_id, "newBlock", block_data, clients).await;
    }
    
    async fn send_new_transaction_notification(
        client_id: usize,
        clients: &Arc<RwLock<HashMap<usize, broadcast::Sender<String>>>>,
    ) {
        let tx_data = serde_json::json!({
            "hash": "0xnewtransaction",
            "from": "0xsender",
            "to": "0xrecipient",
            "amount": 500,
            "fee": 5
        });
        
        Self::send_to_client_by_type(client_id, "newTransaction", tx_data, clients).await;
    }
    
    async fn send_validator_update(
        client_id: usize,
        clients: &Arc<RwLock<HashMap<usize, broadcast::Sender<String>>>>,
    ) {
        let validator_data = serde_json::json!({
            "address": "0xvalidator",
            "stake": "1000000",
            "active": true,
            "updateType": "heartbeat"
        });
        
        Self::send_to_client_by_type(client_id, "validatorUpdate", validator_data, clients).await;
    }
    
    // Method to broadcast to all clients
    pub async fn broadcast(&self, message: WsMessage) -> Result<(), BlockchainError> {
        if let Ok(json) = serde_json::to_string(&message) {
            let clients = self.clients.read().await;
            for (client_id, sender) in clients.iter() {
                if sender.receiver_count() > 0 {
                    if let Err(e) = sender.send(json.clone()) {
                        log::warn!("Failed to send to client {}: {}", client_id, e);
                    }
                }
            }
        }
        Ok(())
    }
}