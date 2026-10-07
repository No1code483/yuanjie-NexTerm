//! 数据层（契约：06_Rust代码契约.md §六）
//! 表归属校验 + 迁移前备份 + 迁移编排器基线机制

use kernel_api::KernelError;
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::plugin::MigrationFile;

/// 数据句柄：插件执行 SQL 前的表归属校验
pub struct DbHandle {
    pool: SqlitePool,
    /// 数据库文件路径（内存库为 None，不支持备份）
    db_path: Option<PathBuf>,
    /// 表归属缓存（启动时从 kernel_table_ownership 加载）：table -> plugin_id
    ownership: RwLock<HashMap<String, String>>,
}

impl DbHandle {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            db_path: None,
            ownership: RwLock::new(HashMap::new()),
        }
    }

    /// 携带数据库文件路径构造（tauri_glue 用，启用迁移备份）
    pub fn with_path(pool: SqlitePool, db_path: PathBuf) -> Self {
        Self {
            pool,
            db_path: Some(db_path),
            ownership: RwLock::new(HashMap::new()),
        }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// 启动时从 kernel_table_ownership 加载归属缓存
    pub async fn load_ownership(&self) -> Result<(), KernelError> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT table_name, plugin_id FROM kernel_table_ownership")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| KernelError::Db(e.to_string()))?;
        let mut map = self.ownership.write().unwrap();
        map.clear();
        for (t, p) in rows {
            map.insert(t, p);
        }
        Ok(())
    }

    /// 查询表归属与前缀标志。
    pub async fn ownership_record(
        &self,
        table_name: &str,
    ) -> Result<Option<(String, bool)>, KernelError> {
        let row: Option<(String, i64)> = sqlx::query_as(
            "SELECT plugin_id, name_prefixed FROM kernel_table_ownership WHERE table_name = ?",
        )
        .bind(table_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| KernelError::Db(e.to_string()))?;
        Ok(row.map(|(plugin_id, name_prefixed)| (plugin_id, name_prefixed != 0)))
    }

    /// 归属登记（迁移编排器/内核 bootstrap 调用）。
    /// 已登记项仅允许同插件、同前缀标志的幂等调用，禁止静默改写。
    pub async fn register_table(
        &self,
        table_name: &str,
        plugin_id: &str,
        name_prefixed: bool,
    ) -> Result<(), KernelError> {
        if let Some((owner, registered_prefixed)) = self.ownership_record(table_name).await? {
            if owner != plugin_id {
                return Err(KernelError::Config(format!(
                    "表 {table_name} 已归属插件 {owner}，不能重新登记给 {plugin_id}"
                )));
            }
            if registered_prefixed != name_prefixed {
                return Err(KernelError::Config(format!(
                    "表 {table_name} 的 name_prefixed 已登记为 {}，不能改写为 {}",
                    i32::from(registered_prefixed),
                    i32::from(name_prefixed)
                )));
            }
            self.ownership
                .write()
                .unwrap()
                .insert(table_name.to_string(), plugin_id.to_string());
            return Ok(());
        }

        sqlx::query(
            "INSERT INTO kernel_table_ownership (table_name, plugin_id, name_prefixed, created_at) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(table_name)
        .bind(plugin_id)
        .bind(if name_prefixed { 1 } else { 0 })
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| KernelError::Db(e.to_string()))?;
        self.ownership
            .write()
            .unwrap()
            .insert(table_name.to_string(), plugin_id.to_string());
        Ok(())
    }

    /// 插件执行 SQL 前的表归属校验（repository 基类统一调用）
    /// 规则：SQL 中出现的表名 ∈ {本插件登记表} ∪ {kernel_*}
    pub fn validate_tables(&self, plugin: &str, sql: &str) -> Result<(), KernelError> {
        for table in extract_table_names(sql) {
            // kernel 拥有的表对全插件放行
            if table.starts_with("kernel_") {
                continue;
            }
            let owner = self.ownership.read().unwrap().get(&table).cloned();
            match owner {
                Some(owner) if owner == plugin => {}
                Some(owner) => {
                    return Err(KernelError::PermissionDenied(format!(
                        "插件 {plugin} 无权访问表 {table}（归属 {owner}）"
                    )));
                }
                None => {
                    return Err(KernelError::PermissionDenied(format!(
                        "表 {table} 未登记归属，插件 {plugin} 拒绝访问"
                    )));
                }
            }
        }
        Ok(())
    }

    /// 迁移前自动备份：backups/pre-migrate-<ns>-<ver>.db（保留最近 10 份）
    pub async fn snapshot_before_migrate(
        &self,
        ns: &str,
        ver: i64,
    ) -> Result<PathBuf, KernelError> {
        let db_path = match &self.db_path {
            Some(p) if p.exists() => p.clone(),
            _ => {
                return Err(KernelError::Config(
                    "无法定位数据库文件（内存库不支持备份）".into(),
                ));
            }
        };

        let dir = db_path.parent().unwrap_or(Path::new(".")).join("backups");
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| KernelError::Db(format!("创建备份目录失败: {e}")))?;
        let target = dir.join(format!("pre-migrate-{ns}-{ver}.db"));
        if tokio::fs::try_exists(&target)
            .await
            .map_err(|e| KernelError::Db(format!("检查备份目标失败: {e}")))?
        {
            return Err(KernelError::Db(format!(
                "备份目标已存在，拒绝覆盖: {}",
                target.display()
            )));
        }

        let target_string = target
            .to_str()
            .ok_or_else(|| KernelError::Db("备份目标路径不是有效 UTF-8".into()))?;
        sqlx::query("VACUUM INTO ?")
            .bind(target_string)
            .execute(&self.pool)
            .await
            .map_err(|e| KernelError::Db(format!("备份失败: {e}")))?;

        // 保留最近 10 份（按文件名排序删除最旧）
        prune_backups(&dir, 10).await?;
        Ok(target)
    }
}

