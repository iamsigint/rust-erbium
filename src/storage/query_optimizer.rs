//! Query optimization and indexing system for high-performance database operations
//!
//! This module provides advanced query optimization including:
//! - B-tree indexes for range queries
//! - Bloom filters for fast existence checks
//! - Query planning and execution optimization
//! - Statistics-based query optimization

use crate::utils::error::{Result, BlockchainError};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

/// Query optimizer for database operations
pub struct QueryOptimizer {
    indexes: HashMap<String, Index>,
    statistics: QueryStatistics,
    bloom_filters: HashMap<String, BloomFilter>,
}

impl QueryOptimizer {
    pub fn new() -> Self {
        Self {
            indexes: HashMap::new(),
            statistics: QueryStatistics::default(),
            bloom_filters: HashMap::new(),
        }
    }

    /// Create an index on a specific field
    pub fn create_index(&mut self, name: &str, index_type: IndexType) -> Result<()> {
        let index = match index_type {
            IndexType::BTree => Index::BTreeIndex(BTreeIndex::new()),
            IndexType::Hash => Index::HashIndex(HashIndex::new()),
            IndexType::Bitmap => Index::BitmapIndex(BitmapIndex::new()),
        };

        self.indexes.insert(name.to_string(), index);

        // Create corresponding bloom filter for fast existence checks
        self.bloom_filters.insert(name.to_string(), BloomFilter::new(10000, 0.01));

        log::info!("Created {} index: {}", index_type.as_str(), name);
        Ok(())
    }

    /// Insert entry into index
    pub fn insert_into_index(&mut self, index_name: &str, key: &[u8], value: &[u8]) -> Result<()> {
        if let Some(index) = self.indexes.get_mut(index_name) {
            match index {
                Index::BTreeIndex(btree) => {
                    btree.insert(key.to_vec(), value.to_vec());
                }
                Index::HashIndex(hash) => {
                    hash.insert(key.to_vec(), value.to_vec());
                }
                Index::BitmapIndex(bitmap) => {
                    bitmap.insert(key.to_vec(), value.to_vec());
                }
            }

            // Update bloom filter
            if let Some(bloom) = self.bloom_filters.get_mut(index_name) {
                bloom.insert(key);
            }
        }

        Ok(())
    }

    /// Query using index
    pub fn query_index(&self, index_name: &str, query: &Query) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let start_time = current_timestamp_ms();

        let results = if let Some(index) = self.indexes.get(index_name) {
            match index {
                Index::BTreeIndex(btree) => self.execute_btree_query(btree, query),
                Index::HashIndex(hash) => self.execute_hash_query(hash, query),
                Index::BitmapIndex(bitmap) => self.execute_bitmap_query(bitmap, query),
            }
        } else {
            Vec::new()
        };

        let duration = current_timestamp_ms() - start_time;
        self.statistics.record_query(duration, results.len());

