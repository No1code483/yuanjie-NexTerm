// Agent 结果合并 — 对标 Codex 的多 Agent 结果合并
// 汇总多个 Agent 的输出，支持多种合并策略

// 合并策略
#[derive(Debug, Clone, PartialEq)]
pub enum MergeStrategy {
    /// 简单拼接（按顺序）
    Concat,
    /// 按优先级选择最佳结果
    BestOf,
    /// 投票合并（多数一致）
    Voting,
    /// 智能合并（依赖 LLM 综合）
    Intelligent,
    /// 取第一个完成的结果
    FirstCompleted,
}

/// Agent 贡献
#[derive(Debug, Clone)]
pub struct AgentContribution {
    /// Agent ID
    pub agent_id: String,
    /// Agent 角色
    pub role: String,
    /// 输出内容
    pub output: String,
    /// 质量评分 (0.0-1.0)
    pub quality_score: f64,
    /// 完成时间 (ms)
    pub duration_ms: u64,
    /// Token 用量
    pub tokens_used: u64,
}

/// 合并结果
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// 合并后内容
    pub content: String,
    /// 使用的策略
    pub strategy: MergeStrategy,
    /// 参与合并的 Agent 数量
    pub contributor_count: usize,
    /// 总 Token 用量
    pub total_tokens: u64,
    /// 总耗时 (ms)
    pub total_duration_ms: u64,
    /// 各 Agent 贡献详情
    pub contributions: Vec<AgentContribution>,
}

/// 结果合并器
pub struct ResultMerger {
    strategy: MergeStrategy,
    contributions: Vec<AgentContribution>,
}

impl ResultMerger {
    pub fn new(strategy: MergeStrategy) -> Self {
        Self {
            strategy,
            contributions: Vec::new(),
        }
    }

    /// 添加 Agent 贡献
    pub fn add_contribution(&mut self, contribution: AgentContribution) {
        self.contributions.push(contribution);
    }

    /// 执行合并
    pub fn merge(&self) -> MergeResult {
        let total_tokens: u64 = self.contributions.iter().map(|c| c.tokens_used).sum();
        let total_duration_ms: u64 = self.contributions.iter().map(|c| c.duration_ms).sum();

        let content = match self.strategy {
            MergeStrategy::Concat => {
                self.contributions
                    .iter()
                    .map(|c| format!("=== {} ({}) ===\n{}", c.role, c.agent_id, c.output))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }
            MergeStrategy::BestOf => {
                self.contributions
                    .iter()
                    .max_by(|a, b| a.quality_score.partial_cmp(&b.quality_score).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|c| c.output.clone())
                    .unwrap_or_default()
            }
            MergeStrategy::FirstCompleted => {
                self.contributions
                    .first()
                    .map(|c| c.output.clone())
                    .unwrap_or_default()
            }
            MergeStrategy::Voting | MergeStrategy::Intelligent => {
                // 投票/智能合并：回退到简单拼接
                // 完整的投票/LLM合并需要额外实现
                self.contributions
                    .iter()
                    .map(|c| c.output.clone())
                    .collect::<Vec<_>>()
                    .join("\n\n---\n\n")
            }
        };

        MergeResult {
            content,
            strategy: self.strategy.clone(),
            contributor_count: self.contributions.len(),
            total_tokens,
            total_duration_ms,
            contributions: self.contributions.clone(),
        }
    }

    /// 贡献数量
    pub fn count(&self) -> usize {
        self.contributions.len()
    }
}

impl Default for ResultMerger {
    fn default() -> Self {
        Self::new(MergeStrategy::Concat)
    }
}

#[cfg(test)]
mod tests {
    //! ResultMerger 单元测试（v1.52 测试体系 Phase 2 - 2.5.1）
    //! 参见：03_测试体系_单元与集成测试.md §2.2.1（agent 优先级 P1）

    use super::*;

    fn make_contribution(id: &str, role: &str, output: &str, score: f64, tokens: u64, dur: u64) -> AgentContribution {
        AgentContribution {
            agent_id: id.to_string(),
            role: role.to_string(),
            output: output.to_string(),
            quality_score: score,
            duration_ms: dur,
            tokens_used: tokens,
        }
    }

    #[test]
    fn test_default_merger_uses_concat_strategy() {
        let merger = ResultMerger::default();
        assert_eq!(merger.strategy, MergeStrategy::Concat);
        assert_eq!(merger.count(), 0);
    }

