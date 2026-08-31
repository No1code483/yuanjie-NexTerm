//! T2.9 数据恢复与导出 - 导出/导入服务
//!
//! 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.5
//!
//! 导出格式（zip）：
//! ```
//! nexterm_export_20260720.zip
//! ├── data.db                          # SQLite 数据库副本（VACUUM INTO）
//! ├── manifest.json                    # 导出元信息
//! ├── checksum.sha256                  # data.db 的 SHA256 校验和
//! └── attachments/                     # 知识库附件文件（T2.9 暂不实现）
//! ```
//!
//! 导入兼容性：
//! - 检测导入文件的 schema_version
//! - 若低于当前版本 → 自动执行迁移
//! - 若高于当前版本 → 拒绝导入并提示升级
//! - 若 checksum 不匹配 → 拒绝导入

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use chrono::Local;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use zip::ZipArchive;
use zip::write::ZipWriter;

use crate::error::app_error::AppError;
use crate::models::restore::{
    ExportManifest, ExportResult, ImportCompatibility, ImportResult,
};
use crate::services::backup_service;

/// 导出 zip 中的文件名常量
const DATA_DB_FILENAME: &str = "data.db";
const MANIFEST_FILENAME: &str = "manifest.json";
const CHECKSUM_FILENAME: &str = "checksum.sha256";

/// 应用版本号（从 Cargo.toml 读取，或硬编码）
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = "NexTerm·元界";

/// 数据库文件名（与 connection.rs 一致）
const DB_FILENAME: &str = "nexterm.db";

// ============================================================================
// 公共 API - 导出
// ============================================================================

/// 全量导出数据库到 zip 文件
///
/// 流程：
/// 1. 创建临时备份（VACUUM INTO）到临时文件
/// 2. 计算 SHA256 校验和
/// 3. 统计关键表行数
/// 4. 生成 manifest.json
/// 5. 打包为 zip（data.db + manifest.json + checksum.sha256）
/// 6. 返回 ExportResult
///
/// 参数：
/// - `pool`：当前数据库连接池
/// - `data_dir`：应用数据目录（导出文件存放在 `<data_dir>/exports/`）
/// - `exported_by`：导出者用户名（可选）
pub async fn export_to_zip(
    pool: &SqlitePool,
    data_dir: &Path,
    exported_by: Option<&str>,
) -> Result<ExportResult, AppError> {
    let exports_dir = data_dir.join("exports");
    tokio::fs::create_dir_all(&exports_dir).await?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let export_filename = format!("nexterm_export_{}.zip", timestamp);
    let export_path = exports_dir.join(&export_filename);

    // 避免同秒重复
    let export_path = ensure_unique_path(&export_path).await?;

    // 1. 创建临时数据库备份（VACUUM INTO 临时文件）
    let temp_db = exports_dir.join(format!("_temp_export_{}.db", timestamp));
    let vacuum_sql = format!("VACUUM INTO '{}'", temp_db.display());
    sqlx::query(&vacuum_sql)
        .execute(pool)
        .await
        .map_err(|e| AppError::Backup(format!("VACUUM INTO 失败: {}", e)))?;

    // 2. 在临时数据库上收集元信息
    let metadata = backup_service::verify_backup(&temp_db).await?;
    let schema_version = metadata.schema_version;
    let table_counts = metadata.table_counts.clone();

    // 3. 计算 SHA256
    let checksum = compute_file_sha256(&temp_db).await?;
    let size_bytes = tokio::fs::metadata(&temp_db).await?.len() as i64;

    // 4. 生成 manifest.json
    let manifest = ExportManifest {
        version: APP_VERSION.to_string(),
        schema_version,
        exported_at: Local::now().to_rfc3339(),
        exported_by: exported_by.map(|s| s.to_string()),
        table_counts: table_counts.clone(),
        app_name: APP_NAME.to_string(),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| AppError::Backup(format!("manifest 序列化失败: {}", e)))?;

    // 5. 打包为 zip
    let zip_path = create_export_zip(
        &export_path,
        &temp_db,
        &manifest_json,
        &checksum,
    )?;

    // 6. 删除临时数据库文件
    let _ = tokio::fs::remove_file(&temp_db).await;

    let exported_at = Local::now().timestamp();

    tracing::info!(
        "导出完成: path={}, size={}bytes, schema_version={}",
        zip_path.display(),
        size_bytes,
        schema_version
    );

    Ok(ExportResult {
        export_path: zip_path.to_string_lossy().to_string(),
        size_bytes,
        exported_at,
        schema_version,
        checksum,
        table_counts,
    })
}

// ============================================================================
// 公共 API - 导入
// ============================================================================

