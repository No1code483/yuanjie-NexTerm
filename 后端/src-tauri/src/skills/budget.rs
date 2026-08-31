/// 技能上下文预算管理
/// 对标 Codex 的 2% 上下文窗口 + 截断警告
pub struct SkillBudgetManager {
    /// 上下文窗口总 token 预算
    total_token_budget: usize,
    /// 技能描述最大占比（默认 2%）
    skill_budget_ratio: f64,
    /// 每个技能描述最大 token 数
    max_skill_description_tokens: usize,
    /// 最低保留技能数
    min_skills_to_keep: usize,
}

impl SkillBudgetManager {
    pub fn new(total_token_budget: usize) -> Self {
        Self {
            total_token_budget,
            skill_budget_ratio: 0.02, // 2%
            max_skill_description_tokens: 500,
            min_skills_to_keep: 3,
        }
    }

    pub fn with_ratio(mut self, ratio: f64) -> Self {
        self.skill_budget_ratio = ratio;
        self
    }

    pub fn with_max_desc_tokens(mut self, max: usize) -> Self {
        self.max_skill_description_tokens = max;
        self
    }

    /// 计算技能描述的总预算
    pub fn skill_budget(&self) -> usize {
        (self.total_token_budget as f64 * self.skill_budget_ratio) as usize
    }

    /// 估算一段文本的 token 数
    pub fn estimate_tokens(text: &str) -> usize {
        // 粗略估算：中文每字1 token，英文每4字符1 token
        let chinese_chars = text.chars().filter(|c| *c as u32 > 0x4e00).count();
        let other_chars = text.len().saturating_sub(chinese_chars);
        chinese_chars + other_chars / 4
    }

    /// 截断技能描述以符合预算
    pub fn truncate_description(&self, description: &str) -> String {
        let tokens = Self::estimate_tokens(description);
        if tokens <= self.max_skill_description_tokens {
            description.to_string()
        } else {
            let chars_per_token = if description.len() > tokens {
                description.len() / tokens
            } else {
                4
            };
            let max_chars = self.max_skill_description_tokens * chars_per_token;
            if max_chars < description.len() {
                format!("{}...", &description[..max_chars])
            } else {
                description.to_string()
            }
        }
    }

    /// 为技能列表分配预算，按优先级排序并截断
    pub fn allocate_budget(
        &self,
        skills: &mut Vec<crate::models::skill::SkillMetadata>,
    ) -> SkillBudgetReport {
        let budget = self.skill_budget();
        let mut used_tokens = 0usize;
        let mut truncated = 0usize;
        let mut skipped = 0usize;
        let mut warnings = Vec::new();

        // 按优先级排序
        skills.sort_by_key(|s| {
            s.policy
                .as_ref()
                .and_then(|p| p.priority)
                .unwrap_or(0)
        });
        skills.reverse();

        let _total_skills = skills.len();
        let max_skills = skills.len().max(self.min_skills_to_keep);

        let mut kept = 0usize;
        for skill in skills.iter_mut().take(max_skills) {
            let desc_tokens = Self::estimate_tokens(&skill.description);

            if used_tokens + desc_tokens > budget {
                if kept >= self.min_skills_to_keep {
                    skipped += 1;
                    continue;
                }
                // 截断描述
                let ratio = (budget.saturating_sub(used_tokens)) as f64 / desc_tokens as f64;
                let max_chars = (skill.description.len() as f64 * ratio) as usize;
                skill.description = if max_chars < skill.description.len() {
                    truncated += 1;
                    format!("{}...", &skill.description[..max_chars.max(10)])
                } else {
                    skill.description.clone()
                };
            }

            used_tokens += Self::estimate_tokens(&skill.description);
            kept += 1;
        }

        if skipped > 0 {
            warnings.push(format!(
                "技能预算超限: {} 个技能因预算不足被跳过 (总预算: {} tokens, 已用: {} tokens)",
                skipped, budget, used_tokens
            ));
        }

        if truncated > 0 {
            warnings.push(format!(
                "{} 个技能描述被截断以适应预算",
                truncated
            ));
        }

        SkillBudgetReport {
            total_budget: budget,
            used_tokens,
            remaining_tokens: budget.saturating_sub(used_tokens),
            skills_loaded: kept,
            skills_skipped: skipped,
            descriptions_truncated: truncated,
            warnings,
        }
    }
}

/// 预算报告
#[derive(Debug, Clone)]
pub struct SkillBudgetReport {
    pub total_budget: usize,
    pub used_tokens: usize,
    pub remaining_tokens: usize,
    pub skills_loaded: usize,
    pub skills_skipped: usize,
    pub descriptions_truncated: usize,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::skill::{SkillMetadata, SkillPolicy, SkillScope};

    fn make_skill(name: &str, desc: &str, priority: i32) -> SkillMetadata {
        SkillMetadata {
            name: name.to_string(),
            description: desc.to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: Some(SkillPolicy {
                allow_implicit_invocation: None,
                auto_discover: None,
                trigger_patterns: None,
                file_patterns: None,
                priority: Some(priority),
            }),
            path: String::new(),
            scope: SkillScope::System,
            plugin_id: None,
            enabled: true,
            file_size: 0,
            created_at: None,
        }
    }

    #[test]
    fn test_estimate_tokens() {
        let tokens = SkillBudgetManager::estimate_tokens("hello world");
        assert!(tokens > 0);
    }

    #[test]
    fn test_budget_allocation() {
        let manager = SkillBudgetManager::new(100_000); // 100K context
        let mut skills = vec![
            make_skill("s1", "Short desc", 10),
            make_skill("s2", "Another description", 8),
            make_skill("s3", "Low priority skill with a very long description that should be handled", 1),
        ];

        let report = manager.allocate_budget(&mut skills);
        assert!(report.skills_loaded > 0);
        assert!(report.used_tokens <= report.total_budget);
    }

    #[test]
    fn test_truncate_description() {
        let manager = SkillBudgetManager::new(100_000).with_max_desc_tokens(10);
        let desc = "This is a very long description that should be truncated";
        let result = manager.truncate_description(desc);
        assert!(result.len() < desc.len());
        assert!(result.ends_with("..."));
    }
}