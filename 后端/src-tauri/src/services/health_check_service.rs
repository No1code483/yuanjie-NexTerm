//! T2.10 异常检测与自愈 - 健康检查服务
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.6
//!
//! 检查维度（4 项）：
//! 1. **PRAGMA integrity_check** — 数据库文件物理完整性
//! 2. **PRAGMA foreign_key_check** — 外键约束违规
//! 3. **关键表行数对比** — 与最近一次备份的元数据对比，检测异常数据丢失/增加
//! 4. **索引覆盖率** — 关键索引是否存在且可用
//!
//! 状态等级：
//! - `Healthy`：所有检查通过
//! - `Warning`：表行数偏差 > 50% 或索引缺失（非致命）
//! - `Critical`：integrity 失败 或 FK 违规 > 0（致命，需立即修复）
//!
//! 触发紧急备份：
//! - `Critical` 状态下，由调度器调用 `backup_scheduler::trigger_emergency_backup`

use std::collections::HashMap;
use std::path::Path;

use sqlx::SqlitePool;

use crate::error::app_error::AppError;
use crate::models::health_check::{
    AutoRepairResult, ForeignKeyCheckResult, HealthCheckResult, HealthStatus, IndexCheckResult,
    IntegrityCheckResult, TableCountCheckResult, TableCountDeviation,
};
use crate::services::backup_service;

/// 表行数偏差阈值（百分比）
///
/// - 偏差 <= WARN_THRESHOLD：Healthy
/// - WARN_THRESHOLD < 偏差 <= CRITICAL_THRESHOLD：Warning
/// - 偏差 > CRITICAL_THRESHOLD：Critical（疑似数据丢失）
const WARN_THRESHOLD: f64 = 50.0;
const CRITICAL_THRESHOLD: f64 = 90.0;

/// 关键索引清单（必须存在的索引）
///
/// 选取数据查询性能敏感的核心索引。如果缺失，会导致查询变慢，但不会导致数据丢失。
/// 注意：索引名必须与迁移文件中定义的 `CREATE INDEX` 名称一致
///
/// Phase 3 §2.2.2 调整（2026-07-24）：
/// - 移除 `idx_users_email`：users 表无 email 列（username 是唯一标识，UNIQUE 约束自动生成索引）
/// - 移除 `idx_conversations_user_id`：conversations 表无 user_id 列（多 AI 群聊设计，
///   参见项目核心设计意图 §六）
/// - 上述 2 项期望索引无法在主库创建（列不存在），保留在清单中会让 check_indexes
///   永远报告 Warning。已在 v108 迁移注释中说明。
/// - v108 补建了 idx_audit_log_table_row / idx_audit_log_created_at（与 v82 的
///   idx_audit_log_table_record / idx_audit_log_changed_at 同列不同名，按本清单期望名补建）
const CRITICAL_INDEXES: &[&str] = &[
    // 会话与消息
    "idx_messages_conversation_id",
    // 知识库
    "idx_kb_entries_category_id",
    "idx_kb_categories_parent_id",
    // 审计日志（T2.7.1）
    "idx_audit_log_table_row",
    "idx_audit_log_created_at",
    // 备份记录（注：表位于独立 backup_records.db，主库由 v108 防御性创建）
    "idx_backup_records_type_created",
    // schema_migrations 主键（隐式）
    // 注：sqlite_master 表的查询会自动使用主键索引
];

// ============================================================================
// 公共 API
// ============================================================================

