//! Yuan Code v3.1 Task 3.1.5 — Agent 安全检查机制
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.1.5
//!
//! 设计：
//! - 在 Agent 执行前对计划中的每个 step 进行安全检查
//! - 敏感操作（删除文件、修改依赖、运行 shell）必须经用户确认
//! - 复用现有 safety/ 模块的 SafetyCategory 概念
//! - 不替代 safety/ 模块，只是为 Agent 增加一层"前置守卫"
//!
//! 边界：
//! - 安全检查不阻断 Agent 调用云端 API（编程 AI 仍走 cloud_api_router）
//! - 仅对"会修改文件系统 / 执行命令"的步骤进行拦截

use crate::agent::types::{AgentPlan, AgentPlanStep};

/// 安全检查结果
#[derive(Debug, Clone)]
pub struct SafetyCheckOutcome {
    /// 是否通过（true = 可执行；false = 需用户确认）
    pub passed: bool,
    /// 需要用户确认的步骤序号
    pub blocked_steps: Vec<u32>,
    /// 拦截原因（每个被拦截步骤一条）
    pub reasons: Vec<String>,
    /// 整体风险等级：safe / warning / dangerous
    pub risk_level: String,
}

impl SafetyCheckOutcome {
    pub fn approved() -> Self {
        Self {
            passed: true,
            blocked_steps: Vec::new(),
            reasons: Vec::new(),
            risk_level: "safe".into(),
        }
    }
}

/// 敏感操作关键词（出现在 step.title / description 中即视为需要确认）
const SENSITIVE_PATTERNS: &[&str] = &[
    // 文件删除类
    "delete", "remove", "rm ", "rmdir", "del ",
    // 依赖变更类
    "npm install", "yarn add", "cargo add", "pip install", "go get",
    "dependency", "依赖",
    // Shell 执行类
    "shell", "bash ", "sh -c", "exec", "subprocess",
    "run command", "execute command",
    // 系统级修改
    "chmod", "chown", "sudo", "registry", "注册表",
    // 网络请求类
    "curl ", "wget ", "http request",
];

/// 对 Agent 执行计划进行安全检查
///
/// 返回值：
/// - `passed = true`：所有步骤均可安全执行
/// - `passed = false`：存在敏感步骤，需用户在 review UI 中确认
pub fn check_plan_safety(plan: &AgentPlan) -> SafetyCheckOutcome {
    let mut blocked = Vec::new();
    let mut reasons = Vec::new();
    let mut max_risk = "safe";

    for step in &plan.steps {
        // step.requires_confirmation 已由 parse_plan_steps 初步标记
        // 此处进一步扫描关键词，确保覆盖
        let combined = format!("{} {}", step.title, step.description).to_lowercase();
        let mut step_blocked = step.requires_confirmation;
        for pattern in SENSITIVE_PATTERNS {
            if combined.contains(pattern) {
                step_blocked = true;
                if max_risk == "safe" {
                    max_risk = "warning";
                }
                if matches!(
                    *pattern,
                    "delete" | "remove" | "rm " | "sudo" | "chmod"
                ) {
                    max_risk = "dangerous";
                }
                break;
            }
        }
        if step_blocked {
            blocked.push(step.step_id);
            reasons.push(format!(
                "Step {}: '{}' 涉及敏感操作，需用户确认",
                step.step_id, step.title
            ));
        }
    }

    let passed = blocked.is_empty();
    SafetyCheckOutcome {
        passed,
        blocked_steps: blocked,
        reasons,
        risk_level: max_risk.into(),
    }
}

/// 对单个步骤进行安全检查（用于动态步骤执行时）
pub fn check_step_safety(step: &AgentPlanStep) -> bool {
    let combined = format!("{} {}", step.title, step.description).to_lowercase();
    if step.requires_confirmation {
        return false;
    }
    for pattern in SENSITIVE_PATTERNS {
        if combined.contains(pattern) {
            return false;
        }
    }
    true
}

