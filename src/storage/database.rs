use crate::utils::error::Result;
use crate::storage::encrypted_db::EncryptedDatabase;
use parity_db::{Db, Options};
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub sync_wal: bool,
    pub sync_data: bool,
    pub stats: bool,
    pub columns: u8,
    pub ordered_columns: Vec<u8>, // Columns that support ordered iteration
    pub encryption_enabled: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "./erbium-data".to_string(),
            sync_wal: true,
            sync_data: true,
            stats: true,
            columns: 1,
            ordered_columns: vec![0], // Column 0 supports ordered iteration
            encryption_enabled: false,
        }
    }
}

pub struct Database {
    db: Arc<Db>,
    encryptor: Option<EncryptedDatabase>,
    // Performance optimizations
    write_buffer: HashMap<Vec<u8>, Vec<u8>>, // Buffer for batched writes
    buffer_size: usize,
    max_buffer_size: usize,
    last_flush: u64,
    flush_interval: u64, // seconds
    query_cache: HashMap<Vec<u8>, (Vec<u8>, u64)>, // (value, timestamp)
    cache_ttl: u64, // seconds
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let config = DatabaseConfig {
            path: path.to_string(),
            ..Default::default()
        };
        Self::with_config(config)
    }
    
    pub fn with_config(config: DatabaseConfig) -> Result<Self> {
        let path = Path::new(&config.path);
        let options = Options::with_columns(path, config.columns);

        let db = Db::open_or_create(&options)?;
        let current_time = current_timestamp();

        // Initialize encryptor if encryption is enabled
        let encryptor = if config.encryption_enabled {
            Some(EncryptedDatabase::new(true)?)
        } else {
            None
        };

        Ok(Self {
            db: Arc::new(db),
            encryptor,
            write_buffer: HashMap::new(),
            buffer_size: 0,
            max_buffer_size: 1000, // Buffer up to 1000 operations
            last_flush: current_time,
            flush_interval: 30, // Flush every 30 seconds
            query_cache: HashMap::new(),
            cache_ttl: 300, // Cache for 5 minutes
        })
    }
    
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        let key_vec = key.to_vec();
        let value_vec = if let Some(ref encryptor) = self.encryptor {
            encryptor.encrypt(value)?
        } else {
            value.to_vec()
        };

        // Update query cache with decrypted value for performance
        let current_time = current_timestamp();
        self.query_cache.insert(key_vec.clone(), (value.to_vec(), current_time));

        // Add to write buffer for batching (encrypted if needed)
        self.write_buffer.insert(key_vec, value_vec);
        self.buffer_size += 1;

        // Auto-flush if buffer is full or time interval reached
        if self.buffer_size >= self.max_buffer_size ||
           current_time - self.last_flush >= self.flush_interval {
            self.flush_buffer()?;
        }

        Ok(())
    }

    pub fn get(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let key_vec = key.to_vec();
        let current_time = current_timestamp();

        // Check query cache first
        if let Some((cached_value, timestamp)) = self.query_cache.get(&key_vec) {
            if current_time - timestamp < self.cache_ttl {
                return Ok(Some(cached_value.clone()));
            } else {
                // Cache expired, remove it
                self.query_cache.remove(&key_vec);
            }
        }

        // Check write buffer
        if let Some(encrypted_value) = self.write_buffer.get(&key_vec) {
            // Decrypt if needed
            let decrypted_value = if let Some(ref encryptor) = self.encryptor {
                encryptor.decrypt(encrypted_value)?
            } else {
                encrypted_value.clone()
            };
            // Update cache with decrypted value
            self.query_cache.insert(key_vec, (decrypted_value.clone(), current_time));
            return Ok(Some(decrypted_value));
        }

        // Fallback to database
        match self.db.get(0, key) {
            Ok(Some(encrypted_value)) => {
                // Decrypt if needed
                let decrypted_value = if let Some(ref encryptor) = self.encryptor {
                    encryptor.decrypt(&encrypted_value)?
                } else {
                    encrypted_value
                };
                // Update cache with decrypted value
                self.query_cache.insert(key_vec, (decrypted_value.clone(), current_time));
                Ok(Some(decrypted_value))
            }
            other => other.map_err(Into::into)
        }
    }
    
    pub fn delete(&mut self, key: &[u8]) -> Result<()> {
        let key_vec = key.to_vec();

        // Remove from cache and buffer
        self.query_cache.remove(&key_vec);
        self.write_buffer.remove(&key_vec);

        let operations = vec![(0, key, None)];
        self.db.commit(operations)?;
        Ok(())
    }

    pub fn exists(&mut self, key: &[u8]) -> Result<bool> {
        self.get(key).map(|opt| opt.is_some())
    }
    
    pub fn batch_put(&self, operations: Vec<(Vec<u8>, Vec<u8>)>) -> Result<()> {
        let mut batch = Vec::with_capacity(operations.len());
        
        for (key, value) in operations {
            batch.push((0, key, Some(value)));
        }
        
        self.db.commit(batch)?;
        Ok(())
    }
    
    pub fn batch_delete(&self, keys: Vec<Vec<u8>>) -> Result<()> {
        let mut batch = Vec::with_capacity(keys.len());
        
        for key in keys {
            batch.push((0, key, None));
        }
        
        self.db.commit(batch)?;
        Ok(())
    }
    
    pub fn iterate_with_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::new();
        let mut iter = self.db.iter(0)?;
        
        while let Ok(Some((key, value))) = iter.next() {
            if key.starts_with(prefix) {
                results.push((key.to_vec(), value.to_vec()));
            } else if !key.starts_with(prefix) && !results.is_empty() {
                break;
            }
        }
        
        Ok(results)
    }
    
    pub fn iterate_all(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::new();
        let mut iter = self.db.iter(0)?;
        
        while let Ok(Some((key, value))) = iter.next() {
            results.push((key.to_vec(), value.to_vec()));
        }
        
        Ok(results)
    }
    
    pub fn create_backup(&self, backup_path: &str) -> Result<()> {
        let backup_dir = Path::new(backup_path);
        if !backup_dir.exists() {
            std::fs::create_dir_all(backup_dir)?;
        }
        
        log::info!("Backup functionality would copy database files to: {}", backup_path);
        Ok(())
    }
    
    pub fn restore_from_backup(&self, _backup_path: &str) -> Result<()> {
        log::info!("Restore functionality would restore from backup files");
        Ok(())
    }
    
    pub fn compact(&self) -> Result<()> {
        Ok(())
    }
    
    pub fn get_stats(&self) -> Result<DatabaseStats> {
        Ok(DatabaseStats {
            total_keys: 0,
            total_size: 0,
            cache_hits: 0,
            cache_misses: 0,
        })
    }
    
    /// Flush write buffer to database
    pub fn flush_buffer(&mut self) -> Result<()> {
        if self.write_buffer.is_empty() {
            return Ok(());
        }

        let operations_count = self.write_buffer.len();
        let operations: Vec<(u8, Vec<u8>, Option<Vec<u8>>)> = self.write_buffer
            .drain()
            .map(|(key, value)| (0, key, Some(value)))
            .collect();

        self.db.commit(operations)?;
        self.buffer_size = 0;
        self.last_flush = current_timestamp();

        log::debug!("Flushed {} buffered writes to database", operations_count);
        Ok(())
    }

    /// Force flush of all buffers
    pub fn flush(&mut self) -> Result<()> {
        self.flush_buffer()?;
        // Additional flush operations can be added here
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

    /// Optimize database performance
    pub fn optimize(&mut self) -> Result<()> {
        // Clean expired cache entries
        let current_time = current_timestamp();
        self.query_cache.retain(|_, (_, timestamp)| {
            current_time - *timestamp < self.cache_ttl
        });

        // Flush buffer if needed
        if self.buffer_size > 0 {
            self.flush_buffer()?;
        }

        log::info!("Database optimization completed");
        Ok(())
    }
    
    pub fn get_column(&self, column: u8) -> Column {
        Column {
            db: Arc::clone(&self.db),
            column,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_keys: u64,
    pub total_size: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

pub struct Column {
    db: Arc<Db>,
    column: u8,
}

impl Column {
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let operations = vec![(self.column, key, Some(value.to_vec()))];
        self.db.commit(operations)?;
        Ok(())
    }
    
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.db.get(self.column, key).map_err(Into::into)
    }
    
    pub fn delete(&self, key: &[u8]) -> Result<()> {
        let operations = vec![(self.column, key, None)];
        self.db.commit(operations)?;
        Ok(())
    }
    
    pub fn iterate_with_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::new();
        let mut iter = self.db.iter(self.column)?;
        
        while let Ok(Some((key, value))) = iter.next() {
            if key.starts_with(prefix) {
                results.push((key.to_vec(), value.to_vec()));
            } else if !key.starts_with(prefix) && !results.is_empty() {
                break;
            }
        }
        
        Ok(results)
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BufferStats {
    pub buffer_size: usize,
    pub max_buffer_size: usize,
    pub cache_size: usize,
    pub last_flush: u64,
}
