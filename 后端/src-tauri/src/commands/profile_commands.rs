use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::profile::{Quote, QuoteDuplicateResult, Resume, UserProfile};
use crate::models::user::{ChangeUsernameRequest, ResetPasswordRequest, UpdateProfileRequest};
use crate::services::auth_service;
use crate::services::intelligence_v4_service;
use crate::services::profile_service;
use crate::commands::common::require_auth;
use crate::db::repositories::{
    kb_repo, chat_repo, user_repo,
};

#[tauri::command]
pub async fn get_profile(
    state: State<'_, AppState>,
    key: String,
) -> Result<ApiResponse<Option<UserProfile>>, String> {
    let _user_id = require_auth(&state).await?;
    match profile_service::get_profile(&state.pool, &key).await {
        Ok(profile) => Ok(ApiResponse::success(profile)),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn set_profile(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<ApiResponse<UserProfile>, String> {
    let _user_id = require_auth(&state).await?;
    match profile_service::set_profile(&state.pool, &key, &value).await {
        Ok(profile) => Ok(ApiResponse::success(profile)),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn get_resumes(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<Resume>>, String> {
    let user_id = require_auth(&state).await?;
    match profile_service::get_resumes(&state.pool, user_id).await {
        Ok(resumes) => Ok(ApiResponse::success(resumes)),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn add_resume(
    state: State<'_, AppState>,
    title: String,
    content: String,
) -> Result<ApiResponse<Resume>, String> {
    let user_id = require_auth(&state).await?;
    match profile_service::add_resume(&state.pool, user_id, &title, &content).await {
        Ok(resume) => {
            intelligence_v4_service::instrument_cmd(&state, "resume", &format!("创建简历/{}", resume.title), None).await;
            Ok(ApiResponse::success(resume))
        }
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn update_resume(
    state: State<'_, AppState>,
    id: i64,
    title: String,
    content: String,
) -> Result<ApiResponse<Resume>, String> {
    let user_id = require_auth(&state).await?;
    match profile_service::update_resume(&state.pool, user_id, id, &title, &content).await {
        Ok(resume) => {
            intelligence_v4_service::instrument_cmd(&state, "resume", &format!("编辑简历/{}", resume.title), None).await;
            Ok(ApiResponse::success(resume))
        }
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_resume(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    match profile_service::delete_resume(&state.pool, user_id, id).await {
        Ok(_) => {
            intelligence_v4_service::instrument_cmd(&state, "resume", "删除简历", None).await;
            Ok(ApiResponse::success(()))
        }
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn get_random_quote(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<Quote>>, String> {
    let user_id = require_auth(&state).await?;
    match profile_service::get_random_quote(&state.pool, user_id).await {
        Ok(quote) => Ok(ApiResponse::success(quote)),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn add_quote(
    state: State<'_, AppState>,
    content: String,
    source: Option<String>,
    quote_type: Option<String>,
) -> Result<ApiResponse<Quote>, String> {
    let user_id = require_auth(&state).await?;
    let qtype = quote_type.unwrap_or_else(|| "daily".into());
    match profile_service::add_quote(&state.pool, user_id, &content, source.as_deref(), &qtype).await {
        Ok(quote) => {
            intelligence_v4_service::instrument_cmd(&state, "quote", "添加语录", None).await;
            Ok(ApiResponse::success(quote))
        }
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn get_all_quotes(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<Quote>>, String> {
    let user_id = require_auth(&state).await?;
    match profile_service::get_all_quotes(&state.pool, user_id).await {
        Ok(quotes) => Ok(ApiResponse::success(quotes)),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn batch_add_quotes(
    state: State<'_, AppState>,
    quotes: Vec<(String, Option<String>, String)>,
) -> Result<ApiResponse<Vec<Quote>>, String> {
    let user_id = require_auth(&state).await?;
    match profile_service::batch_add_quotes(&state.pool, user_id, &quotes).await {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn seed_default_quotes(
    state: State<'_, AppState>,
) -> Result<ApiResponse<usize>, String> {
    let user_id = require_auth(&state).await?;
    let existing = match profile_service::get_all_quotes(&state.pool, user_id).await {
        Ok(quotes) => quotes,
        Err(e) => return Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    };
    if !existing.is_empty() {
        return Ok(ApiResponse::success(0));
    }
    let seed_data = crate::models::seed_quotes::get_seed_quotes();
    match profile_service::batch_add_quotes(&state.pool, user_id, &seed_data).await {
        Ok(result) => Ok(ApiResponse::success(result.len())),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_quote(
    state: State<'_, AppState>,
    id: i64,
) -> Result<ApiResponse<u64>, String> {
    let user_id = require_auth(&state).await?;
    match profile_service::delete_quote_by_id(&state.pool, user_id, id).await {
        Ok(rows) => Ok(ApiResponse::success(rows)),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn profile_change_password(
    state: State<'_, AppState>,
    request: ResetPasswordRequest,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    match auth_service::reset_password(
        &state.pool,
        &state.mek_manager,
        user_id,
        &request.old_password,
        &request.new_password,
    )
    .await
    {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn profile_change_username(
    state: State<'_, AppState>,
    request: ChangeUsernameRequest,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    match auth_service::change_username(&state.pool, user_id, &request.new_username).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

// 更新用户扩展资料（头像 / 签名 / 显示名），None 字段保持原状
#[tauri::command]
pub async fn profile_update_profile(
    state: State<'_, AppState>,
    request: UpdateProfileRequest,
) -> Result<ApiResponse<()>, String> {
    let user_id = require_auth(&state).await?;
    let now = chrono::Local::now().timestamp();
    match user_repo::update_profile(
        &state.pool,
        user_id,
        request.avatar_url.as_deref(),
        request.bio.as_deref(),
        request.display_name.as_deref(),
        now,
    )
    .await
    {
        Ok(_) => {
            intelligence_v4_service::instrument_cmd(&state, "profile", "更新个人资料", None).await;
            Ok(ApiResponse::success(()))
        }
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn save_personal_info(
    state: State<'_, AppState>,
    json: String,
) -> Result<ApiResponse<()>, String> {
    let _user_id = require_auth(&state).await?;
    match profile_service::save_personal_info(&state.pool, &json).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn get_personal_info(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<String>>, String> {
    let _user_id = require_auth(&state).await?;
    match profile_service::get_personal_info(&state.pool).await {
        Ok(info) => Ok(ApiResponse::success(info)),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

#[tauri::command]
pub async fn check_quote_duplicate(
    state: State<'_, AppState>,
    content: String,
) -> Result<ApiResponse<Vec<QuoteDuplicateResult>>, String> {
    let user_id = require_auth(&state).await?;
    match profile_service::check_quote_duplicate(&state.pool, user_id, &content).await {
        Ok(duplicates) => Ok(ApiResponse::success(duplicates)),
        Err(e) => Ok(ApiResponse::error(e.error_code(), &e.to_string())),
    }
}

// ========== Task 11.1: AI 简历润色 ==========

#[tauri::command]
pub async fn resume_polish(
    state: State<'_, AppState>,
    text: String,
    _section: Option<String>,
) -> Result<ApiResponse<String>, String> {
    let _user_id = require_auth(&state).await?;
    // 简单的简历润色实现 - 使用 AI 服务
    let polished = state.intelligence_service
        .query_local_llm(&format!("请润色以下简历内容，保持专业性和简洁性：\n\n{}", text))
        .await
        .unwrap_or_else(|_| text.clone());
    Ok(ApiResponse::success(polished))
}

// ========== Task 11.2: 数据导出 ==========

#[tauri::command]
pub async fn export_user_data(
    state: State<'_, AppState>,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let user_id = require_auth(&state).await?;

    let profile = profile_service::get_profile(&state.pool, "user_profile").await.unwrap_or(None);
    let resumes = profile_service::get_resumes(&state.pool, user_id).await.unwrap_or_default();
    let quotes = profile_service::get_all_quotes(&state.pool, user_id).await.unwrap_or_default();
    let personal_info = profile_service::get_personal_info(&state.pool).await.unwrap_or(None);

    let todos: Vec<serde_json::Value> = sqlx::query_as::<_, (i64, String, String, String, bool, String, String)>(
        "SELECT id, title, date, priority, completed, created_at, updated_at FROM todos WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, title, date, priority, completed, created_at, updated_at)| {
        serde_json::json!({ "id": id, "title": title, "date": date, "priority": priority, "completed": completed, "created_at": created_at, "updated_at": updated_at })
    })
    .collect();

    let journals: Vec<serde_json::Value> = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT date, content, created_at, updated_at FROM journals WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(date, content, created_at, updated_at)| {
        serde_json::json!({ "date": date, "content": content, "created_at": created_at, "updated_at": updated_at })
    })
    .collect();

    let kb_entries = kb_repo::get_all_entries(&state.pool, user_id).await.unwrap_or_default();
    let conversations = chat_repo::get_conversations(&state.pool, user_id).await.unwrap_or_default();

    let export = serde_json::json!({
        "exported_at": chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "user_id": user_id,
        "profile": profile,
        "resumes": resumes,
        "quotes": quotes,
        "personal_info": personal_info,
        "todos": todos,
        "journals": journals,
        "kb_entries": kb_entries,
        "conversations": conversations,
    });

    Ok(ApiResponse::success(export))
}