/// 删除最旧的备份，保留 keep 份
async fn prune_backups(dir: &std::path::Path, keep: usize) -> Result<(), KernelError> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut rd = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| KernelError::Db(format!("读取备份目录失败: {e}")))?;
    while let Some(entry) = rd
        .next_entry()
        .await
        .map_err(|e| KernelError::Db(e.to_string()))?
    {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("db") {
            files.push(p);
        }
    }
    files.sort();
    if files.len() > keep {
        for f in files[..files.len() - keep].iter() {
            let _ = tokio::fs::remove_file(f).await;
        }
    }
    Ok(())
}

/// 从 SQL 中提取表名（阶段1 词法级粗提取：FROM/INTO/UPDATE/JOIN/TABLE 后的标识符）
/// 阶段3 收编 sql_safety.rs 后可换用解析器；词法法对插件间越权拦截足够（白名单制）
pub fn extract_table_names(sql: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = sql.to_lowercase();
    let bytes = lower.as_bytes();
    let kw_list = ["from", "into", "update", "join", "table"];
    let mut i = 0;
    let n = bytes.len();
    while i < n {
        for kw in kw_list.iter() {
            if lower[i..].starts_with(kw) {
                // 词边界：前一个字符必须是非字母数字下划线
                let prev_ok =
                    i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
                if !prev_ok {
                    continue;
                }
                let mut j = i + kw.len();
                // 跳过空格与 IF NOT EXISTS 情况（table 后）
                while j < n && bytes[j] == b' ' {
                    j += 1;
                }
                if kw == &"table" && lower[j..].starts_with("if not exists") {
                    j += "if not exists".len();
                    while j < n && bytes[j] == b' ' {
                        j += 1;
                    }
                }
                // 提取标识符
                let start = j;
                while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                    j += 1;
                }
                if j > start {
                    let name = &lower[start..j];
                    // 排除子查询/函数误报（如 "from (select" / "from ("）
                    if name
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_alphabetic() || c == '_')
                        == Some(true)
                    {
                        out.push(name.to_string());
                    }
                }
                i = j;
                break;
            }
        }
        i += 1;
    }
    out
}

/// 迁移编排器
pub struct MigrationOrchestrator;

/// 迁移集（一个命名空间 + 其脚本链）
pub struct MigrationSet {
    /// 如 "hm_todo"
    pub namespace: String,
    pub files: Vec<MigrationFile>,
}

/// 单命名空间迁移结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct MigrationOutcome {
    pub namespace: String,
    /// 实际应用的版本列表
    pub applied: Vec<i64>,
}

