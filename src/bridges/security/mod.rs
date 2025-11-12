// src/bridges/security/mod.rs
pub mod monitoring;
pub mod slashing;
pub mod emergency;
pub mod bridge_monitor;

pub use monitoring::{SecurityMonitor, SecurityRisk, SuspiciousActivity};
pub use bridge_monitor::{BridgeSecurityMonitor, SecurityConfig, SecurityAlert, SecurityStatus, SlashingEvent, ValidatorScore};