/// 执行完整健康检查
///
/// 流程：
/// 1. 获取数据库文件大小 + schema_version
/// 2. 执行 4 项检查（integrity / FK / table_counts / indexes）
/// 3. 综合评估状态（取最严重等级）
/// 4. 生成修复建议
///
/// 参数：
/// - `pool`：数据库连接池
/// - `data_dir`：应用数据目录（用于读取基线元数据）
pub async fn run_health_check(
    pool: &SqlitePool,
    data_dir: &Path,
) -> Result<HealthCheckResult, AppError> {
    let checked_at = chrono::Local::now().timestamp();

    // 获取数据库文件大小
    let db_path = data_dir.join("nexterm.db");
    let db_size_bytes = if db_path.exists() {
        tokio::fs::metadata(&db_path).await?.len() as i64
    } else {
        0
    };

    // 获取 schema_version
    let schema_version: i64 = sqlx::query_scalar(
        "SELECT MAX(version) FROM schema_migrations WHERE version IS NOT NULL;",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 1. 完整性检查
    let integrity = check_integrity(pool).await?;

    // 2. 外键检查
    let foreign_keys = check_foreign_keys(pool).await?;

    // 3. 表行数对比（与最近一次备份的元数据对比）
    let baseline = get_baseline_counts(data_dir).await?;
    let table_counts = check_table_counts(pool, &baseline).await?;

    // 4. 索引覆盖率
    let indexes = check_indexes(pool).await?;

    // 综合评估状态
    let status = evaluate_status(&integrity, &foreign_keys, &table_counts, &indexes);

    // 生成修复建议
    let repair_suggestion = generate_repair_suggestion(&status, &foreign_keys, &table_counts, &indexes);

    Ok(HealthCheckResult {
        status,
        checked_at,
        db_size_bytes,
        schema_version,
        integrity,
        foreign_keys,
        table_counts,
        indexes,
        repair_suggestion,
    })
}

/// 获取最近一次备份的表行数基线
///
/// 从 backup_records.db 中读取最近一次 backup 的 table_counts_json
async fn get_baseline_counts(data_dir: &Path) -> Result<HashMap<String, i64>, AppError> {
    let backups_dir = data_dir.join("backups");
    let records_db_path = backups_dir.join("backup_records.db");

    if !records_db_path.exists() {
        // 没有备份记录，返回空 HashMap（表行数对比将跳过）
        return Ok(HashMap::new());
    }

    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&records_db_path)
        .read_only(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| AppError::Backup(format!("无法打开 backup_records.db: {}", e)))?;

    // 读取最近一次 backup 的 table_counts_json（按 created_at DESC 排序）
    let json_str: Option<String> = sqlx::query_scalar(
        r#"SELECT table_counts_json FROM backup_records
           WHERE table_counts_json IS NOT NULL
           ORDER BY created_at DESC LIMIT 1;"#,
    )
    .fetch_one(&pool)
    .await
    .ok()
    .flatten();

    pool.close().await;

    if let Some(json) = json_str {
        let counts: HashMap<String, i64> = serde_json::from_str(&json).unwrap_or_default();
        Ok(counts)
    } else {
        Ok(HashMap::new())
    }
}

// ============================================================================
// 内部检查函数
// ============================================================================

/// 1. 完整性检查（PRAGMA integrity_check）
async fn check_integrity(pool: &SqlitePool) -> Result<IntegrityCheckResult, AppError> {
    let message: String = sqlx::query_scalar("PRAGMA integrity_check;")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Backup(format!("integrity_check 执行失败: {}", e)))?;

    let ok = message == "ok";
    Ok(IntegrityCheckResult { ok, message })
}

/// 2. 外键检查（PRAGMA foreign_key_check）
///
/// 返回前 10 条违规详情。违规数 > 0 视为 Critical。
async fn check_foreign_keys(pool: &SqlitePool) -> Result<ForeignKeyCheckResult, AppError> {
    // 启用外键检查（即使默认关闭，PRAGMA foreign_key_check 仍可独立工作）
    let _ = sqlx::query("PRAGMA foreign_keys=ON;").execute(pool).await;

    // pragma_foreign_key_check 返回 4 列：table, rowid, parent, fkid
    let rows: Vec<(String, i64, Option<String>, i64)> = sqlx::query_as(
        "SELECT \"table\", rowid, \"parent\", fkid FROM pragma_foreign_key_check;",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Backup(format!("foreign_key_check 执行失败: {}", e)))?;

    let violation_count = rows.len() as i64;

    // 格式化前 10 条违规
    let violations: Vec<String> = rows
        .iter()
        .take(10)
        .map(|(table, rowid, parent, fkid)| {
            format!(
                "{} row={} parent={} fkid={}",
                table,
                rowid,
                parent.as_deref().unwrap_or("(none)"),
                fkid
            )
        })
        .collect();

    let ok = violation_count == 0;
    Ok(ForeignKeyCheckResult {
        ok,
        violation_count,
        violations,
    })
}

