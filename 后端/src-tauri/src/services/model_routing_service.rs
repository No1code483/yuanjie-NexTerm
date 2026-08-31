//! Yuan Code v3.2 Task 3.4.1 — 模型路由服务（按任务类型路由到云端 API）
//!
//! 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.1（Phase 6 多模型与协作）
//!       + 项目核心设计意图 §三 / §八（强制规则 8.1.1）
//!
//! 核心定位：
//! - Yuan Code 编程 AI 的"按任务类型路由"统一入口
//! - 解析 task_type → ResolvedRoute（provider + model_name + temperature + max_tokens）
//! - 与 cloud_api_router 协作：本服务仅返回路由决策，实际 HTTP 调用由 cloud_api_router 完成
//!
//! 强制约束（项目核心设计意图 §三/§八）：
//! 1. resolve_for_task_type 返回的 provider 必须属于 CLOUD_API_PROVIDERS 白名单
//! 2. 即使 DB 中的 rule 被人为篡改为 ollama，service 层会再次守卫并回退到默认云端 provider
//! 3. default_route_for_task 返回的默认值全部为云端 API provider
//!
//! 与底层智能的边界：
//! - 本服务是路由决策层，不调用本地底层智能模型
//! - 底层智能（D2）的 yuan_code_monitor 仅监测行为，不替代路由决策

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::db::repositories::model_routing_repo;
use crate::error::app_error::AppError;
use crate::models::api_key::CLOUD_API_PROVIDERS;
use crate::models::model_routing::{
    ModelRoutingRule, ResolvedRoute, RouteSource, UpsertRoutingRuleRequest,
    default_route_for_task,
};
use crate::services::ai_model_service::AiModelService;
use crate::services::cloud_api_router::{CloudApiRouter, ProgrammingRequest, ProgrammingResponse};

/// 模型路由服务
pub struct ModelRoutingService {
    /// 复用 CloudApiRouter 完成实际 HTTP 调用（按解析出的 provider+model）
    router: Arc<CloudApiRouter>,
}

impl ModelRoutingService {
    pub fn new(ai_service: Arc<AiModelService>) -> Self {
        let router = Arc::new(CloudApiRouter::new(ai_service));
        Self { router }
    }

    /// 暴露内部 router 供调用方构造请求（非必需，保留扩展点）
    pub fn cloud_router(&self) -> &Arc<CloudApiRouter> {
        &self.router
    }

    /// 解析 task_type → ResolvedRoute
    ///
    /// 决策流程：
    /// 1. 查 model_routing_rules 表（按 task_type）
    /// 2. 若 rule 存在且 is_enabled → 取 rule.provider + model_name
    /// 3. 否则回退到 default_route_for_task
    /// 4. **强制守卫**：解析出的 provider 必须在 CLOUD_API_PROVIDERS 白名单内，
    ///    若被篡改为本地模型（ollama 等），立即回退到默认云端 provider 并记录告警
    pub async fn resolve_for_task_type(
        &self,
        pool: &SqlitePool,
        user_id: i64,
        task_type: &str,
    ) -> Result<ResolvedRoute, AppError> {
        let rule = model_routing_repo::get_by_task_type(pool, user_id, task_type).await?;

        let (provider, model_name, temperature, max_tokens, source) = match rule {
            Some(r) if r.is_enabled => {
                // 守卫 2：DB 中的 provider 必须仍在白名单内（防篡改）
                let provider_lower = r.provider.to_lowercase();
                if !CLOUD_API_PROVIDERS.contains(&provider_lower.as_str()) {
                    tracing::warn!(
                        "[model_routing] task_type={} 的 DB rule.provider='{}' 不在云端白名单内（疑似篡改），\
                         回退到默认云端 provider [项目核心设计意图 §三/§八 强制规则]",
                        task_type,
                        r.provider
                    );
                    let (dp, dm, dt) = default_route_for_task(task_type);
                    (dp.to_string(), dm.to_string(), Some(dt), None, RouteSource::Default)
                } else {
                    (
                        r.provider.clone(),
                        r.model_name.clone(),
                        r.temperature,
                        r.max_tokens,
                        RouteSource::Rule,
                    )
                }
            }
            _ => {
                // rule 不存在或未启用 → 回退到默认云端 provider
                let (dp, dm, dt) = default_route_for_task(task_type);
                (dp.to_string(), dm.to_string(), Some(dt), None, RouteSource::Default)
            }
        };

        Ok(ResolvedRoute {
            task_type: task_type.to_string(),
            provider,
            model_name,
            temperature,
            max_tokens,
            source,
        })
    }

