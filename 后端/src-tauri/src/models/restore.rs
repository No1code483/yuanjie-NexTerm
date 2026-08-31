//! T2.9 数据恢复与导出 - 数据模型
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.4 + §2.5

use serde::{Deserialize, Serialize};

/// 恢复操作结果
///
/// 由 `restore_from_backup` 返回，包含恢复状态和回滚备份路径
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    /// 是否恢复成功
    pub success: bool,
    /// 是否需要重启应用（恢复成功后必须重启）
    pub need_restart: bool,
    /// 回滚备份路径（恢复前的数据库备份，用于失败时回滚）
    pub rollback_backup_path: String,
    /// 恢复后的 schema_version
    pub schema_version: i64,
    /// integrity_check 结果（"ok" 或错误描述）
    pub integrity_check: String,
    /// foreign_key_check 发现的违规数量
    pub foreign_key_violations: i64,
}

/// 备份文件验证结果
///
/// 由 `verify_backup_for_restore` 和 `verify_restored_db` 返回
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreVerification {
    /// 完整性校验是否通过
    pub integrity_ok: bool,
    /// 完整性校验消息（"ok" 或错误描述）
    pub integrity_message: String,
    /// 备份文件的 schema_version
    pub schema_version: i64,
    /// foreign_key_check 发现的违规数量（仅 verify_restored_db 填充，verify_backup_for_restore 默认 0）
    #[serde(default)]
    pub foreign_key_violations: i64,
}

/// 导出操作结果
///
/// 由 `export_to_zip` 返回
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    /// 导出文件路径
    pub export_path: String,
    /// 导出文件大小（字节）
    pub size_bytes: i64,
    /// 导出时间（Unix timestamp，秒）
    pub exported_at: i64,
    /// schema_version
    pub schema_version: i64,
    /// 校验和（SHA256 hex）
    pub checksum: String,
    /// 导出的表行数统计
    pub table_counts: std::collections::HashMap<String, i64>,
}

/// 导出清单（manifest.json）
///
/// 包含在导出 zip 文件中，记录导出元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportManifest {
    /// 应用版本
    pub version: String,
    /// schema_version
    pub schema_version: i64,
    /// 导出时间（ISO 8601 格式）
    pub exported_at: String,
    /// 导出者用户名（如有）
    pub exported_by: Option<String>,
    /// 表行数统计
    pub table_counts: std::collections::HashMap<String, i64>,
    /// 生成导出的应用名称
    pub app_name: String,
}

/// 导入操作结果
///
/// 由 `import_from_zip` 返回
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    /// 是否导入成功
    pub success: bool,
    /// 是否需要重启应用
    pub need_restart: bool,
    /// 导入后的 schema_version
    pub schema_version: i64,
    /// 导入的表行数统计
    pub table_counts: std::collections::HashMap<String, i64>,
    /// 导入时间（Unix timestamp，秒）
    pub imported_at: i64,
    /// 是否执行了迁移（如果导入文件版本低于当前）
    pub migrations_applied: bool,
}

/// 导入兼容性检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportCompatibility {
    /// 导入文件的 schema_version
    pub source_schema_version: i64,
    /// 当前应用的 schema_version
    pub target_schema_version: i64,
    /// 是否兼容（source <= target）
    pub compatible: bool,
    /// 不兼容原因（如不兼容）
    pub reason: Option<String>,
    /// 是否需要执行迁移
    pub needs_migration: bool,
}
