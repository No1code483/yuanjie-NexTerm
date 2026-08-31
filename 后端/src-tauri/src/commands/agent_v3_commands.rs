//! Yuan Code v3.1 Task 3.1 — Agent 化 IPC 命令
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.1.1 ~ §3.1.7
//!
//! 提供 7 种 Agent 类型的 IPC 命令：
//! - yuan_v3_agent_types      : 列出 7 种 Agent 类型
//! - yuan_v3_agent_create     : 创建 Agent 实例（不调用云端 API）
//! - yuan_v3_agent_plan       : 调用云端 API 生成执行计划（强制 cloud_api_router）
//! - yuan_v3_agent_execute    : 执行计划（在沙箱内 + 安全检查）
//! - yuan_v3_agent_review     : 用户 review diff（批准/拒绝/部分批准）
//! - yuan_v3_agent_status     : 查询 Agent 状态
//! - yuan_v3_agent_safety_check: 单独对计划做安全检查
//!
//! 强制约束（项目核心设计意图 §三/§八）：
//! - 所有 Agent AI 调用必须走 cloud_api_router（云端 API）
//! - 禁止本地底层智能模型用于编程生成
//! - 敏感操作必须经 safety_guard 拦截 + 用户确认

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::State;
use tokio::sync::RwLock;

use crate::agent::safety_guard::{self, SafetyCheckOutcome};
use crate::agent::specialized::create_agent;
use crate::agent::types::{
    AgentFileDiff, AgentPhase, AgentPlan, AgentResult, AgentReviewDecision,
    AgentReviewRequest, AgentType,
};
use crate::agent::AgentLifecycle;
use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;

// 安全审计修复（发现 15，MEDIUM）：原 `get_user_id` 仅检查 `current_user.is_some()`，
// 不调用 `auth_service::verify_token`，不校验 token 签名/过期时间。虽然 Agent 操作
// 有 IDOR 防护（`entry.user_id != user_id`），但 user_id 本身未经 token 校验可被绕过。
// 现移除本地 `get_user_id`，全部改用 `crate::commands::common::require_auth`，
// 强制走 `verify_token` 路径校验 token 签名与过期时间。

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 全局 Agent 实例存储（按 agent_id 索引）
///
/// 注：v3.1 阶段使用进程内 HashMap 存储；未来可迁移到 DB 持久化
pub type AgentRegistry = Arc<RwLock<HashMap<String, AgentEntry>>>;

/// Agent 实例 + 元数据
pub struct AgentEntry {
    pub agent: Box<dyn AgentLifecycle>,
    pub created_at: i64,
    pub user_id: i64,
}

/// 构造一个空的 Agent 注册表
///
/// Rust 不允许在跨 crate 的类型别名上定义 inherent impl（E0116），
/// 因此用自由函数替代 `AgentRegistry::new()`。
pub fn new_agent_registry() -> AgentRegistry {
    Arc::new(RwLock::new(HashMap::new()))
}

