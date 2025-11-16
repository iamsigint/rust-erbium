pub mod dht;
pub mod discovery;
pub mod firewall;
pub mod opportunistic;
pub mod p2p;
pub mod p2p_network;
pub mod protocol;
pub mod security;
pub mod sync;
pub mod transport;

// Re-export security components
pub use security::{
    DDoSProtection, NetworkSecurityAudit, NetworkSecurityConfig, NetworkSecurityManager,
    PeerAuthenticator, TrustLevel,
};
