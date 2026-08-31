//! T2.8 自动备份系统 - 数据模型
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.3
//!
//! 包含：
//! - `BackupType` 枚举：4 种备份类型（hourly/daily/pre_op/emergency）
//! - `BackupRecord`：备份元数据记录（用于 list_backups 返回）
//! - `BackupStats`：备份统计信息（用于 get_backup_stats 返回）
//! - `BackupMetadata`：内部校验元数据（不暴露给前端）

use serde::{Deserialize, Serialize};

/// 备份类型枚举
///
/// 设计依据：§2.3.1 备份策略表
/// - `Hourly`：每小时整点触发，保留 24 份
/// - `Daily`：每日 03:00 触发，保留 7 份
/// - `PreOp`：危险操作前触发，保留 10 份
/// - `Emergency`：异常检测触发，永久保留（max_retention = 0 表示无限）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupType {
    Hourly,
    Daily,
    PreOp,
    Emergency,
}

impl BackupType {
    /// 字符串标识（用于文件名前缀和数据库存储）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::PreOp => "pre_op",
            Self::Emergency => "emergency",
        }
    }

    /// 最大保留数量（0 表示无限保留）
    pub fn max_retention(&self) -> usize {
        match self {
            Self::Hourly => 24,
            Self::Daily => 7,
            Self::PreOp => 10,
            Self::Emergency => 0,
        }
    }

    /// 备份子目录名
    pub fn subdir(&self) -> &'static str {
        match self {
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::PreOp => "pre_op",
            Self::Emergency => "emergency",
        }
    }

    /// 从字符串解析（用于 Tauri 命令参数）
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "hourly" => Some(Self::Hourly),
            "daily" => Some(Self::Daily),
            "pre_op" | "preop" => Some(Self::PreOp),
            "emergency" => Some(Self::Emergency),
            _ => None,
        }
    }
}

/// 备份元数据记录（对应 backup_records.db 中的行）
///
/// 用于 `list_backups` 命令返回，包含完整备份信息
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BackupRecord {
    pub id: i64,
    pub backup_path: String,
    pub backup_size_bytes: i64,
    pub backup_type: String,
    pub label: Option<String>,
    pub schema_version: i64,
    pub integrity_check: String,
    pub checksum: String,
    pub table_counts_json: Option<String>,
    pub created_at: i64, // Unix timestamp（秒）
    pub verified: bool,
}

/// 备份统计信息（用于 `get_backup_stats` 命令返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStats {
    pub total_backups: usize,
    pub total_size_bytes: i64,
    pub by_type: BackupTypeStats,
    pub oldest_backup_at: Option<i64>,
    pub newest_backup_at: Option<i64>,
}

/// 按类型统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupTypeStats {
    pub hourly_count: usize,
    pub hourly_size_bytes: i64,
    pub daily_count: usize,
    pub daily_size_bytes: i64,
    pub pre_op_count: usize,
    pub pre_op_size_bytes: i64,
    pub emergency_count: usize,
    pub emergency_size_bytes: i64,
}

/// 内部校验元数据（不暴露给前端）
///
/// 由 `verify_backup()` 返回，包含校验过程的完整信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub integrity_check: String,  // "ok" 或错误描述
    pub schema_version: i64,
    pub checksum: String,         // SHA256 hex
    pub size_bytes: i64,
    pub table_counts: std::collections::HashMap<String, i64>,
}

/// 备份操作结果（用于 `create_backup_now` 命令返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    pub backup_path: String,
    pub backup_type: String,
    pub size_bytes: i64,
    pub schema_version: i64,
    pub checksum: String,
    pub created_at: i64,
}

impl From<(BackupMetadata, String, String, i64)> for BackupResult {
    fn from(
        (metadata, backup_path, backup_type, created_at): (BackupMetadata, String, String, i64),
    ) -> Self {
        Self {
            backup_path,
            backup_type,
            size_bytes: metadata.size_bytes,
            schema_version: metadata.schema_version,
            checksum: metadata.checksum,
            created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_type_as_str() {
        assert_eq!(BackupType::Hourly.as_str(), "hourly");
        assert_eq!(BackupType::Daily.as_str(), "daily");
        assert_eq!(BackupType::PreOp.as_str(), "pre_op");
        assert_eq!(BackupType::Emergency.as_str(), "emergency");
    }

    #[test]
    fn test_backup_type_max_retention() {
        assert_eq!(BackupType::Hourly.max_retention(), 24);
        assert_eq!(BackupType::Daily.max_retention(), 7);
        assert_eq!(BackupType::PreOp.max_retention(), 10);
        assert_eq!(BackupType::Emergency.max_retention(), 0);
    }

    #[test]
    fn test_backup_type_subdir() {
        assert_eq!(BackupType::Hourly.subdir(), "hourly");
        assert_eq!(BackupType::Daily.subdir(), "daily");
        assert_eq!(BackupType::PreOp.subdir(), "pre_op");
        assert_eq!(BackupType::Emergency.subdir(), "emergency");
    }

    #[test]
    fn test_backup_type_from_str() {
        assert_eq!(BackupType::from_str("hourly"), Some(BackupType::Hourly));
        assert_eq!(BackupType::from_str("daily"), Some(BackupType::Daily));
        assert_eq!(BackupType::from_str("pre_op"), Some(BackupType::PreOp));
        assert_eq!(BackupType::from_str("preop"), Some(BackupType::PreOp));
        assert_eq!(BackupType::from_str("emergency"), Some(BackupType::Emergency));
        assert_eq!(BackupType::from_str("invalid"), None);
    }

    #[test]
    fn test_backup_type_serde() {
        let json = serde_json::to_string(&BackupType::Hourly).unwrap();
        assert_eq!(json, "\"hourly\"");

        let bt: BackupType = serde_json::from_str("\"daily\"").unwrap();
        assert_eq!(bt, BackupType::Daily);
    }
}
