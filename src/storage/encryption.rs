/// Master encryption key
#[derive(Debug, Clone)]
pub struct MasterKey {
    /// The actual encryption key bytes
    _key: Vec<u8>,
    /// Key creation timestamp
    _created_at: u64,
    /// Key version for rotation
    _version: u32,
}
