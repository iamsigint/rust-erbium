pub mod p2p;
pub mod protocol;
pub mod discovery;
pub mod sync;
pub mod firewall;
pub mod security;
pub mod p2p_network;
pub mod transport;
pub mod dht;

// Re-export security components
pub use security::{
    NetworkSecurityManager, NetworkSecurityConfig, NetworkSecurityAudit,
    PeerAuthenticator, DDoSProtection, TrustLevel
};
