//! D1.6 MCP 服务器配置持久化仓储层
//!
//! 设计依据：
//!   - 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md（v3.1 MCP 生态）
//!   - 已有功能/06_Yuan_Code/MCP协议.md §八.1（无持久化 → 需实现）
//!
//! 提供 MCP 服务器配置的 CRUD + 预置模板加载，应用重启后可自动恢复。
//!
//! 多用户数据隔离（批次 6，migration 0121）：
//!   - mcp_servers 表已添加 user_id 字段（DEFAULT 1）
//!   - 所有查询/修改/删除操作均按 user_id 过滤，防止跨用户读取/覆盖/删除
//!   - upsert_server 采用 UPDATE-then-INSERT 模式，避免 ON CONFLICT 跨用户覆盖

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::mcp::McpServerRegistration;

/// 持久化的 MCP 服务器记录（对应 mcp_servers 表）
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct McpServerRecord {
    pub id: String,
    pub user_id: i64,
    pub name: String,
    pub command: String,
    pub args: String,        // JSON 数组
    pub env: String,          // JSON 对象
    pub working_dir: Option<String>,
    pub auto_connect: bool,
    pub lifecycle_config: String,  // JSON
    pub enabled: bool,
    pub is_builtin: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl McpServerRecord {
    /// 转换为 McpServerRegistration（运行时模型）
    pub fn to_registration(&self) -> Result<McpServerRegistration, AppError> {
        let args: Vec<String> = if self.args.is_empty() || self.args == "[]" {
            Vec::new()
        } else {
            serde_json::from_str(&self.args)
                .map_err(|e| AppError::Internal(format!("解析 MCP args 失败: {}", e)))?
        };
        let env: Option<std::collections::HashMap<String, String>> =
            if self.env.is_empty() || self.env == "{}" {
                None
            } else {
                serde_json::from_str(&self.env).ok()
            };
        Ok(McpServerRegistration {
            id: self.id.clone(),
            name: self.name.clone(),
            command: self.command.clone(),
            args,
            env,
            enabled: self.enabled,
        })
    }
}

/// 插入或更新 MCP 服务器配置
///
/// 多用户隔离（批次 6）：
///   - UPDATE 阶段仅匹配 `id = ? AND user_id = ?`，不会覆盖其他用户的同 id 服务器
///   - 若 UPDATE 命中 0 行（id 不存在或不属于当前用户），再执行 INSERT
///   - INSERT 显式写入 user_id，确保新配置归属当前用户
pub async fn upsert_server(
    pool: &SqlitePool,
    user_id: i64,
    reg: &McpServerRegistration,
    auto_connect: bool,
) -> Result<(), AppError> {
    let args_json = serde_json::to_string(&reg.args)
        .map_err(|e| AppError::Internal(format!("序列化 args 失败: {}", e)))?;
    let env_json = match &reg.env {
        Some(e) => serde_json::to_string(e).unwrap_or_else(|_| "{}".into()),
        None => "{}".into(),
    };

    // 1. 尝试 UPDATE（仅当 id 存在且属于当前用户时命中）
    let updated = sqlx::query(
        r#"UPDATE mcp_servers
           SET name = ?, command = ?, args = ?, env = ?,
               auto_connect = ?, enabled = ?, updated_at = datetime('now')
           WHERE id = ? AND user_id = ?"#,
    )
    .bind(&reg.name)
    .bind(&reg.command)
    .bind(&args_json)
    .bind(&env_json)
    .bind(auto_connect)
    .bind(reg.enabled)
    .bind(&reg.id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?
    .rows_affected();

    // 2. UPDATE 未命中 → 执行 INSERT（新配置，归属当前用户）
    if updated == 0 {
        sqlx::query(
            r#"INSERT INTO mcp_servers
               (id, user_id, name, command, args, env, auto_connect, enabled, is_builtin,
                lifecycle_config, working_dir, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, '{}', NULL, datetime('now'), datetime('now'))"#,
        )
        .bind(&reg.id)
        .bind(user_id)
        .bind(&reg.name)
        .bind(&reg.command)
        .bind(&args_json)
        .bind(&env_json)
        .bind(auto_connect)
        .bind(reg.enabled)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    }

    tracing::info!(
        "🔌 [D1.6] MCP 服务器配置已持久化: {} ({}) user_id={}",
        reg.name, reg.id, user_id
    );
    Ok(())
}

/// 加载指定用户已启用的 MCP 服务器配置
///
/// 多用户隔离（批次 6）：按 user_id 过滤，用户只能加载自己的配置。
pub async fn load_enabled_servers(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<McpServerRegistration>, AppError> {
    let records: Vec<McpServerRecord> = sqlx::query_as(
        r#"SELECT id, user_id, name, command, args, env, working_dir, auto_connect,
                  lifecycle_config, enabled, is_builtin, created_at, updated_at
           FROM mcp_servers
           WHERE enabled = 1 AND user_id = ?
           ORDER BY is_builtin DESC, name ASC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    records
        .iter()
        .map(|r| r.to_registration())
        .collect()
}

/// 加载指定用户所有 MCP 服务器配置（含禁用的）
///
/// 多用户隔离（批次 6）：按 user_id 过滤，用户只能看到自己的配置。
pub async fn load_all_servers(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Vec<McpServerRecord>, AppError> {
    sqlx::query_as(
        r#"SELECT id, user_id, name, command, args, env, working_dir, auto_connect,
                  lifecycle_config, enabled, is_builtin, created_at, updated_at
           FROM mcp_servers
           WHERE user_id = ?
           ORDER BY is_builtin DESC, name ASC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

/// 删除 MCP 服务器配置
///
/// 多用户隔离（批次 6）：按 user_id 过滤，用户只能删除自己的配置（预置模板不可删）。
pub async fn delete_server(
    pool: &SqlitePool,
    user_id: i64,
    server_id: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM mcp_servers WHERE id = ? AND user_id = ? AND is_builtin = 0")
        .bind(server_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 更新启用状态
///
/// 多用户隔离（批次 6）：按 user_id 过滤，用户只能修改自己的配置。
pub async fn set_enabled(
    pool: &SqlitePool,
    user_id: i64,
    server_id: &str,
    enabled: bool,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE mcp_servers SET enabled = ?, updated_at = datetime('now') WHERE id = ? AND user_id = ?",
    )
    .bind(enabled)
    .bind(server_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}
