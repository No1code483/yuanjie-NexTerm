use tauri::State;

use crate::db::connection::AppState;
use crate::db::repositories::terminal_repo;
use crate::models::api_response::ApiResponse;
use crate::models::terminal::{
    BuiltinCommandResult, DirectoryListing, SshConnectionInfo, SshProfile, SshProfileInput,
    SystemInfo, TerminalConfig, TerminalConfigInput, TerminalHistory, TerminalHistoryInput,
    TerminalSession, TerminalTabLayout, TerminalTabLayoutInput, TerminalTheme, WslStatus,
};
use crate::services::terminal_service::SessionInfo;
use crate::services::intelligence_v4_service;

// 安全审计修复（发现 3，HIGH）：所有 terminal_* 命令原无认证，PTY 会话可被未授权
// 创建/写入/杀死，`terminal_execute_builtin` 可执行任意内置命令。现强制在入口
// 调用 `require_auth(&state).await?` 校验当前用户 token。
// 注：`terminal_detect_wsl`（系统查询）与 `terminal_get_themes`（静态列表）无敏感
// 操作，不加认证。

#[tauri::command]
pub async fn terminal_create_session(
    state: State<'_, AppState>,
    session_type: String,
    tab_id: Option<String>,
    pane_id: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<String>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match state
        .terminal_mux
        .create_session(user_id, &session_type, tab_id, pane_id, cols.unwrap_or(80), rows.unwrap_or(24), app_handle)
        .await
    {
        Ok(session_id) => {
            let _ = intelligence_v4_service::instrument_cmd(
                &state, "terminal", "create_session",
                Some(&format!("type: {}", session_type)),
            ).await;
            Ok(ApiResponse::success(session_id))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn terminal_write_input(
    state: State<'_, AppState>,
    session_id: String,
    input: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match state.terminal_service.write_input(&session_id, &input) {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn terminal_resize(
    state: State<'_, AppState>,
    session_id: String,
    cols: u16,
    rows: u16,
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match state.terminal_mux.resize(&session_id, cols, rows, &app_handle).await {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn terminal_kill_session(
    state: State<'_, AppState>,
    session_id: String,
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<i32>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match state.terminal_mux.kill_session(user_id, &session_id, &app_handle).await {
        Ok(exit_code) => {
            let _ = intelligence_v4_service::instrument_cmd(
                &state, "terminal", "kill_session",
                Some(&session_id),
            ).await;
            Ok(ApiResponse::success(exit_code))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn terminal_list_sessions(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<SessionInfo>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let sessions = state.terminal_service.list_sessions();
    Ok(ApiResponse::success(sessions))
}

#[tauri::command]
pub async fn terminal_execute_builtin(
    state: State<'_, AppState>,
    command: String,
) -> Result<ApiResponse<BuiltinCommandResult>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let cmd_lower = command.trim().to_lowercase();
    let parts: Vec<&str> = cmd_lower.split_whitespace().collect();
    let cmd_name = parts.first().map(|s| *s).unwrap_or("");

    if cmd_name == "search" {
        let keyword = parts.get(1).map(|s| *s).unwrap_or("");
        if keyword.is_empty() {
            return Ok(ApiResponse::success(BuiltinCommandResult {
                output: "用法: search <关键词>\n在命令历史中搜索匹配的记录（最多50条）".into(),
                exit_code: 1,
            }));
        }

        match terminal_repo::search_history(&state.pool, user_id, keyword, 50).await {
            Ok(results) => {
                if results.is_empty() {
                    return Ok(ApiResponse::success(BuiltinCommandResult {
                        output: format!("未找到包含 \"{}\" 的历史记录", keyword),
                        exit_code: 1,
                    }));
                } else {
                    let mut output = format!("搜索 \"{}\" — 找到 {} 条记录:\n\n", keyword, results.len());
                    for (i, entry) in results.iter().enumerate() {
                        let time = chrono::DateTime::from_timestamp_millis(entry.created_at)
                            .map(|dt| dt.format("%m-%d %H:%M").to_string())
                            .unwrap_or_else(|| "未知时间".into());
                        let preview = entry.output.as_deref().unwrap_or("").chars().take(80).collect::<String>();
                        output.push_str(&format!(
                            "{}. [{}] {}\n   → {}{}\n\n",
                            i + 1,
                            time,
                            entry.command,
                            preview,
                            if entry.output.as_deref().unwrap_or("").len() > 80 { "..." } else { "" }
                        ));
                    }
                    return Ok(ApiResponse::success(BuiltinCommandResult {
                        output,
                        exit_code: 0,
                    }));
                }
            }
            Err(e) => return Ok(ApiResponse::success(BuiltinCommandResult {
                output: format!("搜索失败: {}", e),
                exit_code: 1,
            })),
        }
    }

    let start = std::time::Instant::now();
    let result = state.terminal_service.execute_builtin(&command);
    let duration_ms = start.elapsed().as_millis() as i64;

    let now = chrono::Utc::now().timestamp_millis();
    let _ = terminal_repo::save_history(
        &state.pool,
        user_id,
        &TerminalHistoryInput {
            command: command.clone(),
            output: Some(result.output.clone()),
            exit_code: result.exit_code,
            session_type: "builtin".into(),
            duration_ms: Some(duration_ms),
        },
        now,
    )
    .await;

    let op = format!("执行终端命令/{}", cmd_name);
    intelligence_v4_service::instrument_cmd(&state, "terminal", &op, None).await;
    Ok(ApiResponse::success(result))
}

#[tauri::command]
pub async fn terminal_get_system_info(
    state: State<'_, AppState>,
) -> Result<ApiResponse<SystemInfo>, String> {
    crate::commands::common::require_auth(&state).await?;
    let info = state.terminal_service.get_system_info();
    Ok(ApiResponse::success(info))
}

#[tauri::command]
pub async fn terminal_list_directory(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<ApiResponse<DirectoryListing>, String> {
    crate::commands::common::require_auth(&state).await?;
    match state.terminal_service.list_directory(path.as_deref()) {
        Ok(listing) => Ok(ApiResponse::success(listing)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn terminal_get_history(
    state: State<'_, AppState>,
    limit: Option<i64>,
    session_type: Option<String>,
) -> Result<ApiResponse<Vec<TerminalHistory>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match terminal_repo::get_history(&state.pool, user_id, limit.unwrap_or(50), session_type.as_deref()).await
    {
        Ok(history) => Ok(ApiResponse::success(history)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn terminal_clear_history(
    state: State<'_, AppState>,
    session_type: Option<String>,
) -> Result<ApiResponse<u64>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match terminal_repo::clear_history(&state.pool, user_id, session_type.as_deref()).await {
        Ok(count) => Ok(ApiResponse::success(count)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn terminal_get_history_count(
    state: State<'_, AppState>,
) -> Result<ApiResponse<i64>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    match terminal_repo::get_history_count(&state.pool, user_id).await {
        Ok(count) => Ok(ApiResponse::success(count)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn terminal_save_layout(
    state: State<'_, AppState>,
    tabs: Vec<TerminalTabLayoutInput>,
) -> Result<ApiResponse<()>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let now = chrono::Utc::now().timestamp_millis();
    terminal_repo::save_tab_layout(&state.pool, user_id, &tabs, now)
        .await
        .map(|_| ApiResponse::success(()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn terminal_load_layout(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<TerminalTabLayout>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    terminal_repo::load_tab_layout(&state.pool, user_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn terminal_clear_layout(
    state: State<'_, AppState>,
) -> Result<ApiResponse<u64>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    terminal_repo::clear_tab_layout(&state.pool, user_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn terminal_detect_wsl(
    state: State<'_, AppState>,
) -> Result<ApiResponse<WslStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    let status = crate::services::terminal_service::TerminalService::detect_wsl();
    Ok(ApiResponse::success(status))
}

/// 创建 WSL 会话（支持指定发行版和 Shell）
/// 参考 Windows Terminal 的 wsl.exe -d <distro> 模式
#[tauri::command]
pub async fn terminal_create_wsl_session(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    distro: Option<String>,
    shell: Option<String>,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let session_id = state
        .terminal_service
        .create_session_with_opts(
            "wsl",
            distro.as_deref(),
            shell.as_deref(),
            app_handle,
        )
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(session_id))
}

// ========== Mux 层命令 ==========

/// 获取所有持久化的活跃会话
#[tauri::command]
pub async fn terminal_mux_get_active_sessions(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<TerminalSession>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    state.terminal_mux
        .get_active_sessions(user_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 清理所有持久化会话记录
#[tauri::command]
pub async fn terminal_mux_clear_all_sessions(
    state: State<'_, AppState>,
) -> Result<ApiResponse<u64>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    state.terminal_mux
        .clear_all_sessions(user_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 使用 FTS5 全文搜索历史命令
#[tauri::command]
pub async fn terminal_search_history_fts(
    state: State<'_, AppState>,
    keyword: String,
    limit: Option<i64>,
) -> Result<ApiResponse<Vec<TerminalHistory>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    terminal_repo::search_history_fts(&state.pool, user_id, &keyword, limit.unwrap_or(50))
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

// ========== SSH 管理命令 ==========

/// 获取所有 SSH 连接配置
#[tauri::command]
pub async fn terminal_ssh_list_profiles(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<SshProfile>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    crate::db::repositories::ssh_repo::list_profiles(&state.pool, user_id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 保存 SSH 连接配置
#[tauri::command]
pub async fn terminal_ssh_save_profile(
    state: State<'_, AppState>,
    input: SshProfileInput,
) -> Result<ApiResponse<SshProfile>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    crate::db::repositories::ssh_repo::save_profile(&state.pool, user_id, &input, &id, now)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 删除 SSH 连接配置
#[tauri::command]
pub async fn terminal_ssh_delete_profile(
    state: State<'_, AppState>,
    id: String,
) -> Result<ApiResponse<u64>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    crate::db::repositories::ssh_repo::delete_profile(&state.pool, user_id, &id)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 建立 SSH 连接
#[tauri::command]
pub async fn terminal_ssh_connect(
    state: State<'_, AppState>,
    profile_id: String,
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<SshConnectionInfo>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let profile = crate::db::repositories::ssh_repo::get_profile(&state.pool, user_id, &profile_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "SSH 配置不存在".to_string())?;

    let session_id = uuid::Uuid::new_v4().to_string();
    state.ssh_service.connect(
        &session_id,
        &profile.host,
        profile.port as u16,
        &profile.username,
        None,
        profile.private_key_path.as_deref(),
        &app_handle,
    ).map_err(|e| e.to_string())?;

    state.ssh_service.start_read_loop(session_id.clone(), app_handle);

    Ok(ApiResponse::success(SshConnectionInfo {
        session_id,
        profile_name: profile.name,
        host: profile.host,
        port: profile.port,
        username: profile.username,
        status: "connected".into(),
    }))
}

/// 断开 SSH 连接
#[tauri::command]
pub async fn terminal_ssh_disconnect(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    state.ssh_service.disconnect(&session_id)
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(()))
}

/// 写入 SSH 数据
#[tauri::command]
pub async fn terminal_ssh_write(
    state: State<'_, AppState>,
    session_id: String,
    data: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    state.ssh_service.write(&session_id, data.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(()))
}

/// 调整 SSH PTY 尺寸
#[tauri::command]
pub async fn terminal_ssh_resize(
    state: State<'_, AppState>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    state.ssh_service.resize(&session_id, cols, rows)
        .map_err(|e| e.to_string())?;
    Ok(ApiResponse::success(()))
}

// ========== 终端配置 & 主题 ==========

/// 获取内置终端主题列表
#[tauri::command]
pub async fn terminal_get_themes(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<TerminalTheme>>, String> {
    crate::commands::common::require_auth(&state).await?;
    Ok(ApiResponse::success(builtin_themes()))
}

/// 获取终端配置
#[tauri::command]
pub async fn terminal_get_config(
    state: State<'_, AppState>,
) -> Result<ApiResponse<TerminalConfig>, String> {
    crate::commands::common::require_auth(&state).await?;
    terminal_repo::get_terminal_config(&state.pool)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

/// 保存终端配置
#[tauri::command]
pub async fn terminal_save_config(
    state: State<'_, AppState>,
    input: TerminalConfigInput,
) -> Result<ApiResponse<TerminalConfig>, String> {
    crate::commands::common::require_auth(&state).await?;
    let now = chrono::Utc::now().timestamp_millis();
    terminal_repo::save_terminal_config(&state.pool, &input, now)
        .await
        .map(ApiResponse::success)
        .map_err(|e| e.to_string())
}

fn builtin_themes() -> Vec<TerminalTheme> {
    vec![
        TerminalTheme {
            name: "Dark+".into(),
            foreground: "#D4D4D4".into(),
            background: "#1E1E1E".into(),
            cursor: "#FFFFFF".into(),
            cursor_accent: "#1E1E1E".into(),
            selection: "#264F78".into(),
            black: "#000000".into(),
            red: "#CD3131".into(),
            green: "#0DBC79".into(),
            yellow: "#E5E510".into(),
            blue: "#2472C8".into(),
            magenta: "#BC3FBC".into(),
            cyan: "#11A8CD".into(),
            white: "#E5E5E5".into(),
            bright_black: "#666666".into(),
            bright_red: "#F14C4C".into(),
            bright_green: "#23D18B".into(),
            bright_yellow: "#F5F543".into(),
            bright_blue: "#3B8EEA".into(),
            bright_magenta: "#D670D6".into(),
            bright_cyan: "#29B8DB".into(),
            bright_white: "#FFFFFF".into(),
        },
        TerminalTheme {
            name: "Monokai".into(),
            foreground: "#F8F8F2".into(),
            background: "#272822".into(),
            cursor: "#F8F8F0".into(),
            cursor_accent: "#272822".into(),
            selection: "#49483E".into(),
            black: "#272822".into(),
            red: "#F92672".into(),
            green: "#A6E22E".into(),
            yellow: "#F4BF75".into(),
            blue: "#66D9EF".into(),
            magenta: "#AE81FF".into(),
            cyan: "#A1EFE4".into(),
            white: "#F8F8F2".into(),
            bright_black: "#75715E".into(),
            bright_red: "#F92672".into(),
            bright_green: "#A6E22E".into(),
            bright_yellow: "#F4BF75".into(),
            bright_blue: "#66D9EF".into(),
            bright_magenta: "#AE81FF".into(),
            bright_cyan: "#A1EFE4".into(),
            bright_white: "#F9F8F5".into(),
        },
        TerminalTheme {
            name: "Dracula".into(),
            foreground: "#F8F8F2".into(),
            background: "#282A36".into(),
            cursor: "#F8F8F2".into(),
            cursor_accent: "#282A36".into(),
            selection: "#44475A".into(),
            black: "#21222C".into(),
            red: "#FF5555".into(),
            green: "#50FA7B".into(),
            yellow: "#F1FA8C".into(),
            blue: "#BD93F9".into(),
            magenta: "#FF79C6".into(),
            cyan: "#8BE9FD".into(),
            white: "#F8F8F2".into(),
            bright_black: "#6272A4".into(),
            bright_red: "#FF6E6E".into(),
            bright_green: "#69FF94".into(),
            bright_yellow: "#FFFFA5".into(),
            bright_blue: "#D6ACFF".into(),
            bright_magenta: "#FF92DF".into(),
            bright_cyan: "#A4FFFF".into(),
            bright_white: "#FFFFFF".into(),
        },
        TerminalTheme {
            name: "Nord".into(),
            foreground: "#D8DEE9".into(),
            background: "#2E3440".into(),
            cursor: "#D8DEE9".into(),
            cursor_accent: "#2E3440".into(),
            selection: "#434C5E".into(),
            black: "#3B4252".into(),
            red: "#BF616A".into(),
            green: "#A3BE8C".into(),
            yellow: "#EBCB8B".into(),
            blue: "#81A1C1".into(),
            magenta: "#B48EAD".into(),
            cyan: "#88C0D0".into(),
            white: "#E5E9F0".into(),
            bright_black: "#4C566A".into(),
            bright_red: "#BF616A".into(),
            bright_green: "#A3BE8C".into(),
            bright_yellow: "#EBCB8B".into(),
            bright_blue: "#81A1C1".into(),
            bright_magenta: "#B48EAD".into(),
            bright_cyan: "#8FBCBB".into(),
            bright_white: "#ECEFF4".into(),
        },
        TerminalTheme {
            name: "One Dark".into(),
            foreground: "#ABB2BF".into(),
            background: "#282C34".into(),
            cursor: "#528BFF".into(),
            cursor_accent: "#282C34".into(),
            selection: "#3E4451".into(),
            black: "#282C34".into(),
            red: "#E06C75".into(),
            green: "#98C379".into(),
            yellow: "#E5C07B".into(),
            blue: "#61AFEF".into(),
            magenta: "#C678DD".into(),
            cyan: "#56B6C2".into(),
            white: "#ABB2BF".into(),
            bright_black: "#5C6370".into(),
            bright_red: "#E06C75".into(),
            bright_green: "#98C379".into(),
            bright_yellow: "#E5C07B".into(),
            bright_blue: "#61AFEF".into(),
            bright_magenta: "#C678DD".into(),
            bright_cyan: "#56B6C2".into(),
            bright_white: "#FFFFFF".into(),
        },
        TerminalTheme {
            name: "Solarized Dark".into(),
            foreground: "#839496".into(),
            background: "#002B36".into(),
            cursor: "#839496".into(),
            cursor_accent: "#002B36".into(),
            selection: "#073642".into(),
            black: "#073642".into(),
            red: "#DC322F".into(),
            green: "#859900".into(),
            yellow: "#B58900".into(),
            blue: "#268BD2".into(),
            magenta: "#D33682".into(),
            cyan: "#2AA198".into(),
            white: "#EEE8D5".into(),
            bright_black: "#002B36".into(),
            bright_red: "#CB4B16".into(),
            bright_green: "#586E75".into(),
            bright_yellow: "#657B83".into(),
            bright_blue: "#839496".into(),
            bright_magenta: "#6C71C4".into(),
            bright_cyan: "#93A1A1".into(),
            bright_white: "#FDF6E3".into(),
        },
        TerminalTheme {
            name: "Gruvbox Dark".into(),
            foreground: "#EBDBB2".into(),
            background: "#282828".into(),
            cursor: "#EBDBB2".into(),
            cursor_accent: "#282828".into(),
            selection: "#504945".into(),
            black: "#282828".into(),
            red: "#CC241D".into(),
            green: "#98971A".into(),
            yellow: "#D79921".into(),
            blue: "#458588".into(),
            magenta: "#B16286".into(),
            cyan: "#689D6A".into(),
            white: "#A89984".into(),
            bright_black: "#928374".into(),
            bright_red: "#FB4934".into(),
            bright_green: "#B8BB26".into(),
            bright_yellow: "#FABD2F".into(),
            bright_blue: "#83A598".into(),
            bright_magenta: "#D3869B".into(),
            bright_cyan: "#8EC07C".into(),
            bright_white: "#EBDBB2".into(),
        },
    ]
}