impl MigrationOrchestrator {
    /// 应用顺序：kernel 自身 → 按拓扑序逐插件
    /// 每插件记录 (namespace, version)：应用其链中 version 之后的新脚本
    /// 基线机制：首装时若库中已有同名表（旧结构存量），先登记基线版本，不重放
    pub async fn apply_all(
        &self,
        db: &DbHandle,
        sets: Vec<MigrationSet>,
    ) -> Result<Vec<MigrationOutcome>, KernelError> {
        let mut outcomes = Vec::new();
        for set in sets {
            let mut applied = Vec::new();
            for file in &set.files {
                let already: Option<i64> = sqlx::query_scalar(
                    "SELECT version FROM plugin_migrations WHERE namespace = ? AND version = ?",
                )
                .bind(&set.namespace)
                .bind(file.version)
                .fetch_optional(db.pool())
                .await
                .map_err(|e| KernelError::Db(e.to_string()))?;

                if already.is_some() {
                    continue; // 基线不重放
                }

                // 表已存在而版本未记录 → 视为旧结构存量，登记基线不执行（幂等保护）
                if table_exists(db, &set.namespace, file.sql).await? {
                    sqlx::query("INSERT OR IGNORE INTO plugin_migrations (namespace, version, applied_at) VALUES (?, ?, ?)")
                        .bind(&set.namespace)
                        .bind(file.version)
                        .bind(chrono::Utc::now().to_rfc3339())
                        .execute(db.pool())
                        .await
                        .map_err(|e| KernelError::Db(e.to_string()))?;
                    continue;
                }

                // 文件库迁移必须先完成备份；内存库仅用于测试，不要求快照。
                if db.db_path.is_some() {
                    db.snapshot_before_migrate(&set.namespace, file.version)
                        .await?;
                }
                let mut tx = db
                    .pool()
                    .begin()
                    .await
                    .map_err(|e| KernelError::Db(e.to_string()))?;
                for stmt in split_sql_statements(file.sql) {
                    sqlx::query(&stmt).execute(&mut *tx).await.map_err(|e| {
                        KernelError::Db(format!("{} v{}: {e}", set.namespace, file.version))
                    })?;
                }
                sqlx::query("INSERT INTO plugin_migrations (namespace, version, applied_at) VALUES (?, ?, ?)")
                    .bind(&set.namespace)
                    .bind(file.version)
                    .bind(chrono::Utc::now().to_rfc3339())
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| KernelError::Db(e.to_string()))?;
                tx.commit()
                    .await
                    .map_err(|e| KernelError::Db(e.to_string()))?;
                applied.push(file.version);
            }
            outcomes.push(MigrationOutcome {
                namespace: set.namespace,
                applied,
            });
        }
        Ok(outcomes)
    }
}

/// 判断迁移脚本的目标表是否已存在于库中（脚本须为 CREATE TABLE IF NOT EXISTS 形态，
/// 提取首个 CREATE TABLE 的表名查询 sqlite_master）
async fn table_exists(db: &DbHandle, _ns: &str, sql: &str) -> Result<bool, KernelError> {
    let lower = sql.to_lowercase();
    let idx = match lower.find("create table") {
        Some(i) => i,
        None => return Ok(false), // 非 CREATE TABLE 脚本不做存在性判断
    };
    let rest = &sql[idx + "create table".len()..];
    let rest = rest.trim_start();
    let rest = rest
        .strip_prefix("if not exists")
        .unwrap_or(rest)
        .trim_start();
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        return Ok(false);
    }
    let found: Option<i64> =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(&name)
            .fetch_one(db.pool())
            .await
            .map_err(|e| KernelError::Db(e.to_string()))?;
    Ok(found.unwrap_or(0) > 0)
}

