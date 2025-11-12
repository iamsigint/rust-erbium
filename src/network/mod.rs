pub mod p2p;
pub mod protocol;
pub mod discovery;
pub mod sync;
pub mod firewall;
pub mod security;

// Re-export security components
pub use security::{
    NetworkSecurityManager, NetworkSecurityConfig, NetworkSecurityAudit,
    PeerAuthenticator, DDoSProtection, TrustLevel
};
