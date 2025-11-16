//! Compliance frameworks and regulatory requirements
//!
//! This module defines compliance frameworks and their requirements.

use serde::{Deserialize, Serialize};

/// Compliance framework types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ComplianceFramework {
    GDPR,
    CCPA,
    SOX,
    PCIDSS,
    HIPAA,
}

/// Framework configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkConfig;

/// Regulatory requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryRequirement;
