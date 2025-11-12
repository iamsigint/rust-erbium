//! Bridge Security Monitoring and Slashing
//!
//! This module implements security monitoring for cross-chain bridges,
//! including validator slashing for malicious behavior and security alerts.

use crate::utils::error::{Result, BlockchainError};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Bridge security monitor
pub struct BridgeSecurityMonitor {
    /// Security alerts and incidents
    alerts: Arc<RwLock<Vec<SecurityAlert>>>,
    /// Validator reputation scores
    validator_scores: Arc<RwLock<HashMap<String, ValidatorScore>>>,
    /// Slashing events
    slashing_events: Arc<RwLock<Vec<SlashingEvent>>>,
    /// Security configuration
    config: SecurityConfig,
}

/// Security alert types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityAlert {
    /// Invalid transaction detected
    InvalidTransaction {
        bridge_id: String,
        tx_hash: String,
        reason: String,
        severity: AlertSeverity,
        timestamp: u64,
    },
    /// Validator misbehavior detected
    ValidatorMisbehavior {
        validator_id: String,
        bridge_id: String,
        misbehavior_type: MisbehaviorType,
        evidence: Vec<u8>,
        timestamp: u64,
    },
    /// Bridge under attack
    BridgeAttack {
        bridge_id: String,
        attack_type: AttackType,
        affected_transactions: Vec<String>,
        timestamp: u64,
    },
    /// Configuration violation
    ConfigViolation {
        bridge_id: String,
        violation_type: String,
        details: String,
        timestamp: u64,
    },
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Types of validator misbehavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MisbehaviorType {
    /// Double signing detected
    DoubleSigning,
    /// Invalid signature
    InvalidSignature,
    /// Censorship of valid transactions
    Censorship,
    /// Incorrect validation
    InvalidValidation,
    /// Timeout without justification
    UnjustifiedTimeout,
}

/// Types of bridge attacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttackType {
    /// DDoS attack
    DDoS,
    /// Sybil attack
    Sybil,
    /// Eclipse attack
    Eclipse,
    /// Man-in-the-middle
    MITM,
    /// Bridge manipulation
    BridgeManipulation,
}

/// Validator reputation score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorScore {
    pub validator_id: String,
    pub bridge_id: String,
    pub score: f64, // 0.0 to 1.0
    pub total_stake: u64,
    pub slashed_amount: u64,
    pub last_update: u64,
    pub incidents: Vec<String>,
}

