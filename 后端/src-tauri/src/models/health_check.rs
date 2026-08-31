//! T2.10 异常检测与自愈 - 数据模型
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.6
//!
//! 检查维度：
//! 1. PRAGMA integrity_check（数据库文件物理完整性）
//! 2. PRAGMA foreign_key_check（外键约束违规）
//! 3. 关键表行数对比（与最近一次备份对比，检测异常数据丢失）
//! 4. 索引覆盖率（关键索引是否存在且可用）

use serde::{Deserialize, Serialize};

/// 健康状态等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// 健康：所有检查通过
    Healthy,
    /// 警告：存在非致命问题（如索引缺失、表行数偏差）
    Warning,
    /// 严重：存在致命问题（integrity 失败、FK 违规）
    Critical,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Warning => "warning",
            HealthStatus::Critical => "critical",
        }
    }

    /// 是否需要触发紧急备份
    pub fn should_trigger_emergency_backup(&self) -> bool {
        matches!(self, HealthStatus::Critical)
    }

    /// 是否需要自动修复
    pub fn needs_repair(&self) -> bool {
        matches!(self, HealthStatus::Warning | HealthStatus::Critical)
    }
}

/// 健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// 整体状态（取所有检查项中最严重的）
    pub status: HealthStatus,
    /// 检查时间（Unix timestamp，秒）
    pub checked_at: i64,
    /// 数据库文件大小（字节）
    pub db_size_bytes: i64,
    /// schema_version
    pub schema_version: i64,
    /// 1. 完整性检查结果
    pub integrity: IntegrityCheckResult,
    /// 2. 外键检查结果
    pub foreign_keys: ForeignKeyCheckResult,
    /// 3. 表行数对比结果
    pub table_counts: TableCountCheckResult,
    /// 4. 索引覆盖率结果
    pub indexes: IndexCheckResult,
    /// 修复建议（如有）
    pub repair_suggestion: Option<String>,
}

/// 1. 完整性检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheckResult {
    /// 是否通过（"ok" 表示通过）
    pub ok: bool,
    /// PRAGMA integrity_check 的原始返回值
    pub message: String,
}

/// 2. 外键检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyCheckResult {
    /// 是否通过（违规数 = 0）
    pub ok: bool,
    /// 违规数量
    pub violation_count: i64,
    /// 违规详情（前 10 条，每条格式: "table row: parent_table"）
    pub violations: Vec<String>,
}

/// 3. 表行数对比结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCountCheckResult {
    /// 是否通过（所有表行数与基线偏差 <= 阈值）
    pub ok: bool,
    /// 偏差较大的表（表名 → 偏差百分比）
    pub deviations: Vec<TableCountDeviation>,
    /// 检查的表总数
    pub tables_checked: usize,
}

/// 单张表的行数偏差
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCountDeviation {
    pub table_name: String,
    /// 基线行数（来自最近一次备份）
    pub baseline_count: i64,
    /// 当前行数
    pub current_count: i64,
    /// 偏差百分比（绝对值，0.0-100.0+）
    pub deviation_percent: f64,
    /// 偏差方向（"increase" | "decrease"）
    pub direction: String,
}

/// 4. 索引覆盖率结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexCheckResult {
    /// 是否通过（所有关键索引存在且可用）
    pub ok: bool,
    /// 缺失的索引列表
    pub missing_indexes: Vec<String>,
    /// 检查的索引总数
    pub indexes_checked: usize,
}

/// 自动修复结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRepairResult {
    /// 修复是否成功
    pub success: bool,
    /// 修复的项数
    pub repaired_items: usize,
    /// 移到 _orphaned 表的记录数
    pub orphaned_records_moved: i64,
    /// 重建的索引数
    pub indexes_rebuilt: usize,
    /// 修复详情
    pub details: Vec<String>,
    /// 修复后的状态（重新检查）
    pub post_repair_status: Option<HealthStatus>,
}

/// 健康检查历史记录（存储在 backup_records.db 或独立 db）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckRecord {
    pub id: i64,
    pub checked_at: i64,
    pub status: String,
    pub db_size_bytes: i64,
    pub schema_version: i64,
    pub integrity_ok: bool,
    pub fk_violation_count: i64,
    pub table_count_deviations: i64,
    pub missing_indexes_count: i64,
    pub repaired: bool,
    pub repair_success: Option<bool>,
}