    /// 列出所有路由规则（前端 UI 用）
    pub async fn list_rules(&self, pool: &SqlitePool, user_id: i64) -> Result<Vec<ModelRoutingRule>, AppError> {
        model_routing_repo::list_all(pool, user_id).await
    }

    /// 新增或更新路由规则
    ///
    /// 守卫 1：调用 UpsertRoutingRuleRequest::validate 拒绝本地 provider
    pub async fn upsert_rule(
        &self,
        pool: &SqlitePool,
        user_id: i64,
        request: UpsertRoutingRuleRequest,
    ) -> Result<ModelRoutingRule, AppError> {
        // 守卫 1：模型层校验（拒绝 ollama 等本地 provider）
        request.validate().map_err(AppError::Validation)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let is_enabled = request.is_enabled.unwrap_or(true);

        let rule = model_routing_repo::upsert(
            pool,
            user_id,
            &request.task_type,
            &request.provider,
            &request.model_name,
            request.temperature,
            request.max_tokens,
            is_enabled,
            now,
        )
        .await?;

        tracing::info!(
            "[model_routing] 路由规则已配置: task_type={}, provider={}, model={}, enabled={}",
            rule.task_type,
            rule.provider,
            rule.model_name,
            rule.is_enabled
        );

        Ok(rule)
    }

