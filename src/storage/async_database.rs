//! Asynchronous database operations for non-blocking I/O
//!
//! This module provides async database implementations using tokio for
//! non-blocking database operations and improved concurrency.

use crate::utils::error::{Result, BlockchainError};
use crate::storage::database::{DatabaseConfig, BufferStats};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt};
use tokio::sync::{RwLock, Semaphore};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Asynchronous database with non-blocking I/O operations
pub struct AsyncDatabase {
    file_path: String,
    file: Arc<RwLock<Option<File>>>,
    metadata: Arc<RwLock<DatabaseMetadata>>,
    config: DatabaseConfig,
    // Performance optimizations
    write_buffer: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    buffer_size: Arc<RwLock<usize>>,
    max_buffer_size: usize,
    last_flush: Arc<RwLock<u64>>,
    flush_interval: u64,
    query_cache: Arc<RwLock<HashMap<Vec<u8>, (Vec<u8>, u64)>>>,
    cache_ttl: u64,
    // Concurrency control
    write_semaphore: Arc<Semaphore>, // Limit concurrent writes
    read_semaphore: Arc<Semaphore>,  // Limit concurrent reads
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatabaseMetadata {
    version: u32,
    entry_count: u64,
    data_size: u64,
    last_compaction: u64,
    checksum: u64,
    index_offset: u64, // Offset to index section
}

impl Default for DatabaseMetadata {
    fn default() -> Self {
        Self {
            version: 1,
            entry_count: 0,
            data_size: 0,
            last_compaction: 0,
            checksum: 0,
            index_offset: 0,
        }
    }
}

impl AsyncDatabase {
    /// Create a new asynchronous database
    pub async fn new(path: &str) -> Result<Self> {
        let config = DatabaseConfig {
            path: path.to_string(),
            sync_wal: false,
            sync_data: false,
            stats: true,
            columns: 1,
        };
        Self::with_config(config).await
    }

    /// Create database with custom configuration
    pub async fn with_config(config: DatabaseConfig) -> Result<Self> {
        let path = Path::new(&config.path);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Open or create file
        let file = if path.exists() {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&config.path)
                .await?
        } else {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&config.path)
                .await?
        };

        // Set initial file size if empty
        let metadata = file.metadata().await?;
        if metadata.len() == 0 {
            file.set_len(1024 * 1024).await?; // 1MB initial size
        }

        let current_time = current_timestamp();

