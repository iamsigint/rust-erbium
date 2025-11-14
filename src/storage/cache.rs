use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use std::hash::Hash;

// Optimized cache key for better hashing performance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey(Vec<u8>);

impl CacheKey {
    pub fn new(key: Vec<u8>) -> Self {
        CacheKey(key)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: CacheKey,
    pub value: Vec<u8>,
    pub timestamp: u64,
    pub access_count: u64,
    pub last_accessed: u64,
    pub size_bytes: usize,
}

#[derive(Debug)]
pub struct Cache {
    data: HashMap<CacheKey, CacheEntry>,
    lru_queue: VecDeque<CacheKey>, // LRU order for O(1) eviction
    max_size: usize,
    current_size: usize,
    max_memory_mb: usize,
    current_memory_bytes: usize,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new(1000) // Default cache size of 1000 entries
    }
}

impl Cache {
    pub fn new(max_size: usize) -> Self {
        Self::with_memory_limit(max_size, 100) // 100MB default
    }

    pub fn with_memory_limit(max_size: usize, max_memory_mb: usize) -> Self {
        Self {
            data: HashMap::with_capacity(max_size),
            lru_queue: VecDeque::with_capacity(max_size),
            max_size,
            current_size: 0,
            max_memory_mb,
            current_memory_bytes: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let cache_key = CacheKey::new(key);
        let timestamp = current_timestamp();
        let size_bytes = cache_key.0.len() + value.len();

        // Check memory limits
        if self.current_memory_bytes + size_bytes > self.max_memory_mb * 1024 * 1024 {
            self.evict_by_memory(size_bytes);
        }

        // If cache is full, evict LRU entries
        if self.current_size >= self.max_size && !self.data.contains_key(&cache_key) {
            self.evict_lru();
        }

        let entry = CacheEntry {
            key: cache_key.clone(),
            value: value.clone(),
            timestamp,
            access_count: 0,
            last_accessed: timestamp,
            size_bytes,
        };

        // Remove old entry if exists
        if let Some(old_entry) = self.data.remove(&cache_key) {
            self.current_memory_bytes -= old_entry.size_bytes;
            // Remove from LRU queue
            if let Some(pos) = self.lru_queue.iter().position(|k| k == &cache_key) {
                self.lru_queue.remove(pos);
            }
            self.current_size -= 1;
        }

        // Add new entry
        self.data.insert(cache_key.clone(), entry);
        self.lru_queue.push_back(cache_key);
        self.current_size += 1;
        self.current_memory_bytes += size_bytes;
    }

    pub fn get(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        let cache_key = CacheKey(key.to_vec());

        if let Some(entry) = self.data.get_mut(&cache_key) {
            entry.access_count += 1;
            entry.last_accessed = current_timestamp();
            self.hits += 1;

            // Move to back of LRU queue (most recently used)
            if let Some(pos) = self.lru_queue.iter().position(|k| k == &cache_key) {
                self.lru_queue.remove(pos);
            }
            self.lru_queue.push_back(cache_key);

            Some(entry.value.clone())
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn remove(&mut self, key: &[u8]) -> Option<CacheEntry> {
        let cache_key = CacheKey(key.to_vec());

        if let Some(entry) = self.data.remove(&cache_key) {
            self.current_memory_bytes -= entry.size_bytes;
            self.current_size -= 1;

            // Remove from LRU queue
            if let Some(pos) = self.lru_queue.iter().position(|k| k == &cache_key) {
                self.lru_queue.remove(pos);
            }

            Some(entry)
        } else {
            None
        }
    }

    pub fn contains(&self, key: &[u8]) -> bool {
        let cache_key = CacheKey(key.to_vec());
        self.data.contains_key(&cache_key)
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.lru_queue.clear();
        self.current_size = 0;
        self.current_memory_bytes = 0;
        self.hits = 0;
        self.misses = 0;
        self.evictions = 0;
    }

    pub fn size(&self) -> usize {
        self.current_size
    }

    pub fn memory_usage_mb(&self) -> f64 {
        self.current_memory_bytes as f64 / (1024.0 * 1024.0)
    }

    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            size: self.current_size,
            max_size: self.max_size,
            memory_usage_mb: self.memory_usage_mb(),
            max_memory_mb: self.max_memory_mb,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            hit_ratio: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }

    fn evict_lru(&mut self) {
        if let Some(lru_key) = self.lru_queue.front() {
            if let Some(entry) = self.data.remove(lru_key) {
                self.current_memory_bytes -= entry.size_bytes;
                self.current_size -= 1;
                self.evictions += 1;
            }
            self.lru_queue.pop_front();
        }
    }

    fn evict_by_memory(&mut self, needed_bytes: usize) {
        while self.current_memory_bytes + needed_bytes > self.max_memory_mb * 1024 * 1024
              && !self.lru_queue.is_empty() {
            self.evict_lru();
        }
    }

    pub fn cleanup_old_entries(&mut self, max_age_seconds: u64) -> usize {
        let current_time = current_timestamp();
        let mut removed_count = 0;
        let mut keys_to_remove = Vec::new();

        // Find old entries
        for (key, entry) in &self.data {
            if current_time - entry.timestamp > max_age_seconds {
                keys_to_remove.push(key.clone());
            }
        }

        // Remove old entries
        for key in keys_to_remove {
            if let Some(entry) = self.data.remove(&key) {
                self.current_memory_bytes -= entry.size_bytes;
                self.current_size -= 1;
                removed_count += 1;

                // Remove from LRU queue
                if let Some(pos) = self.lru_queue.iter().position(|k| k == &key) {
                    self.lru_queue.remove(pos);
                }
            }
        }

        removed_count
    }

    // Batch operations for better performance
    pub fn batch_put(&mut self, entries: Vec<(Vec<u8>, Vec<u8>)>) {
        for (key, value) in entries {
            self.put(key, value);
        }
    }

    pub fn batch_get(&mut self, keys: Vec<&[u8]>) -> Vec<Option<Vec<u8>>> {
        keys.into_iter().map(|key| self.get(key)).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub memory_usage_mb: f64,
    pub max_memory_mb: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub hit_ratio: f64,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// Thread-safe cache implementation
use std::sync::{Arc, RwLock};

pub type SharedCache = Arc<RwLock<Cache>>;

pub fn create_shared_cache(max_size: usize) -> SharedCache {
    Arc::new(RwLock::new(Cache::new(max_size)))
}

pub fn create_shared_cache_with_memory_limit(max_size: usize, max_memory_mb: usize) -> SharedCache {
    Arc::new(RwLock::new(Cache::with_memory_limit(max_size, max_memory_mb)))
}

// Performance monitoring
pub struct CacheMonitor {
    cache: SharedCache,
    stats_history: Vec<CacheStats>,
    max_history_size: usize,
}

impl CacheMonitor {
    pub fn new(cache: SharedCache, max_history_size: usize) -> Self {
        Self {
            cache,
            stats_history: Vec::with_capacity(max_history_size),
            max_history_size,
        }
    }

    pub fn record_stats(&mut self) {
        if let Ok(cache) = self.cache.read() {
            let stats = cache.get_stats();
            self.stats_history.push(stats);

            if self.stats_history.len() > self.max_history_size {
                self.stats_history.remove(0);
            }
        }
    }

    pub fn get_average_hit_ratio(&self) -> f64 {
        if self.stats_history.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.stats_history.iter().map(|s| s.hit_ratio).sum();
        sum / self.stats_history.len() as f64
    }

    pub fn get_performance_report(&self) -> CachePerformanceReport {
        let avg_hit_ratio = self.get_average_hit_ratio();
        let latest_stats = self.stats_history.last().cloned();

        CachePerformanceReport {
            average_hit_ratio: avg_hit_ratio,
            latest_stats,
            history_size: self.stats_history.len(),
            recommendations: self.generate_recommendations(avg_hit_ratio),
        }
    }

    fn generate_recommendations(&self, hit_ratio: f64) -> Vec<String> {
        let mut recommendations = Vec::new();

        if hit_ratio < 0.5 {
            recommendations.push("Consider increasing cache size for better hit ratio".to_string());
        }

        if hit_ratio > 0.95 {
            recommendations.push("Cache is very effective, consider optimizing memory usage".to_string());
        }

        if let Some(latest) = self.stats_history.last() {
            if latest.evictions > latest.hits / 10 {
                recommendations.push("High eviction rate detected, consider increasing cache size".to_string());
            }
        }

        recommendations
    }
}

#[derive(Debug, Clone)]
pub struct CachePerformanceReport {
    pub average_hit_ratio: f64,
    pub latest_stats: Option<CacheStats>,
    pub history_size: usize,
    pub recommendations: Vec<String>,
}