        Ok(results)
    }

    /// Check existence using bloom filter
    pub fn might_contain(&self, index_name: &str, key: &[u8]) -> bool {
        self.bloom_filters.get(index_name)
            .map(|bloom| bloom.might_contain(key))
            .unwrap_or(false)
    }

    /// Optimize query plan
    pub fn optimize_query(&self, query: &Query) -> QueryPlan {
        let mut plan = QueryPlan::new();

        // Analyze query selectivity
        let selectivity = self.estimate_selectivity(query);

        // Choose best index
        if let Some(best_index) = self.select_best_index(query, selectivity) {
            plan.add_step(QueryStep::IndexScan {
                index_name: best_index,
                query: query.clone(),
            });
        } else {
            plan.add_step(QueryStep::FullScan {
                query: query.clone(),
            });
        }

        // Add filtering steps if needed
        if selectivity < 0.1 {
            plan.add_step(QueryStep::Filter {
                condition: query.clone(),
            });
        }

        plan
    }

    /// Estimate query selectivity (0.0 to 1.0)
    fn estimate_selectivity(&self, query: &Query) -> f64 {
        match query {
            Query::Exact(key) => 0.001, // Very selective
            Query::Range { start, end } => {
                // Estimate based on key distribution
                0.1 // Conservative estimate
            }
            Query::Prefix(prefix) => {
                // Estimate based on prefix length
                let prefix_len = prefix.len();
                1.0 / (10.0_f64).powf(prefix_len as f64 / 2.0)
            }
            Query::Composite(queries) => {
                // Intersection selectivity
                queries.iter()
                    .map(|q| self.estimate_selectivity(q))
                    .fold(1.0, |acc, sel| acc * sel)
            }
        }
    }

    /// Select best index for query
    fn select_best_index(&self, query: &Query, selectivity: f64) -> Option<String> {
        // Simple index selection based on query type and selectivity
        match query {
            Query::Exact(_) => {
                // Prefer hash indexes for exact matches
                self.indexes.iter()
                    .find(|(_, index)| matches!(index, Index::HashIndex(_)))
                    .map(|(name, _)| name.clone())
            }
            Query::Range { .. } => {
                // Prefer B-tree indexes for range queries
                self.indexes.iter()
                    .find(|(_, index)| matches!(index, Index::BTreeIndex(_)))
                    .map(|(name, _)| name.clone())
            }
            Query::Prefix(_) => {
                // Use any available index
                self.indexes.keys().next().cloned()
            }
            Query::Composite(_) => {
                // Use most selective index
                self.indexes.keys().next().cloned()
            }
        }
    }

    fn execute_btree_query(&self, btree: &BTreeIndex, query: &Query) -> Vec<(Vec<u8>, Vec<u8>)> {
        match query {
            Query::Exact(key) => {
                btree.get(key).map(|v| vec![(key.clone(), v)]).unwrap_or_default()
            }
            Query::Range { start, end } => {
                btree.range(start, end)
            }
            Query::Prefix(prefix) => {
                btree.prefix_scan(prefix)
            }
            Query::Composite(_) => Vec::new(), // Not supported for B-tree
        }
    }

    fn execute_hash_query(&self, hash: &HashIndex, query: &Query) -> Vec<(Vec<u8>, Vec<u8>)> {
        match query {
            Query::Exact(key) => {
                hash.get(key).map(|v| vec![(key.clone(), v)]).unwrap_or_default()
            }
            _ => Vec::new(), // Hash indexes only support exact matches
        }
    }

    fn execute_bitmap_query(&self, bitmap: &BitmapIndex, query: &Query) -> Vec<(Vec<u8>, Vec<u8>)> {
        match query {
            Query::Exact(key) => {
                bitmap.get(key).map(|v| vec![(key.clone(), v)]).unwrap_or_default()
            }
            _ => Vec::new(), // Bitmap indexes support limited queries
        }
    }

    /// Get query statistics
    pub fn get_statistics(&self) -> &QueryStatistics {
        &self.statistics
    }

    /// Optimize indexes based on usage patterns
    pub fn optimize_indexes(&mut self) -> Result<()> {
        // Analyze query patterns and rebuild inefficient indexes
        for (name, index) in &mut self.indexes {
            match index {
                Index::BTreeIndex(btree) => {
                    if btree.needs_rebuild() {
                        log::info!("Rebuilding B-tree index: {}", name);
                        btree.rebuild();
                    }
                }
                Index::HashIndex(hash) => {
                    if hash.load_factor() > 0.8 {
                        log::info!("Rehashing index: {}", name);
                        hash.rehash();
                    }
                }
                Index::BitmapIndex(bitmap) => {
                    if bitmap.should_compress() {
                        log::info!("Compressing bitmap index: {}", name);
                        bitmap.compress();
                    }
                }
            }
        }

        Ok(())
    }
}

/// Query types supported by the optimizer
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    Exact(Vec<u8>),
    Range { start: Vec<u8>, end: Vec<u8> },
    Prefix(Vec<u8>),
    Composite(Vec<Query>),
}

/// Index types
#[derive(Debug, Clone)]
pub enum IndexType {
    BTree,
    Hash,
    Bitmap,
}

