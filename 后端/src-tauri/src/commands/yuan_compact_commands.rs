use tauri::State;

use crate::compact::strategy::CompactStrategy;
use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::compact::{
    BudgetStatus, CompactionConfig, CompactionReason, CompactionRequest, CompactionResult,
    CompactionSession, ConversationMessage, TokenBudget,
};
use crate::services::compact_service::CompactService;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn yuan_compact_config_get(
    state: State<'_, AppState>,
    service: State<'_, CompactService>,
) -> Result<ApiResponse<CompactionConfig>, String> {
    crate::commands::common::require_auth(&state).await?;
    let config = service.get_config().await;
    Ok(ApiResponse::success(config))
}

#[tauri::command]
pub async fn yuan_compact_config_update(
    state: State<'_, AppState>,
    config: CompactionConfig,
    service: State<'_, CompactService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service.update_config(config).await;
    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn yuan_compact_estimate(
    state: State<'_, AppState>,
    messages: Vec<ConversationMessage>,
    service: State<'_, CompactService>,
) -> Result<ApiResponse<TokenBudget>, String> {
    crate::commands::common::require_auth(&state).await?;
    let budget = service.estimate_tokens(messages).await;
    Ok(ApiResponse::success(budget))
}

#[tauri::command]
pub async fn yuan_compact_check(
    state: State<'_, AppState>,
    messages: Vec<ConversationMessage>,
    service: State<'_, CompactService>,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let needs = service.needs_compaction(messages).await;
    Ok(ApiResponse::success(needs))
}

#[tauri::command]
pub async fn yuan_compact_execute(
    state: State<'_, AppState>,
    request: CompactionRequest,
    service: State<'_, CompactService>,
) -> Result<ApiResponse<CompactionResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = service.compact(request).await;
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn yuan_compact_session(
    state: State<'_, AppState>,
    session_id: String,
    service: State<'_, CompactService>,
) -> Result<ApiResponse<Option<CompactionSession>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let session = service.get_session(&session_id).await;
    Ok(ApiResponse::success(session))
}

#[tauri::command]
pub async fn yuan_compact_reset(
    state: State<'_, AppState>,
    session_id: String,
    service: State<'_, CompactService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service.reset_session(&session_id).await;
    Ok(ApiResponse::success(()))
}

// ===== Phase 6.2: Token 预算可视化 =====

/// 获取上下文统计信息（前端可视化用）
#[tauri::command]
pub async fn yuan_compact_stats(
    state: State<'_, AppState>,
    messages: Vec<ConversationMessage>,
) -> Result<ApiResponse<crate::compact::strategy::ContextStats>, String> {
    crate::commands::common::require_auth(&state).await?;
    let stats = CompactStrategy::context_stats(&messages);
    Ok(ApiResponse::success(stats))
}

/// 获取 Token 预算可视化数据
#[tauri::command]
pub async fn yuan_compact_budget_visualization(
    state: State<'_, AppState>,
    messages: Vec<ConversationMessage>,
    max_tokens: usize,
) -> Result<ApiResponse<BudgetVisualization>, String> {
    crate::commands::common::require_auth(&state).await?;
    let total: usize = messages.iter().map(|m| m.token_count).sum();
    let ratio = if max_tokens == 0 {
        1.0
    } else {
        total as f64 / max_tokens as f64
    };

    let status = BudgetStatus::from_ratio(ratio);

    let mut role_breakdown: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for msg in &messages {
        *role_breakdown.entry(msg.role.clone()).or_default() += msg.token_count;
    }

    let mut tier_breakdown: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for msg in &messages {
        if let Some(ref tier) = msg.tier {
            let tier_key = match tier {
                crate::models::compact::MessageTier::Hot => "hot",
                crate::models::compact::MessageTier::Warm => "warm",
                crate::models::compact::MessageTier::Cold => "cold",
            };
            *tier_breakdown.entry(tier_key.to_string()).or_default() += msg.token_count;
        }
    }

    Ok(ApiResponse::success(BudgetVisualization {
        total_tokens: total,
        max_tokens,
        usage_ratio: ratio,
        status,
        available_tokens: max_tokens.saturating_sub(total),
        message_count: messages.len(),
        role_breakdown,
        tier_breakdown,
        decisions_count: messages.iter().filter(|m| m.is_decision).count(),
        errors_count: messages.iter().filter(|m| m.is_error).count(),
    }))
}

// ===== Phase 6.3: 自动压缩触发 =====

/// 自动压缩：检测是否需要压缩，是则执行
#[tauri::command]
pub async fn yuan_compact_auto(
    state: State<'_, AppState>,
    session_id: String,
    messages: Vec<ConversationMessage>,
    system_prompt: Option<String>,
    service: State<'_, CompactService>,
) -> Result<ApiResponse<Option<CompactionResult>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let needs = service.needs_compaction(messages.clone()).await;

    if !needs {
        return Ok(ApiResponse::success(None));
    }

    let request = CompactionRequest {
        session_id,
        messages,
        config: None,
        reason: Some(CompactionReason::TokenBudgetExceeded),
        system_prompt,
    };

    let result = service.compact(request).await;
    Ok(ApiResponse::success(Some(result)))
}

/// 手动触发压缩
#[tauri::command]
pub async fn yuan_compact_manual(
    state: State<'_, AppState>,
    request: CompactionRequest,
    service: State<'_, CompactService>,
) -> Result<ApiResponse<CompactionResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let result = service.compact(request).await;
    Ok(ApiResponse::success(result))
}

/// 强制压缩（忽略阈值检查）
#[tauri::command]
pub async fn yuan_compact_force(
    state: State<'_, AppState>,
    session_id: String,
    messages: Vec<ConversationMessage>,
    system_prompt: Option<String>,
    service: State<'_, CompactService>,
) -> Result<ApiResponse<CompactionResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    let request = CompactionRequest {
        session_id,
        messages,
        config: None,
        reason: Some(CompactionReason::UserRequested),
        system_prompt,
    };

    let result = service.compact(request).await;
    Ok(ApiResponse::success(result))
}

/// Token 预算可视化数据
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BudgetVisualization {
    pub total_tokens: usize,
    pub max_tokens: usize,
    pub usage_ratio: f64,
    pub status: BudgetStatus,
    pub available_tokens: usize,
    pub message_count: usize,
    pub role_breakdown: std::collections::HashMap<String, usize>,
    pub tier_breakdown: std::collections::HashMap<String, usize>,
    pub decisions_count: usize,
    pub errors_count: usize,
}