/// 按 ';' 分割 SQL 语句（阶段1 简单分割：脚本由内核侧编写，不含触发器/字符串分号）
fn split_sql_statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_db() -> DbHandle {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect mem db");
        let db = DbHandle::new(pool);
        sqlx::query("CREATE TABLE IF NOT EXISTS kernel_table_ownership (table_name TEXT PRIMARY KEY, plugin_id TEXT NOT NULL, name_prefixed INTEGER NOT NULL, created_at TEXT NOT NULL)")
            .execute(db.pool()).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS plugin_migrations (namespace TEXT NOT NULL, version INTEGER NOT NULL, applied_at TEXT NOT NULL, PRIMARY KEY (namespace, version))")
            .execute(db.pool()).await.unwrap();
        db
    }

    async fn file_db() -> (tempfile::TempDir, PathBuf, DbHandle) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let db_path = dir.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .pragma("wal_autocheckpoint", "0");
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("connect file db");
        let db = DbHandle::with_path(pool, db_path.clone());
        sqlx::query("CREATE TABLE kernel_table_ownership (table_name TEXT PRIMARY KEY, plugin_id TEXT NOT NULL, name_prefixed INTEGER NOT NULL, created_at TEXT NOT NULL)")
            .execute(db.pool()).await.unwrap();
        sqlx::query("CREATE TABLE plugin_migrations (namespace TEXT NOT NULL, version INTEGER NOT NULL, applied_at TEXT NOT NULL, PRIMARY KEY (namespace, version))")
            .execute(db.pool()).await.unwrap();
        sqlx::query("CREATE TABLE seed_data (value TEXT NOT NULL)")
            .execute(db.pool())
            .await
            .unwrap();
        sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(db.pool())
            .await
            .expect("checkpoint schema before WAL-only seed");
        sqlx::query("INSERT INTO seed_data (value) VALUES ('committed in wal')")
            .execute(db.pool())
            .await
            .unwrap();
        let wal_path = db_path.with_extension("db-wal");
        assert!(
            tokio::fs::metadata(&wal_path).await.unwrap().len() > 0,
            "committed seed should leave a non-empty WAL"
        );
        (dir, db_path, db)
    }

    fn setup_kernel_tables() {
        // 与 06 §六表结构一致（mem_db 内已建）
    }

    #[tokio::test]
    async fn ownership_registration_is_conflict_safe() {
        let db = mem_db().await;
        db.register_table("resumes", "boards.profile", false)
            .await
            .unwrap();
        db.register_table("resumes", "boards.profile", false)
            .await
            .unwrap();

        let row = db.ownership_record("resumes").await.unwrap();
        assert_eq!(row, Some(("boards.profile".into(), false)));

        let err = db
            .register_table("resumes", "boards.other", false)
            .await
            .unwrap_err();
        assert!(matches!(err, KernelError::Config(_)), "{err:?}");

        let err = db
            .register_table("resumes", "boards.profile", true)
            .await
            .unwrap_err();
        assert!(matches!(err, KernelError::Config(_)), "{err:?}");
        assert_eq!(
            db.ownership_record("resumes").await.unwrap(),
            Some(("boards.profile".into(), false))
        );
    }

    // 必测 1：跨插件表查询被拒
    #[tokio::test]
    async fn cross_plugin_table_query_rejected() {
        let db = mem_db().await;
        db.register_table("hw_notes", "_hello", true).await.unwrap();
        // 本插件访问自己的表 → OK
        db.validate_tables("_hello", "SELECT * FROM hw_notes")
            .unwrap();
        // 其他插件访问 → 拒绝
        let err = db
            .validate_tables("_other", "SELECT * FROM hw_notes")
            .unwrap_err();
        assert!(
            matches!(err, KernelError::PermissionDenied(_)),
            "实际: {err:?}"
        );
        // 未登记表 → 拒绝
        let err = db
            .validate_tables("_hello", "SELECT * FROM ghost_table")
            .unwrap_err();
        assert!(
            matches!(err, KernelError::PermissionDenied(_)),
            "实际: {err:?}"
        );
    }

    // 必测 2：kernel_* 表放行
    #[tokio::test]
    async fn kernel_tables_allowed() {
        let db = mem_db().await;
        db.validate_tables("_hello", "SELECT * FROM kernel_domain_events")
            .unwrap();
        db.validate_tables(
            "_hello",
            "INSERT INTO kernel_table_ownership VALUES ('t','p',1,'x')",
        )
        .unwrap();
    }

    // 必测 3：migration 基线不重放（表已存在时登记版本但不执行）
    #[tokio::test]
    async fn migration_baseline_not_replayed() {
        let db = mem_db().await;
        // 预置旧结构存量表（模拟旧版单体建的表）
        sqlx::query("CREATE TABLE hw_notes (id INTEGER PRIMARY KEY, note TEXT NOT NULL)")
            .execute(db.pool())
            .await
            .unwrap();

        let orch = MigrationOrchestrator;
        let sets = vec![MigrationSet {
            namespace: "_hello".into(),
            files: vec![MigrationFile {
                version: 1,
                sql: "CREATE TABLE IF NOT EXISTS hw_notes (id INTEGER PRIMARY KEY, note TEXT NOT NULL, extra TEXT)",
            }],
        }];
        let outcomes = orch.apply_all(&db, sets).await.unwrap();
        // 首个插件基线登记；此处表已存在 → 不重放（不报错）
        assert_eq!(outcomes[0].namespace, "_hello");

        // plugin_migrations 有版本记录
        let v: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM plugin_migrations WHERE namespace='_hello' AND version=1",
        )
        .fetch_optional(db.pool())
        .await
        .unwrap();
        assert_eq!(v, Some(1));

        // 旧列未被迁移脚本改动（不重放证据：extra 列不存在）
        let cols: Vec<(String,)> = sqlx::query_as("SELECT name FROM pragma_table_info('hw_notes')")
            .fetch_all(db.pool())
            .await
            .unwrap();
        let names: Vec<String> = cols.into_iter().map(|c| c.0).collect();
        assert!(
            !names.contains(&"extra".to_string()),
            "基线机制应跳过存量表: {names:?}"
        );
    }

    // 必测 4：全新表正常迁移 + 重复执行幂等
    #[tokio::test]
    async fn migration_fresh_and_idempotent() {
        let db = mem_db().await;
        let orch = MigrationOrchestrator;
        let mk = || {
            MigrationSet {
            namespace: "hm_todo".into(),
            files: vec![MigrationFile {
                version: 1,
                sql: "CREATE TABLE IF NOT EXISTS hm_todos (id INTEGER PRIMARY KEY, title TEXT NOT NULL)",
            }],
        }
        };
        let outcomes = orch.apply_all(&db, vec![mk()]).await.unwrap();
        assert_eq!(outcomes[0].applied, vec![1], "新表应执行");
        // 二次执行 → 不重放
        let outcomes = orch.apply_all(&db, vec![mk()]).await.unwrap();
        assert!(outcomes[0].applied.is_empty(), "已应用版本不重放");
    }

    #[tokio::test]
    async fn file_migration_snapshots_committed_wal_before_applying() {
        let (_dir, db_path, db) = file_db().await;
        let outcomes = MigrationOrchestrator
            .apply_all(
                &db,
                vec![MigrationSet {
                    namespace: "boards.profile".into(),
                    files: vec![MigrationFile {
                        version: 1,
                        sql: "CREATE TABLE migrated_data (id INTEGER PRIMARY KEY); INSERT INTO migrated_data (id) VALUES (1)",
                    }],
                }],
            )
            .await
            .unwrap();

        assert_eq!(outcomes[0].applied, vec![1]);
        let backup_path = db_path
            .parent()
            .unwrap()
            .join("backups/pre-migrate-boards.profile-1.db");
        assert!(backup_path.is_file(), "迁移前备份应存在: {backup_path:?}");

        let backup = SqlitePool::connect(&format!("sqlite:{}", backup_path.display()))
            .await
            .expect("reopen backup database");
        let quick_check: String = sqlx::query_scalar("PRAGMA quick_check")
            .fetch_one(&backup)
            .await
            .expect("run quick_check on independently reopened backup");
        assert_eq!(quick_check, "ok", "独立重开的迁移备份必须通过 quick_check");
        let seed: String = sqlx::query_scalar("SELECT value FROM seed_data")
            .fetch_one(&backup)
            .await
            .expect("read pre-migration data from backup");
        assert_eq!(seed, "committed in wal");
        let backup_migrated_table: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='migrated_data'",
        )
        .fetch_one(&backup)
        .await
        .unwrap();
        assert_eq!(backup_migrated_table, 0, "备份必须生成于迁移 SQL 之前");

        let migrated_id: i64 = sqlx::query_scalar("SELECT id FROM migrated_data")
            .fetch_one(db.pool())
            .await
            .expect("migration SQL should affect live database");
        assert_eq!(migrated_id, 1);
        let recorded: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM plugin_migrations WHERE namespace=? AND version=?",
        )
        .bind("boards.profile")
        .bind(1_i64)
        .fetch_optional(db.pool())
        .await
        .unwrap();
        assert_eq!(recorded, Some(1));
    }

    #[tokio::test]
    async fn file_migration_refuses_to_overwrite_existing_backup() {
        let (_dir, db_path, db) = file_db().await;
        let backup_dir = db_path.parent().unwrap().join("backups");
        tokio::fs::create_dir(&backup_dir)
            .await
            .expect("create backup directory");
        let backup_path = backup_dir.join("pre-migrate-boards.profile-1.db");
        tokio::fs::write(&backup_path, b"existing backup")
            .await
            .expect("create existing backup marker");

        let result = MigrationOrchestrator
            .apply_all(
                &db,
                vec![MigrationSet {
                    namespace: "boards.profile".into(),
                    files: vec![MigrationFile {
                        version: 1,
                        sql: "INSERT INTO kernel_table_ownership (table_name, plugin_id, name_prefixed, created_at) VALUES ('resumes', 'boards.profile', 0, 'test'); CREATE TABLE migrated_data (id INTEGER PRIMARY KEY)",
                    }],
                }],
            )
            .await;
        assert!(result.is_err(), "已有目标必须中止迁移");
        assert_eq!(
            tokio::fs::read(&backup_path).await.unwrap(),
            b"existing backup",
            "已有备份不得被覆盖"
        );

        let migrated_table: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='migrated_data'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(migrated_table, 0, "已有目标拒绝后不得执行迁移 SQL");
        let recorded: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM plugin_migrations WHERE namespace=? AND version=?",
        )
        .bind("boards.profile")
        .bind(1_i64)
        .fetch_optional(db.pool())
        .await
        .unwrap();
        assert_eq!(recorded, None, "已有目标拒绝后不得登记迁移版本");
        let ownership_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_table_ownership WHERE table_name='resumes'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(ownership_rows, 0, "已有目标拒绝后不得登记表归属");
    }

    #[tokio::test]
    async fn file_migration_stops_when_backup_cannot_be_created() {
        let (dir, _db_path, db) = file_db().await;
        tokio::fs::write(dir.path().join("backups"), b"not a directory")
            .await
            .expect("create blocking backups file");

        let result = MigrationOrchestrator
            .apply_all(
                &db,
                vec![MigrationSet {
                    namespace: "boards.profile".into(),
                    files: vec![MigrationFile {
                        version: 1,
                        sql: "INSERT INTO kernel_table_ownership (table_name, plugin_id, name_prefixed, created_at) VALUES ('resumes', 'boards.profile', 0, 'test'); CREATE TABLE migrated_data (id INTEGER PRIMARY KEY)",
                    }],
                }],
            )
            .await;
        assert!(result.is_err(), "备份失败必须中止迁移");

        let migrated_table: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='migrated_data'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(migrated_table, 0, "备份失败后不得执行迁移 SQL");
        let recorded: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM plugin_migrations WHERE namespace=? AND version=?",
        )
        .bind("boards.profile")
        .bind(1_i64)
        .fetch_optional(db.pool())
        .await
        .unwrap();
        assert_eq!(recorded, None, "备份失败后不得登记迁移版本");
        let ownership_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM kernel_table_ownership WHERE table_name='resumes'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(ownership_rows, 0, "备份失败后不得登记表归属");
    }

    // 迁移事务性：脚本中途失败 → 已执行语句与版本记录一并回滚
    #[tokio::test]
    async fn file_migration_mid_script_failure_rolls_back() {
        let (_dir, _db_path, db) = file_db().await;
        let result = MigrationOrchestrator
            .apply_all(
                &db,
                vec![MigrationSet {
                    namespace: "boards.profile".into(),
                    files: vec![MigrationFile {
                        version: 1,
                        sql: "CREATE TABLE partial_table (id INTEGER PRIMARY KEY); \
                              INSERT INTO nonexistent_table (id) VALUES (1)",
                    }],
                }],
            )
            .await;
        assert!(result.is_err(), "中途失败必须报错");

        let partial_table: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='partial_table'",
        )
        .fetch_one(db.pool())
        .await
        .unwrap();
        assert_eq!(partial_table, 0, "失败前已执行的语句必须回滚");
        let recorded: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM plugin_migrations WHERE namespace=? AND version=?",
        )
        .bind("boards.profile")
        .bind(1_i64)
        .fetch_optional(db.pool())
        .await
        .unwrap();
        assert_eq!(recorded, None, "失败后不得登记迁移版本");
    }

    // 词法提取回归
    #[test]
    fn extract_tables_basic() {
        let sql = "SELECT a.* FROM hw_notes n JOIN kernel_domain_events d ON 1; \
                   INSERT INTO other_tbl VALUES (1); UPDATE tbl_x SET a=1";
        let tables = extract_table_names(sql);
        assert!(tables.contains(&"hw_notes".to_string()), "{tables:?}");
        assert!(
            tables.contains(&"kernel_domain_events".to_string()),
            "{tables:?}"
        );
        assert!(tables.contains(&"other_tbl".to_string()), "{tables:?}");
        assert!(tables.contains(&"tbl_x".to_string()), "{tables:?}");
        // 误报防护："from" 出现在标识符内不提取
        let t2 = extract_table_names("SELECT profile_info FROM x");
        assert!(!t2.contains(&"info".to_string()), "{t2:?}");
    }

    #[test]
    fn ownership_table_shape() {
        // 记录 06 §六表结构（内存库建表走 mem_db）
        setup_kernel_tables();
    }
}
