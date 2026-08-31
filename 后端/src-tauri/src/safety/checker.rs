use regex::Regex;

use crate::models::safety::{CategoryResult, SafetyCategory};

pub struct SafetyChecker {
    patterns: Vec<(SafetyCategory, Regex)>,
}

impl SafetyChecker {
    pub fn new() -> Self {
        let patterns = vec![
            (
                SafetyCategory::DangerousCode,
                Regex::new(
                    r"(?im)(rm\s+-rf\s+/|del\s+/F\s+/S\s+/Q\s+\\|format\s+[c-z]:|:\(\)\s*\{|mkfs\.|dd\s+if=)"
                ).unwrap()
            ),
            (
                SafetyCategory::CommandInjection,
                Regex::new(
                    r"(?im)([;|&`$]\s*(?:curl|wget|nc|bash|sh|powershell|cmd)\s)|(eval\s*\(|exec\s*\(|system\s*\()"
                ).unwrap()
            ),
            (
                SafetyCategory::FileSystemAccess,
                Regex::new(
                    r"(?im)(\b/etc/passwd\b|\b/etc/shadow\b|\bC:\\Windows\\System32\b|\b/root/\b|\b\.ssh/\b)"
                ).unwrap()
            ),
            (
                SafetyCategory::NetworkAccess,
                Regex::new(
                    r"(?im)(socat\s|nc\s+-[ln]|ngrok\s|ssh\s+-R|tunnel\s|reverse\s+shell)"
                ).unwrap()
            ),
            (
                SafetyCategory::PiiExposure,
                Regex::new(
                    r"(?im)(\b(?:AKIA|ASIA)[A-Z0-9]{16}\b|sk-[a-zA-Z0-9]{32,}|ghp_[a-zA-Z0-9]{36})"
                ).unwrap()
            ),
            (
                SafetyCategory::SandboxEscape,
                Regex::new(
                    r"(?im)(chroot\s|mount\s|nsenter\s|unshare\s|/proc/self/|/dev/mem|/dev/kmem)"
                ).unwrap()
            ),
            (
                SafetyCategory::ModelPrompt,
                Regex::new(
                    r"(?im)(ignore\s+(?:all\s+)?(?:previous|above)\s+instructions|you\s+are\s+now\s+DAN|pretend\s+you\s+are)"
                ).unwrap()
            ),
        ];

        Self { patterns }
    }

    pub fn check_category(
        &self,
        category: &SafetyCategory,
        content: &str,
    ) -> CategoryResult {
        let matches: Vec<String> = self
            .patterns
            .iter()
            .filter(|(c, _)| std::mem::discriminant(c) == std::mem::discriminant(category))
            .flat_map(|(_, re)| {
                re.find_iter(content)
                    .map(|m| m.as_str().to_string())
                    .collect::<Vec<_>>()
            })
            .collect();

        let match_count = matches.len();
        let score = if match_count == 0 {
            0.0
        } else {
            (match_count as f64 * 0.25).min(1.0)
        };

        let risk = crate::models::safety::RiskLevel::from_score(score).as_str().to_string();

        let details = if match_count == 0 {
            "未检测到风险".into()
        } else {
            format!("检测到 {} 处匹配", match_count)
        };

        CategoryResult {
            category: category.as_str().to_string(),
            score,
            risk,
            matches,
            details,
        }
    }

    pub fn check_all(
        &self,
        content: &str,
        categories: &[SafetyCategory],
    ) -> Vec<CategoryResult> {
        categories
            .iter()
            .map(|c| self.check_category(c, content))
            .collect()
    }

    pub fn check_selected(
        &self,
        content: &str,
        category_names: &[String],
    ) -> Vec<CategoryResult> {
        let categories: Vec<SafetyCategory> = category_names
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
            .collect();

        self.check_all(content, &categories)
    }

    pub fn all_categories() -> Vec<SafetyCategory> {
        vec![
            SafetyCategory::DangerousCode,
            SafetyCategory::CommandInjection,
            SafetyCategory::FileSystemAccess,
            SafetyCategory::NetworkAccess,
            SafetyCategory::PiiExposure,
            SafetyCategory::SandboxEscape,
            SafetyCategory::ModelPrompt,
        ]
    }
}