pub mod approval;
pub mod checker;
pub mod policy;
pub mod runtime;

use crate::models::safety::{
    RiskLevel, SafetyCategory, SafetyCheckRequest, SafetyCheckResult, SafetyProfile,
};

use self::checker::SafetyChecker;

pub struct SafetyManager {
    checker: SafetyChecker,
    active_profile: SafetyProfile,
}

impl SafetyManager {
    pub fn new() -> Self {
        Self {
            checker: SafetyChecker::new(),
            active_profile: SafetyProfile::default(),
        }
    }

    pub fn with_profile(profile: SafetyProfile) -> Self {
        Self {
            checker: SafetyChecker::new(),
            active_profile: profile,
        }
    }

    pub fn set_profile(&mut self, profile: SafetyProfile) {
        self.active_profile = profile;
    }

    pub fn profile(&self) -> &SafetyProfile {
        &self.active_profile
    }

    pub fn check(&self, req: &SafetyCheckRequest) -> SafetyCheckResult {
        if !self.active_profile.enabled {
            return SafetyCheckResult {
                overall_score: 0.0,
                risk_level: "safe".into(),
                passed: true,
                categories: vec![],
                summary: "安全评估已禁用".into(),
                recommendation: "none".into(),
            };
        }

        let categories_to_check: Vec<SafetyCategory> = if let Some(custom_checks) = &req.checks {
            custom_checks
                .iter()
                .filter_map(|name| match name.as_str() {
                    "dangerous_code" => Some(SafetyCategory::DangerousCode),
                    "filesystem_access" => Some(SafetyCategory::FileSystemAccess),
                    "network_access" => Some(SafetyCategory::NetworkAccess),
                    "command_injection" => Some(SafetyCategory::CommandInjection),
                    "pii_exposure" => Some(SafetyCategory::PiiExposure),
                    "model_prompt" => Some(SafetyCategory::ModelPrompt),
                    "sandbox_escape" => Some(SafetyCategory::SandboxEscape),
                    _ => None,
                })
                .collect()
        } else {
            self.active_profile
                .checks
                .iter()
                .filter_map(|name| match name.as_str() {
                    "dangerous_code" => Some(SafetyCategory::DangerousCode),
                    "filesystem_access" => Some(SafetyCategory::FileSystemAccess),
                    "network_access" => Some(SafetyCategory::NetworkAccess),
                    "command_injection" => Some(SafetyCategory::CommandInjection),
                    "pii_exposure" => Some(SafetyCategory::PiiExposure),
                    "model_prompt" => Some(SafetyCategory::ModelPrompt),
                    "sandbox_escape" => Some(SafetyCategory::SandboxEscape),
                    _ => None,
                })
                .collect()
        };

        let results = self
            .checker
            .check_all(&req.content, &categories_to_check);

        let overall_score = if results.is_empty() {
            0.0
        } else {
            results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64
        };

        let risk_level = RiskLevel::from_score(overall_score);

        let passed = if self.active_profile.auto_block {
            match &self.active_profile.block_on {
                Some(level) if level == "critical" => !risk_level.is_blocked(),
                Some(level) if level == "high" => {
                    !risk_level.is_blocked() && !matches!(risk_level, RiskLevel::High)
                }
                Some(level) if level == "medium" => risk_level.can_proceed(),
                _ => true,
            }
        } else {
            true
        };

        let summary = match risk_level {
            RiskLevel::Safe => "内容安全，未检测到风险".into(),
            RiskLevel::Low => "检测到低风险内容".into(),
            RiskLevel::Medium => "检测到中等风险内容，建议审查".into(),
            RiskLevel::High => "检测到高风险内容，需要审查".into(),
            RiskLevel::Critical => "检测到严重风险内容".into(),
        };

        let recommendation = if risk_level.is_blocked() {
            "block".into()
        } else if risk_level.needs_review() {
            "review".into()
        } else {
            "allow".into()
        };

        SafetyCheckResult {
            overall_score,
            risk_level: risk_level.as_str().into(),
            passed,
            categories: results,
            summary,
            recommendation,
        }
    }

    pub fn quick_check(&self, content: &str) -> bool {
        let req = SafetyCheckRequest {
            content: content.to_string(),
            content_type: "text".into(),
            context: None,
            checks: None,
        };
        self.check(&req).passed
    }

    pub fn content_is_safe(&self, content: &str) -> bool {
        self.quick_check(content)
    }
}