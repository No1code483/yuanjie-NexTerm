// 引擎 Tauri 命令 — 前端调用的引擎接口

use tauri::{State, Emitter, AppHandle};
use crate::engine::YuanEngine;
use crate::engine::config::SessionConfig;
use crate::engine::SessionInfo;
use crate::error::app_error::AppError;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::db::connection::AppState;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 引擎应用状态
pub struct EngineState {
    pub engine: Arc<Mutex<YuanEngine>>,
}

/// 创建会话
#[tauri::command]
pub async fn engine_create_session(
    state: State<'_, AppState>,
    engine_state: State<'_, EngineState>,
    model: Option<String>,
    system_prompt: Option<String>,
    workspace_path: Option<String>,
    token_budget: Option<u64>,
    sandbox_enabled: Option<bool>,
) -> Result<String, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut engine = engine_state.engine.lock().await;

    let config = SessionConfig {
        model: model.unwrap_or_else(|| "gpt-4o".into()),
        system_prompt: system_prompt.unwrap_or_default(),
        workspace_path,
        token_budget: token_budget.unwrap_or(200_000),
        sandbox_enabled: sandbox_enabled.unwrap_or(true),
        ..Default::default()
    };

    engine
        .create_session(Some(config))
        .await
        .map_err(|e: AppError| e.to_string())
}

/// 获取会话信息
#[tauri::command]
pub async fn engine_get_session(
    state: State<'_, AppState>,
    engine_state: State<'_, EngineState>,
    session_id: String,
) -> Result<serde_json::Value, String> {
    crate::commands::common::require_auth(&state).await?;
    let engine = engine_state.engine.lock().await;
    let session = engine
        .get_session(&session_id)
        .await
        .map_err(|e: AppError| e.to_string())?;

    let session = session.read().await;
    Ok(serde_json::json!({
        "id": session.id(),
        "created_at": session.created_at().to_string(),
        "turn_count": session.turn_count(),
        "is_active": session.is_active(),
        "state": format!("{:?}", session.state()),
    }))
}

/// 列出所有会话
#[tauri::command]
pub async fn engine_list_sessions(
    state: State<'_, AppState>,
    engine_state: State<'_, EngineState>,
) -> Result<Vec<SessionInfo>, String> {
    crate::commands::common::require_auth(&state).await?;
    let engine = engine_state.engine.lock().await;
    Ok(engine.list_sessions().await)
}

/// 销毁会话
#[tauri::command]
pub async fn engine_destroy_session(
    state: State<'_, AppState>,
    engine_state: State<'_, EngineState>,
    session_id: String,
) -> Result<(), String> {
    crate::commands::common::require_auth(&state).await?;
    let mut engine = engine_state.engine.lock().await;
    engine
        .destroy_session(&session_id)
        .await
        .map_err(|e: AppError| e.to_string())
}

/// 开始回合
#[tauri::command]
pub async fn engine_start_turn(
    state: State<'_, AppState>,
    engine_state: State<'_, EngineState>,
    session_id: String,
    user_input: String,
) -> Result<serde_json::Value, String> {
    crate::commands::common::require_auth(&state).await?;
    let engine = engine_state.engine.lock().await;
    let session = engine
        .get_session(&session_id)
        .await
        .map_err(|e: AppError| e.to_string())?;

    let mut session = session.write().await;
    let turn = session
        .start_turn(user_input)
        .await
        .map_err(|e: AppError| e.to_string())?;

    Ok(serde_json::json!({
        "turn_id": turn.id(),
        "turn_number": turn.number(),
        "status": "started",
    }))
}

/// 完成回合
#[tauri::command]
pub async fn engine_complete_turn(
    state: State<'_, AppState>,
    engine_state: State<'_, EngineState>,
    session_id: String,
    response: String,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
) -> Result<(), String> {
    crate::commands::common::require_auth(&state).await?;
    let engine = engine_state.engine.lock().await;
    let session = engine
        .get_session(&session_id)
        .await
        .map_err(|e: AppError| e.to_string())?;

    let token_usage = if let (Some(input), Some(output)) = (input_tokens, output_tokens) {
        Some(crate::engine::session::TokenUsage::new(input, output))
    } else {
        None
    };

    let mut session = session.write().await;
    session
        .complete_turn(response, token_usage)
        .await
        .map_err(|e: AppError| e.to_string())
}

