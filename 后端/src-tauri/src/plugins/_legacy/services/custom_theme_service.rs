//! 自定义主题服务层（C1.5 / v1.51.7）
//!
//! 业务封装层，连接 repositories/custom_theme_repo 和 commands/custom_theme_commands。
//! 规范：`功能展望/体验深化/01_主题自定义系统_未来展望.md` §2.5
//!
//! 当前为薄封装（仅透传 + 时间戳生成），未来可在此扩展：
//! - 变量 JSON 合法性校验
//! - 主题名称规范化（trim + 长度限制）
//! - 删除前依赖检查（moduleThemes 是否引用）

use sqlx::SqlitePool;

use crate::db::repositories::custom_theme_repo;
use crate::error::app_error::AppError;
use crate::models::custom_theme::CustomTheme;

/// 合法的基础主题名白名单（必须与前端 PRESET_THEMES 一致）
const VALID_BASE_THEMES: &[&str] = &[
    "terminal",
    "matrix",
    "dracula",
    "monokai",
    "one-dark",
    "nord",
    "solarized-dark",
    "solarized-light",
    "github-light",
    "github-dark",
];

/// 主题名称最大长度（防止滥用）
const MAX_NAME_LEN: usize = 32;

/// 校验基础主题名是否合法
fn is_valid_base_theme(base_theme: &str) -> bool {
    VALID_BASE_THEMES.contains(&base_theme)
}

/// 校验主题名称合法性（非空 + 长度限制）
fn is_valid_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty() && trimmed.len() <= MAX_NAME_LEN
}

/// 校验 variables 是否为合法 JSON 对象（{ "key": "value", ... }）
fn is_valid_variables(variables: &str) -> bool {
    // 尝试解析为 JSON 对象；空字符串视为 {} 兜底
    let trimmed = variables.trim();
    if trimmed.is_empty() {
        return true; // service 层会在写入前补充 "{}"
    }
    serde_json::from_str::<serde_json::Value>(trimmed)
        .map(|v| v.is_object())
        .unwrap_or(false)
}

pub async fn list(pool: &SqlitePool, user_id: i64) -> Result<Vec<CustomTheme>, AppError> {
    custom_theme_repo::list(pool, user_id).await
}

pub async fn get_by_id(pool: &SqlitePool, user_id: i64, id: i64) -> Result<Option<CustomTheme>, AppError> {
    custom_theme_repo::get_by_id(pool, user_id, id).await
}

pub async fn get_by_name(pool: &SqlitePool, user_id: i64, name: &str) -> Result<Option<CustomTheme>, AppError> {
    custom_theme_repo::get_by_name(pool, user_id, name).await
}

/// 新增或更新自定义主题（UPSERT）
///
/// 错误：
/// - AppError::Validation：名称/基础主题/变量校验失败
pub async fn upsert(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    base_theme: Option<&str>,
    variables: &str,
) -> Result<CustomTheme, AppError> {
    let trimmed_name = name.trim();
    if !is_valid_name(trimmed_name) {
        return Err(AppError::Validation(format!(
            "主题名称不合法（需 1-{} 字符）",
            MAX_NAME_LEN
        )));
    }

    let base = base_theme.unwrap_or("terminal");
    if !is_valid_base_theme(base) {
        return Err(AppError::Validation(format!(
            "基础主题名不合法：{}（必须是预设主题之一）",
            base
        )));
    }

    let variables_str = if variables.trim().is_empty() {
        "{}".to_string()
    } else if !is_valid_variables(variables) {
        return Err(AppError::Validation(
            "variables 必须是合法的 JSON 对象字符串".to_string(),
        ));
    } else {
        variables.to_string()
    };

    let now = chrono::Utc::now().timestamp_millis();
    custom_theme_repo::upsert(pool, user_id, trimmed_name, base, &variables_str, now).await
}

pub async fn delete_by_id(pool: &SqlitePool, user_id: i64, id: i64) -> Result<u64, AppError> {
    custom_theme_repo::delete_by_id(pool, user_id, id).await
}
