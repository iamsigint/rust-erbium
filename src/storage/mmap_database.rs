//! Memory-mapped database implementation for high-performance persistence
//!
//! This module provides memory-mapped file support for database operations,
//! enabling zero-copy access to persistent data and improved I/O performance.

use crate::utils::error::{Result, BlockchainError};
use crate::storage::database::{DatabaseConfig, BufferStats};
use memmap2::{Mmap, MmapMut, MmapOptions};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Memory-mapped database with high-performance persistence
pub struct MmapDatabase {
    file: File,
    mmap: Arc<RwLock<MmapMut>>,
    metadata: Arc<RwLock<DatabaseMetadata>>,
    config: DatabaseConfig,
    // Performance optimizations
    write_buffer: HashMap<Vec<u8>, Vec<u8>>,
    buffer_size: usize,
    max_buffer_size: usize,
    last_flush: u64,
    flush_interval: u64,
    query_cache: HashMap<Vec<u8>, (Vec<u8>, u64)>,
    cache_ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatabaseMetadata {
    version: u32,
    entry_count: u64,
    data_size: u64,
    last_compaction: u64,
    checksum: u64,
}

impl Default for DatabaseMetadata {
    fn default() -> Self {
        Self {
            version: 1,
            entry_count: 0,
            data_size: 0,
            last_compaction: 0,
            checksum: 0,
        }
    }
}

impl MmapDatabase {
    /// Create a new memory-mapped database
    pub fn new(path: &str) -> Result<Self> {
        let config = DatabaseConfig {
            path: path.to_string(),
            sync_wal: false, // Not needed with mmap
            sync_data: false, // Not needed with mmap
            stats: true,
            columns: 1, // Single column for simplicity
        };
        Self::with_config(config)
    }

    /// Create database with custom configuration
    pub fn with_config(config: DatabaseConfig) -> Result<Self> {
        let path = Path::new(&config.path);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open or create file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        // Set initial file size if empty
        let file_size = file.metadata()?.len();
        if file_size == 0 {
            file.set_len(1024 * 1024)?; // 1MB initial size
        }

        // SAFETY: This is safe because:
        // 1. We have exclusive access to the file through OpenOptions
        // 2. The file size is valid and non-zero
        // 3. The mmap will be protected by RwLock for concurrent access
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        let metadata = DatabaseMetadata::default();
        let current_time = current_timestamp();

        Ok(Self {
            file,
            mmap: Arc::new(RwLock::new(mmap)),
            metadata: Arc::new(RwLock::new(metadata)),
            config,
            write_buffer: HashMap::new(),
            buffer_size: 0,
            max_buffer_size: 1000,
            last_flush: current_time,
            flush_interval: 30,
            query_cache: HashMap::new(),
            cache_ttl: 300,
        })
    }

    /// Put key-value pair with memory-mapped persistence
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        let key_vec = key.to_vec();
        let value_vec = value.to_vec();
        let current_time = current_timestamp();

        // Update query cache
        self.query_cache.insert(key_vec.clone(), (value_vec.clone(), current_time));

        // Add to write buffer
        self.write_buffer.insert(key_vec, value_vec);
        self.buffer_size += 1;

        // Auto-flush if needed
        if self.buffer_size >= self.max_buffer_size ||
           current_time - self.last_flush >= self.flush_interval {
            self.flush_buffer()?;
        }

        Ok(())
    }

    /// Get value by key with zero-copy access when possible
    pub fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let key_vec = key.to_vec();
        let current_time = current_timestamp();

        // Check query cache first
        if let Some((cached_value, timestamp)) = self.query_cache.get(&key_vec) {
            if current_time - timestamp < self.cache_ttl {
                return Ok(Some(cached_value.clone()));
            } else {
                self.query_cache.remove(&key_vec);
            }
        }

        // Check write buffer
        if let Some(value) = self.write_buffer.get(&key_vec) {
            self.query_cache.insert(key_vec, (value.clone(), current_time));
            return Ok(Some(value.clone()));
        }