/// 中止回合
#[tauri::command]
pub async fn engine_abort_turn(
    state: State<'_, AppState>,
    engine_state: State<'_, EngineState>,
    session_id: String,
) -> Result<(), String> {
    crate::commands::common::require_auth(&state).await?;
    let engine = engine_state.engine.lock().await;
    let session = engine
        .get_session(&session_id)
        .await
        .map_err(|e: AppError| e.to_string())?;

    let mut session = session.write().await;
    session.abort_turn().await.map_err(|e: AppError| e.to_string())
}

/// 暂停会话
#[tauri::command]
pub async fn engine_pause_session(
    state: State<'_, AppState>,
    engine_state: State<'_, EngineState>,
    session_id: String,
) -> Result<(), String> {
    crate::commands::common::require_auth(&state).await?;
    let engine = engine_state.engine.lock().await;
    let session = engine
        .get_session(&session_id)
        .await
        .map_err(|e: AppError| e.to_string())?;

    let mut session = session.write().await;
    session.pause().await;
    Ok(())
}

/// 继续会话
#[tauri::command]
pub async fn engine_resume_session(
    state: State<'_, AppState>,
    engine_state: State<'_, EngineState>,
    session_id: String,
) -> Result<(), String> {
    crate::commands::common::require_auth(&state).await?;
    let engine = engine_state.engine.lock().await;
    let session = engine
        .get_session(&session_id)
        .await
        .map_err(|e: AppError| e.to_string())?;

    let mut session = session.write().await;
    session.resume().await;
    Ok(())
}

/// 获取会话的回合列表
#[tauri::command]
pub async fn engine_get_turns(
    state: State<'_, AppState>,
    engine_state: State<'_, EngineState>,
    session_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let engine = engine_state.engine.lock().await;
    let session = engine
        .get_session(&session_id)
        .await
        .map_err(|e: AppError| e.to_string())?;

    let session = session.read().await;
    let turns: Vec<serde_json::Value> = session
        .turns()
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id(),
                "number": t.number(),
                "user_input": t.user_input,
                "response": t.response,
                "status": format!("{:?}", t.status()),
                "duration_secs": t.duration_secs(),
            })
        })
        .collect();

    Ok(turns)
}

/// 获取引擎统计
#[tauri::command]
pub async fn engine_stats(
    state: State<'_, AppState>,
    engine_state: State<'_, EngineState>,
) -> Result<serde_json::Value, String> {
    crate::commands::common::require_auth(&state).await?;
    let engine = engine_state.engine.lock().await;
    Ok(serde_json::json!({
        "session_count": engine.session_count(),
    }))
}

/// 订阅引擎事件 — 将后端 EventBus 事件转发到前端
/// 前端通过 listen('yuan-engine-event') 接收
#[tauri::command]
pub async fn engine_subscribe_events(
    state: State<'_, AppState>,
    app_handle: AppHandle,
    engine_state: State<'_, EngineState>,
    session_id: String,
) -> Result<(), String> {
    crate::commands::common::require_auth(&state).await?;
    let engine = engine_state.engine.lock().await;
    let session = engine
        .get_session(&session_id)
        .await
        .map_err(|e: AppError| e.to_string())?;

    let session = session.read().await;
    let event_bus = session.event_bus();
    let mut rx = {
        let bus = event_bus.read().await;
        bus.subscribe()
    };

    // 后台任务：监听 EventBus 并转发到 Tauri 事件系统
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let event_json = serde_json::to_value(&event).unwrap_or_default();
                    // 使用 Tauri 的 emit 发送到前端
                    let _ = app_handle.emit("yuan-engine-event", event_json);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("引擎事件落后 {} 条", n);
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // 事件总线关闭，退出
                    break;
                }
            }
        }
    });

    Ok(())
}