//! 自定义主题模型（C1.5 / v1.51.7）
//!
//! 对应数据库表 `custom_themes`，替代 localStorage 持久化用户自定义主题。
//! 规范：`功能展望/体验深化/01_主题自定义系统_未来展望.md` §2.5

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 数据库 `custom_themes` 表对应结构体
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CustomTheme {
    /// 主键自增
    pub id: i64,
    /// 用户 ID（多用户隔离）
    pub user_id: i64,
    /// 主题名称（用户可读，唯一约束）
    pub name: String,
    /// 基础主题名（terminal/matrix/dracula 等 10 套预设之一）
    pub base_theme: String,
    /// JSON 字符串：{ "--theme-bg-primary": "#xxx", ... }
    pub variables: String,
    /// 创建时间（毫秒时间戳）
    pub created_at: i64,
    /// 更新时间（毫秒时间戳）
    pub updated_at: i64,
}

/// 创建/更新自定义主题的请求体
#[derive(Debug, Serialize, Deserialize)]
pub struct UpsertCustomThemeRequest {
    /// 主题名称
    pub name: String,
    /// 基础主题名（可选，默认 "terminal"）
    pub base_theme: Option<String>,
    /// 变量覆盖（JSON 字符串）
    pub variables: String,
}