impl IndexType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexType::BTree => "B-Tree",
            IndexType::Hash => "Hash",
            IndexType::Bitmap => "Bitmap",
        }
    }
}

/// Index implementations
#[derive(Debug)]
pub enum Index {
    BTreeIndex(BTreeIndex),
    HashIndex(HashIndex),
    BitmapIndex(BitmapIndex),
}

/// B-Tree index for range queries and ordered data
#[derive(Debug)]
pub struct BTreeIndex {
    data: BTreeMap<Vec<u8>, Vec<u8>>,
    size_bytes: usize,
    max_size_bytes: usize,
}

impl BTreeIndex {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            size_bytes: 0,
            max_size_bytes: 100 * 1024 * 1024, // 100MB
        }
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        let old_size = self.data.get(&key).map(|v| key.len() + v.len()).unwrap_or(0);
        self.size_bytes = self.size_bytes - old_size + key.len() + value.len();
        self.data.insert(key, value);
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.get(key).cloned()
    }

    pub fn range(&self, start: &[u8], end: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.data.range(start.to_vec()..end.to_vec())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn prefix_scan(&self, prefix: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
        let end_prefix = increment_prefix(prefix);
        self.data.range(prefix.to_vec()..end_prefix)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn needs_rebuild(&self) -> bool {
        self.size_bytes > self.max_size_bytes
    }

    pub fn rebuild(&mut self) {
        // In a real implementation, this would rebuild the B-tree structure
        // For now, just reset size tracking
        self.size_bytes = self.data.iter()
            .map(|(k, v)| k.len() + v.len())
            .sum();
    }
}

/// Hash index for exact match queries
#[derive(Debug)]
pub struct HashIndex {
    data: HashMap<Vec<u8>, Vec<u8>>,
    capacity: usize,
}

impl HashIndex {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            capacity: 10000,
        }
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.get(key).cloned()
    }

    pub fn load_factor(&self) -> f64 {
        self.data.len() as f64 / self.capacity as f64
    }

    pub fn rehash(&mut self) {
        self.capacity *= 2;
        // In a real implementation, this would rehash the table
    }
}

/// Bitmap index for low-cardinality data
#[derive(Debug)]
pub struct BitmapIndex {
    bitmaps: HashMap<Vec<u8>, Vec<u8>>, // Simplified bitmap representation
    compressed: bool,
}

impl BitmapIndex {
    pub fn new() -> Self {
        Self {
            bitmaps: HashMap::new(),
            compressed: false,
        }
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.bitmaps.insert(key, value);
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.bitmaps.get(key).cloned()
    }

    pub fn should_compress(&self) -> bool {
        !self.compressed && self.bitmaps.len() > 1000
    }

    pub fn compress(&mut self) {
        // In a real implementation, this would compress the bitmaps
        self.compressed = true;
    }
}

/// Query execution plan
#[derive(Debug, Clone)]
pub struct QueryPlan {
    steps: Vec<QueryStep>,
}

impl QueryPlan {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn add_step(&mut self, step: QueryStep) {
        self.steps.push(step);
    }

    pub fn steps(&self) -> &[QueryStep] {
        &self.steps
    }
}

/// Query execution steps
#[derive(Debug, Clone)]
pub enum QueryStep {
    IndexScan { index_name: String, query: Query },
    FullScan { query: Query },
    Filter { condition: Query },
}

/// Query statistics for optimization
#[derive(Debug, Clone, Default)]
pub struct QueryStatistics {
    total_queries: u64,
    total_query_time: u64,
    avg_query_time: f64,
    max_query_time: u64,
    min_query_time: u64,
    queries_per_second: f64,
}

impl QueryStatistics {
    pub fn record_query(&mut self, duration_ms: u64, result_count: usize) {
        self.total_queries += 1;
        self.total_query_time += duration_ms;

        if self.total_queries == 1 {
            self.min_query_time = duration_ms;
            self.max_query_time = duration_ms;
        } else {
            self.min_query_time = self.min_query_time.min(duration_ms);
            self.max_query_time = self.max_query_time.max(duration_ms);
        }

        self.avg_query_time = self.total_query_time as f64 / self.total_queries as f64;
        self.queries_per_second = 1000.0 / self.avg_query_time;
    }
}