    /// 启用/禁用规则
    pub async fn set_rule_enabled(
        &self,
        pool: &SqlitePool,
        user_id: i64,
        id: i64,
        is_enabled: bool,
    ) -> Result<(), AppError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        model_routing_repo::set_enabled(pool, user_id, id, is_enabled, now).await
    }

    /// 删除规则
    pub async fn delete_rule(&self, pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
        model_routing_repo::delete(pool, user_id, id).await
    }

    /// 按 task_type 路由并发起编程请求（一站式便捷调用）
    ///
    /// 流程：
    /// 1. resolve_for_task_type 解析云端 provider+model
    /// 2. 构造 ProgrammingRequest（强制走 CloudApiRouter，确保云端 API）
    /// 3. CloudApiRouter 完成 HTTP 调用（含 api_key 解密 + validate_cloud_only 守卫）
    pub async fn route_and_call_programming(
        &self,
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        task_type: &str,
        prompt: String,
        system_prompt: Option<String>,
        agent_id: Option<String>,
        stream: bool,
        app_handle: Option<&tauri::AppHandle>,
        conversation_id: Option<i64>,
    ) -> Result<ProgrammingResponse, AppError> {
        // 1. 解析路由（强制云端 provider）
        let route = self.resolve_for_task_type(pool, user_id, task_type).await?;

        // 2. 构造编程请求
        let req = ProgrammingRequest {
            provider: route.provider.clone(),
            model_name: route.model_name.clone(),
            prompt,
            system_prompt,
            temperature: route.temperature,
            max_tokens: route.max_tokens,
            stream,
            conversation_id,
            agent_id,
        };

        // 3. 调用 CloudApiRouter（守卫 3：validate_cloud_only 再次校验 provider）
        if let Some(handle) = app_handle {
            self.router
                .route_programming_request_stream(pool, mek_manager, user_id, req, handle)
                .await
        } else {
            self.router
                .route_programming_request(pool, mek_manager, user_id, req)
                .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::model_routing::UpsertRoutingRuleRequest;

    async fn setup() -> SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(include_str!("../../migrations/0110_create_model_routing_rules_table/up.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(include_str!("../../migrations/0109_create_api_keys_table/up.sql"))
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    fn make_service() -> ModelRoutingService {
        let ai = Arc::new(AiModelService::new());
        ModelRoutingService::new(ai)
    }

    #[tokio::test]
    async fn test_resolve_falls_back_to_default_cloud_when_no_rule() {
        let pool = setup().await;
        let svc = make_service();

        let route = svc.resolve_for_task_type(&pool, 1, "programming").await.unwrap();
        assert_eq!(route.source, RouteSource::Default);
        assert!(
            CLOUD_API_PROVIDERS.contains(&route.provider.as_str()),
            "默认 provider 必须是云端 API: {}",
            route.provider
        );
    }

    #[tokio::test]
    async fn test_resolve_uses_rule_when_enabled() {
        let pool = setup().await;
        let svc = make_service();

        let req = UpsertRoutingRuleRequest {
            task_type: "programming".into(),
            provider: "anthropic".into(),
            model_name: "claude-sonnet-4-20250514".into(),
            temperature: Some(0.1),
            max_tokens: Some(8192),
            is_enabled: Some(true),
        };
        svc.upsert_rule(&pool, 1, req).await.unwrap();

        let route = svc.resolve_for_task_type(&pool, 1, "programming").await.unwrap();
        assert_eq!(route.source, RouteSource::Rule);
        assert_eq!(route.provider, "anthropic");
        assert_eq!(route.model_name, "claude-sonnet-4-20250514");
        assert_eq!(route.temperature, Some(0.1));
        assert_eq!(route.max_tokens, Some(8192));
    }

    #[tokio::test]
    async fn test_resolve_falls_back_when_rule_disabled() {
        let pool = setup().await;
        let svc = make_service();

        let req = UpsertRoutingRuleRequest {
            task_type: "programming".into(),
            provider: "anthropic".into(),
            model_name: "claude-sonnet-4-20250514".into(),
            temperature: None,
            max_tokens: None,
            is_enabled: Some(false), // 禁用
        };
        svc.upsert_rule(&pool, 1, req).await.unwrap();

        let route = svc.resolve_for_task_type(&pool, 1, "programming").await.unwrap();
        assert_eq!(route.source, RouteSource::Default);
        // 默认 programming 是 openai/gpt-4o
        assert_eq!(route.provider, "openai");
        assert_eq!(route.model_name, "gpt-4o");
    }

    #[tokio::test]
    async fn test_upsert_rejects_local_provider() {
        let pool = setup().await;
        let svc = make_service();

        let req = UpsertRoutingRuleRequest {
            task_type: "programming".into(),
            provider: "ollama".into(), // 本地底层智能模型
            model_name: "qwen3:8b".into(),
            temperature: None,
            max_tokens: None,
            is_enabled: None,
        };
        let err = svc.upsert_rule(&pool, 1, req).await.unwrap_err();
        match err {
            AppError::Validation(msg) => {
                assert!(msg.contains("云端 API"), "应拒绝 ollama: {}", msg);
                assert!(msg.contains("禁止"), "应提示禁止本地模型: {}", msg);
            }
            _ => panic!("应返回 Validation 错误，实际: {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_resolve_falls_back_when_db_rule_provider_tampered_to_local() {
        // 模拟 DB 被篡改：直接 INSERT 一条 provider='ollama' 的规则（绕过 service 校验）
        let pool = setup().await;
        sqlx::query(
            "INSERT INTO model_routing_rules (task_type, provider, model_name, temperature, max_tokens, is_enabled, is_cloud_only, priority, created_at, updated_at)
             VALUES ('programming', 'ollama', 'qwen3:8b', 0.2, NULL, 1, 1, 100, 1000, 1000)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let svc = make_service();
        let route = svc.resolve_for_task_type(&pool, 1, "programming").await.unwrap();
        // 守卫 2 应检测到篡改并回退到默认云端 provider
        assert_eq!(route.source, RouteSource::Default);
        assert!(
            CLOUD_API_PROVIDERS.contains(&route.provider.as_str()),
            "篡改后应回退到云端 provider: {}",
            route.provider
        );
    }

    #[tokio::test]
    async fn test_all_task_types_default_to_cloud() {
        let pool = setup().await;
        let svc = make_service();

        for &tt in crate::models::model_routing::TaskType::all_str() {
            let route = svc.resolve_for_task_type(&pool, 1, tt).await.unwrap();
            assert!(
                CLOUD_API_PROVIDERS.contains(&route.provider.as_str()),
                "task_type {} 默认 provider {} 必须是云端 API",
                tt,
                route.provider
            );
        }
    }
}
