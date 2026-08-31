use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TerminalHistory {
    pub id: i64,
    pub user_id: i64,
    pub command: String,
    pub output: Option<String>,
    pub exit_code: i32,
    pub session_type: String,
    pub created_at: i64,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalHistoryInput {
    pub command: String,
    pub output: Option<String>,
    pub exit_code: i32,
    pub session_type: String,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TerminalTabLayout {
    pub id: i64,
    pub user_id: i64,
    pub tab_id: String,
    pub tab_type: String,
    pub title: String,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub pane_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalTabLayoutInput {
    pub tab_id: String,
    pub tab_type: String,
    pub title: String,
    pub sort_order: i32,
    pub is_active: bool,
    pub pane_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinCommandResult {
    pub output: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub current_dir: String,
    pub username: String,
    pub os: String,
    pub hostname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryListing {
    pub path: String,
    pub entries: Vec<DirectoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WslStatus {
    pub installed: bool,
    pub running: bool,
    pub default_distro: Option<String>,
    pub distributions: Vec<WslDistribution>,
    pub wsl_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WslDistribution {
    pub name: String,
    pub running: bool,
    pub version: u32,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TerminalSession {
    pub id: String,
    pub user_id: i64,
    pub session_type: String,
    pub tab_id: Option<String>,
    pub pane_id: Option<String>,
    pub cols: i64,
    pub rows: i64,
    pub status: String,
    pub created_at: i64,
    pub killed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSessionInput {
    pub id: String,
    pub session_type: String,
    pub tab_id: Option<String>,
    pub pane_id: Option<String>,
    pub cols: i64,
    pub rows: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SshProfile {
    pub id: String,
    pub user_id: i64,
    pub name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub private_key_path: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshProfileInput {
    pub name: String,
    pub host: String,
    pub port: Option<i64>,
    pub username: String,
    pub auth_type: String,
    pub private_key_path: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConnectionInfo {
    pub session_id: String,
    pub profile_name: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalTheme {
    pub name: String,
    pub foreground: String,
    pub background: String,
    pub cursor: String,
    pub cursor_accent: String,
    pub selection: String,
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    pub bright_black: String,
    pub bright_red: String,
    pub bright_green: String,
    pub bright_yellow: String,
    pub bright_blue: String,
    pub bright_magenta: String,
    pub bright_cyan: String,
    pub bright_white: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TerminalConfig {
    pub id: String,
    pub font_family: String,
    pub font_size: i64,
    pub line_height: f64,
    pub cursor_style: String,
    pub cursor_blink: bool,
    pub theme_name: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfigInput {
    pub font_family: Option<String>,
    pub font_size: Option<i64>,
    pub line_height: Option<f64>,
    pub cursor_style: Option<String>,
    pub cursor_blink: Option<bool>,
    pub theme_name: Option<String>,
}