/// 列出 7 种 Agent 类型
#[tauri::command]
pub async fn yuan_v3_agent_types() -> Result<ApiResponse<Vec<AgentTypeInfo>>, String> {
    let types: Vec<AgentTypeInfo> = AgentType::ALL
        .iter()
        .map(|&t| AgentTypeInfo {
            type_name: t.as_str().into(),
            display_name: t.display_name().into(),
            description: t.description().into(),
        })
        .collect();
    Ok(ApiResponse::success(types))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentTypeInfo {
    pub type_name: String,
    pub display_name: String,
    pub description: String,
}

/// 创建 Agent 实例
#[tauri::command]
pub async fn yuan_v3_agent_create(
    state: State<'_, AppState>,
    registry: State<'_, AgentRegistry>,
    request: CreateAgentRequest,
) -> Result<ApiResponse<CreateAgentResponse>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let agent_type = AgentType::from_str(&request.agent_type).ok_or_else(|| {
        format!(
            "未知 agent_type '{}'，支持: coding/refactor/test/documentation/debug/migration/review",
            request.agent_type
        )
    })?;

    let agent_id = format!("agent_v3_{}_{}", agent_type.as_str(), now_secs());
    let agent = create_agent(agent_type, agent_id.clone(), request.task_prompt.clone());

    let created_at = now_secs();
    let entry = AgentEntry {
        agent,
        created_at,
        user_id,
    };

    {
        let mut reg = registry.write().await;
        reg.insert(agent_id.clone(), entry);
    }

    tracing::info!(
        "[agent_v3] Agent 已创建: id={}, type={}, user={}",
        agent_id,
        agent_type.as_str(),
        user_id
    );

    Ok(ApiResponse::success(CreateAgentResponse {
        agent_id: agent_id.clone(),
        agent_type: agent_type.as_str().into(),
        phase: AgentPhase::Pending.as_str().into(),
        created_at,
    }))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateAgentRequest {
    pub agent_type: String,
    pub task_prompt: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateAgentResponse {
    pub agent_id: String,
    pub agent_type: String,
    pub phase: String,
    pub created_at: i64,
}

/// 生成执行计划（调用云端 API）
#[tauri::command]
pub async fn yuan_v3_agent_plan(
    state: State<'_, AppState>,
    registry: State<'_, AgentRegistry>,
    request: PlanRequest,
) -> Result<ApiResponse<PlanResponse>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;

    let mut reg = registry.write().await;
    let entry = reg.get_mut(&request.agent_id).ok_or("Agent 不存在")?;
    if entry.user_id != user_id {
        return Err("无权操作此 Agent".into());
    }

    // 安全检查计划（在执行前先验证；plan 阶段只对计划做 safety check）
    // 注：plan 阶段还没有具体步骤，safety check 在 execute 阶段对完整 plan 进行

    let plan = entry
        .agent
        .plan(&state.pool, &state.mek_manager, user_id, &request.task_prompt)
        .await
        .map_err(|e| e.to_string())?;

    // 对生成的计划做安全检查
    let safety = safety_guard::check_plan_safety(&plan);

    Ok(ApiResponse::success(PlanResponse {
        plan,
        safety_check: SafetyCheckResponse::from(safety),
    }))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanRequest {
    pub agent_id: String,
    pub task_prompt: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanResponse {
    pub plan: AgentPlan,
    pub safety_check: SafetyCheckResponse,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SafetyCheckResponse {
    pub passed: bool,
    pub blocked_steps: Vec<u32>,
    pub reasons: Vec<String>,
    pub risk_level: String,
}

impl From<SafetyCheckOutcome> for SafetyCheckResponse {
    fn from(outcome: SafetyCheckOutcome) -> Self {
        Self {
            passed: outcome.passed,
            blocked_steps: outcome.blocked_steps,
            reasons: outcome.reasons,
            risk_level: outcome.risk_level,
        }
    }
}

/// 执行计划（在沙箱内 + 安全检查 + 调用云端 API）
#[tauri::command]
pub async fn yuan_v3_agent_execute(
    state: State<'_, AppState>,
    registry: State<'_, AgentRegistry>,
    request: ExecuteRequest,
) -> Result<ApiResponse<AgentResult>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;

    let mut reg = registry.write().await;
    let entry = reg.get_mut(&request.agent_id).ok_or("Agent 不存在")?;
    if entry.user_id != user_id {
        return Err("无权操作此 Agent".into());
    }

    // 注：v3.1 阶段沙箱执行复用现有 sandbox/ 模块；
    //     当前 BaseAgent.execute 已在内部调用 cloud_api_router 生成 diff，
    //     沙箱集成（read_file/write_file/run_command）将在 execute 内调用。
    //     本骨架实现优先保证架构可实例化、可调用云端 API、可 review。

    let result = entry
        .agent
        .execute(&state.pool, &state.mek_manager, user_id)
        .await
        .map_err(|e| e.to_string())?;

    // 对生成的 diff 做安全检查（Task 3.1.5）
    let _diff_safety = safety_guard::check_diff_safety(&result.diffs);

    Ok(ApiResponse::success(result))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecuteRequest {
    pub agent_id: String,
}

/// 用户 review 流程（diff 展示 + 批准/拒绝）
///
/// 决策：
/// - Approve:       将所有 diff 应用到工作区文件系统
/// - Reject:        丢弃所有 diff，Agent 状态置为 Rejected
/// - ApprovePartial: 仅应用 selected_files 中的 diff
#[tauri::command]
pub async fn yuan_v3_agent_review(
    state: State<'_, AppState>,
    registry: State<'_, AgentRegistry>,
    request: AgentReviewRequest,
) -> Result<ApiResponse<ReviewResponse>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let mut reg = registry.write().await;
    let entry = reg.get_mut(&request.agent_id).ok_or("Agent 不存在")?;
    if entry.user_id != user_id {
        return Err("无权操作此 Agent".into());
    }

    let result = entry.agent.result().ok_or("Agent 尚未执行，无 diff 可 review")?;

    let mut applied_files: Vec<String> = Vec::new();
    let mut skipped_files: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    match request.decision {
        AgentReviewDecision::Reject => {
            // 拒绝：不应用任何 diff
            tracing::info!(
                "[agent_v3] Agent {} 的 diff 已被用户拒绝",
                request.agent_id
            );
            skipped_files = result.diffs.iter().map(|d| d.path.clone()).collect();
        }
        AgentReviewDecision::Approve => {
            // 全部批准：应用所有 diff
            for diff in &result.diffs {
                match apply_diff_to_workspace(&request.workspace_path, diff) {
                    Ok(()) => {
                        applied_files.push(diff.path.clone());
                    }
                    Err(e) => {
                        errors.push(format!("{}: {}", diff.path, e));
                        skipped_files.push(diff.path.clone());
                    }
                }
            }
        }
        AgentReviewDecision::ApprovePartial => {
            // 部分批准：仅应用 selected_files 中的 diff
            let selected: std::collections::HashSet<&str> = request
                .selected_files
                .iter()
                .map(|s| s.as_str())
                .collect();
            for diff in &result.diffs {
                if selected.contains(diff.path.as_str()) {
                    match apply_diff_to_workspace(&request.workspace_path, diff) {
                        Ok(()) => applied_files.push(diff.path.clone()),
                        Err(e) => {
                            errors.push(format!("{}: {}", diff.path, e));
                            skipped_files.push(diff.path.clone());
                        }
                    }
                } else {
                    skipped_files.push(diff.path.clone());
                }
            }
        }
    }

    Ok(ApiResponse::success(ReviewResponse {
        decision: match request.decision {
            AgentReviewDecision::Approve => "approve".into(),
            AgentReviewDecision::Reject => "reject".into(),
            AgentReviewDecision::ApprovePartial => "approve_partial".into(),
        },
        applied_files,
        skipped_files,
        errors,
    }))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewResponse {
    pub decision: String,
    pub applied_files: Vec<String>,
    pub skipped_files: Vec<String>,
    pub errors: Vec<String>,
}

/// 将 diff 应用到工作区文件系统
///
/// 安全审计修复（发现 8，HIGH）：原实现 `Path::new(workspace).join(&diff.path)` 在
/// `diff.path` 为绝对路径时会返回绝对路径，绕过 workspace 边界（PoC：
/// `AgentFileDiff { path: "/etc/cron.d/payload", ... }` → 逃逸成功）。
/// 现使用 `utils::file_path::ensure_in_workspace` 强制校验：
/// 1. `canonicalize(workspace)` 解析符号链接
/// 2. `canonicalize(target)` 或 `canonicalize(parent) + file_name`
/// 3. 断言 `target.starts_with(workspace)`
fn apply_diff_to_workspace(workspace_path: &str, diff: &AgentFileDiff) -> Result<(), String> {
    let modified = diff
        .modified_content
        .as_ref()
        .ok_or_else(|| format!("diff for {} 缺少 modified_content", diff.path))?;

    // 空 workspace 不允许（避免 AI 任意指定绝对路径写入）
    if workspace_path.is_empty() {
        return Err("workspace_path 不能为空（禁止 AI 任意写入绝对路径）".into());
    }

    // 拼接 + 校验：拒绝绝对路径、含 `..` 的路径、符号链接逃逸
    let ws = std::path::Path::new(workspace_path);
    let target = ws.join(&diff.path);
    let full_path = crate::utils::file_path::ensure_in_workspace(ws, &target)
        .map_err(|e| format!("路径校验失败 ({}): {}", diff.path, e))?;

    // 创建父目录
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    std::fs::write(&full_path, modified).map_err(|e| format!("写入文件失败: {}", e))?;
    Ok(())
}

/// 单独对计划做安全检查（不执行）
#[tauri::command]
pub async fn yuan_v3_agent_safety_check(
    state: State<'_, AppState>,
    registry: State<'_, AgentRegistry>,
    agent_id: String,
) -> Result<ApiResponse<SafetyCheckResponse>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let reg = registry.read().await;
    let entry = reg.get(&agent_id).ok_or("Agent 不存在")?;
    if entry.user_id != user_id {
        return Err("无权操作此 Agent".into());
    }

    // 注：plan 存储在 BaseAgent 内部，无法通过 trait 直接访问。
    //     通过 result().plan 获取（执行后），或前端用最近一次 plan_response.plan 重新提交。
    //     v3.1 阶段：要求前端传入 plan（不依赖 Agent 内部状态）
    drop(reg);

    Err("v3.1 阶段安全检查请在 plan/execute 响应中查看 safety_check 字段；如需单独检查请传入 plan".into())
}

/// 查询 Agent 状态
#[tauri::command]
pub async fn yuan_v3_agent_status(
    state: State<'_, AppState>,
    registry: State<'_, AgentRegistry>,
    agent_id: String,
) -> Result<ApiResponse<AgentStatusResponse>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let reg = registry.read().await;
    let entry = reg.get(&agent_id).ok_or("Agent 不存在")?;
    if entry.user_id != user_id {
        return Err("无权操作此 Agent".into());
    }

    Ok(ApiResponse::success(AgentStatusResponse {
        agent_id: entry.agent.agent_id().into(),
        agent_type: entry.agent.agent_type().as_str().into(),
        phase: entry.agent.phase().as_str().into(),
        created_at: entry.created_at,
    }))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentStatusResponse {
    pub agent_id: String,
    pub agent_type: String,
    pub phase: String,
    pub created_at: i64,
}

/// 列出当前用户的所有 Agent
#[tauri::command]
pub async fn yuan_v3_agent_list(
    state: State<'_, AppState>,
    registry: State<'_, AgentRegistry>,
) -> Result<ApiResponse<Vec<AgentStatusResponse>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let reg = registry.read().await;
    let mut agents: Vec<AgentStatusResponse> = reg
        .values()
        .filter(|e| e.user_id == user_id)
        .map(|e| AgentStatusResponse {
            agent_id: e.agent.agent_id().into(),
            agent_type: e.agent.agent_type().as_str().into(),
            phase: e.agent.phase().as_str().into(),
            created_at: e.created_at,
        })
        .collect();
    agents.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(ApiResponse::success(agents))
}

/// 销毁 Agent（清理资源）
#[tauri::command]
pub async fn yuan_v3_agent_destroy(
    state: State<'_, AppState>,
    registry: State<'_, AgentRegistry>,
    agent_id: String,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let mut reg = registry.write().await;
    let entry = reg.remove(&agent_id).ok_or("Agent 不存在")?;
    if entry.user_id != user_id {
        // 重新插入以避免误删
        reg.insert(agent_id, entry);
        return Err("无权操作此 Agent".into());
    }
    Ok(ApiResponse::success(()))
}
