//! Yuan Code v3.2 Task 3.4.1 — 模型路由配置 IPC 命令
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.1（Phase 6 多模型与协作）
//!       + 项目核心设计意图 §三（Yuan Code 编程 AI 必须走云端 API 模型）
//!
//! 命令清单：
//! - model_routing_list              : 列出所有路由规则
//! - model_routing_upsert            : 新增/更新规则（强制云端 provider 守卫）
//! - model_routing_set_enabled       : 启用/禁用规则
//! - model_routing_delete            : 删除规则
//! - model_routing_resolve           : 解析 task_type → 路由结果（前端展示用）
//! - model_routing_task_types       : 列出支持的 task_type 白名单
//!
//! 强制约束（项目核心设计意图 §三/§八）：
//! - upsert 入口拒绝本地底层智能模型（service 层 validate 守卫）
//! - resolve 出口再次校验 provider 在云端白名单内（service 层 fallback 守卫）
//! - 本命令层不绕过任何守卫

use std::sync::Arc;

use tauri::State;

use crate::commands::common::require_auth;
use crate::db::connection::AppState;
use crate::error::app_error::AppError;
use crate::models::api_response::ApiResponse;
use crate::models::model_routing::{
    ModelRoutingRule, ResolvedRoute, TaskType, UpsertRoutingRuleRequest,
};
use crate::services::ai_model_service::AiModelService;
use crate::services::model_routing_service::ModelRoutingService;

/// 构建 ModelRoutingService（每次调用新建，无状态）
///
/// 注：service 内部 router 仍复用 AiModelService（无状态），不引入全局状态。
fn make_service() -> ModelRoutingService {
    let ai = Arc::new(AiModelService::new());
    ModelRoutingService::new(ai)
}

/// 列出所有路由规则
#[tauri::command]
pub async fn model_routing_list(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<ModelRoutingRule>>, String> {
    let user_id = require_auth(&state).await?;
    let svc = make_service();
    match svc.list_rules(&state.pool, user_id).await {
        Ok(rules) => Ok(ApiResponse::success(rules)),
        Err(e) => Err(e.to_string()),
    }
}

/// 新增或更新路由规则
///
/// 强制约束：service::upsert_rule 调用 UpsertRoutingRuleRequest::validate，
/// 拒绝本地底层智能模型 provider（如 ollama）。
#[tauri::command]
pub async fn model_routing_upsert(
    state: State<'_, AppState>,
    request: UpsertRoutingRuleRequest,
) -> Result<ApiResponse<ModelRoutingRule>, String> {
    let user_id = require_auth(&state).await?;
    let svc = make_service();
    match svc.upsert_rule(&state.pool, user_id, request).await {
        Ok(rule) => Ok(ApiResponse::success(rule)),
        Err(AppError::Validation(msg)) => Ok(ApiResponse::error(1003, &msg)),
        Err(e) => Err(e.to_string()),
    }
}

/// 启用/禁用规则
#[tauri::command]
pub async fn model_routing_set_enabled(
    state: State<'_, AppState>,
    id: i64,
    is_enabled: bool,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    let svc = make_service();
    match svc.set_rule_enabled(&state.pool, user_id, id, is_enabled).await {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.to_string()),
    }
}

/// 删除规则
#[tauri::command]
pub async fn model_routing_delete(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    let svc = make_service();
    match svc.delete_rule(&state.pool, user_id, id).await {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.to_string()),
    }
}

/// 解析 task_type → ResolvedRoute（前端展示用，不发起实际调用）
///
/// 返回包含 source（Rule | Default），前端可区分"用户配置"还是"系统默认"
#[tauri::command]
pub async fn model_routing_resolve(
    state: State<'_, AppState>,
    task_type: String,
) -> Result<ApiResponse<ResolvedRouteResponse>, String> {
    let user_id = require_auth(&state).await?;
    let svc = make_service();
    match svc.resolve_for_task_type(&state.pool, user_id, &task_type).await {
        Ok(route) => Ok(ApiResponse::success(ResolvedRouteResponse::from(route))),
        Err(e) => Err(e.to_string()),
    }
}

/// 列出支持的 task_type 白名单（前端 UI 用）
#[tauri::command]
pub async fn model_routing_task_types() -> Result<ApiResponse<Vec<TaskTypeInfo>>, String> {
    let infos: Vec<TaskTypeInfo> = TaskType::all_str()
        .iter()
        .map(|&tt| {
            let (default_provider, default_model, default_temp) =
                crate::models::model_routing::default_route_for_task(tt);
            TaskTypeInfo {
                task_type: tt.to_string(),
                display_name: task_type_display_name(tt),
                description: task_type_description(tt),
                default_provider: default_provider.to_string(),
                default_model: default_model.to_string(),
                default_temperature: default_temp,
            }
        })
        .collect();
    Ok(ApiResponse::success(infos))
}

/// 路由解析结果（IPC 响应形态，携带 source 标识）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResolvedRouteResponse {
    pub task_type: String,
    pub provider: String,
    pub model_name: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub source: String, // "rule" | "default"
}

impl From<ResolvedRoute> for ResolvedRouteResponse {
    fn from(r: ResolvedRoute) -> Self {
        let source = match r.source {
            crate::models::model_routing::RouteSource::Rule => "rule",
            crate::models::model_routing::RouteSource::Default => "default",
        };
        Self {
            task_type: r.task_type,
            provider: r.provider,
            model_name: r.model_name,
            temperature: r.temperature,
            max_tokens: r.max_tokens,
            source: source.into(),
        }
    }
}

/// TaskType 元信息（前端 UI 用）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskTypeInfo {
    pub task_type: String,
    pub display_name: String,
    pub description: String,
    pub default_provider: String,
    pub default_model: String,
    pub default_temperature: f64,
}

fn task_type_display_name(tt: &str) -> String {
    match tt {
        "programming" => "编程（Programming）".into(),
        "analysis" => "分析（Analysis）".into(),
        "review" => "审查（Review）".into(),
        "documentation" => "文档（Documentation）".into(),
        "general" => "通用（General）".into(),
        _ => tt.into(),
    }
}

fn task_type_description(tt: &str) -> String {
    match tt {
        "programming" => "代码生成 / 补全 / 重构 / 测试 / 迁移 / 调试（Agent 7 种类型的代码任务）".into(),
        "analysis" => "代码分析 / 项目索引摘要 / 调用图分析".into(),
        "review" => "代码审查 / 安全检查 / diff 评估".into(),
        "documentation" => "文档生成 / 注释 / README / API 文档".into(),
        "general" => "未指定任务类型时的兜底路由".into(),
        _ => tt.into(),
    }
}