/// 3. 表行数对比（与基线对比）
///
/// 偏差评估：
/// - 偏差 <= 50%：通过
/// - 偏差 > 50%：Warning
/// - 偏差 > 90%：Critical
///
/// 若无基线（首次运行或无备份），跳过此检查并返回 ok=true
async fn check_table_counts(
    pool: &SqlitePool,
    baseline: &HashMap<String, i64>,
) -> Result<TableCountCheckResult, AppError> {
    if baseline.is_empty() {
        // 无基线，跳过
        return Ok(TableCountCheckResult {
            ok: true,
            deviations: vec![],
            tables_checked: 0,
        });
    }

    let mut deviations = vec![];
    let mut all_ok = true;

    for table_name in backup_service::CRITICAL_TABLES {
        let baseline_count = match baseline.get(*table_name) {
            Some(c) => *c,
            None => continue,
        };

        // T2.11 SQL 注入防护：表名来自代码内部常量（CRITICAL_TABLES），
        // 理论安全。但为统一规范，仍调用白名单校验。
        // 校验失败（理论上不会发生）将导致该表跳过，避免潜在风险。
        if crate::db::sql_safety::validate_identifier(table_name).is_err() {
            continue;
        }

        let current_count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {};", table_name))
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        if baseline_count == 0 {
            // 基线为 0，若当前也为 0 则无偏差，否则视为新增数据（不算异常）
            continue;
        }

        let diff = (current_count - baseline_count).abs() as f64;
        let deviation_percent = (diff / baseline_count as f64) * 100.0;

        if deviation_percent > WARN_THRESHOLD {
            let direction = if current_count > baseline_count {
                "increase"
            } else {
                "decrease"
            };
            deviations.push(TableCountDeviation {
                table_name: table_name.to_string(),
                baseline_count,
                current_count,
                deviation_percent,
                direction: direction.to_string(),
            });

            if deviation_percent > CRITICAL_THRESHOLD {
                all_ok = false; // Critical 偏差视为不通过
            }
        }
    }

    Ok(TableCountCheckResult {
        ok: all_ok,
        deviations,
        tables_checked: backup_service::CRITICAL_TABLES.len(),
    })
}