/// Slashing event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingEvent {
    pub validator_id: String,
    pub bridge_id: String,
    pub slash_amount: u64,
    pub reason: String,
    pub evidence_hash: String,
    pub timestamp: u64,
    pub processed: bool,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub alert_thresholds: AlertThresholds,
    pub slashing_parameters: SlashingParameters,
    pub monitoring_intervals: MonitoringIntervals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub max_invalid_txs_per_hour: u32,
    pub max_misbehaviors_per_day: u32,
    pub score_degradation_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingParameters {
    pub double_sign_slash_percent: u8, // Percentage of stake to slash
    pub invalid_sig_slash_percent: u8,
    pub censorship_slash_percent: u8,
    pub min_slash_amount: u64,
    pub max_slash_amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringIntervals {
    pub alert_check_interval_secs: u64,
    pub score_update_interval_secs: u64,
    pub slashing_check_interval_secs: u64,
}

impl BridgeSecurityMonitor {
    /// Create a new bridge security monitor
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            alerts: Arc::new(RwLock::new(Vec::new())),
            validator_scores: Arc::new(RwLock::new(HashMap::new())),
            slashing_events: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Report a security alert
    pub async fn report_alert(&self, alert: SecurityAlert) -> Result<()> {
        let mut alerts = self.alerts.write().await;
        alerts.push(alert.clone());

        // Log the alert
        match &alert {
            SecurityAlert::InvalidTransaction { bridge_id, tx_hash, reason, severity, .. } => {
                log::warn!("Bridge {}: Invalid transaction {} - {} (severity: {:?})",
                          bridge_id, tx_hash, reason, severity);
            }
            SecurityAlert::ValidatorMisbehavior { validator_id, bridge_id, misbehavior_type, .. } => {
                log::error!("Bridge {}: Validator {} misbehavior: {:?}",
                           bridge_id, validator_id, misbehavior_type);
            }
            SecurityAlert::BridgeAttack { bridge_id, attack_type, .. } => {
                log::error!("Bridge {}: Under attack - {:?}", bridge_id, attack_type);
            }
            SecurityAlert::ConfigViolation { bridge_id, violation_type, .. } => {
                log::warn!("Bridge {}: Configuration violation - {}", bridge_id, violation_type);
            }
        }

        Ok(())
    }

    /// Update validator score
    pub async fn update_validator_score(
        &self,
        validator_id: String,
        bridge_id: String,
        score_change: f64,
        reason: String,
    ) -> Result<()> {
        let mut scores = self.validator_scores.write().await;

        let score = scores.entry(format!("{}_{}", validator_id, bridge_id))
            .or_insert(ValidatorScore {
                validator_id: validator_id.clone(),
                bridge_id: bridge_id.clone(),
                score: 1.0, // Start with perfect score
                total_stake: 0,
                slashed_amount: 0,
                last_update: current_timestamp(),
                incidents: Vec::new(),
            });

        // Update score (clamped between 0.0 and 1.0)
        score.score = (score.score + score_change).max(0.0).min(1.0);
        score.last_update = current_timestamp();
        score.incidents.push(reason);

        log::info!("Updated validator {} score for bridge {}: {:.3}",
                  validator_id, bridge_id, score.score);

        Ok(())
    }

    /// Process slashing for validator misbehavior
    pub async fn process_slashing(
        &self,
        validator_id: String,
        bridge_id: String,
        misbehavior_type: MisbehaviorType,
        evidence: Vec<u8>,
    ) -> Result<Option<SlashingEvent>> {
        let slash_percent = match misbehavior_type {
            MisbehaviorType::DoubleSigning => self.config.slashing_parameters.double_sign_slash_percent,
            MisbehaviorType::InvalidSignature => self.config.slashing_parameters.invalid_sig_slash_percent,
            MisbehaviorType::Censorship => self.config.slashing_parameters.censorship_slash_percent,
            _ => 5, // Default 5% for other misbehaviors
        };

        let scores = self.validator_scores.read().await;
        let validator_key = format!("{}_{}", validator_id, bridge_id);

        if let Some(score) = scores.get(&validator_key) {
            let slash_amount = ((score.total_stake as f64) * (slash_percent as f64) / 100.0) as u64;
            let clamped_slash = slash_amount
                .max(self.config.slashing_parameters.min_slash_amount)
                .min(self.config.slashing_parameters.max_slash_amount);

            let slashing_event = SlashingEvent {
                validator_id: validator_id.clone(),
                bridge_id: bridge_id.clone(),
                slash_amount: clamped_slash,
                reason: format!("{:?}", misbehavior_type),
                evidence_hash: format!("{:x}", sha256::digest(&evidence)),
                timestamp: current_timestamp(),
                processed: false,
            };

            let mut slashing_events = self.slashing_events.write().await;
            slashing_events.push(slashing_event.clone());

            log::warn!("Slashing validator {} for bridge {}: {} tokens ({:?})",
                      validator_id, bridge_id, clamped_slash, misbehavior_type);

            Ok(Some(slashing_event))
        } else {
            Ok(None)
        }
    }

    /// Get security alerts
    pub async fn get_alerts(&self, severity_filter: Option<AlertSeverity>) -> Vec<SecurityAlert> {
        let alerts = self.alerts.read().await;

        if let Some(filter) = severity_filter {
            alerts.iter()
                .filter(|alert| {
                    match alert {
                        SecurityAlert::InvalidTransaction { severity, .. } => severity == &filter,
                        SecurityAlert::ValidatorMisbehavior { .. } => matches!(filter, AlertSeverity::High | AlertSeverity::Critical),
                        SecurityAlert::BridgeAttack { .. } => matches!(filter, AlertSeverity::Critical),
                        SecurityAlert::ConfigViolation { .. } => matches!(filter, AlertSeverity::Medium | AlertSeverity::High),
                    }
                })
                .cloned()
                .collect()
        } else {
            alerts.clone()
        }
    }

    /// Get validator scores
    pub async fn get_validator_scores(&self) -> HashMap<String, ValidatorScore> {
        self.validator_scores.read().await.clone()
    }

    /// Get slashing events
    pub async fn get_slashing_events(&self, processed: Option<bool>) -> Vec<SlashingEvent> {
        let events = self.slashing_events.read().await;

        if let Some(processed_filter) = processed {
            events.iter()
                .filter(|event| event.processed == processed_filter)
                .cloned()
                .collect()
        } else {
            events.clone()
        }
    }

    /// Get security status
    pub async fn get_security_status(&self) -> SecurityStatus {
        let alerts = self.alerts.read().await;
        let scores = self.validator_scores.read().await;
        let slashings = self.slashing_events.read().await;

        let critical_alerts = alerts.iter()
            .filter(|alert| matches!(alert, SecurityAlert::BridgeAttack { .. } | SecurityAlert::ValidatorMisbehavior { .. }))
            .count();

        let low_score_validators = scores.values()
            .filter(|score| score.score < self.config.alert_thresholds.score_degradation_threshold)
            .count();

        SecurityStatus {
            total_alerts: alerts.len(),
            critical_alerts,
            low_score_validators,
            pending_slashes: slashings.iter().filter(|s| !s.processed).count(),
            last_update: current_timestamp(),
        }
    }

    /// Run security checks
    pub async fn run_security_checks(&self) -> Result<()> {
        // Check for alert thresholds
        let alerts = self.alerts.read().await;
        let recent_alerts: Vec<_> = alerts.iter()
            .filter(|alert| {
                let alert_time = match alert {
                    SecurityAlert::InvalidTransaction { timestamp, .. } => *timestamp,
                    SecurityAlert::ValidatorMisbehavior { timestamp, .. } => *timestamp,
                    SecurityAlert::BridgeAttack { timestamp, .. } => *timestamp,
                    SecurityAlert::ConfigViolation { timestamp, .. } => *timestamp,
                };
                current_timestamp() - alert_time < 3600 // Last hour
            })
            .collect();

        if recent_alerts.len() > self.config.alert_thresholds.max_invalid_txs_per_hour as usize {
            log::error!("Security alert: Too many alerts in the last hour ({})", recent_alerts.len());
        }

        Ok(())
    }
}

/// Security status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityStatus {
    pub total_alerts: usize,
    pub critical_alerts: usize,
    pub low_score_validators: usize,
    pub pending_slashes: usize,
    pub last_update: u64,
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_monitor_creation() {
        let config = SecurityConfig {
            alert_thresholds: AlertThresholds {
                max_invalid_txs_per_hour: 10,
                max_misbehaviors_per_day: 5,
                score_degradation_threshold: 0.5,
            },
            slashing_parameters: SlashingParameters {
                double_sign_slash_percent: 50,
                invalid_sig_slash_percent: 10,
                censorship_slash_percent: 20,
                min_slash_amount: 1000,
                max_slash_amount: 100000,
            },
            monitoring_intervals: MonitoringIntervals {
                alert_check_interval_secs: 60,
                score_update_interval_secs: 300,
                slashing_check_interval_secs: 600,
            },
        };

        let monitor = BridgeSecurityMonitor::new(config);
        assert_eq!(monitor.config.alert_thresholds.max_invalid_txs_per_hour, 10);
    }

    #[tokio::test]
    async fn test_alert_reporting() {
        let config = SecurityConfig {
            alert_thresholds: AlertThresholds {
                max_invalid_txs_per_hour: 10,
                max_misbehaviors_per_day: 5,
                score_degradation_threshold: 0.5,
            },
            slashing_parameters: SlashingParameters {
                double_sign_slash_percent: 50,
                invalid_sig_slash_percent: 10,
                censorship_slash_percent: 20,
                min_slash_amount: 1000,
                max_slash_amount: 100000,
            },
            monitoring_intervals: MonitoringIntervals {
                alert_check_interval_secs: 60,
                score_update_interval_secs: 300,
                slashing_check_interval_secs: 600,
            },
        };

        let monitor = BridgeSecurityMonitor::new(config);

        let alert = SecurityAlert::InvalidTransaction {
            bridge_id: "ethereum".to_string(),
            tx_hash: "0x123".to_string(),
            reason: "Invalid signature".to_string(),
            severity: AlertSeverity::High,
            timestamp: current_timestamp(),
        };

        monitor.report_alert(alert).await.unwrap();

        let alerts = monitor.get_alerts(None).await;
        assert_eq!(alerts.len(), 1);
    }

    #[tokio::test]
    async fn test_validator_score_update() {
        let config = SecurityConfig {
            alert_thresholds: AlertThresholds {
                max_invalid_txs_per_hour: 10,
                max_misbehaviors_per_day: 5,
                score_degradation_threshold: 0.5,
            },
            slashing_parameters: SlashingParameters {
                double_sign_slash_percent: 50,
                invalid_sig_slash_percent: 10,
                censorship_slash_percent: 20,
                min_slash_amount: 1000,
                max_slash_amount: 100000,
            },
            monitoring_intervals: MonitoringIntervals {
                alert_check_interval_secs: 60,
                score_update_interval_secs: 300,
                slashing_check_interval_secs: 600,
            },
        };

        let monitor = BridgeSecurityMonitor::new(config);

        monitor.update_validator_score(
            "validator1".to_string(),
            "ethereum".to_string(),
            -0.1,
            "Invalid signature".to_string(),
        ).await.unwrap();

        let scores = monitor.get_validator_scores().await;
        assert_eq!(scores.len(), 1);
        assert_eq!(scores["validator1_ethereum"].score, 0.9);
    }
}