    #[test]
    fn test_concat_merges_all_outputs_with_headers() {
        let mut merger = ResultMerger::new(MergeStrategy::Concat);
        merger.add_contribution(make_contribution("a1", "coder", "output1", 0.8, 100, 50));
        merger.add_contribution(make_contribution("a2", "reviewer", "output2", 0.9, 200, 60));

        let result = merger.merge();
        assert_eq!(result.contributor_count, 2);
        assert_eq!(result.total_tokens, 300);
        assert_eq!(result.total_duration_ms, 110);
        assert!(result.content.contains("output1"));
        assert!(result.content.contains("output2"));
        assert!(result.content.contains("coder"));
        assert!(result.content.contains("a1"));
    }

    #[test]
    fn test_best_of_picks_highest_quality() {
        let mut merger = ResultMerger::new(MergeStrategy::BestOf);
        merger.add_contribution(make_contribution("a1", "coder", "low", 0.5, 10, 5));
        merger.add_contribution(make_contribution("a2", "coder", "high", 0.95, 20, 8));
        merger.add_contribution(make_contribution("a3", "coder", "mid", 0.7, 15, 6));

        let result = merger.merge();
        assert_eq!(result.content, "high", "BestOf 应选最高分");
        assert_eq!(result.contributor_count, 3);
    }

    #[test]
    fn test_best_of_with_empty_contributions_returns_empty() {
        let merger = ResultMerger::new(MergeStrategy::BestOf);
        let result = merger.merge();
        assert_eq!(result.content, "");
        assert_eq!(result.contributor_count, 0);
    }

    #[test]
    fn test_first_completed_returns_first_contribution() {
        let mut merger = ResultMerger::new(MergeStrategy::FirstCompleted);
        merger.add_contribution(make_contribution("a1", "coder", "first", 0.1, 10, 5));
        merger.add_contribution(make_contribution("a2", "coder", "second", 0.99, 20, 8));

        let result = merger.merge();
        assert_eq!(result.content, "first", "FirstCompleted 应返回第一个");
    }

    #[test]
    fn test_voting_falls_back_to_concat_with_separators() {
        let mut merger = ResultMerger::new(MergeStrategy::Voting);
        merger.add_contribution(make_contribution("a1", "coder", "v1", 0.5, 10, 5));
        merger.add_contribution(make_contribution("a2", "coder", "v2", 0.5, 10, 5));

        let result = merger.merge();
        assert!(result.content.contains("v1"));
        assert!(result.content.contains("v2"));
        assert!(result.content.contains("---"), "Voting 回退应使用分隔符");
    }

    #[test]
    fn test_intelligent_falls_back_to_concat_with_separators() {
        let mut merger = ResultMerger::new(MergeStrategy::Intelligent);
        merger.add_contribution(make_contribution("a1", "coder", "i1", 0.5, 10, 5));
        merger.add_contribution(make_contribution("a2", "coder", "i2", 0.5, 10, 5));

        let result = merger.merge();
        assert!(result.content.contains("i1"));
        assert!(result.content.contains("i2"));
    }

    #[test]
    fn test_count_reflects_added_contributions() {
        let mut merger = ResultMerger::new(MergeStrategy::Concat);
        assert_eq!(merger.count(), 0);
        merger.add_contribution(make_contribution("a1", "r", "x", 0.1, 1, 1));
        assert_eq!(merger.count(), 1);
        merger.add_contribution(make_contribution("a2", "r", "y", 0.1, 1, 1));
        assert_eq!(merger.count(), 2);
    }

    #[test]
    fn test_merge_result_preserves_strategy() {
        let mut merger = ResultMerger::new(MergeStrategy::BestOf);
        merger.add_contribution(make_contribution("a1", "r", "x", 0.5, 1, 1));
        let result = merger.merge();
        assert_eq!(result.strategy, MergeStrategy::BestOf);
    }

    #[test]
    fn test_merge_result_contributions_cloned() {
        let mut merger = ResultMerger::new(MergeStrategy::Concat);
        merger.add_contribution(make_contribution("a1", "coder", "out", 0.5, 100, 50));
        let result = merger.merge();
        assert_eq!(result.contributions.len(), 1);
        assert_eq!(result.contributions[0].agent_id, "a1");
        assert_eq!(result.contributions[0].tokens_used, 100);
    }

    #[test]
    fn test_merge_sums_tokens_and_duration_correctly() {
        let mut merger = ResultMerger::new(MergeStrategy::Concat);
        merger.add_contribution(make_contribution("a1", "r", "x", 0.1, 100, 10));
        merger.add_contribution(make_contribution("a2", "r", "y", 0.1, 200, 20));
        merger.add_contribution(make_contribution("a3", "r", "z", 0.1, 300, 30));
        let result = merger.merge();
        assert_eq!(result.total_tokens, 600);
        assert_eq!(result.total_duration_ms, 60);
    }
}