/// 4. 索引覆盖率检查
///
/// 通过查询 sqlite_master 验证关键索引是否存在
async fn check_indexes(pool: &SqlitePool) -> Result<IndexCheckResult, AppError> {
    let existing_indexes: Vec<String> = sqlx::query_scalar(
        r#"SELECT name FROM sqlite_master
           WHERE type='index' AND name NOT LIKE 'sqlite_%';"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Backup(format!("查询索引列表失败: {}", e)))?;

    let existing_set: std::collections::HashSet<&str> =
        existing_indexes.iter().map(|s| s.as_str()).collect();

    let missing_indexes: Vec<String> = CRITICAL_INDEXES
        .iter()
        .filter(|idx| !existing_set.contains(*idx))
        .map(|s| s.to_string())
        .collect();

    let ok = missing_indexes.is_empty();
    Ok(IndexCheckResult {
        ok,
        missing_indexes,
        indexes_checked: CRITICAL_INDEXES.len(),
    })
}

/// 综合评估状态（取所有检查中最严重的等级）
fn evaluate_status(
    integrity: &IntegrityCheckResult,
    foreign_keys: &ForeignKeyCheckResult,
    table_counts: &TableCountCheckResult,
    indexes: &IndexCheckResult,
) -> HealthStatus {
    // integrity 失败 → Critical
    if !integrity.ok {
        return HealthStatus::Critical;
    }

    // FK 违规 > 0 → Critical
    if !foreign_keys.ok {
        return HealthStatus::Critical;
    }

    // 表行数偏差 > 90% → Critical
    let has_critical_deviation = table_counts
        .deviations
        .iter()
        .any(|d| d.deviation_percent > CRITICAL_THRESHOLD);
    if has_critical_deviation {
        return HealthStatus::Critical;
    }

    // 表行数偏差 > 50% 或索引缺失 → Warning
    let has_warning_deviation = !table_counts.deviations.is_empty();
    if has_warning_deviation || !indexes.ok {
        return HealthStatus::Warning;
    }

    HealthStatus::Healthy
}

/// 生成修复建议
fn generate_repair_suggestion(
    status: &HealthStatus,
    foreign_keys: &ForeignKeyCheckResult,
    table_counts: &TableCountCheckResult,
    indexes: &IndexCheckResult,
) -> Option<String> {
    match status {
        HealthStatus::Healthy => None,
        HealthStatus::Warning => {
            let mut suggestions = vec![];
            if !indexes.ok {
                suggestions.push(format!(
                    "缺失 {} 个关键索引：{}（建议重启应用或手动 REINDEX）",
                    indexes.missing_indexes.len(),
                    indexes.missing_indexes.join(", ")
                ));
            }
            if !table_counts.deviations.is_empty() {
                suggestions.push(format!(
                    "检测到 {} 张表行数偏差较大（建议检查业务逻辑或恢复备份）",
                    table_counts.deviations.len()
                ));
            }
            if suggestions.is_empty() {
                None
            } else {
                Some(suggestions.join("；"))
            }
        }
        HealthStatus::Critical => {
            let mut suggestions = vec![];
            if !foreign_keys.ok {
                suggestions.push(format!(
                    "外键违规 {} 条（建议运行 auto_repair 将孤儿记录移到 _orphaned 表，或恢复备份）",
                    foreign_keys.violation_count
                ));
            }
            if !table_counts.deviations.is_empty() {
                let critical_count = table_counts
                    .deviations
                    .iter()
                    .filter(|d| d.deviation_percent > CRITICAL_THRESHOLD)
                    .count();
                suggestions.push(format!(
                    "检测到 {} 张表行数严重偏差（疑似数据丢失，建议立即恢复备份）",
                    critical_count
                ));
            }
            if suggestions.is_empty() {
                Some("数据库存在严重问题，建议立即恢复备份".to_string())
            } else {
                Some(suggestions.join("；"))
            }
        }
    }
}

// ============================================================================
// 自动修复 API（T2.10.2）
// ============================================================================

/// 执行自动修复
///
/// 修复策略：
/// 1. **FK 违规** → 将孤儿记录从原表移动到 `<table>_orphaned` 表（保留数据，不删除）
/// 2. **索引缺失** → 仅记录警告，不自动创建（需要业务层提供索引定义）
/// 3. **索引损坏** → 执行 `REINDEX <name>` 重建已存在的索引
/// 4. **表行数偏差** → 不修复（业务数据问题，需用户介入）
/// 5. **integrity 失败** → 不修复（数据库物理损坏，需恢复备份）
///
/// 修复后重新执行健康检查，返回修复后的状态
///
/// 参数：
/// - `pool`：数据库连接池
/// - `result`：原始健康检查结果
/// - `data_dir`：应用数据目录（用于修复后重新检查）
pub async fn auto_repair(
    pool: &SqlitePool,
    result: &HealthCheckResult,
    data_dir: &Path,
) -> Result<AutoRepairResult, AppError> {
    let mut details = vec![];
    let mut repaired_items = 0usize;
    let mut orphaned_records_moved = 0i64;
    let mut indexes_rebuilt = 0usize;

    // 1. 修复 FK 违规（移动孤儿记录到 _orphaned 表）
    if !result.foreign_keys.ok {
        match repair_orphaned_records(pool, &result.foreign_keys).await {
            Ok(moved) => {
                orphaned_records_moved = moved;
                if moved > 0 {
                    details.push(format!(
                        "已将 {} 条孤儿记录移动到 _orphaned 表",
                        moved
                    ));
                    repaired_items += 1;
                }
            }
            Err(e) => {
                details.push(format!("修复孤儿记录失败: {}", e));
            }
        }
    }

    // 2. 修复索引（重建已存在但可能损坏的索引）
    if !result.indexes.ok {
        match rebuild_indexes(pool).await {
            Ok(count) => {
                indexes_rebuilt = count;
                if count > 0 {
                    details.push(format!("已重建 {} 个索引", count));
                    repaired_items += 1;
                }
            }
            Err(e) => {
                details.push(format!("重建索引失败: {}", e));
            }
        }
    }

    // 3. 修复后重新检查状态
    let post_repair_status = if repaired_items > 0 {
        match run_health_check(pool, data_dir).await {
            Ok(new_result) => Some(new_result.status),
            Err(_) => None,
        }
    } else {
        None
    };

    let success = repaired_items > 0;
    Ok(AutoRepairResult {
        success,
        repaired_items,
        orphaned_records_moved,
        indexes_rebuilt,
        details,
        post_repair_status,
    })
}

/// 修复 FK 违规：将孤儿记录从原表移动到 `<table>_orphaned` 表
///
/// 流程：
/// 1. 对每个违规的 (table, rowid)，创建 `<table>_orphaned` 表（如不存在）
/// 2. 将违规记录从原表复制到 _orphaned 表
/// 3. 从原表删除违规记录
///
/// 注意：_orphaned 表的 schema 与原表相同（通过 `CREATE TABLE _orphaned AS SELECT * FROM original WHERE 1=0` 创建）
///
/// 参数 `fk_result` 未使用：函数内重新查询 pragma_foreign_key_check 获取最新违规列表，
/// 避免使用过期的检查结果（修复过程中可能产生新违规）
#[allow(unused_variables)]
async fn repair_orphaned_records(
    pool: &SqlitePool,
    fk_result: &ForeignKeyCheckResult,
) -> Result<i64, AppError> {
    // 重新查询所有违规（不限于前 10 条）
    let rows: Vec<(String, i64, Option<String>, i64)> = sqlx::query_as(
        r#"SELECT "table", rowid, "parent", fkid FROM pragma_foreign_key_check;"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Backup(format!("查询 FK 违规失败: {}", e)))?;

    if rows.is_empty() {
        return Ok(0);
    }

    let mut moved_count = 0i64;

    // 按表分组处理（同一张表的违规一起处理）
    let mut by_table: HashMap<String, Vec<i64>> = HashMap::new();
    for (table, rowid, _parent, _fkid) in &rows {
        by_table.entry(table.clone()).or_default().push(*rowid);
    }

    for (table, rowids) in by_table {
        // T2.11 SQL 注入防护：表名来自 pragma_foreign_key_check 的返回值，
        // 理论上是数据库内部表名。但为防御二阶注入，仍校验。
        // 校验失败的表将跳过修复，避免执行危险 SQL。
        if crate::db::sql_safety::validate_identifier(&table).is_err() {
            tracing::warn!("跳过孤儿记录修复：表名 '{}' 包含非法字符", table);
            continue;
        }
        let orphaned_table = format!("{}_orphaned", table);
        // 校验生成的 _orphaned 表名也合法（理论上一定合法，但防御性编程）
        if crate::db::sql_safety::validate_identifier(&orphaned_table).is_err() {
            tracing::warn!("跳过孤儿记录修复：生成的表名 '{}' 非法", orphaned_table);
            continue;
        }

        // 1. 创建 _orphaned 表（schema 与原表相同，但无 FK / 索引）
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS {} AS SELECT * FROM {} WHERE 1=0;",
            orphaned_table, table
        );
        sqlx::query(&create_sql)
            .execute(pool)
            .await
            .map_err(|e| AppError::Backup(format!("创建 {} 失败: {}", orphaned_table, e)))?;

        // 2. 复制违规记录到 _orphaned 表
        let rowid_list = rowids
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let insert_sql = format!(
            "INSERT INTO {} SELECT * FROM {} WHERE rowid IN ({});",
            orphaned_table, table, rowid_list
        );
        let insert_result = sqlx::query(&insert_sql).execute(pool).await;
        if let Err(e) = insert_result {
            tracing::warn!("复制孤儿记录到 {} 失败: {}", orphaned_table, e);
            continue;
        }

        // 3. 从原表删除违规记录
        let delete_sql = format!("DELETE FROM {} WHERE rowid IN ({});", table, rowid_list);
        let delete_result = sqlx::query(&delete_sql).execute(pool).await;
        if let Err(e) = delete_result {
            tracing::warn!("从 {} 删除孤儿记录失败: {}", table, e);
            continue;
        }

        moved_count += rowids.len() as i64;
        tracing::info!(
            "已将 {} 条孤儿记录从 {} 移动到 {}",
            rowids.len(),
            table,
            orphaned_table
        );
    }

    Ok(moved_count)
}

/// 重建索引
///
/// 对所有现存索引执行 `REINDEX`，重建可能损坏的索引。
/// 注意：此函数不会创建缺失的索引（需要业务层提供索引定义）。
async fn rebuild_indexes(pool: &SqlitePool) -> Result<usize, AppError> {
    // 查询所有现存索引
    let index_names: Vec<String> = sqlx::query_scalar(
        r#"SELECT name FROM sqlite_master
           WHERE type='index' AND name NOT LIKE 'sqlite_%';"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Backup(format!("查询索引列表失败: {}", e)))?;

    let mut rebuilt = 0usize;
    for name in &index_names {
        // REINDEX <name> 重建指定索引
        let sql = format!("REINDEX {};", name);
        match sqlx::query(&sql).execute(pool).await {
            Ok(_) => rebuilt += 1,
            Err(e) => {
                tracing::warn!("重建索引 {} 失败: {}", name, e);
            }
        }
    }

    Ok(rebuilt)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_integrity_ok() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        let result = check_integrity(&pool).await.unwrap();
        assert!(result.ok);
        assert_eq!(result.message, "ok");

        pool.close().await;
    }

    #[tokio::test]
    async fn test_check_foreign_keys_no_violations() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 创建无 FK 的简单表
        sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY);")
            .execute(&pool)
            .await
            .unwrap();

        let result = check_foreign_keys(&pool).await.unwrap();
        assert!(result.ok);
        assert_eq!(result.violation_count, 0);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_check_foreign_keys_with_violation() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 创建父表和子表（带 FK）
        sqlx::query("CREATE TABLE parent (id INTEGER PRIMARY KEY);")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE child (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES parent(id));")
            .execute(&pool)
            .await
            .unwrap();

        // 启用 FK 后插入违规记录
        sqlx::query("PRAGMA foreign_keys=ON;").execute(&pool).await.unwrap();
        // 直接插入违规（绕过 FK 检查）
        sqlx::query("PRAGMA foreign_keys=OFF;").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO child (id, parent_id) VALUES (1, 999);")
            .execute(&pool)
            .await
            .unwrap();

        let result = check_foreign_keys(&pool).await.unwrap();
        assert!(!result.ok);
        assert!(result.violation_count > 0);
        assert!(!result.violations.is_empty());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_check_table_counts_no_baseline() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 创建一些表
        for table in backup_service::CRITICAL_TABLES {
            sqlx::query(&format!("CREATE TABLE IF NOT EXISTS {} (id INTEGER PRIMARY KEY);", table))
                .execute(&pool)
                .await
                .unwrap();
        }

        let baseline = HashMap::new();
        let result = check_table_counts(&pool, &baseline).await.unwrap();
        assert!(result.ok);
        assert_eq!(result.tables_checked, 0);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_check_table_counts_with_baseline() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 创建 users 表并插入 100 条记录
        sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY);")
            .execute(&pool)
            .await
            .unwrap();
        for i in 0..100 {
            sqlx::query("INSERT INTO users (id) VALUES (?);")
                .bind(i)
                .execute(&pool)
                .await
                .unwrap();
        }

        // 基线为 100 条
        let mut baseline = HashMap::new();
        baseline.insert("users".to_string(), 100);

        let result = check_table_counts(&pool, &baseline).await.unwrap();
        assert!(result.ok); // 偏差为 0
        assert_eq!(result.tables_checked, backup_service::CRITICAL_TABLES.len());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_check_table_counts_with_severe_deviation() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 创建 users 表并插入 10 条记录（基线 100 → 当前 10，偏差 90%，等于 CRITICAL_THRESHOLD）
        sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY);")
            .execute(&pool)
            .await
            .unwrap();
        for i in 0..10 {
            sqlx::query("INSERT INTO users (id) VALUES (?);")
                .bind(i)
                .execute(&pool)
                .await
                .unwrap();
        }

        // 基线为 100 条
        let mut baseline = HashMap::new();
        baseline.insert("users".to_string(), 100);

        let result = check_table_counts(&pool, &baseline).await.unwrap();
        // 偏差 90%，等于 CRITICAL_THRESHOLD（不大于，所以不视为 critical）
        // 但偏差 > WARN_THRESHOLD（50%），所以会出现在 deviations 列表中
        assert_eq!(result.deviations.len(), 1);
        assert_eq!(result.deviations[0].table_name, "users");
        // 偏差为 90%，等于 CRITICAL_THRESHOLD，不大于，所以 ok=true
        assert!(result.ok);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_check_indexes_all_present() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 创建测试表
        sqlx::query("CREATE TABLE test_table (id INTEGER PRIMARY KEY, col TEXT);")
            .execute(&pool)
            .await
            .unwrap();

        // 创建所有关键索引
        for idx in CRITICAL_INDEXES {
            sqlx::query(&format!("CREATE INDEX {} ON test_table (col);", idx))
                .execute(&pool)
                .await
                .unwrap();
        }

        let result = check_indexes(&pool).await.unwrap();
        assert!(result.ok);
        assert_eq!(result.indexes_checked, CRITICAL_INDEXES.len());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_check_indexes_missing() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 不创建任何索引
        let result = check_indexes(&pool).await.unwrap();
        assert!(!result.ok);
        assert!(!result.missing_indexes.is_empty());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_get_baseline_counts_no_records_db() {
        let temp = tempfile::tempdir().unwrap();
        // 不创建 backup_records.db
        let baseline = get_baseline_counts(temp.path()).await.unwrap();
        assert!(baseline.is_empty());
    }

    #[test]
    fn test_evaluate_status_healthy() {
        let integrity = IntegrityCheckResult { ok: true, message: "ok".into() };
        let foreign_keys = ForeignKeyCheckResult {
            ok: true,
            violation_count: 0,
            violations: vec![],
        };
        let table_counts = TableCountCheckResult {
            ok: true,
            deviations: vec![],
            tables_checked: 10,
        };
        let indexes = IndexCheckResult {
            ok: true,
            missing_indexes: vec![],
            indexes_checked: 8,
        };

        let status = evaluate_status(&integrity, &foreign_keys, &table_counts, &indexes);
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[test]
    fn test_evaluate_status_integrity_failure() {
        let integrity = IntegrityCheckResult {
            ok: false,
            message: "database disk image is malformed".into(),
        };
        let foreign_keys = ForeignKeyCheckResult {
            ok: true,
            violation_count: 0,
            violations: vec![],
        };
        let table_counts = TableCountCheckResult {
            ok: true,
            deviations: vec![],
            tables_checked: 10,
        };
        let indexes = IndexCheckResult {
            ok: true,
            missing_indexes: vec![],
            indexes_checked: 8,
        };

        let status = evaluate_status(&integrity, &foreign_keys, &table_counts, &indexes);
        assert_eq!(status, HealthStatus::Critical);
    }

    #[test]
    fn test_evaluate_status_fk_violation() {
        let integrity = IntegrityCheckResult { ok: true, message: "ok".into() };
        let foreign_keys = ForeignKeyCheckResult {
            ok: false,
            violation_count: 3,
            violations: vec!["child row=1 parent=parent fkid=0".into()],
        };
        let table_counts = TableCountCheckResult {
            ok: true,
            deviations: vec![],
            tables_checked: 10,
        };
        let indexes = IndexCheckResult {
            ok: true,
            missing_indexes: vec![],
            indexes_checked: 8,
        };

        let status = evaluate_status(&integrity, &foreign_keys, &table_counts, &indexes);
        assert_eq!(status, HealthStatus::Critical);
    }

    #[test]
    fn test_evaluate_status_warning_index_missing() {
        let integrity = IntegrityCheckResult { ok: true, message: "ok".into() };
        let foreign_keys = ForeignKeyCheckResult {
            ok: true,
            violation_count: 0,
            violations: vec![],
        };
        let table_counts = TableCountCheckResult {
            ok: true,
            deviations: vec![],
            tables_checked: 10,
        };
        let indexes = IndexCheckResult {
            ok: false,
            missing_indexes: vec!["idx_messages_conversation_id".into()],
            indexes_checked: 6,
        };

        let status = evaluate_status(&integrity, &foreign_keys, &table_counts, &indexes);
        assert_eq!(status, HealthStatus::Warning);
    }

    #[test]
    fn test_health_status_methods() {
        assert_eq!(HealthStatus::Healthy.as_str(), "healthy");
        assert_eq!(HealthStatus::Warning.as_str(), "warning");
        assert_eq!(HealthStatus::Critical.as_str(), "critical");

        assert!(!HealthStatus::Healthy.should_trigger_emergency_backup());
        assert!(!HealthStatus::Warning.should_trigger_emergency_backup());
        assert!(HealthStatus::Critical.should_trigger_emergency_backup());

        assert!(!HealthStatus::Healthy.needs_repair());
        assert!(HealthStatus::Warning.needs_repair());
        assert!(HealthStatus::Critical.needs_repair());
    }

    // ========================================================================
    // T2.10.2 自动修复测试
    // ========================================================================

    #[tokio::test]
    async fn test_repair_orphaned_records_moves_violations() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 创建父表和子表（带 FK）
        sqlx::query("CREATE TABLE parent (id INTEGER PRIMARY KEY, name TEXT);")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            r#"CREATE TABLE child (
                id INTEGER PRIMARY KEY,
                parent_id INTEGER REFERENCES parent(id),
                name TEXT
            );"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // 插入 2 条违规记录（绕过 FK 检查）
        sqlx::query("PRAGMA foreign_keys=OFF;").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO child (id, parent_id, name) VALUES (1, 999, 'orphan1');")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO child (id, parent_id, name) VALUES (2, 998, 'orphan2');")
            .execute(&pool)
            .await
            .unwrap();
        // 1 条正常记录
        sqlx::query("INSERT INTO parent (id, name) VALUES (1, 'p1');")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO child (id, parent_id, name) VALUES (3, 1, 'normal');")
            .execute(&pool)
            .await
            .unwrap();

        // 执行修复
        let fk_result = ForeignKeyCheckResult {
            ok: false,
            violation_count: 2,
            violations: vec![],
        };
        let moved = repair_orphaned_records(&pool, &fk_result).await.unwrap();
        assert_eq!(moved, 2);

        // 验证：原 child 表只剩 1 条正常记录
        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM child;")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, 1);

        // 验证：child_orphaned 表有 2 条记录
        let orphaned: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM child_orphaned;")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(orphaned, 2);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_rebuild_indexes() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 创建表和索引
        sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT);")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE INDEX idx_test_name ON test(name);")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO test (name) VALUES ('a'), ('b'), ('c');")
            .execute(&pool)
            .await
            .unwrap();

        let rebuilt = rebuild_indexes(&pool).await.unwrap();
        assert_eq!(rebuilt, 1); // 1 个索引（idx_test_name）

        pool.close().await;
    }

    #[tokio::test]
    async fn test_auto_repair_no_issues() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();
        let db_path = data_dir.join("nexterm.db");

        // 创建干净的数据库（无 FK 违规、无索引缺失）
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 创建 schema_migrations 表 + 所有 CRITICAL_INDEXES + CRITICAL_TABLES
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now')),
                applied_by TEXT,
                migration_hash TEXT NOT NULL,
                rollback_script TEXT,
                checksum_verified INTEGER NOT NULL DEFAULT 1
            );"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO schema_migrations (version, description, migration_hash) VALUES (89, 'test', 'h89');")
            .execute(&pool)
            .await
            .unwrap();

        // 创建测试表
        sqlx::query("CREATE TABLE test_table (id INTEGER PRIMARY KEY, col TEXT);")
            .execute(&pool)
            .await
            .unwrap();
        for idx in CRITICAL_INDEXES {
            sqlx::query(&format!("CREATE INDEX {} ON test_table (col);", idx))
                .execute(&pool)
                .await
                .unwrap();
        }

        let result = run_health_check(&pool, data_dir).await.unwrap();
        let repair_result = auto_repair(&pool, &result, data_dir).await.unwrap();

        // 无问题，repair_result.success = false（没有修复任何项）
        assert!(!repair_result.success);
        assert_eq!(repair_result.repaired_items, 0);
        assert_eq!(repair_result.orphaned_records_moved, 0);

        pool.close().await;
    }

    #[tokio::test]
    async fn test_auto_repair_fixes_fk_violations() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();
        let db_path = data_dir.join("nexterm.db");

        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        // 创建 schema_migrations + 父子表（带 FK）+ 关键索引
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now')),
                applied_by TEXT,
                migration_hash TEXT NOT NULL,
                rollback_script TEXT,
                checksum_verified INTEGER NOT NULL DEFAULT 1
            );"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO schema_migrations (version, description, migration_hash) VALUES (89, 'test', 'h89');")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE parent (id INTEGER PRIMARY KEY);")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE child (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES parent(id));")
            .execute(&pool)
            .await
            .unwrap();
        // 创建关键索引（避免索引缺失告警）
        sqlx::query("CREATE TABLE test_table (col TEXT);")
            .execute(&pool)
            .await
            .unwrap();
        for idx in CRITICAL_INDEXES {
            sqlx::query(&format!("CREATE INDEX {} ON test_table (col);", idx))
                .execute(&pool)
                .await
                .unwrap();
        }

        // 插入违规记录
        sqlx::query("PRAGMA foreign_keys=OFF;").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO child (id, parent_id) VALUES (1, 999);")
            .execute(&pool)
            .await
            .unwrap();

        let result = run_health_check(&pool, data_dir).await.unwrap();
        assert_eq!(result.status, HealthStatus::Critical); // FK 违规 → Critical

        let repair_result = auto_repair(&pool, &result, data_dir).await.unwrap();
        assert!(repair_result.success);
        assert_eq!(repair_result.orphaned_records_moved, 1);
        assert!(repair_result.details.iter().any(|d| d.contains("孤儿记录")));

        // 验证修复后状态（无 FK 违规 → 不再 Critical）
        if let Some(post_status) = repair_result.post_repair_status {
            assert_ne!(post_status, HealthStatus::Critical);
        }

        pool.close().await;
    }
}
