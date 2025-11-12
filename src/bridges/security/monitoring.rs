// src/bridges/security/monitoring.rs
use std::collections::{HashMap, VecDeque};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct SuspiciousActivity {
    pub activity_type: SuspiciousActivityType,
    pub description: String,
    pub severity: SecurityRisk,
    pub timestamp: u64,
    pub involved_parties: Vec<String>,
    pub amount: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum SuspiciousActivityType {
    LargeTransfer,
    RapidTransfers,
    UnusualPattern,
    FailedVerification,
    EmergencyTrigger,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SecurityRisk {
    Low,
    Medium,
    High,
    Critical,
}

pub struct SecurityMonitor {
    activities: VecDeque<SuspiciousActivity>,
    transfer_limits: TransferLimits,
    user_limits: HashMap<String, UserLimit>, // address -> limits
    _last_audit: u64,
    emergency_mode: bool,
    risk_threshold: f64,
}

#[derive(Debug, Clone)]
pub struct TransferLimits {
    pub single_transfer_max: u64,
    pub daily_transfer_max: u64,
    pub hourly_transfer_max: u64,
}

#[derive(Debug, Clone)]
pub struct UserLimit {
    pub daily_volume: u64,
    pub hourly_volume: u64,
    pub transfer_count: u32,
    pub last_transfer: u64,
}

impl SecurityMonitor {
    pub fn new() -> Self {
        Self {
            activities: VecDeque::new(),
            transfer_limits: TransferLimits {
                single_transfer_max: 1_000_000_000, // 1M tokens
                daily_transfer_max: 10_000_000_000, // 10M tokens
                hourly_transfer_max: 1_000_000_000, // 1M tokens
            },
            user_limits: HashMap::new(),
            _last_audit: SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            emergency_mode: false,
            risk_threshold: 0.8, // 80% risk threshold for emergency
        }
    }
    
    pub fn monitor_transfer_patterns(
        &mut self,
        amount: u64,
        sender: &str,
        _chain_id: &str,
    ) -> SecurityRisk {
        let mut risk_level = SecurityRisk::Low;
        let current_time = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Check single transfer limit
        if amount > self.transfer_limits.single_transfer_max {
            risk_level = SecurityRisk::High;
            self.record_suspicious_activity(SuspiciousActivity {
                activity_type: SuspiciousActivityType::LargeTransfer,
                description: format!("Large transfer detected: {} from {}", amount, sender),
                severity: SecurityRisk::High,
                timestamp: current_time,
                involved_parties: vec![sender.to_string()],
                amount: Some(amount),
            });
        }
        
        // Update user limits and check patterns
        let user_limit = self.user_limits.entry(sender.to_string())
            .or_insert_with(|| UserLimit {
                daily_volume: 0,
                hourly_volume: 0,
                transfer_count: 0,
                last_transfer: current_time,
            });
        
        // Reset daily volume if new day
        if current_time - user_limit.last_transfer > 86400 {
            user_limit.daily_volume = 0;
            user_limit.hourly_volume = 0;
            user_limit.transfer_count = 0;
        }
        // Reset hourly volume if new hour
        else if current_time - user_limit.last_transfer > 3600 {
            user_limit.hourly_volume = 0;
            user_limit.transfer_count = 0;
        }
        
        // Update volumes
        user_limit.daily_volume += amount;
        user_limit.hourly_volume += amount;
        user_limit.transfer_count += 1;
        user_limit.last_transfer = current_time;
        
        // Capture values before releasing the borrow
        let daily_volume = user_limit.daily_volume;
        let hourly_volume = user_limit.hourly_volume;
        let transfer_count = user_limit.transfer_count;
        let daily_max = self.transfer_limits.daily_transfer_max;
        let hourly_max = self.transfer_limits.hourly_transfer_max;
        
        // Check daily limit
        if daily_volume > daily_max {
            risk_level = SecurityRisk::High;
            self.record_suspicious_activity(SuspiciousActivity {
                activity_type: SuspiciousActivityType::RapidTransfers,
                description: format!("Daily limit exceeded: {} from {}", daily_volume, sender),
                severity: SecurityRisk::High,
                timestamp: current_time,
                involved_parties: vec![sender.to_string()],
                amount: Some(daily_volume),
            });
        }
        
        // Check hourly limit
        if hourly_volume > hourly_max {
            risk_level = SecurityRisk::Medium;
            self.record_suspicious_activity(SuspiciousActivity {
                activity_type: SuspiciousActivityType::RapidTransfers,
                description: format!("Hourly limit exceeded: {} from {}", hourly_volume, sender),
                severity: SecurityRisk::Medium,
                timestamp: current_time,
                involved_parties: vec![sender.to_string()],
                amount: Some(hourly_volume),
            });
        }
        
        // Check for rapid transfers (more than 10 transfers in last hour)
        if transfer_count > 10 {
            risk_level = SecurityRisk::Medium;
            self.record_suspicious_activity(SuspiciousActivity {
                activity_type: SuspiciousActivityType::RapidTransfers,
                description: format!("Rapid transfers detected: {} from {}", transfer_count, sender),
                severity: SecurityRisk::Medium,
                timestamp: current_time,
                involved_parties: vec![sender.to_string()],
                amount: None,
            });
        }
        
        risk_level
    }
    
    pub fn record_suspicious_activity(&mut self, activity: SuspiciousActivity) {
        self.activities.push_back(activity);
        
        // Keep only last 1000 activities
        if self.activities.len() > 1000 {
            self.activities.pop_front();
        }
        
        // Check if we need to trigger emergency mode
        self.check_emergency_conditions();
    }
    
    pub fn emergency_shutdown(&mut self) {
        self.emergency_mode = true;
        
        let emergency_activity = SuspiciousActivity {
            activity_type: SuspiciousActivityType::EmergencyTrigger,
            description: "EMERGENCY SHUTDOWN ACTIVATED".to_string(),
            severity: SecurityRisk::Critical,
            timestamp: SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            involved_parties: vec![],
            amount: None,
        };
        
        self.activities.push_back(emergency_activity);
        
        log::error!("SECURITY MONITOR: EMERGENCY SHUTDOWN ACTIVATED");
    }
    
    pub fn can_resume_operations(&self) -> bool {
        if !self.emergency_mode {
            return true;
        }
        
        // Check if recent activities indicate it's safe to resume
        let recent_critical = self.activities
            .iter()
            .rev()
            .take(100)
            .filter(|a| matches!(a.severity, SecurityRisk::Critical))
            .count();
        
        recent_critical == 0
    }
    
    pub fn get_risk_score(&self) -> f64 {
        let recent_activities = self.activities
            .iter()
            .rev()
            .take(100)
            .collect::<Vec<_>>();
        
        if recent_activities.is_empty() {
            return 0.0;
        }
        
        let total_risk: f64 = recent_activities.iter()
            .map(|a| match a.severity {
                SecurityRisk::Low => 0.1,
                SecurityRisk::Medium => 0.4,
                SecurityRisk::High => 0.7,
                SecurityRisk::Critical => 1.0,
            })
            .sum();
        
        total_risk / recent_activities.len() as f64
    }
    
    fn check_emergency_conditions(&mut self) {
        let risk_score = self.get_risk_score();
        
        if risk_score >= self.risk_threshold && !self.emergency_mode {
            log::warn!("High risk score detected: {}. Considering emergency measures.", risk_score);
            // In a real implementation, this would trigger alerts and potentially auto-shutdown
        }
    }
    
    pub fn get_recent_activities(&self, count: usize) -> Vec<&SuspiciousActivity> {
        self.activities
            .iter()
            .rev()
            .take(count)
            .collect()
    }
}