/// 检查导入文件兼容性
///
/// 在实际导入前调用，检查 schema_version 兼容性
pub async fn check_import_compatibility(
    zip_path: &str,
    current_schema_version: i64,
) -> Result<ImportCompatibility, AppError> {
    let path = PathBuf::from(zip_path);
    if !path.exists() {
        return Err(AppError::Backup("导入文件不存在".into()));
    }

    // 从 zip 中读取 manifest.json
    let manifest = read_manifest_from_zip(&path)?;

    let source_version = manifest.schema_version;
    let compatible = source_version <= current_schema_version;
    let needs_migration = source_version < current_schema_version;

    let reason = if !compatible {
        Some(format!(
            "导入文件 schema_version={} 高于当前版本 {}，请升级应用后导入",
            source_version, current_schema_version
        ))
    } else if needs_migration {
        Some(format!(
            "导入文件 schema_version={} 低于当前版本 {}，导入后将自动执行迁移",
            source_version, current_schema_version
        ))
    } else {
        None
    };

    Ok(ImportCompatibility {
        source_schema_version: source_version,
        target_schema_version: current_schema_version,
        compatible,
        reason,
        needs_migration,
    })
}

/// 从 zip 文件导入数据
///
/// 流程：
/// 1. 检查兼容性
/// 2. 解压 data.db 到临时文件
/// 3. 校验 SHA256
/// 4. 关闭当前连接池
/// 5. 复制 data.db 到数据库位置
/// 6. 删除 -wal 和 -shm
/// 7. 返回成功，前端重启后自动执行迁移
///
/// 参数：
/// - `pool`：当前数据库连接池（导入后将关闭）
/// - `data_dir`：应用数据目录
/// - `zip_path`：导入 zip 文件路径
pub async fn import_from_zip(
    pool: &SqlitePool,
    data_dir: &Path,
    zip_path: &str,
) -> Result<ImportResult, AppError> {
    let path = PathBuf::from(zip_path);
    if !path.exists() {
        return Err(AppError::Backup("导入文件不存在".into()));
    }

    tracing::info!("开始数据导入: zip_path={}", zip_path);

    // 1. 读取 manifest 并检查兼容性
    let manifest = read_manifest_from_zip(&path)?;
    let current_schema_version: i64 = sqlx::query_scalar(
        "SELECT MAX(version) FROM schema_migrations WHERE version IS NOT NULL;",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if manifest.schema_version > current_schema_version {
        return Err(AppError::Backup(format!(
            "导入文件 schema_version={} 高于当前版本 {}，请升级应用后导入",
            manifest.schema_version, current_schema_version
        )));
    }

    // 2. 解压 data.db 到临时文件
    let exports_dir = data_dir.join("exports");
    tokio::fs::create_dir_all(&exports_dir).await?;
    let temp_db = exports_dir.join(format!(
        "_temp_import_{}.db",
        Local::now().format("%Y%m%d_%H%M%S")
    ));

    let expected_checksum = extract_db_from_zip(&path, &temp_db)?;

    // 3. 校验 SHA256
    let actual_checksum = compute_file_sha256(&temp_db).await?;
    if actual_checksum != expected_checksum {
        let _ = tokio::fs::remove_file(&temp_db).await;
        return Err(AppError::Backup(format!(
            "校验和不匹配: 期望={}, 实际={}",
            expected_checksum, actual_checksum
        )));
    }
    tracing::info!("导入文件校验通过: checksum={}", actual_checksum);

    // 4. 备份当前数据库（回滚点）
    let rollback_backup = backup_service::create_backup(
        pool,
        data_dir,
        crate::models::backup::BackupType::PreOp,
        Some("pre_import"),
    )
    .await?;

    // 5. 关闭连接池
    pool.close().await;

    // 6. 复制 data.db 到数据库位置
    let db_path = data_dir.join(DB_FILENAME);
    if let Err(e) = tokio::fs::copy(&temp_db, &db_path).await {
        // 复制失败 → 回滚
        tracing::error!("复制导入文件失败: {}，尝试回滚", e);
        let _ = tokio::fs::copy(&rollback_backup.backup_path, &db_path).await;
        return Err(AppError::Backup(format!("复制导入文件失败（已回滚）: {}", e)));
    }

    // 7. 清理 WAL/SHM
    let wal_path = data_dir.join(format!("{}-wal", DB_FILENAME));
    let shm_path = data_dir.join(format!("{}-shm", DB_FILENAME));
    if wal_path.exists() {
        let _ = tokio::fs::remove_file(&wal_path).await;
    }
    if shm_path.exists() {
        let _ = tokio::fs::remove_file(&shm_path).await;
    }

    // 8. 删除临时文件
    let _ = tokio::fs::remove_file(&temp_db).await;

    let needs_migration = manifest.schema_version < current_schema_version;
    let imported_at = Local::now().timestamp();

    tracing::info!(
        "导入完成: schema_version={}, needs_migration={}",
        manifest.schema_version,
        needs_migration
    );

    Ok(ImportResult {
        success: true,
        need_restart: true,
        schema_version: manifest.schema_version,
        table_counts: manifest.table_counts,
        imported_at,
        migrations_applied: false, // 迁移在重启后由 run_migrations 自动执行
    })
}

// ============================================================================
// 内部辅助函数
// ============================================================================

/// 创建导出 zip 文件
fn create_export_zip(
    zip_path: &Path,
    db_path: &Path,
    manifest_json: &str,
    checksum: &str,
) -> Result<PathBuf, AppError> {
    let file = std::fs::File::create(zip_path)
        .map_err(|e| AppError::Backup(format!("创建 zip 文件失败: {}", e)))?;
    let mut zip = ZipWriter::new(file);

    // 添加 data.db
    let db_bytes = std::fs::read(db_path)
        .map_err(|e| AppError::Backup(format!("读取 data.db 失败: {}", e)))?;
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(DATA_DB_FILENAME, options)
        .map_err(|e| AppError::Backup(format!("zip 写入 data.db 失败: {}", e)))?;
    zip.write_all(&db_bytes)
        .map_err(|e| AppError::Backup(format!("zip 写入 data.db 内容失败: {}", e)))?;

    // 添加 manifest.json
    zip.start_file(MANIFEST_FILENAME, options)
        .map_err(|e| AppError::Backup(format!("zip 写入 manifest.json 失败: {}", e)))?;
    zip.write_all(manifest_json.as_bytes())
        .map_err(|e| AppError::Backup(format!("zip 写入 manifest.json 内容失败: {}", e)))?;

    // 添加 checksum.sha256
    let checksum_content = format!("{}  {}\n", checksum, DATA_DB_FILENAME);
    zip.start_file(CHECKSUM_FILENAME, options)
        .map_err(|e| AppError::Backup(format!("zip 写入 checksum.sha256 失败: {}", e)))?;
    zip.write_all(checksum_content.as_bytes())
        .map_err(|e| AppError::Backup(format!("zip 写入 checksum.sha256 内容失败: {}", e)))?;

    zip.finish()
        .map_err(|e| AppError::Backup(format!("zip 完成失败: {}", e)))?;

    Ok(zip_path.to_path_buf())
}

/// 从 zip 读取 manifest.json
fn read_manifest_from_zip(zip_path: &Path) -> Result<ExportManifest, AppError> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| AppError::Backup(format!("打开 zip 文件失败: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Backup(format!("读取 zip 归档失败: {}", e)))?;

    let mut manifest_file = archive
        .by_name(MANIFEST_FILENAME)
        .map_err(|e| AppError::Backup(format!("zip 中未找到 {}: {}", MANIFEST_FILENAME, e)))?;
    let mut manifest_json = String::new();
    manifest_file
        .read_to_string(&mut manifest_json)
        .map_err(|e| AppError::Backup(format!("读取 manifest.json 失败: {}", e)))?;

    let manifest: ExportManifest = serde_json::from_str(&manifest_json)
        .map_err(|e| AppError::Backup(format!("解析 manifest.json 失败: {}", e)))?;

    Ok(manifest)
}

/// 从 zip 解压 data.db 到指定路径，返回期望的 SHA256 校验和
fn extract_db_from_zip(zip_path: &Path, dest_db: &Path) -> Result<String, AppError> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| AppError::Backup(format!("打开 zip 文件失败: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Backup(format!("读取 zip 归档失败: {}", e)))?;

    // 读取 checksum.sha256
    let mut checksum_content = String::new();
    {
        let mut checksum_file = archive
            .by_name(CHECKSUM_FILENAME)
            .map_err(|e| AppError::Backup(format!("zip 中未找到 {}: {}", CHECKSUM_FILENAME, e)))?;
        checksum_file
            .read_to_string(&mut checksum_content)
            .map_err(|e| AppError::Backup(format!("读取 checksum.sha256 失败: {}", e)))?;
    }

    // 解析校验和（格式: "<sha256>  data.db\n"）
    let expected_checksum = checksum_content
        .split_whitespace()
        .next()
        .ok_or_else(|| AppError::Backup("checksum.sha256 格式无效".into()))?
        .to_string();

    // 解压 data.db
    let mut db_file = archive
        .by_name(DATA_DB_FILENAME)
        .map_err(|e| AppError::Backup(format!("zip 中未找到 {}: {}", DATA_DB_FILENAME, e)))?;

    let mut dest_file = std::fs::File::create(dest_db)
        .map_err(|e| AppError::Backup(format!("创建临时文件失败: {}", e)))?;
    std::io::copy(&mut db_file, &mut dest_file)
        .map_err(|e| AppError::Backup(format!("解压 data.db 失败: {}", e)))?;

    Ok(expected_checksum)
}

/// 计算文件 SHA256 校验和（同步版本，用于 zip 打包流程）
async fn compute_file_sha256(path: &Path) -> Result<String, AppError> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = tokio::io::AsyncReadExt::read(&mut file, &mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// 避免同秒重复创建导出文件
async fn ensure_unique_path(path: &Path) -> Result<PathBuf, AppError> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| AppError::Backup("无法解析导出文件名".into()))?;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("zip");
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Backup("无法获取父目录".into()))?;

    for i in 1..100 {
        let new_name = format!("{}_{}.{}", stem, i, ext);
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }
    Err(AppError::Backup(
        "无法生成唯一导出文件名（重试 100 次失败）".into(),
    ))
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_export_to_zip_creates_valid_zip() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();

        // 创建主库（含 schema_migrations 表 + 测试数据）
        let db_path = data_dir.join("main.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap();

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
        sqlx::query("INSERT INTO schema_migrations (version, description, migration_hash) VALUES (89, 'test', 'hash89');")
            .execute(&pool)
            .await
            .unwrap();

        // 创建关键表
        for table in backup_service::CRITICAL_TABLES {
            sqlx::query(&format!("CREATE TABLE IF NOT EXISTS {} (id INTEGER PRIMARY KEY);", table))
                .execute(&pool)
                .await
                .unwrap();
        }

        // 执行导出
        let result = export_to_zip(&pool, data_dir, Some("test_user"))
            .await
            .unwrap();

        assert!(result.size_bytes > 0);
        assert_eq!(result.schema_version, 89);
        assert!(!result.checksum.is_empty());
        assert!(result.export_path.ends_with(".zip"));

        // 验证 zip 文件存在
        assert!(tokio::fs::metadata(&result.export_path).await.is_ok());

        pool.close().await;
    }

    #[tokio::test]
    async fn test_check_import_compatibility_higher_version_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();

        // 创建一个 schema_version=100 的导出 zip
        let db_path = data_dir.join("main.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap();

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
        sqlx::query("INSERT INTO schema_migrations (version, description, migration_hash) VALUES (100, 'future', 'hash100');")
            .execute(&pool)
            .await
            .unwrap();

        for table in backup_service::CRITICAL_TABLES {
            sqlx::query(&format!("CREATE TABLE IF NOT EXISTS {} (id INTEGER PRIMARY KEY);", table))
                .execute(&pool)
                .await
                .unwrap();
        }

        let export_result = export_to_zip(&pool, data_dir, None).await.unwrap();
        pool.close().await;

        // 检查兼容性（当前版本 89 < 导入版本 100）
        let compat = check_import_compatibility(&export_result.export_path, 89)
            .await
            .unwrap();
        assert!(!compat.compatible);
        assert_eq!(compat.source_schema_version, 100);
        assert_eq!(compat.target_schema_version, 89);
        assert!(compat.reason.is_some());
    }

    #[tokio::test]
    async fn test_check_import_compatibility_lower_version_needs_migration() {
        let temp = tempfile::tempdir().unwrap();
        let data_dir = temp.path();

        // 创建一个 schema_version=80 的导出 zip
        let db_path = data_dir.join("main.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap();

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
        sqlx::query("INSERT INTO schema_migrations (version, description, migration_hash) VALUES (80, 'older', 'hash80');")
            .execute(&pool)
            .await
            .unwrap();

        for table in backup_service::CRITICAL_TABLES {
            sqlx::query(&format!("CREATE TABLE IF NOT EXISTS {} (id INTEGER PRIMARY KEY);", table))
                .execute(&pool)
                .await
                .unwrap();
        }

        let export_result = export_to_zip(&pool, data_dir, None).await.unwrap();
        pool.close().await;

        // 检查兼容性（当前版本 89 > 导入版本 80）
        let compat = check_import_compatibility(&export_result.export_path, 89)
            .await
            .unwrap();
        assert!(compat.compatible);
        assert!(compat.needs_migration);
        assert_eq!(compat.source_schema_version, 80);
    }

    #[tokio::test]
    async fn test_check_import_compatibility_nonexistent_file() {
        let result = check_import_compatibility("/nonexistent.zip", 89).await;
        assert!(result.is_err());
    }
}
