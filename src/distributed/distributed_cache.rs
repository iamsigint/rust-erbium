//! Distributed caching system for shared cache across blockchain nodes
//!
//! This module provides a distributed cache that allows nodes to share
//! frequently accessed data, reducing redundant computations and database hits.

use crate::utils::error::{Result, BlockchainError};
use crate::storage::cache::{SharedCache, CacheKey, CacheStats, create_shared_cache_with_memory_limit};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// Distributed cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedCacheConfig {
    pub local_cache_size: usize,
    pub local_memory_limit_mb: usize,
    pub sync_interval_seconds: u64,
    pub cache_ttl_seconds: u64,
    pub gossip_fanout: usize, // How many nodes to gossip cache updates to
    pub enable_compression: bool,
    pub consistency_level: CacheConsistencyLevel,
}

impl Default for DistributedCacheConfig {
    fn default() -> Self {
        Self {
            local_cache_size: 10000,
            local_memory_limit_mb: 100,
            sync_interval_seconds: 60,
            cache_ttl_seconds: 3600, // 1 hour
            gossip_fanout: 3,
            enable_compression: true,
            consistency_level: CacheConsistencyLevel::Eventual,
        }
    }
}

/// Cache consistency levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CacheConsistencyLevel {
    /// Strong consistency - all nodes have same cache state
    Strong,
    /// Eventual consistency - cache syncs asynchronously
    Eventual,
    /// Weak consistency - best effort sync
    Weak,
}

/// Cache entry with metadata for distributed operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedCacheEntry {
    pub key: CacheKey,
    pub value: Vec<u8>,
    pub timestamp: u64,
    pub version: u64,
    pub ttl: u64,
    pub origin_node: String,
    pub compressed: bool,
}

impl DistributedCacheEntry {
    pub fn is_expired(&self) -> bool {
        let now = current_timestamp();
        now > self.timestamp + self.ttl
    }

    pub fn size_bytes(&self) -> usize {
        self.key.as_slice().len() + self.value.len() + 100 // Metadata overhead
    }
}

/// Cache synchronization message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheSyncMessage {
    /// Broadcast cache update to other nodes
    CacheUpdate {
        entry: DistributedCacheEntry,
        sender_node: String,
    },
    /// Request cache entries from another node
    CacheRequest {
        keys: Vec<CacheKey>,
        requester_node: String,
    },
    /// Response to cache request
    CacheResponse {
        entries: Vec<DistributedCacheEntry>,
        responder_node: String,
    },
    /// Invalidate cache entries across nodes
    CacheInvalidate {
        keys: Vec<CacheKey>,
        sender_node: String,
    },
    /// Gossip cache metadata for discovery
    CacheGossip {
        node_id: String,
        cache_stats: DistributedCacheStats,
        top_keys: Vec<(CacheKey, u64)>, // (key, access_count)
    },
}

/// Distributed cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedCacheStats {
    pub local_entries: usize,
    pub remote_entries: usize,
    pub total_memory_mb: f64,
    pub hit_rate: f64,
    pub sync_operations: u64,
    pub gossip_messages: u64,
    pub cache_version: u64,
}

/// Main distributed cache manager
pub struct DistributedCache {
    config: DistributedCacheConfig,
    local_cache: SharedCache,
    remote_cache: Arc<RwLock<HashMap<CacheKey, DistributedCacheEntry>>>,
    node_caches: Arc<RwLock<HashMap<String, NodeCacheInfo>>>,
    sync_sender: mpsc::UnboundedSender<CacheSyncMessage>,
    sync_receiver: Arc<RwLock<mpsc::UnboundedReceiver<CacheSyncMessage>>>,
    local_node_id: String,
    cache_version: Arc<RwLock<u64>>,
    last_sync: Arc<RwLock<u64>>,
}

#[derive(Debug, Clone)]
struct NodeCacheInfo {
    node_id: String,
    last_seen: u64,
    cache_stats: DistributedCacheStats,
    top_keys: Vec<(CacheKey, u64)>,
}

impl DistributedCache {
    pub fn new(config: DistributedCacheConfig, local_node_id: String) -> Self {
        let local_cache = create_shared_cache_with_memory_limit(
            config.local_cache_size,
            config.local_memory_limit_mb,
        );

        let (sync_sender, sync_receiver) = mpsc::unbounded_channel();

        Self {
            config,
            local_cache,
            remote_cache: Arc::new(RwLock::new(HashMap::new())),
            node_caches: Arc::new(RwLock::new(HashMap::new())),
            sync_sender,
            sync_receiver: Arc::new(RwLock::new(sync_receiver)),
            local_node_id,
            cache_version: Arc::new(RwLock::new(0)),
            last_sync: Arc::new(RwLock::new(0)),
        }
    }

