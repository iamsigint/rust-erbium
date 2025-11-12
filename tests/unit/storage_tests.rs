// tests/unit/storage_tests.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_paritydb_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().to_str().unwrap();
        
        let db = Database::new(db_path).unwrap();
        
        // Test put/get
        db.put(b"key1", b"value1").unwrap();
        let value = db.get(b"key1").unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));
        
        // Test exists
        assert!(db.exists(b"key1").unwrap());
        assert!(!db.exists(b"key2").unwrap());
        
        // Test delete
        db.delete(b"key1").unwrap();
        assert!(!db.exists(b"key1").unwrap());
    }

    #[test]
    fn test_storage_manager_integration() {
        let temp_dir = tempdir().unwrap();
        let config = StorageConfig {
            database_path: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };
        
        let mut storage = StorageManager::new(config).unwrap();
        
        // Test encrypted storage
        storage.put(b"secret", b"data", true).unwrap();
        let retrieved = storage.get(b"secret", true).unwrap();
        assert_eq!(retrieved, Some(b"data".to_vec()));
    }
}