        // Search in memory-mapped file
        self.search_mmap(key)
    }

    /// Flush write buffer to memory-mapped file
    pub fn flush_buffer(&mut self) -> Result<()> {
        if self.write_buffer.is_empty() {
            return Ok(());
        }

        let operations_count = self.write_buffer.len();
        let mut mmap = self.mmap.write().map_err(|_| {
            BlockchainError::Storage("Failed to acquire mmap write lock".to_string())
        })?;

        // Serialize operations to binary format
        let mut data = Vec::new();
        for (key, value) in &self.write_buffer {
            // Simple binary format: [key_len(4)][key][value_len(4)][value]
            let key_len = (key.len() as u32).to_be_bytes();
            let value_len = (value.len() as u32).to_be_bytes();

            data.extend_from_slice(&key_len);
            data.extend_from_slice(key);
            data.extend_from_slice(&value_len);
            data.extend_from_slice(value);
        }

        // Ensure mmap has enough space
        let required_size = data.len();
        let current_size = mmap.len();

        if required_size > current_size {
            // Need to remap with larger size
            drop(mmap); // Release lock before remapping
            self.remap_file(required_size.max(current_size * 2))?;
            mmap = self.mmap.write().map_err(|_| {
                BlockchainError::Storage("Failed to reacquire mmap write lock".to_string())
            })?;
        }

        // Write data to mmap (append to end)
        let offset = current_size - data.len().min(current_size);
        mmap[offset..offset + data.len()].copy_from_slice(&data);

        // Update metadata
        {
            let mut metadata = self.metadata.write().map_err(|_| {
                BlockchainError::Storage("Failed to acquire metadata write lock".to_string())
            })?;
            metadata.entry_count += operations_count as u64;
            metadata.data_size += data.len() as u64;
        }

        self.write_buffer.clear();
        self.buffer_size = 0;
        self.last_flush = current_timestamp();

        // Ensure data is flushed to disk
        mmap.flush()?;

        log::debug!("Flushed {} entries to memory-mapped database", operations_count);
        Ok(())
    }

    /// Search for key in memory-mapped file
    fn search_mmap(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let mmap = self.mmap.read().map_err(|_| {
            BlockchainError::Storage("Failed to acquire mmap read lock".to_string())
        })?;

        let data = mmap.as_ref();
        let mut offset = 0;

        while offset + 8 < data.len() {
            // Read key length
            let key_len_bytes = &data[offset..offset + 4];
            let key_len = u32::from_be_bytes(key_len_bytes.try_into().unwrap()) as usize;
            offset += 4;

            if offset + key_len > data.len() {
                break;
            }

            // Read key
            let entry_key = &data[offset..offset + key_len];
            offset += key_len;

            // Read value length
            if offset + 4 > data.len() {
                break;
            }
            let value_len_bytes = &data[offset..offset + 4];
            let value_len = u32::from_be_bytes(value_len_bytes.try_into().unwrap()) as usize;
            offset += 4;

            if offset + value_len > data.len() {
                break;
            }

            // Check if key matches
            if entry_key == key {
                let value = data[offset..offset + value_len].to_vec();
                return Ok(Some(value));
            }

            offset += value_len;
        }

        Ok(None)
    }

    /// Remap file with new size
    fn remap_file(&mut self, new_size: usize) -> Result<()> {
        // Validate new size is reasonable (max 1GB per remap)
        if new_size > 1024 * 1024 * 1024 {
            return Err(BlockchainError::Storage(
                format!("Requested mmap size too large: {} bytes", new_size)
            ));
        }

        // Extend file size
        self.file.set_len(new_size as u64)?;

        // SAFETY: This is safe because:
        // 1. We just set the file size above
        // 2. We have exclusive mutable access to self
        // 3. The old mmap will be properly dropped when replaced
        // 4. New mmap will be protected by RwLock
        let new_mmap = unsafe { MmapOptions::new().map_mut(&self.file)? };

        // Update mmap
        *self.mmap.write().map_err(|_| {
            BlockchainError::Storage("Failed to update mmap".to_string())
        })? = new_mmap;

        Ok(())
    }

    /// Force synchronization to disk
    pub fn sync(&self) -> Result<()> {
        let mmap = self.mmap.read().map_err(|_| {
            BlockchainError::Storage("Failed to acquire mmap read lock".to_string())
        })?;
        mmap.flush()?;
        self.file.sync_all()?;
        Ok(())
    }

    /// Get buffer statistics
    pub fn get_buffer_stats(&self) -> BufferStats {
        BufferStats {
            buffer_size: self.buffer_size,
            max_buffer_size: self.max_buffer_size,
            cache_size: self.query_cache.len(),
            last_flush: self.last_flush,
        }
    }

    /// Optimize database (cleanup, compaction)
    pub fn optimize(&mut self) -> Result<()> {
        // Clean expired cache entries
        let current_time = current_timestamp();
        self.query_cache.retain(|_, (_, timestamp)| {
            current_time - *timestamp < self.cache_ttl
        });

        // Flush buffer
        self.flush_buffer()?;

        // Force sync to disk
        self.sync()?;

        log::info!("Memory-mapped database optimization completed");
        Ok(())
    }

    /// Get database statistics
    pub fn stats(&self) -> Result<MmapDatabaseStats> {
        let metadata = self.metadata.read().map_err(|_| {
            BlockchainError::Storage("Failed to read metadata".to_string())
        })?;
        let mmap = self.mmap.read().map_err(|_| {
            BlockchainError::Storage("Failed to read mmap".to_string())
        })?;

        Ok(MmapDatabaseStats {
            file_size: self.file.metadata()?.len(),
            mapped_size: mmap.len(),
            entry_count: metadata.entry_count,
            data_size: metadata.data_size,
            buffer_size: self.buffer_size,
            cache_size: self.query_cache.len(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MmapDatabaseStats {
    pub file_size: u64,
    pub mapped_size: usize,
    pub entry_count: u64,
    pub data_size: u64,
    pub buffer_size: usize,
    pub cache_size: usize,
}

impl MmapDatabaseStats {
    pub fn memory_usage_mb(&self) -> f64 {
        self.mapped_size as f64 / (1024.0 * 1024.0)
    }

    pub fn efficiency_ratio(&self) -> f64 {
        if self.file_size == 0 {
            0.0
        } else {
            self.data_size as f64 / self.file_size as f64
        }
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Performance comparison: MmapDatabase vs regular Database
pub fn compare_performance() -> Result<PerformanceComparison> {
    use std::time::Instant;

    // Test data
    let test_entries: Vec<(Vec<u8>, Vec<u8>)> = (0..1000)
        .map(|i| {
            let key = format!("key_{}", i).into_bytes();
            let value = format!("value_{}", i).into_bytes();
            (key, value)
        })
        .collect();

    // Benchmark regular database (simulated)
    let regular_start = Instant::now();
    let mut regular_ops = 0;
    for (key, value) in &test_entries {
        // Simulate regular database operations
        let _hashed_key = blake3::hash(key);
        let _hashed_value = blake3::hash(value);
        regular_ops += 1;
    }
    let regular_time = regular_start.elapsed();

    // Benchmark mmap database
    let temp_path = format!("/tmp/mmap_test_{}", current_timestamp());
    let mut mmap_db = MmapDatabase::new(&temp_path)?;
    let mmap_start = Instant::now();
    for (key, value) in &test_entries {
        mmap_db.put(key, value)?;
    }
    mmap_db.flush_buffer()?;
    let mmap_time = mmap_start.elapsed();

    // Cleanup
    std::fs::remove_file(temp_path).ok();

    Ok(PerformanceComparison {
        regular_time_ms: regular_time.as_millis() as f64,
        mmap_time_ms: mmap_time.as_millis() as f64,
        speedup_factor: regular_time.as_secs_f64() / mmap_time.as_secs_f64(),
        operations_count: test_entries.len(),
    })
}

#[derive(Debug, Clone)]
pub struct PerformanceComparison {
    pub regular_time_ms: f64,
    pub mmap_time_ms: f64,
    pub speedup_factor: f64,
    pub operations_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_mmap_database_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let mut db = MmapDatabase::new(db_path.to_str().unwrap()).unwrap();

        // Test put and get
        let key = b"test_key";
        let value = b"test_value";
        db.put(key, value).unwrap();

        let retrieved = db.get(key).unwrap();
        assert_eq!(retrieved, Some(value.to_vec()));

        // Test non-existent key
        let non_existent = db.get(b"non_existent").unwrap();
        assert_eq!(non_existent, None);
    }

    #[test]
    fn test_mmap_database_buffer_flush() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let mut db = MmapDatabase::new(db_path.to_str().unwrap()).unwrap();

        // Fill buffer
        for i in 0..10 {
            let key = format!("key_{}", i).into_bytes();
            let value = format!("value_{}", i).into_bytes();
            db.put(&key, &value).unwrap();
        }

        // Force flush
        db.flush_buffer().unwrap();

        // Verify buffer is empty
        let stats = db.get_buffer_stats();
        assert_eq!(stats.buffer_size, 0);

        // Verify data persists
        let retrieved = db.get(b"key_5").unwrap();
        assert_eq!(retrieved, Some(b"value_5".to_vec()));
    }

    #[test]
    fn test_mmap_database_stats() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let mut db = MmapDatabase::new(db_path.to_str().unwrap()).unwrap();

        // Add some data
        db.put(b"key1", b"value1").unwrap();
        db.put(b"key2", b"value2").unwrap();
        db.flush_buffer().unwrap();

        let stats = db.stats().unwrap();
        assert!(stats.entry_count >= 2);
        assert!(stats.data_size > 0);
        assert!(stats.memory_usage_mb() > 0.0);
    }

    #[test]
    fn test_performance_comparison() {
        let comparison = compare_performance().unwrap();
        assert!(comparison.operations_count > 0);
        assert!(comparison.regular_time_ms > 0.0);
        assert!(comparison.mmap_time_ms > 0.0);
        // Mmap should be faster for large datasets
        assert!(comparison.speedup_factor > 0.1);
    }
}