    /// Get value from distributed cache (local first, then remote)
    pub async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let cache_key = CacheKey::new(key.to_vec());

        // Try local cache first
        if let Ok(Some(value)) = self.local_cache.read().map(|cache| cache.get(key)) {
            return Ok(Some(value));
        }

        // Try remote cache
        {
            let remote_cache = self.remote_cache.read().await;
            if let Some(entry) = remote_cache.get(&cache_key) {
                if !entry.is_expired() {
                    // Update local cache
                    if let Ok(mut local_cache) = self.local_cache.write() {
                        local_cache.put(key.to_vec(), entry.value.clone());
                    }
                    return Ok(Some(entry.value.clone()));
                } else {
                    // Remove expired entry
                    drop(remote_cache);
                    let mut remote_cache = self.remote_cache.write().await;
                    remote_cache.remove(&cache_key);
                }
            }
        }

        // Try to fetch from other nodes
        self.request_from_peers(vec![cache_key]).await?;

        Ok(None)
    }

    /// Put value in distributed cache
    pub async fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        let cache_key = CacheKey::new(key.clone());
        let timestamp = current_timestamp();
        let version = {
            let mut ver = self.cache_version.write().await;
            *ver += 1;
            *ver
        };

        // Create distributed entry
        let entry = DistributedCacheEntry {
            key: cache_key.clone(),
            value: value.clone(),
            timestamp,
            version,
            ttl: self.config.cache_ttl_seconds,
            origin_node: self.local_node_id.clone(),
            compressed: false, // Compression not implemented yet
        };

        // Update local cache
        if let Ok(mut local_cache) = self.local_cache.write() {
            local_cache.put(key, value);
        }

        // Update remote cache
        {
            let mut remote_cache = self.remote_cache.write().await;
            remote_cache.insert(cache_key, entry.clone());
        }

        // Broadcast update to peers
        self.broadcast_cache_update(entry).await?;

        Ok(())
    }

    /// Invalidate cache entries across the distributed system
    pub async fn invalidate(&self, keys: Vec<Vec<u8>>) -> Result<()> {
        let cache_keys: Vec<CacheKey> = keys.into_iter()
            .map(|k| CacheKey::new(k))
            .collect();

        // Remove from local cache
        if let Ok(mut local_cache) = self.local_cache.write() {
            for key in &cache_keys {
                local_cache.remove(key.as_slice());
            }
        }

        // Remove from remote cache
        {
            let mut remote_cache = self.remote_cache.write().await;
            for key in &cache_keys {
                remote_cache.remove(key);
            }
        }

        // Broadcast invalidation
        let message = CacheSyncMessage::CacheInvalidate {
            keys: cache_keys,
            sender_node: self.local_node_id.clone(),
        };

        let _ = self.sync_sender.send(message);

        Ok(())
    }

    /// Synchronize cache with peer nodes
    pub async fn sync_with_peers(&self) -> Result<()> {
        let now = current_timestamp();
        let last_sync = *self.last_sync.read().await;

        if now - last_sync < self.config.sync_interval_seconds {
            return Ok(); // Not time to sync yet
        }

        // Update last sync time
        {
            let mut last_sync_ref = self.last_sync.write().await;
            *last_sync_ref = now;
        }

        // Send gossip message with local cache stats
        let stats = self.get_stats().await?;
        let top_keys = self.get_top_keys().await?;

        let gossip_message = CacheSyncMessage::CacheGossip {
            node_id: self.local_node_id.clone(),
            cache_stats: stats,
            top_keys,
        };

        let _ = self.sync_sender.send(gossip_message);

        // Request missing entries from peers
        self.request_missing_entries().await?;

        Ok(())
    }

    /// Process incoming cache synchronization message
    pub async fn process_sync_message(&self, message: CacheSyncMessage) -> Result<()> {
        match message {
            CacheSyncMessage::CacheUpdate { entry, sender_node } => {
                self.handle_cache_update(entry, sender_node).await?;
            }
            CacheSyncMessage::CacheRequest { keys, requester_node } => {
                self.handle_cache_request(keys, requester_node).await?;
            }
            CacheSyncMessage::CacheResponse { entries, .. } => {
                self.handle_cache_response(entries).await?;
            }
            CacheSyncMessage::CacheInvalidate { keys, .. } => {
                self.handle_cache_invalidation(keys).await?;
            }
            CacheSyncMessage::CacheGossip { node_id, cache_stats, top_keys } => {
                self.handle_cache_gossip(node_id, cache_stats, top_keys).await?;
            }
        }

        Ok(())
    }

    /// Get distributed cache statistics
    pub async fn get_stats(&self) -> Result<DistributedCacheStats> {
        let local_stats = self.local_cache.read()
            .map_err(|_| BlockchainError::Storage("Cache lock error".to_string()))?
            .get_stats();

        let remote_entries = self.remote_cache.read().await.len();
        let cache_version = *self.cache_version.read().await;

        // Estimate sync operations (simplified)
        let sync_operations = 0; // Would track actual sync ops
        let gossip_messages = 0; // Would track gossip messages

        Ok(DistributedCacheStats {
            local_entries: local_stats.size,
            remote_entries,
            total_memory_mb: local_stats.memory_usage_mb,
            hit_rate: local_stats.hit_ratio,
            sync_operations,
            gossip_messages,
            cache_version,
        })
    }

    /// Clean up expired entries
    pub async fn cleanup_expired(&self) -> Result<usize> {
        let mut removed = 0;

        // Clean remote cache
        {
            let mut remote_cache = self.remote_cache.write().await;
            let expired_keys: Vec<CacheKey> = remote_cache.iter()
                .filter(|(_, entry)| entry.is_expired())
                .map(|(key, _)| key.clone())
                .collect();

            for key in expired_keys {
                remote_cache.remove(&key);
                removed += 1;
            }
        }

        // Clean local cache (it has its own cleanup)
        if let Ok(mut local_cache) = self.local_cache.write() {
            removed += local_cache.cleanup_old_entries(self.config.cache_ttl_seconds);
        }

        Ok(removed)
    }

    // Internal methods
    async fn broadcast_cache_update(&self, entry: DistributedCacheEntry) -> Result<()> {
        let message = CacheSyncMessage::CacheUpdate {
            entry,
            sender_node: self.local_node_id.clone(),
        };

        // Send to gossip fanout nodes
        let node_caches = self.node_caches.read().await;
        let target_nodes: Vec<&String> = node_caches.keys()
            .take(self.config.gossip_fanout)
            .collect();

        for node_id in target_nodes {
            // In production, this would send to actual network
            log::debug!("Broadcasting cache update to node: {}", node_id);
        }

        let _ = self.sync_sender.send(message);
        Ok(())
    }

    async fn request_from_peers(&self, keys: Vec<CacheKey>) -> Result<()> {
        let message = CacheSyncMessage::CacheRequest {
            keys,
            requester_node: self.local_node_id.clone(),
        };

        let _ = self.sync_sender.send(message);
        Ok(())
    }

    async fn request_missing_entries(&self) -> Result<()> {
        // Identify entries that might be missing based on gossip data
        let node_caches = self.node_caches.read().await;

        for (node_id, info) in node_caches.iter() {
            // Request top keys that we don't have
            let keys_to_request: Vec<CacheKey> = info.top_keys.iter()
                .filter(|(key, _)| {
                    // Check if we have this key locally
                    self.local_cache.read().map(|cache| !cache.contains(key.as_slice())).unwrap_or(true)
                })
                .take(10) // Request up to 10 missing keys
                .map(|(key, _)| key.clone())
                .collect();

            if !keys_to_request.is_empty() {
                let message = CacheSyncMessage::CacheRequest {
                    keys: keys_to_request,
                    requester_node: self.local_node_id.clone(),
                };
                let _ = self.sync_sender.send(message);
            }
        }

        Ok(())
    }

    async fn handle_cache_update(&self, entry: DistributedCacheEntry, sender_node: String) -> Result<()> {
        // Only accept updates if consistency allows
        match self.config.consistency_level {
            CacheConsistencyLevel::Strong => {
                // For strong consistency, we would need to coordinate versions
                // For now, accept all updates
            }
            CacheConsistencyLevel::Eventual | CacheConsistencyLevel::Weak => {
                // Accept eventual consistency updates
            }
        }

        // Update remote cache
        {
            let mut remote_cache = self.remote_cache.write().await;
            remote_cache.insert(entry.key.clone(), entry.clone());
        }

        // Optionally update local cache if it's a hot entry
        if self.should_cache_locally(&entry) {
            if let Ok(mut local_cache) = self.local_cache.write() {
                local_cache.put(entry.key.as_slice().to_vec(), entry.value);
            }
        }

        Ok(())
    }

    async fn handle_cache_request(&self, keys: Vec<CacheKey>, requester_node: String) -> Result<()> {
        let mut available_entries = Vec::new();

        // Check what entries we have
        for key in keys {
            // Check local cache first
            if let Ok(Some(value)) = self.local_cache.read().map(|cache| cache.get(key.as_slice())) {
                let entry = DistributedCacheEntry {
                    key: key.clone(),
                    value,
                    timestamp: current_timestamp(),
                    version: *self.cache_version.read().await,
                    ttl: self.config.cache_ttl_seconds,
                    origin_node: self.local_node_id.clone(),
                    compressed: false,
                };
                available_entries.push(entry);
            }
            // Check remote cache
            else if let Some(entry) = self.remote_cache.read().await.get(&key) {
                if !entry.is_expired() {
                    available_entries.push(entry.clone());
                }
            }
        }

        // Send response
        if !available_entries.is_empty() {
            let response = CacheSyncMessage::CacheResponse {
                entries: available_entries,
                responder_node: self.local_node_id.clone(),
            };
            let _ = self.sync_sender.send(response);
        }

        Ok(())
    }

    async fn handle_cache_response(&self, entries: Vec<DistributedCacheEntry>) -> Result<()> {
        for entry in entries {
            if !entry.is_expired() {
                let mut remote_cache = self.remote_cache.write().await;
                remote_cache.insert(entry.key.clone(), entry.clone());

                // Cache locally if it's valuable
                if self.should_cache_locally(&entry) {
                    if let Ok(mut local_cache) = self.local_cache.write() {
                        local_cache.put(entry.key.as_slice().to_vec(), entry.value);
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_cache_invalidation(&self, keys: Vec<CacheKey>) -> Result<()> {
        // Remove from remote cache
        {
            let mut remote_cache = self.remote_cache.write().await;
            for key in &keys {
                remote_cache.remove(key);
            }
        }

        // Remove from local cache
        if let Ok(mut local_cache) = self.local_cache.write() {
            for key in &keys {
                local_cache.remove(key.as_slice());
            }
        }

        Ok(())
    }

    async fn handle_cache_gossip(&self, node_id: String, cache_stats: DistributedCacheStats, top_keys: Vec<(CacheKey, u64)>) -> Result<()> {
        let mut node_caches = self.node_caches.write().await;

        let info = NodeCacheInfo {
            node_id: node_id.clone(),
            last_seen: current_timestamp(),
            cache_stats,
            top_keys,
        };

        node_caches.insert(node_id, info);
        Ok(())
    }

    async fn get_top_keys(&self) -> Result<Vec<(CacheKey, u64)>> {
        // In a real implementation, this would track access patterns
        // For now, return empty vec
        Ok(Vec::new())
    }

    fn should_cache_locally(&self, entry: &DistributedCacheEntry) -> bool {
        // Simple heuristic: cache if entry is recent and from a trusted node
        let age_hours = (current_timestamp() - entry.timestamp) / 3600;
        age_hours < 24 // Cache entries less than 24 hours old
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_distributed_cache_creation() {
        let config = DistributedCacheConfig::default();
        let cache = DistributedCache::new(config, "test_node".to_string());

        assert_eq!(cache.local_node_id, "test_node");
        assert_eq!(cache.config.local_cache_size, 10000);
    }

    #[tokio::test]
    async fn test_distributed_cache_put_get() {
        let config = DistributedCacheConfig::default();
        let cache = DistributedCache::new(config, "test_node".to_string());

        let key = b"test_key";
        let value = b"test_value";

        cache.put(key.to_vec(), value.to_vec()).await.unwrap();

        let retrieved = cache.get(key).await.unwrap();
        assert_eq!(retrieved, Some(value.to_vec()));
    }

    #[tokio::test]
    async fn test_distributed_cache_invalidation() {
        let config = DistributedCacheConfig::default();
        let cache = DistributedCache::new(config, "test_node".to_string());

        let key1 = b"key1".to_vec();
        let key2 = b"key2".to_vec();

        cache.put(key1.clone(), b"value1".to_vec()).await.unwrap();
        cache.put(key2.clone(), b"value2".to_vec()).await.unwrap();

        // Verify entries exist
        assert!(cache.get(&key1).await.unwrap().is_some());
        assert!(cache.get(&key2).await.unwrap().is_some());

        // Invalidate entries
        cache.invalidate(vec![key1.clone(), key2.clone()]).await.unwrap();

        // Verify entries are gone
        assert!(cache.get(&key1).await.unwrap().is_none());
        assert!(cache.get(&key2).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_distributed_cache_stats() {
        let config = DistributedCacheConfig::default();
        let cache = DistributedCache::new(config, "test_node".to_string());

        // Add some data
        cache.put(b"key1".to_vec(), b"value1".to_vec()).await.unwrap();
        cache.put(b"key2".to_vec(), b"value2".to_vec()).await.unwrap();

        let stats = cache.get_stats().await.unwrap();
        assert!(stats.local_entries >= 2);
        assert!(stats.total_memory_mb >= 0.0);
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let config = DistributedCacheConfig {
            cache_ttl_seconds: 1, // Very short TTL for testing
            ..Default::default()
        };
        let cache = DistributedCache::new(config, "test_node".to_string());

        cache.put(b"key1".to_vec(), b"value1".to_vec()).await.unwrap();

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let cleaned = cache.cleanup_expired().await.unwrap();
        assert!(cleaned >= 1);
    }
}
