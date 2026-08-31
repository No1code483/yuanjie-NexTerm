use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    #[serde(rename = "safe")]
    Safe,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "critical")]
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &str {
        match self {
            RiskLevel::Safe => "safe",
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        }
    }

    pub fn from_score(score: f64) -> Self {
        if score >= 0.9 {
            RiskLevel::Critical
        } else if score >= 0.7 {
            RiskLevel::High
        } else if score >= 0.4 {
            RiskLevel::Medium
        } else if score >= 0.1 {
            RiskLevel::Low
        } else {
            RiskLevel::Safe
        }
    }

    pub fn can_proceed(&self) -> bool {
        matches!(self, RiskLevel::Safe | RiskLevel::Low)
    }

    pub fn needs_review(&self) -> bool {
        matches!(self, RiskLevel::Medium | RiskLevel::High)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, RiskLevel::Critical)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyCategory {
    DangerousCode,
    FileSystemAccess,
    NetworkAccess,
    CommandInjection,
    PiiExposure,
    ModelPrompt,
    SandboxEscape,
}

impl SafetyCategory {
    pub fn as_str(&self) -> &str {
        match self {
            SafetyCategory::DangerousCode => "dangerous_code",
            SafetyCategory::FileSystemAccess => "filesystem_access",
            SafetyCategory::NetworkAccess => "network_access",
            SafetyCategory::CommandInjection => "command_injection",
            SafetyCategory::PiiExposure => "pii_exposure",
            SafetyCategory::ModelPrompt => "model_prompt",
            SafetyCategory::SandboxEscape => "sandbox_escape",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryResult {
    pub category: String,
    pub score: f64,
    pub risk: String,
    pub matches: Vec<String>,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheckResult {
    pub overall_score: f64,
    pub risk_level: String,
    pub passed: bool,
    pub categories: Vec<CategoryResult>,
    pub summary: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheckRequest {
    pub content: String,
    pub content_type: String,
    pub context: Option<String>,
    pub checks: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyProfile {
    pub name: String,
    pub enabled: bool,
    pub checks: Vec<String>,
    pub threshold: f64,
    pub auto_block: bool,
    pub block_on: Option<String>,
}

impl Default for SafetyProfile {
    fn default() -> Self {
        Self {
            name: "default".into(),
            enabled: true,
            checks: vec![
                "dangerous_code".into(),
                "command_injection".into(),
                "sandbox_escape".into(),
            ],
            threshold: 0.6,
            auto_block: true,
            block_on: Some("critical".into()),
        }
    }
}

impl SafetyProfile {
    pub fn strict() -> Self {
        Self {
            name: "strict".into(),
            enabled: true,
            checks: vec![
                "dangerous_code".into(),
                "filesystem_access".into(),
                "network_access".into(),
                "command_injection".into(),
                "pii_exposure".into(),
                "sandbox_escape".into(),
            ],
            threshold: 0.4,
            auto_block: true,
            block_on: Some("high".into()),
        }
    }

    pub fn permissive() -> Self {
        Self {
            name: "permissive".into(),
            enabled: true,
            checks: vec!["command_injection".into(), "sandbox_escape".into()],
            threshold: 0.8,
            auto_block: false,
            block_on: None,
        }
    }

    pub fn disabled() -> Self {
        Self {
            name: "disabled".into(),
            enabled: false,
            checks: vec![],
            threshold: 1.0,
            auto_block: false,
            block_on: None,
        }
    }
}