/// 对 Agent 生成的 diff 进行安全检查
///
/// 检查项：
/// - 是否修改了关键配置文件（package.json / Cargo.toml / go.mod 等）
/// - 是否删除了大量代码（removed_lines > 100）
/// - 是否引入了危险 API 调用
pub fn check_diff_safety(diffs: &[crate::agent::types::AgentFileDiff]) -> SafetyCheckOutcome {
    let mut blocked = Vec::new();
    let mut reasons = Vec::new();
    let mut max_risk = "safe";

    const PROTECTED_FILES: &[&str] = &[
        "package.json", "package-lock.json", "yarn.lock",
        "Cargo.toml", "Cargo.lock",
        "go.mod", "go.sum",
        "requirements.txt", "Pipfile", "pyproject.toml",
        ".env", ".env.local", ".env.production",
        "tsconfig.json", "vite.config.ts", "webpack.config.js",
    ];

    for (idx, diff) in diffs.iter().enumerate() {
        let path_lower = diff.path.to_lowercase();
        let filename = path_lower.rsplit('/').next().unwrap_or("");

        // 修改受保护文件
        if PROTECTED_FILES.iter().any(|p| filename == *p) {
            blocked.push(idx as u32);
            reasons.push(format!(
                "Diff {}: 修改受保护文件 {}（配置/依赖文件，建议人工 review)",
                idx, diff.path
            ));
            if max_risk == "safe" {
                max_risk = "warning";
            }
        }

        // 大量删除
        if diff.removed_lines > 100 {
            blocked.push(idx as u32);
            reasons.push(format!(
                "Diff {}: 删除 {} 行（>100，建议人工 review）",
                idx, diff.removed_lines
            ));
            if max_risk != "dangerous" {
                max_risk = "warning";
            }
        }

        // 危险 API
        if let Some(ref content) = diff.modified_content {
            let lower = content.to_lowercase();
            if lower.contains("eval(") || lower.contains("exec(") || lower.contains("system(") {
                blocked.push(idx as u32);
                reasons.push(format!(
                    "Diff {}: 包含危险 API 调用（eval/exec/system），需用户确认",
                    idx
                ));
                max_risk = "dangerous";
            }
        }
    }

    let passed = blocked.is_empty();
    SafetyCheckOutcome {
        passed,
        blocked_steps: blocked,
        reasons,
        risk_level: max_risk.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_step(id: u32, title: &str, desc: &str, requires_confirmation: bool) -> AgentPlanStep {
        AgentPlanStep {
            step_id: id,
            title: title.into(),
            description: desc.into(),
            target_files: Vec::new(),
            requires_confirmation,
            status: "pending".into(),
        }
    }

    fn make_plan(steps: Vec<AgentPlanStep>) -> AgentPlan {
        AgentPlan {
            agent_id: "test".into(),
            agent_type: "coding".into(),
            title: "test".into(),
            summary: "".into(),
            steps,
            estimated_steps: 0,
            estimated_duration_sec: None,
            created_at: 0,
        }
    }

    #[test]
    fn test_check_plan_safety_approves_safe_steps() {
        let plan = make_plan(vec![
            make_step(1, "Add function foo", "", false),
            make_step(2, "Update tests", "", false),
        ]);
        let outcome = check_plan_safety(&plan);
        assert!(outcome.passed);
        assert_eq!(outcome.risk_level, "safe");
        assert!(outcome.blocked_steps.is_empty());
    }

    #[test]
    fn test_check_plan_safety_blocks_delete_step() {
        let plan = make_plan(vec![
            make_step(1, "Add function", "", false),
            make_step(2, "Delete old files", "", false),
        ]);
        let outcome = check_plan_safety(&plan);
        assert!(!outcome.passed);
        assert!(outcome.blocked_steps.contains(&2));
        assert_eq!(outcome.risk_level, "dangerous");
    }

    #[test]
    fn test_check_plan_safety_blocks_dependency_changes() {
        let plan = make_plan(vec![
            make_step(1, "npm install new-package", "", false),
        ]);
        let outcome = check_plan_safety(&plan);
        assert!(!outcome.passed);
        assert_eq!(outcome.risk_level, "warning");
    }

    #[test]
    fn test_check_step_safety_approves_safe() {
        let step = make_step(1, "Add function", "", false);
        assert!(check_step_safety(&step));
    }

    #[test]
    fn test_check_step_safety_blocks_explicit_confirmation() {
        let step = make_step(1, "anything", "", true);
        assert!(!check_step_safety(&step));
    }

    #[test]
    fn test_check_diff_safety_approves_safe_diff() {
        let diff = crate::agent::types::AgentFileDiff {
            path: "src/main.rs".into(),
            unified_diff: "+fn foo() {}".into(),
            original_content: Some("".into()),
            modified_content: Some("fn foo() {}".into()),
            added_lines: 1,
            removed_lines: 0,
            is_new_file: true,
        };
        let outcome = check_diff_safety(&[diff]);
        assert!(outcome.passed);
    }

    #[test]
    fn test_check_diff_safety_blocks_protected_file() {
        let diff = crate::agent::types::AgentFileDiff {
            path: "package.json".into(),
            unified_diff: "+\"dep\": \"1.0\"".into(),
            original_content: Some("{}".into()),
            modified_content: Some("{\"dep\": \"1.0\"}".into()),
            added_lines: 1,
            removed_lines: 0,
            is_new_file: false,
        };
        let outcome = check_diff_safety(&[diff]);
        assert!(!outcome.passed);
        assert_eq!(outcome.risk_level, "warning");
    }

    #[test]
    fn test_check_diff_safety_blocks_dangerous_api() {
        let diff = crate::agent::types::AgentFileDiff {
            path: "src/eval.rs".into(),
            unified_diff: "+eval(user_input)".into(),
            original_content: Some("".into()),
            modified_content: Some("eval(user_input)".into()),
            added_lines: 1,
            removed_lines: 0,
            is_new_file: true,
        };
        let outcome = check_diff_safety(&[diff]);
        assert!(!outcome.passed);
        assert_eq!(outcome.risk_level, "dangerous");
    }

    #[test]
    fn test_check_diff_safety_blocks_large_removal() {
        let diff = crate::agent::types::AgentFileDiff {
            path: "src/legacy.rs".into(),
            unified_diff: "-line1\n-line2\n...".into(),
            original_content: Some("old".into()),
            modified_content: Some("new".into()),
            added_lines: 1,
            removed_lines: 150,
            is_new_file: false,
        };
        let outcome = check_diff_safety(&[diff]);
        assert!(!outcome.passed);
    }
}