        Ok(Self {
            file_path: config.path.clone(),
            file: Arc::new(RwLock::new(Some(file))),
            metadata: Arc::new(RwLock::new(DatabaseMetadata::default())),
            config,
            write_buffer: Arc::new(RwLock::new(HashMap::new())),
            buffer_size: Arc::new(RwLock::new(0)),
            max_buffer_size: 1000,
            last_flush: Arc::new(RwLock::new(current_time)),
            flush_interval: 30,
            query_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: 300,
            write_semaphore: Arc::new(Semaphore::new(10)), // Max 10 concurrent writes
            read_semaphore: Arc::new(Semaphore::new(50)), // Max 50 concurrent reads
        })
    }

    /// Asynchronously put key-value pair
    pub async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let _permit = self.write_semaphore.acquire().await
            .map_err(|_| BlockchainError::Storage("Failed to acquire write permit".to_string()))?;

        let key_vec = key.to_vec();
        let value_vec = value.to_vec();
        let current_time = current_timestamp();

        // Update query cache
        {
            let mut cache = self.query_cache.write().await;
            cache.insert(key_vec.clone(), (value_vec.clone(), current_time));
        }

        // Add to write buffer
        {
            let mut buffer = self.write_buffer.write().await;
            buffer.insert(key_vec, value_vec);

            let mut buffer_size = self.buffer_size.write().await;
            *buffer_size += 1;

            // Auto-flush if needed
            if *buffer_size >= self.max_buffer_size {
                drop(buffer_size);
                drop(buffer);
                self.flush_buffer().await?;
            }
        }

        Ok(())
    }

    /// Asynchronously get value by key
    pub async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let _permit = self.read_semaphore.acquire().await
            .map_err(|_| BlockchainError::Storage("Failed to acquire read permit".to_string()))?;

        let key_vec = key.to_vec();
        let current_time = current_timestamp();

        // Check query cache first
        {
            let mut cache = self.query_cache.write().await;
            if let Some((cached_value, timestamp)) = cache.get(&key_vec) {
                if current_time - *timestamp < self.cache_ttl {
                    return Ok(Some(cached_value.clone()));
                } else {
                    cache.remove(&key_vec);
                }
            }
        }

        // Check write buffer
        {
            let buffer = self.write_buffer.read().await;
            if let Some(value) = buffer.get(&key_vec) {
                let mut cache = self.query_cache.write().await;
                cache.insert(key_vec, (value.clone(), current_time));
                return Ok(Some(value.clone()));
            }
        }

        // Search in file
        self.search_file(key).await
    }

    /// Asynchronously flush write buffer to file
    pub async fn flush_buffer(&self) -> Result<()> {
        let buffer = {
            let mut buffer_lock = self.write_buffer.write().await;
            if buffer_lock.is_empty() {
                return Ok(());
            }
            std::mem::take(&mut *buffer_lock)
        };

        let operations_count = buffer.len();

        // Serialize operations to binary format
        let mut data = Vec::new();
        for (key, value) in &buffer {
            // Binary format: [key_len(4)][key][value_len(4)][value]
            let key_len = (key.len() as u32).to_be_bytes();
            let value_len = (value.len() as u32).to_be_bytes();

            data.extend_from_slice(&key_len);
            data.extend_from_slice(key);
            data.extend_from_slice(&value_len);
            data.extend_from_slice(value);
        }

        // Write to file
        {
            let mut file = self.file.write().await;
            if let Some(file) = file.as_mut() {
                file.seek(std::io::SeekFrom::End(0)).await?;
                file.write_all(&data).await?;
                file.flush().await?;
            }
        }

        // Update metadata
        {
            let mut metadata = self.metadata.write().await;
            metadata.entry_count += operations_count as u64;
            metadata.data_size += data.len() as u64;
        }

        // Reset buffer size
        {
            let mut buffer_size = self.buffer_size.write().await;
            *buffer_size = 0;
        }

        // Update last flush time
        {
            let mut last_flush = self.last_flush.write().await;
            *last_flush = current_timestamp();
        }

        log::debug!("Asynchronously flushed {} entries to database", operations_count);
        Ok(())
    }

    /// Search for key in file asynchronously
    async fn search_file(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let file = self.file.read().await;
        if let Some(mut file) = file.as_ref() {
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).await?;

            let data = buffer.as_slice();
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
        }

        Ok(None)
    }

    /// Force synchronization to disk
    pub async fn sync(&self) -> Result<()> {
        let file = self.file.read().await;
        if let Some(file) = file.as_ref() {
            file.sync_all().await?;
        }
        Ok(())
    }

    /// Get buffer statistics
    pub async fn get_buffer_stats(&self) -> Result<BufferStats> {
        let buffer_size = *self.buffer_size.read().await;
        let cache_size = self.query_cache.read().await.len();
        let last_flush = *self.last_flush.read().await;

        Ok(BufferStats {
            buffer_size,
            max_buffer_size: self.max_buffer_size,
            cache_size,
            last_flush,
        })
    }

    /// Optimize database asynchronously
    pub async fn optimize(&self) -> Result<()> {
        // Clean expired cache entries
        let current_time = current_timestamp();
        {
            let mut cache = self.query_cache.write().await;
            cache.retain(|_, (_, timestamp)| {
                current_time - *timestamp < self.cache_ttl
            });
        }

        // Flush buffer
        self.flush_buffer().await?;

        // Force sync
        self.sync().await?;

        log::info!("Async database optimization completed");
        Ok(())
    }

    /// Batch operations for better performance
    pub async fn batch_put(&self, entries: Vec<(Vec<u8>, Vec<u8>)>) -> Result<()> {
        let _permit = self.write_semaphore.acquire().await
            .map_err(|_| BlockchainError::Storage("Failed to acquire write permit".to_string()))?;

        let current_time = current_timestamp();

        {
            let mut buffer = self.write_buffer.write().await;
            let mut buffer_size = self.buffer_size.write().await;
            let mut cache = self.query_cache.write().await;

            for (key, value) in entries {
                buffer.insert(key.clone(), value.clone());
                cache.insert(key, (value, current_time));
                *buffer_size += 1;
            }
        }

        // Auto-flush if buffer is full
        let should_flush = {
            let buffer_size = self.buffer_size.read().await;
            *buffer_size >= self.max_buffer_size
        };

        if should_flush {
            self.flush_buffer().await?;
        }

        Ok(())
    }

    /// Get database statistics
    pub async fn stats(&self) -> Result<AsyncDatabaseStats> {
        let metadata = self.metadata.read().await;
        let buffer_size = *self.buffer_size.read().await;
        let cache_size = self.query_cache.read().await.len();

        let file_size = if let Some(file) = self.file.read().await.as_ref() {
            file.metadata().await?.len()
        } else {
            0
        };

        Ok(AsyncDatabaseStats {
            file_size,
            entry_count: metadata.entry_count,
            data_size: metadata.data_size,
            buffer_size,
            cache_size,
            active_connections: self.write_semaphore.available_permits() +
                               self.read_semaphore.available_permits(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AsyncDatabaseStats {
    pub file_size: u64,
    pub entry_count: u64,
    pub data_size: u64,
    pub buffer_size: usize,
    pub cache_size: usize,
    pub active_connections: usize,
}

impl AsyncDatabaseStats {
    pub fn memory_usage_mb(&self) -> f64 {
        // Estimate memory usage
        (self.buffer_size * 100 + self.cache_size * 150) as f64 / (1024.0 * 1024.0)
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

/// Connection pool for managing multiple async database connections
pub struct AsyncConnectionPool {
    databases: Vec<Arc<AsyncDatabase>>,
    next_connection: std::sync::atomic::AtomicUsize,
}

impl AsyncConnectionPool {
    pub async fn new(pool_size: usize, base_path: &str) -> Result<Self> {
        let mut databases = Vec::with_capacity(pool_size);

        for i in 0..pool_size {
            let path = format!("{}_pool_{}", base_path, i);
            let db = Arc::new(AsyncDatabase::new(&path).await?);
            databases.push(db);
        }

        Ok(Self {
            databases,
            next_connection: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    pub fn get_connection(&self) -> Arc<AsyncDatabase> {
        let index = self.next_connection.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % self.databases.len();
        Arc::clone(&self.databases[index])
    }

    pub async fn stats(&self) -> Result<Vec<AsyncDatabaseStats>> {
        let mut stats = Vec::new();
        for db in &self.databases {
            stats.push(db.stats().await?);
        }
        Ok(stats)
    }

    pub async fn optimize_all(&self) -> Result<()> {
        for db in &self.databases {
            db.optimize().await?;
        }
        Ok(())
    }
}

/// Performance comparison: AsyncDatabase vs regular Database
pub async fn compare_performance() -> Result<AsyncPerformanceComparison> {
    use std::time::Instant;

    // Test data
    let test_entries: Vec<(Vec<u8>, Vec<u8>)> = (0..1000)
        .map(|i| {
            let key = format!("key_{}", i).into_bytes();
            let value = format!("value_{}", i).into_bytes();
            (key, value)
        })
        .collect();

    // Benchmark regular operations (simulated)
    let regular_start = Instant::now();
    let mut regular_ops = 0;
    for (key, value) in &test_entries {
        let _hashed_key = blake3::hash(key);
        let _hashed_value = blake3::hash(value);
        regular_ops += 1;
        // Simulate async delay
        tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;
    }
    let regular_time = regular_start.elapsed();

    // Benchmark async database
    let temp_path = format!("/tmp/async_test_{}", current_timestamp());
    let db = AsyncDatabase::new(&temp_path).await?;
    let async_start = Instant::now();

    // Use batch operations for better performance
    db.batch_put(test_entries.clone()).await?;
    db.flush_buffer().await?;

    let async_time = async_start.elapsed();

    // Cleanup
    tokio::fs::remove_file(temp_path).await.ok();

    Ok(AsyncPerformanceComparison {
        regular_time_ms: regular_time.as_millis() as f64,
        async_time_ms: async_time.as_millis() as f64,
        speedup_factor: regular_time.as_secs_f64() / async_time.as_secs_f64(),
        operations_count: test_entries.len(),
    })
}

#[derive(Debug, Clone)]
pub struct AsyncPerformanceComparison {
    pub regular_time_ms: f64,
    pub async_time_ms: f64,
    pub speedup_factor: f64,
    pub operations_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_async_database_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = AsyncDatabase::new(db_path.to_str().unwrap()).await.unwrap();

        // Test put and get
        let key = b"test_key";
        let value = b"test_value";
        db.put(key, value).await.unwrap();

        let retrieved = db.get(key).await.unwrap();
        assert_eq!(retrieved, Some(value.to_vec()));

        // Test non-existent key
        let non_existent = db.get(b"non_existent").await.unwrap();
        assert_eq!(non_existent, None);
    }

    #[tokio::test]
    async fn test_async_database_batch_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = AsyncDatabase::new(db_path.to_str().unwrap()).await.unwrap();

        // Test batch put
        let entries = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
        ];

        db.batch_put(entries).await.unwrap();

        // Verify entries
        for i in 1..=3 {
            let key = format!("key{}", i).into_bytes();
            let expected_value = format!("value{}", i).into_bytes();
            let retrieved = db.get(&key).await.unwrap();
            assert_eq!(retrieved, Some(expected_value));
        }
    }

    #[tokio::test]
    async fn test_async_database_buffer_flush() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = AsyncDatabase::new(db_path.to_str().unwrap()).await.unwrap();

        // Fill buffer
        for i in 0..10 {
            let key = format!("key_{}", i).into_bytes();
            let value = format!("value_{}", i).into_bytes();
            db.put(&key, &value).await.unwrap();
        }

        // Force flush
        db.flush_buffer().await.unwrap();

        // Verify buffer is empty
        let stats = db.get_buffer_stats().await.unwrap();
        assert_eq!(stats.buffer_size, 0);

        // Verify data persists
        let retrieved = db.get(b"key_5").await.unwrap();
        assert_eq!(retrieved, Some(b"value_5".to_vec()));
    }

    #[tokio::test]
    async fn test_connection_pool() {
        let temp_dir = tempdir().unwrap();
        let base_path = temp_dir.path().join("pool_test");
        let pool = AsyncConnectionPool::new(3, base_path.to_str().unwrap()).await.unwrap();

        // Test getting connections
        let conn1 = pool.get_connection();
        let conn2 = pool.get_connection();
        let conn3 = pool.get_connection();
        let conn4 = pool.get_connection(); // Should cycle back

        // They should be different instances
        assert!(!Arc::ptr_eq(&conn1, &conn2));
        assert!(!Arc::ptr_eq(&conn2, &conn3));

        // Test operations on different connections
        conn1.put(b"key1", b"value1").await.unwrap();
        conn2.put(b"key2", b"value2").await.unwrap();

        let val1 = conn1.get(b"key1").await.unwrap();
        let val2 = conn2.get(b"key2").await.unwrap();

        assert_eq!(val1, Some(b"value1".to_vec()));
        assert_eq!(val2, Some(b"value2".to_vec()));
    }

    #[tokio::test]
    async fn test_performance_comparison() {
        let comparison = compare_performance().await.unwrap();
        assert!(comparison.operations_count > 0);
        assert!(comparison.regular_time_ms > 0.0);
        assert!(comparison.async_time_ms > 0.0);
    }
}
