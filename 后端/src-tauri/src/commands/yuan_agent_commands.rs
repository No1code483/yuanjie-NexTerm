use serde::{Deserialize, Serialize};
use tauri::State;
use std::fs;

use crate::agent::status::AgentStatus;
use crate::db::connection::AppState;
use crate::models::agent::{
    AgentDeployRequest, AgentDeployResponse, AgentListResponse, AgentMessageRequest,
    AgentRoleConfig, AgentSpawnRequest, LiveAgentSnapshot, PermissionLevel,
};
use crate::models::api_response::ApiResponse;
use crate::services::agent_service::AgentService;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

// ===== Agent Execute Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecuteRequest {
    pub prompt: String,
    pub context_files: Vec<String>,
    pub workspace_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifiedFile {
    pub path: String,
    pub original: String,
    pub modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExecuteResponse {
    pub agent_id: String,
    pub plan: String,
    pub result: String,
    pub modified_files: Vec<ModifiedFile>,
    pub terminal_commands: Vec<String>,
}

#[tauri::command]
pub async fn yuan_agent_spawn(
    state: State<'_, AppState>,
    request: AgentSpawnRequest,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<LiveAgentSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .spawn_agent(request)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_agent_fork(
    state: State<'_, AppState>,
    parent_agent_id: String,
    new_role: Option<AgentRoleConfig>,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<LiveAgentSnapshot>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .fork_agent(&parent_agent_id, new_role)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_agent_list(
    state: State<'_, AppState>,
    session_id: String,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<AgentListResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_agents(&session_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_agent_list_all(
    state: State<'_, AppState>,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<Vec<LiveAgentSnapshot>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_all_agents()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_agent_status(
    state: State<'_, AppState>,
    agent_id: String,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .get_agent_status(&agent_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_agent_abort(
    state: State<'_, AppState>,
    agent_id: String,
    reason: String,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .abort_agent(&agent_id, &reason)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_agent_transition(
    state: State<'_, AppState>,
    agent_id: String,
    target_status: String,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let status = match target_status.as_str() {
        "thinking" => AgentStatus::Thinking,
        "executing" => AgentStatus::Executing,
        "waiting_for_user" => AgentStatus::WaitingForUser,
        "idle" => AgentStatus::Idle,
        "completed" => AgentStatus::CompletedEmpty,
        _ => {
            return Err(format!("Invalid target status: {}", target_status));
        }
    };

    service
        .transition_agent(&agent_id, status)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_agent_send_message(
    state: State<'_, AppState>,
    request: AgentMessageRequest,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .send_agent_message(request)
        .await
        .map(|msg_id| ApiResponse::success(msg_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_agent_receive_messages(
    state: State<'_, AppState>,
    agent_id: String,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<Vec<crate::agent::communication::AgentMessage>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .receive_agent_messages(&agent_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_agent_list_roles(
    state: State<'_, AppState>,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<Vec<AgentRoleConfig>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .list_roles()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_agent_get_count(
    state: State<'_, AppState>,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    crate::commands::common::require_auth(&state).await?;
    let (total, active) = service.get_agent_count().await.map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(serde_json::json!({
        "total": total,
        "active": active
    })))
}

#[tauri::command]
pub async fn yuan_agent_cleanup(
    state: State<'_, AppState>,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<usize>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .cleanup_terminal()
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn yuan_agent_list_templates(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<crate::agent::templates::AgentTemplate>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let manager = crate::agent::TemplateManager::new();
    let templates = manager.list_all().into_iter().cloned().collect();
    Ok(ApiResponse::success(templates))
}

#[tauri::command]
pub async fn yuan_agent_deploy(
    state: State<'_, AppState>,
    request: AgentDeployRequest,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<AgentDeployResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    let session_id = request
        .session_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let mut deployed = Vec::new();

    let templates = crate::agent::TemplateManager::new();
    let all_templates = templates.list_all();

    for item in &request.agents {
        let template = all_templates.iter().find(|t| t.name == item.id);
        let role = match template {
            Some(t) => AgentRoleConfig {
                name: t.display_name.clone(),
                system_prompt: t.system_prompt.clone(),
                available_tools: t.tools.clone(),
                permission_level: crate::models::agent::PermissionLevel::from_str("restricted"),
                max_turns: item.max_iterations.unwrap_or(t.max_iterations),
                token_budget: Some(t.token_budget as i64),
            },
            None => {
                let name = item.id.clone();
                AgentRoleConfig {
                    name: name.clone(),
                    system_prompt: format!("You are a helpful {} agent.", name),
                    available_tools: vec!["read_file".into(), "write_file".into()],
                    permission_level: crate::models::agent::PermissionLevel::Restricted,
                    max_turns: item.max_iterations.unwrap_or(30),
                    token_budget: None,
                }
            }
        };

        let spawn_req = AgentSpawnRequest {
            session_id: session_id.clone(),
            role,
            parent_id: None,
        };

        match service.spawn_agent(spawn_req).await {
            Ok(snapshot) => {
                deployed.push(snapshot.agent_id);
            }
            Err(e) => {
                tracing::warn!("Failed to deploy agent {}: {}", item.id, e);
            }
        }
    }

    Ok(ApiResponse::success(AgentDeployResponse {
        success: !deployed.is_empty(),
        message: format!("Deployed {} agents", deployed.len()),
        deployed,
    }))
}

/// 执行 Agent 代码编辑任务
/// 接收自然语言指令 + 上下文文件路径，生成代码修改计划并执行
#[tauri::command]
pub async fn yuan_agent_execute(
    state: State<'_, AppState>,
    request: AgentExecuteRequest,
    service: State<'_, AgentService>,
) -> Result<ApiResponse<AgentExecuteResponse>, String> {
    crate::commands::common::require_auth(&state).await?;
    let session_id = uuid::Uuid::new_v4().to_string();

    // 读取上下文文件内容
    let mut context_contents = Vec::new();
    for file_path in &request.context_files {
        let normalized = file_path.replace('\\', "/");
        let full_path = if request.workspace_path.is_empty() {
            normalized.clone()
        } else {
            format!("{}/{}", request.workspace_path.replace('\\', "/"), normalized)
        };

        match fs::read_to_string(&full_path) {
            Ok(content) => {
                context_contents.push(format!(
                    "// === {} ===\n{}\n// === END {} ===",
                    file_path, content, file_path
                ));
            }
            Err(e) => {
                tracing::warn!("Failed to read context file {}: {}", file_path, e);
            }
        }
    }

    // 构建系统提示词
    let context_block = if context_contents.is_empty() {
        String::from("(无上下文文件)")
    } else {
        context_contents.join("\n\n")
    };

    let system_prompt = format!(
        r#"You are a code editing agent in the Yuan Code editor.
Your task is to analyze the user's request and generate a plan for code modifications.

## User Request
{}

## Context Files
{}

## Instructions
1. Analyze the user's request and the provided context files
2. Create a step-by-step plan for the code modifications
3. Specify which files need to be modified and what changes to make
4. List any terminal commands that should be executed

## Response Format
Respond with JSON:
{{
  "plan": "detailed plan description",
  "result": "summary of what will be done",
  "modified_files": [
    {{ "path": "relative/path/to/file", "original": "original content", "modified": "modified content" }}
  ],
  "terminal_commands": ["command1", "command2"]
}}
"#,
        request.prompt, context_block
    );

    // 创建 Coder Agent 角色
    let role = AgentRoleConfig {
        name: format!("agent_exec_{}", &session_id[..8].to_string()),
        system_prompt,
        available_tools: vec![
            "read_file".into(),
            "write_file".into(),
            "list_files".into(),
            "execute_command".into(),
        ],
        permission_level: PermissionLevel::Restricted,
        max_turns: 30,
        token_budget: Some(128_000),
    };

    // 提交子 Agent 执行
    let spawn_req = AgentSpawnRequest {
        session_id: session_id.clone(),
        role,
        parent_id: None,
    };

    let snapshot = service
        .spawn_agent(spawn_req)
        .await
        .map_err(|e| format!("Failed to spawn agent: {}", e))?;

    // 构建响应 — 实际 AI 推理由前端通过流式/轮询驱动
    // 后端在此返回 Agent 已就绪的信息，前端负责展示执行流程
    let response = AgentExecuteResponse {
        agent_id: snapshot.agent_id.clone(),
        plan: format!(
            "分析请求: {}\n上下文文件: {}\nAgent {} 已就绪，等待执行。",
            request.prompt,
            request.context_files.join(", "),
            snapshot.agent_id
        ),
        result: format!(
            "Coder Agent {} 已成功创建。\n会话: {}\n模型: 默认\nToken 预算: 128,000",
            snapshot.agent_id, session_id
        ),
        modified_files: Vec::new(),
        terminal_commands: Vec::new(),
    };

    Ok(ApiResponse::success(response))
}