/// Bloom filter for fast existence checks
#[derive(Debug, Clone)]
pub struct BloomFilter {
    bits: Vec<u8>,
    hash_functions: usize,
    size: usize,
}

impl BloomFilter {
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        let size = ((expected_items as f64 * false_positive_rate.ln()) / (1.0 / 2_f64.ln().powi(2))).ceil() as usize;
        let hash_functions = ((size as f64 / expected_items as f64) * 2_f64.ln()).round() as usize;

        Self {
            bits: vec![0; (size + 7) / 8], // Round up to bytes
            hash_functions,
            size,
        }
    }

    pub fn insert(&mut self, item: &[u8]) {
        for i in 0..self.hash_functions {
            let hash = self.hash(item, i);
            let byte_index = hash / 8;
            let bit_index = hash % 8;
            if byte_index < self.bits.len() {
                self.bits[byte_index] |= 1 << bit_index;
            }
        }
    }

    pub fn might_contain(&self, item: &[u8]) -> bool {
        for i in 0..self.hash_functions {
            let hash = self.hash(item, i);
            let byte_index = hash / 8;
            let bit_index = hash % 8;
            if byte_index >= self.bits.len() ||
               (self.bits[byte_index] & (1 << bit_index)) == 0 {
                return false;
            }
        }
        true
    }

    fn hash(&self, item: &[u8], seed: usize) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        seed.hash(&mut hasher);
        (hasher.finish() as usize) % self.size
    }
}

fn increment_prefix(prefix: &[u8]) -> Vec<u8> {
    let mut result = prefix.to_vec();
    for i in (0..result.len()).rev() {
        if result[i] < 255 {
            result[i] += 1;
            result.truncate(i + 1);
            return result;
        }
    }
    // If all bytes are 255, return empty vec to indicate no upper bound
    Vec::new()
}

fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree_index() {
        let mut index = BTreeIndex::new();

        index.insert(b"key1".to_vec(), b"value1".to_vec());
        index.insert(b"key2".to_vec(), b"value2".to_vec());
        index.insert(b"key3".to_vec(), b"value3".to_vec());

        assert_eq!(index.get(b"key1"), Some(b"value1".to_vec()));
        assert_eq!(index.get(b"key4"), None);

        let range = index.range(b"key1", b"key3");
        assert_eq!(range.len(), 2);
    }

    #[test]
    fn test_hash_index() {
        let mut index = HashIndex::new();

        index.insert(b"key1".to_vec(), b"value1".to_vec());
        index.insert(b"key2".to_vec(), b"value2".to_vec());

        assert_eq!(index.get(b"key1"), Some(b"value1".to_vec()));
        assert_eq!(index.get(b"key3"), None);
    }

    #[test]
    fn test_bloom_filter() {
        let mut filter = BloomFilter::new(1000, 0.01);

        filter.insert(b"test1");
        filter.insert(b"test2");

        assert!(filter.might_contain(b"test1"));
        assert!(filter.might_contain(b"test2"));
        // Bloom filters can have false positives but not false negatives
        // We can't reliably test for absence due to false positive rate
    }

    #[test]
    fn test_query_optimizer() {
        let mut optimizer = QueryOptimizer::new();

        optimizer.create_index("test_index", IndexType::BTree).unwrap();
        optimizer.insert_into_index("test_index", b"key1", b"value1").unwrap();

        let query = Query::Exact(b"key1".to_vec());
        let results = optimizer.query_index("test_index", &query).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], (b"key1".to_vec(), b"value1".to_vec()));
    }

    #[test]
    fn test_query_plan_optimization() {
        let optimizer = QueryOptimizer::new();
        let query = Query::Exact(b"test_key".to_vec());

        let plan = optimizer.optimize_query(&query);
        assert!(!plan.steps().is_empty());
    }
}
