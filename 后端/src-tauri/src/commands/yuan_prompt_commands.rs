use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::prompt::{
    AgentsMdFile, AssembleSystemPromptRequest, AssembleSystemPromptResult,
    HierarchicalInstructions, PromptTemplateMeta, PromptTemplateType, RenderPromptRequest,
    RenderPromptResult,
};
use crate::services::prompt_service::PromptService;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

// ===== 2.1 模板引擎 =====

/// 列出所有可用模板
#[tauri::command]
pub async fn yuan_prompt_list_templates(
    state: State<'_, AppState>,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<Vec<PromptTemplateMeta>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(service.list_templates().await))
}

/// 获取指定模板的元数据
#[tauri::command]
pub async fn yuan_prompt_get_template(
    state: State<'_, AppState>,
    template_type: String,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<Option<PromptTemplateMeta>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let tt = match template_type.as_str() {
        "system" => PromptTemplateType::System,
        "review" => PromptTemplateType::Review,
        "compact" => PromptTemplateType::Compact,
        "plan" => PromptTemplateType::Plan,
        "sub_agent" => PromptTemplateType::SubAgent,
        "realtime" => PromptTemplateType::Realtime,
        "skills_how_to" => PromptTemplateType::SkillsHowTo,
        _ => return Err(format!("未知模板类型: {}", template_type)),
    };
    Ok(ApiResponse::success(service.get_template(&tt).await))
}

/// 渲染模板
#[tauri::command]
pub async fn yuan_prompt_render(
    state: State<'_, AppState>,
    request: RenderPromptRequest,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<RenderPromptResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .render_template(request)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 设置自定义模板
#[tauri::command]
pub async fn yuan_prompt_set_custom_template(
    state: State<'_, AppState>,
    template_type: String,
    content: String,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let tt = match template_type.as_str() {
        "system" => PromptTemplateType::System,
        "review" => PromptTemplateType::Review,
        "compact" => PromptTemplateType::Compact,
        "plan" => PromptTemplateType::Plan,
        "sub_agent" => PromptTemplateType::SubAgent,
        "realtime" => PromptTemplateType::Realtime,
        "skills_how_to" => PromptTemplateType::SkillsHowTo,
        _ => return Err(format!("未知模板类型: {}", template_type)),
    };
    service.set_custom_template(tt, content).await;
    Ok(ApiResponse::success(()))
}

/// 移除自定义模板
#[tauri::command]
pub async fn yuan_prompt_remove_custom_template(
    state: State<'_, AppState>,
    template_type: String,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    let tt = match template_type.as_str() {
        "system" => PromptTemplateType::System,
        "review" => PromptTemplateType::Review,
        "compact" => PromptTemplateType::Compact,
        "plan" => PromptTemplateType::Plan,
        "sub_agent" => PromptTemplateType::SubAgent,
        "realtime" => PromptTemplateType::Realtime,
        "skills_how_to" => PromptTemplateType::SkillsHowTo,
        _ => return Err(format!("未知模板类型: {}", template_type)),
    };
    service.remove_custom_template(&tt).await;
    Ok(ApiResponse::success(()))
}

/// 设置模板变量默认值
#[tauri::command]
pub async fn yuan_prompt_set_variable_default(
    state: State<'_, AppState>,
    name: String,
    value: String,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service.set_variable_default(&name, &value).await;
    Ok(ApiResponse::success(()))
}

// ===== 2.2 AGENTS.md 发现 =====

/// 发现 AGENTS.md 文件
#[tauri::command]
pub async fn yuan_agents_discover(
    state: State<'_, AppState>,
    cwd: Option<String>,
    project_root: Option<String>,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<Vec<AgentsMdFile>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .discover_agents_md(cwd, project_root)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 获取指令源文件路径列表
#[tauri::command]
pub async fn yuan_agents_sources(
    state: State<'_, AppState>,
    cwd: Option<String>,
    project_root: Option<String>,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<Vec<String>>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .get_instruction_sources(cwd, project_root)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

// ===== 2.3 层级指令组装 =====

/// 组装层级指令
#[tauri::command]
pub async fn yuan_agents_assemble(
    state: State<'_, AppState>,
    agents_files: Vec<AgentsMdFile>,
    user_instructions: Option<String>,
    max_bytes: Option<usize>,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<HierarchicalInstructions>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(
        service
            .assemble_instructions(&agents_files, user_instructions.as_deref(), max_bytes)
            .await,
    ))
}

// ===== 2.4 字节预算 =====

/// 设置项目文档最大字节预算
#[tauri::command]
pub async fn yuan_agents_set_max_bytes(
    state: State<'_, AppState>,
    max_bytes: usize,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    service.set_project_doc_max_bytes(max_bytes).await;
    Ok(ApiResponse::success(()))
}

/// 获取当前项目文档最大字节预算
#[tauri::command]
pub async fn yuan_agents_get_max_bytes(
    state: State<'_, AppState>,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<usize>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(service.get_project_doc_max_bytes().await))
}

// ===== 2.5 系统 Prompt 组装 (含子 Agent 指令) =====

/// 组装完整系统 Prompt
#[tauri::command]
pub async fn yuan_prompt_assemble(
    state: State<'_, AppState>,
    request: AssembleSystemPromptRequest,
    service: State<'_, PromptService>,
) -> Result<ApiResponse<AssembleSystemPromptResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    service
        .assemble_system_prompt(request)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}