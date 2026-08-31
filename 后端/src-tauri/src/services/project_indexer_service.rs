use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::fs;

use crate::error::app_error::AppError;
use crate::models::project_index::{
    FindReferencesRequest, GetRelatedFilesRequest, IndexProjectRequest, IndexProjectResult,
    ProjectIndexDependency, ProjectIndexStatus,
    ProjectIndexSymbol, ReferenceHit, RelatedFile, SearchSymbolsRequest, SymbolSearchHit,
};

/// 项目索引器：扫描项目 → 提取符号 → 建立依赖图
///
/// 设计意图（D1.4）：为 Yuan Code v3 提供项目级上下文，
/// 让代码补全/分析能引用同项目其他文件的符号、导入关系。
///
/// 增量索引：基于 content_hash 跳过未变更文件
/// 符号提取：基于正则的轻量级解析（v3.1 将接入 LSP 增强）
pub struct ProjectIndexer;

/// 默认排除的目录/文件模式（与 yuancode_service 一致 + 索引专属扩展）
const IGNORED_DIRS: &[&str] = &[
    ".git", "node_modules", "target", "__pycache__", ".venv", "venv",
    ".idea", ".vscode", "dist", "build", ".DS_Store", ".next", ".nuxt",
    "coverage", ".cache", ".turbo", ".parcel-cache", "out",
];

/// 支持索引的文件扩展名（→ 语言映射）
const INDEXED_EXTENSIONS: &[(&str, &str)] = &[
    ("rs", "rust"), ("py", "python"), ("js", "javascript"), ("ts", "typescript"),
    ("tsx", "tsx"), ("jsx", "jsx"), ("go", "go"), ("java", "java"),
    ("c", "c"), ("cpp", "c++"), ("h", "c"), ("hpp", "c++"),
    ("kt", "kotlin"), ("swift", "swift"), ("rb", "ruby"), ("php", "php"),
    ("vue", "vue"), ("svelte", "svelte"), ("lua", "lua"), ("dart", "dart"),
];

impl ProjectIndexer {
    /// 索引一个项目（增量：跳过未变更文件）
    pub async fn index_project(
        pool: &SqlitePool,
        req: IndexProjectRequest,
    ) -> Result<IndexProjectResult, AppError> {
        let start = Instant::now();
        let root = PathBuf::from(&req.root_path);
        if !root.exists() || !root.is_dir() {
            return Err(AppError::Validation(format!(
                "项目根目录不存在或不是目录: {}",
                req.root_path
            )));
        }

        // 1. 创建或更新项目记录
        let project_id = Self::upsert_project(pool, &root).await?;
        Self::set_project_status(pool, project_id, "indexing", None).await?;

        // 2. 扫描所有支持索引的文件
        let files = Self::scan_files(&root).await?;

        // 3. 加载已有文件索引（path → (id, content_hash)），用于增量判断
        let existing_files = Self::load_existing_files(pool, project_id).await?;

        let mut files_indexed: usize = 0;
        let mut symbols_indexed: usize = 0;
        let mut imports_indexed: usize = 0;
        let mut skipped_unchanged: usize = 0;

        // 4. 逐个文件索引（保持顺序，避免大量并发占用资源）
        for file_rel_path in &files {
            let abs_path = root.join(file_rel_path);
            let content = match fs::read_to_string(&abs_path).await {
                Ok(c) => c,
                Err(_) => continue, // 二进制文件或读取失败，跳过
            };

            let content_hash = compute_hash(&content);
            let mtime = file_mtime(&abs_path).ok();
            let language = detect_language(file_rel_path);
            let line_count = content.lines().count() as i64;

            // 增量：hash 一致则跳过
            if !req.force_full {
                if let Some((_id, Some(prev_hash))) = existing_files.get(file_rel_path.as_str()) {
                    if *prev_hash == content_hash {
                        skipped_unchanged += 1;
                        continue;
                    }
                }
            }

            // upsert 文件记录
            let file_id = Self::upsert_file(
                pool,
                project_id,
                file_rel_path,
                &abs_path.to_string_lossy(),
                language.as_deref(),
                content.len() as i64,
                mtime,
                &content_hash,
                line_count,
            )
            .await?;

            // 删除该文件旧符号 + 旧导入
            Self::delete_file_symbols_and_imports(pool, file_id).await?;

            // 提取符号 + 导入
            if let Some(lang) = &language {
                let (symbols, imports) = extract_symbols_and_imports(&content, lang);
                for sym in symbols {
                    Self::insert_symbol(pool, file_id, &sym).await?;
                    symbols_indexed += 1;
                }
                for imp in imports {
                    Self::insert_import(pool, file_id, &imp).await?;
                    imports_indexed += 1;
                }
            }

            files_indexed += 1;
        }

        // 5. 构建依赖图（清空旧依赖，基于当前 imports 重建）
        Self::rebuild_dependencies(pool, project_id).await?;

        // 6. 更新项目统计 + 状态
        let now = chrono::Utc::now().timestamp();
        let total_files = files_indexed as i64 + skipped_unchanged as i64;
        sqlx::query(
            "UPDATE project_index_projects SET indexed_at = ?, file_count = ?, symbol_count = ?, status = 'idle', last_error = NULL WHERE id = ?",
        )
        .bind(now)
        .bind(total_files)
        .bind(symbols_indexed as i64)
        .bind(project_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("更新项目统计失败: {}", e)))?;

        Ok(IndexProjectResult {
            project_id,
            files_indexed,
            symbols_indexed,
            imports_indexed,
            duration_ms: start.elapsed().as_millis() as u64,
            skipped_unchanged,
        })
    }

    /// 获取项目索引状态
    pub async fn get_status(
        pool: &SqlitePool,
        root_path: &str,
    ) -> Result<Option<ProjectIndexStatus>, AppError> {
        let row = sqlx::query_as::<_, (i64, String, Option<String>, String, i64, i64, Option<i64>, Option<String>)>(
            "SELECT id, root_path, name, status, file_count, symbol_count, indexed_at, last_error \
             FROM project_index_projects WHERE root_path = ?",
        )
        .bind(root_path)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(format!("查询项目状态失败: {}", e)))?;

        Ok(row.map(|(id, root, name, status, fc, sc, ia, le)| ProjectIndexStatus {
            project_id: id,
            root_path: root,
            name,
            status,
            file_count: fc,
            symbol_count: sc,
            indexed_at: ia,
            last_error: le,
        }))
    }

    /// 搜索符号（基于 LIKE 模糊匹配，v3.1 接入 FTS）
    pub async fn search_symbols(
        pool: &SqlitePool,
        req: SearchSymbolsRequest,
    ) -> Result<Vec<SymbolSearchHit>, AppError> {
        let limit = req.limit.unwrap_or(50) as i64;
        let pattern = format!("%{}%", req.query);

        let rows = if let Some(kind) = &req.kind {
            if req.exported_only {
                sqlx::query_as::<_, ProjectIndexSymbol>(
                    "SELECT id, file_id, name, kind, line_start, line_end, column_start, column_end, \
                     signature, documentation, is_exported \
                     FROM project_index_symbols \
                     WHERE file_id IN (SELECT id FROM project_index_files WHERE project_id = ?) \
                     AND name LIKE ? AND kind = ? AND is_exported = 1 \
                     ORDER BY name LIMIT ?",
                )
                .bind(req.project_id)
                .bind(&pattern)
                .bind(kind)
                .bind(limit)
                .fetch_all(pool)
                .await
            } else {
                sqlx::query_as::<_, ProjectIndexSymbol>(
                    "SELECT id, file_id, name, kind, line_start, line_end, column_start, column_end, \
                     signature, documentation, is_exported \
                     FROM project_index_symbols \
                     WHERE file_id IN (SELECT id FROM project_index_files WHERE project_id = ?) \
                     AND name LIKE ? AND kind = ? \
                     ORDER BY name LIMIT ?",
                )
                .bind(req.project_id)
                .bind(&pattern)
                .bind(kind)
                .bind(limit)
                .fetch_all(pool)
                .await
            }
        } else if req.exported_only {
            sqlx::query_as::<_, ProjectIndexSymbol>(
                "SELECT id, file_id, name, kind, line_start, line_end, column_start, column_end, \
                 signature, documentation, is_exported \
                 FROM project_index_symbols \
                 WHERE file_id IN (SELECT id FROM project_index_files WHERE project_id = ?) \
                 AND name LIKE ? AND is_exported = 1 \
                 ORDER BY name LIMIT ?",
            )
            .bind(req.project_id)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as::<_, ProjectIndexSymbol>(
                "SELECT id, file_id, name, kind, line_start, line_end, column_start, column_end, \
                 signature, documentation, is_exported \
                 FROM project_index_symbols \
                 WHERE file_id IN (SELECT id FROM project_index_files WHERE project_id = ?) \
                 AND name LIKE ? \
                 ORDER BY name LIMIT ?",
            )
            .bind(req.project_id)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
        .map_err(|e| AppError::Internal(format!("搜索符号失败: {}", e)))?;

        // 加载 file_path + language
        let mut hits = Vec::with_capacity(rows.len());
        for sym in rows {
            let file_meta: Option<(String, Option<String>)> = sqlx::query_as::<_, (String, Option<String>)>(
                "SELECT path, language FROM project_index_files WHERE id = ?",
            )
            .bind(sym.file_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| AppError::Internal(format!("查询文件元数据失败: {}", e)))?;

            if let Some((file_path, language)) = file_meta {
                hits.push(SymbolSearchHit {
                    symbol: sym,
                    file_path,
                    language,
                });
            }
        }
        Ok(hits)
    }

    /// 查找符号引用（哪些文件导入/使用了该符号）
    pub async fn find_references(
        pool: &SqlitePool,
        req: FindReferencesRequest,
    ) -> Result<Vec<ReferenceHit>, AppError> {
        let pattern = format!("%{}%", req.symbol_name);
        let rows = sqlx::query_as::<_, (i64, String, Option<String>, i64, String)>(
            "SELECT i.id, f.path, i.imported_symbol, i.line_number, i.import_kind \
             FROM project_index_imports i \
             JOIN project_index_files f ON i.file_id = f.id \
             WHERE f.project_id = ? AND (i.imported_symbol LIKE ? OR i.imported_path LIKE ?) \
             ORDER BY i.line_number LIMIT 200",
        )
        .bind(req.project_id)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("查找引用失败: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|(_id, file_path, imported_symbol, line, import_kind)| ReferenceHit {
                file_path,
                line,
                imported_symbol,
                import_kind,
            })
            .collect())
    }

    /// 获取相关文件（依赖图：import / imported_by / call / called_by）
    pub async fn get_related_files(
        pool: &SqlitePool,
        req: GetRelatedFilesRequest,
    ) -> Result<Vec<RelatedFile>, AppError> {
        let limit = req.limit.unwrap_or(20) as i64;

        // 当前文件 import 的文件
        let outgoing: Vec<ProjectIndexDependency> = sqlx::query_as::<_, ProjectIndexDependency>(
            "SELECT id, project_id, from_file_id, to_file_id, from_path, to_path, dependency_type, strength \
             FROM project_index_dependencies \
             WHERE project_id = ? AND from_path = ? \
             ORDER BY strength DESC LIMIT ?",
        )
        .bind(req.project_id)
        .bind(&req.file_path)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("查询出向依赖失败: {}", e)))?;

        // 引用当前文件的文件（incoming）
        let incoming: Vec<ProjectIndexDependency> = sqlx::query_as::<_, ProjectIndexDependency>(
            "SELECT id, project_id, from_file_id, to_file_id, from_path, to_path, dependency_type, strength \
             FROM project_index_dependencies \
             WHERE project_id = ? AND to_path = ? \
             ORDER BY strength DESC LIMIT ?",
        )
        .bind(req.project_id)
        .bind(&req.file_path)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("查询入向依赖失败: {}", e)))?;

        let mut results = Vec::new();
        for dep in outgoing {
            results.push(RelatedFile {
                path: dep.to_path,
                relation: "import".to_string(),
                dependency_type: dep.dependency_type,
                strength: dep.strength,
            });
        }
        for dep in incoming {
            results.push(RelatedFile {
                path: dep.from_path,
                relation: "imported_by".to_string(),
                dependency_type: dep.dependency_type,
                strength: dep.strength,
            });
        }
        Ok(results)
    }

    // ============ 内部辅助函数 ============

    async fn upsert_project(pool: &SqlitePool, root: &Path) -> Result<i64, AppError> {
        let root_str = root.to_string_lossy().to_string();
        let name = root
            .file_name()
            .and_then(|n| n.to_str())
            .map(String::from);
        let now = chrono::Utc::now().timestamp();

        // 先尝试更新
        let existing: Option<(i64,)> = sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM project_index_projects WHERE root_path = ?",
        )
        .bind(&root_str)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(format!("查询项目失败: {}", e)))?;

        if let Some((id,)) = existing {
            sqlx::query(
                "UPDATE project_index_projects SET name = COALESCE(?, name), status = 'indexing' WHERE id = ?",
            )
            .bind(&name)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("更新项目失败: {}", e)))?;
            return Ok(id);
        }

        // 新建
        let result = sqlx::query(
            "INSERT INTO project_index_projects (root_path, name, status, file_count, symbol_count) \
             VALUES (?, ?, 'indexing', 0, 0)",
        )
        .bind(&root_str)
        .bind(&name)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("创建项目失败: {}", e)))?;

        let _ = now; // 占位，避免 unused 警告
        Ok(result.last_insert_rowid())
    }

    async fn set_project_status(
        pool: &SqlitePool,
        project_id: i64,
        status: &str,
        error: Option<String>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE project_index_projects SET status = ?, last_error = ? WHERE id = ?",
        )
        .bind(status)
        .bind(error)
        .bind(project_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("更新项目状态失败: {}", e)))?;
        Ok(())
    }

    async fn scan_files(root: &Path) -> Result<Vec<String>, AppError> {
        let mut results = Vec::new();
        Self::scan_dir(root, root, &mut results).await?;
        Ok(results)
    }

    async fn scan_dir(
        root: &Path,
        current: &Path,
        results: &mut Vec<String>,
    ) -> Result<(), AppError> {
        let mut entries = fs::read_dir(current).await.map_err(|e| {
            AppError::Internal(format!("读取目录失败: {}: {}", current.display(), e))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| AppError::Internal(format!("读取目录项失败: {}", e)))?
        {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            if IGNORED_DIRS.contains(&name.as_str()) {
                continue;
            }

            if path.is_dir() {
                Box::pin(Self::scan_dir(root, &path, results)).await?;
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if INDEXED_EXTENSIONS.iter().any(|(e, _)| *e == ext) {
                        if let Ok(rel) = path.strip_prefix(root) {
                            results.push(rel.to_string_lossy().replace('\\', "/"));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn load_existing_files(
        pool: &SqlitePool,
        project_id: i64,
    ) -> Result<HashMap<String, (i64, Option<String>)>, AppError> {
        let rows: Vec<(i64, String, Option<String>)> = sqlx::query_as::<_, (i64, String, Option<String>)>(
            "SELECT id, path, content_hash FROM project_index_files WHERE project_id = ?",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("加载已有文件失败: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|(id, path, hash)| (path, (id, hash)))
            .collect())
    }

    async fn upsert_file(
        pool: &SqlitePool,
        project_id: i64,
        rel_path: &str,
        abs_path: &str,
        language: Option<&str>,
        size_bytes: i64,
        last_modified: Option<i64>,
        content_hash: &str,
        line_count: i64,
    ) -> Result<i64, AppError> {
        let now = chrono::Utc::now().timestamp();

        // 检查是否存在
        let existing: Option<(i64,)> = sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM project_index_files WHERE project_id = ? AND path = ?",
        )
        .bind(project_id)
        .bind(rel_path)
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Internal(format!("查询文件失败: {}", e)))?;

        if let Some((id,)) = existing {
            sqlx::query(
                "UPDATE project_index_files SET absolute_path = ?, language = ?, size_bytes = ?, \
                 last_modified = ?, content_hash = ?, line_count = ?, indexed_at = ? WHERE id = ?",
            )
            .bind(abs_path)
            .bind(language)
            .bind(size_bytes)
            .bind(last_modified)
            .bind(content_hash)
            .bind(line_count)
            .bind(now)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("更新文件失败: {}", e)))?;
            Ok(id)
        } else {
            let result = sqlx::query(
                "INSERT INTO project_index_files \
                 (project_id, path, absolute_path, language, size_bytes, last_modified, content_hash, line_count, indexed_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(project_id)
            .bind(rel_path)
            .bind(abs_path)
            .bind(language)
            .bind(size_bytes)
            .bind(last_modified)
            .bind(content_hash)
            .bind(line_count)
            .bind(now)
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("插入文件失败: {}", e)))?;
            Ok(result.last_insert_rowid())
        }
    }

    async fn delete_file_symbols_and_imports(
        pool: &SqlitePool,
        file_id: i64,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM project_index_symbols WHERE file_id = ?")
            .bind(file_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("删除旧符号失败: {}", e)))?;
        sqlx::query("DELETE FROM project_index_imports WHERE file_id = ?")
            .bind(file_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("删除旧导入失败: {}", e)))?;
        Ok(())
    }

    async fn insert_symbol(
        pool: &SqlitePool,
        file_id: i64,
        sym: &ExtractedSymbol,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO project_index_symbols \
             (file_id, name, kind, line_start, line_end, column_start, column_end, signature, documentation, is_exported) \
             VALUES (?, ?, ?, ?, ?, 0, 0, NULL, NULL, ?)",
        )
        .bind(file_id)
        .bind(&sym.name)
        .bind(&sym.kind)
        .bind(sym.line_start)
        .bind(sym.line_end)
        .bind(sym.is_exported as i64)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("插入符号失败: {}", e)))?;
        Ok(())
    }

    async fn insert_import(
        pool: &SqlitePool,
        file_id: i64,
        imp: &ExtractedImport,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO project_index_imports \
             (file_id, imported_path, imported_symbol, line_number, import_kind) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(file_id)
        .bind(&imp.imported_path)
        .bind(&imp.imported_symbol)
        .bind(imp.line_number)
        .bind(&imp.import_kind)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("插入导入失败: {}", e)))?;
        Ok(())
    }

    async fn rebuild_dependencies(
        pool: &SqlitePool,
        project_id: i64,
    ) -> Result<(), AppError> {
        // 清空旧依赖
        sqlx::query("DELETE FROM project_index_dependencies WHERE project_id = ?")
            .bind(project_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("清空依赖失败: {}", e)))?;

        // 加载项目内所有文件：path → id
        let files: Vec<(i64, String)> = sqlx::query_as::<_, (i64, String)>(
            "SELECT id, path FROM project_index_files WHERE project_id = ?",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("加载项目文件失败: {}", e)))?;
        let path_to_id: HashMap<String, i64> =
            files.iter().map(|(id, path)| (path.clone(), *id)).collect();

        // 加载所有导入
        let imports: Vec<(i64, String, String, i64, String)> = sqlx::query_as::<_, (i64, String, String, i64, String)>(
            "SELECT i.id, f.path, i.imported_path, i.line_number, i.import_kind \
             FROM project_index_imports i \
             JOIN project_index_files f ON i.file_id = f.id \
             WHERE f.project_id = ?",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Internal(format!("加载导入失败: {}", e)))?;

        for (_id, from_path, imported_path, _line, import_kind) in imports {
            // 解析 imported_path 到项目内文件
            let to_file_id = Self::resolve_import(&from_path, &imported_path, &path_to_id);
            let to_path_str = to_file_id
                .as_ref()
                .and_then(|tid| path_to_id.iter().find_map(|(p, id)| if id == tid { Some(p.clone()) } else { None }))
                .unwrap_or_else(|| imported_path.clone());

            sqlx::query(
                "INSERT OR IGNORE INTO project_index_dependencies \
                 (project_id, from_file_id, to_file_id, from_path, to_path, dependency_type, strength) \
                 VALUES (?, ?, ?, ?, ?, ?, 1.0)",
            )
            .bind(project_id)
            .bind(path_to_id.get(&from_path).copied().unwrap_or(0))
            .bind(to_file_id)
            .bind(&from_path)
            .bind(&to_path_str)
            .bind(&import_kind)
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(format!("插入依赖失败: {}", e)))?;
        }

        Ok(())
    }

    /// 解析 import 路径到项目内文件 ID
    /// 处理：相对路径（./foo, ../bar）、模块名（含 / 时尝试匹配）
    fn resolve_import(
        from_path: &str,
        imported_path: &str,
        path_to_id: &HashMap<String, i64>,
    ) -> Option<i64> {
        // 直接匹配
        if let Some(id) = path_to_id.get(imported_path) {
            return Some(*id);
        }

        // 相对路径解析（from_path 所在目录 + imported_path）
        if imported_path.starts_with("./") || imported_path.starts_with("../") {
            let from_dir = Path::new(from_path).parent()?.to_string_lossy().replace('\\', "/");
            let combined = if from_dir.is_empty() {
                imported_path.trim_start_matches("./").to_string()
            } else {
                format!("{}/{}", from_dir, imported_path.trim_start_matches("./"))
            };
            // 尝试多种扩展名
            for ext in &["", ".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go"] {
                let candidate = format!("{}{}", combined, ext);
                if let Some(id) = path_to_id.get(&candidate) {
                    return Some(*id);
                }
            }
            // index 文件
            for index_name in &["index.ts", "index.tsx", "index.js", "mod.rs", "__init__.py"] {
                let candidate = format!("{}/{}", combined, index_name);
                if let Some(id) = path_to_id.get(&candidate) {
                    return Some(*id);
                }
            }
        }

        None
    }
}

// ============ 辅助类型 + 函数 ============

#[derive(Debug, Clone)]
struct ExtractedSymbol {
    name: String,
    kind: String,           // 'function' | 'class' | 'type' | 'interface' | 'variable' | 'constant' | 'enum' | 'module'
    line_start: i64,
    line_end: i64,
    is_exported: bool,
}

#[derive(Debug, Clone)]
struct ExtractedImport {
    imported_path: String,
    imported_symbol: Option<String>,
    line_number: i64,
    import_kind: String,
}

/// 基于正则的轻量级符号 + 导入提取（v3.0 实现）
/// v3.1 将接入 LSP 增强解析
fn extract_symbols_and_imports(content: &str, language: &str) -> (Vec<ExtractedSymbol>, Vec<ExtractedImport>) {
    let mut symbols = Vec::new();
    let mut imports = Vec::new();

    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let line_no = (i + 1) as i64;
        let trimmed = line.trim();

        // 导入解析
        if let Some(imp) = parse_import_line(trimmed, language, line_no) {
            imports.push(imp);
            continue;
        }

        // 符号解析
        if let Some(sym) = parse_symbol_line(trimmed, language, line_no) {
            symbols.push(sym);
        }
    }

    (symbols, imports)
}

fn parse_import_line(line: &str, language: &str, line_no: i64) -> Option<ExtractedImport> {
    match language {
        "rust" => {
            // use std::collections::HashMap;
            // use crate::foo::{Bar, Baz};
            if let Some(rest) = line.strip_prefix("use ") {
                let stmt = rest.trim_end_matches(';').trim();
                let (path_part, symbol_part) = if let Some(open) = stmt.find('{') {
                    let path = &stmt[..open].trim_end_matches("::");
                    let symbols_str = stmt[open..].trim_matches(|c| c == '{' || c == '}' || c == ' ');
                    (path.to_string(), Some(symbols_str.to_string()))
                } else {
                    (stmt.to_string(), None)
                };
                return Some(ExtractedImport {
                    imported_path: path_part,
                    imported_symbol: symbol_part,
                    line_number: line_no,
                    import_kind: "use".to_string(),
                });
            }
        }
        "typescript" | "tsx" | "javascript" | "jsx" | "vue" | "svelte" => {
            // import { foo } from './bar';
            // import foo from './bar';
            // import * as bar from 'module';
            if line.starts_with("import ") {
                if let Some(from_pos) = line.find("from ") {
                    let path_part = line[from_pos + 5..].trim().trim_matches(|c| c == '\'' || c == '"' || c == ';').to_string();
                    let symbol_part = line[7..from_pos].trim().trim_matches(|c| c == '{' || c == '}' || c == ' ').to_string();
                    let symbol = if symbol_part.is_empty() { None } else { Some(symbol_part) };
                    return Some(ExtractedImport {
                        imported_path: path_part,
                        imported_symbol: symbol,
                        line_number: line_no,
                        import_kind: "import".to_string(),
                    });
                }
                // import './polyfills';
                let path_part = line[7..].trim().trim_matches(|c| c == '\'' || c == '"' || c == ';').to_string();
                if !path_part.is_empty() {
                    return Some(ExtractedImport {
                        imported_path: path_part,
                        imported_symbol: None,
                        line_number: line_no,
                        import_kind: "import".to_string(),
                    });
                }
            }
            // const x = require('./foo');
            if line.contains("require(") {
                if let Some(start) = line.find("require(\"") {
                    if let Some(end) = line[start + 9..].find("\")") {
                        let path_part = line[start + 9..start + 9 + end].to_string();
                        return Some(ExtractedImport {
                            imported_path: path_part,
                            imported_symbol: None,
                            line_number: line_no,
                            import_kind: "require".to_string(),
                        });
                    }
                }
            }
        }
        "python" => {
            // import os.path
            // from collections import OrderedDict
            if line.starts_with("import ") {
                let path_part = line[7..].trim().trim_end_matches(';').to_string();
                return Some(ExtractedImport {
                    imported_path: path_part,
                    imported_symbol: None,
                    line_number: line_no,
                    import_kind: "import".to_string(),
                });
            }
            if line.starts_with("from ") {
                let rest = line[5..].trim();
                if let Some(imp_pos) = rest.find(" import ") {
                    let path_part = rest[..imp_pos].trim().to_string();
                    let symbol_part = rest[imp_pos + 8..].trim().trim_end_matches(';').to_string();
                    return Some(ExtractedImport {
                        imported_path: path_part,
                        imported_symbol: Some(symbol_part),
                        line_number: line_no,
                        import_kind: "import".to_string(),
                    });
                }
            }
        }
        "go" => {
            // import "fmt"
            // import "path/to/pkg"
            if line.starts_with("import ") {
                let path_part = line[7..].trim().trim_matches(|c| c == '"' || c == ';').to_string();
                return Some(ExtractedImport {
                    imported_path: path_part,
                    imported_symbol: None,
                    line_number: line_no,
                    import_kind: "import".to_string(),
                });
            }
        }
        _ => {}
    }
    None
}

fn parse_symbol_line(line: &str, language: &str, line_no: i64) -> Option<ExtractedSymbol> {
    match language {
        "rust" => {
            // pub fn name(...]
            // fn name(...]
            // pub struct Name
            // struct Name
            // pub enum Name
            // enum Name
            // pub trait Name
            // trait Name
            // pub type Name
            // type Name
            // const NAME
            // static NAME
            let is_pub = line.starts_with("pub ");
            let after_pub = if is_pub { &line[4..] } else { line };

            if let Some(rest) = after_pub.strip_prefix("fn ") {
                let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next()?.to_string();
                if name.is_empty() { return None; }
                return Some(ExtractedSymbol {
                    name,
                    kind: "function".to_string(),
                    line_start: line_no,
                    line_end: line_no,
                    is_exported: is_pub,
                });
            }
            for keyword in &["struct ", "enum ", "trait ", "type "] {
                if let Some(rest) = after_pub.strip_prefix(keyword) {
                    let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next()?.to_string();
                    if name.is_empty() { return None; }
                    return Some(ExtractedSymbol {
                        name,
                        kind: keyword.trim().to_string(),
                        line_start: line_no,
                        line_end: line_no,
                        is_exported: is_pub,
                    });
                }
            }
            for keyword in &["const ", "static "] {
                if let Some(rest) = after_pub.strip_prefix(keyword) {
                    let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next()?.to_string();
                    if name.is_empty() { return None; }
                    return Some(ExtractedSymbol {
                        name,
                        kind: "constant".to_string(),
                        line_start: line_no,
                        line_end: line_no,
                        is_exported: is_pub,
                    });
                }
            }
        }
        "typescript" | "tsx" | "javascript" | "jsx" => {
            // function name(
            // const name = ...
            // class Name
            // interface Name
            // type Name =
            // export ...
            let is_exported = line.starts_with("export ");
            let after_export = if is_exported { &line[7..] } else { line };

            if let Some(rest) = after_export.strip_prefix("function ") {
                let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '$').next()?.to_string();
                if name.is_empty() { return None; }
                return Some(ExtractedSymbol {
                    name,
                    kind: "function".to_string(),
                    line_start: line_no,
                    line_end: line_no,
                    is_exported,
                });
            }
            if let Some(rest) = after_export.strip_prefix("class ") {
                let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next()?.to_string();
                if name.is_empty() { return None; }
                return Some(ExtractedSymbol {
                    name,
                    kind: "class".to_string(),
                    line_start: line_no,
                    line_end: line_no,
                    is_exported,
                });
            }
            if let Some(rest) = after_export.strip_prefix("interface ") {
                let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next()?.to_string();
                if name.is_empty() { return None; }
                return Some(ExtractedSymbol {
                    name,
                    kind: "interface".to_string(),
                    line_start: line_no,
                    line_end: line_no,
                    is_exported,
                });
            }
            if let Some(rest) = after_export.strip_prefix("type ") {
                let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next()?.to_string();
                if name.is_empty() { return None; }
                return Some(ExtractedSymbol {
                    name,
                    kind: "type".to_string(),
                    line_start: line_no,
                    line_end: line_no,
                    is_exported,
                });
            }
            // const Name = ...
            if let Some(rest) = after_export.strip_prefix("const ") {
                let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '$').next()?.to_string();
                if name.is_empty() { return None; }
                // 大写开头视为常量，小写视为变量
                let kind = if name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    "constant"
                } else {
                    "variable"
                };
                return Some(ExtractedSymbol {
                    name,
                    kind: kind.to_string(),
                    line_start: line_no,
                    line_end: line_no,
                    is_exported,
                });
            }
        }
        "python" => {
            // def name(...
            // async def name(...
            // class Name
            let (after_async, is_async) = if let Some(rest) = line.strip_prefix("async ") {
                (rest, true)
            } else {
                (line, false)
            };
            let _ = is_async;
            if let Some(rest) = after_async.strip_prefix("def ") {
                let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next()?.to_string();
                if name.is_empty() { return None; }
                let is_exported = !name.starts_with('_');
                return Some(ExtractedSymbol {
                    name,
                    kind: "function".to_string(),
                    line_start: line_no,
                    line_end: line_no,
                    is_exported,
                });
            }
            if let Some(rest) = after_async.strip_prefix("class ") {
                let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next()?.to_string();
                if name.is_empty() { return None; }
                return Some(ExtractedSymbol {
                    name,
                    kind: "class".to_string(),
                    line_start: line_no,
                    line_end: line_no,
                    is_exported: true,
                });
            }
        }
        "go" => {
            // func Name(
            // func (r *Receiver) Name(
            // type Name struct / interface
            if let Some(rest) = line.strip_prefix("func ") {
                // 处理 receiver 形式
                let after_receiver = if rest.starts_with('(') {
                    if let Some(close) = rest.find(") ") {
                        &rest[close + 2..]
                    } else {
                        rest
                    }
                } else {
                    rest
                };
                let name = after_receiver.split(|c: char| !c.is_alphanumeric() && c != '_').next()?.to_string();
                if name.is_empty() { return None; }
                let is_exported = !name.chars().next().map(|c| c.is_lowercase()).unwrap_or(true);
                return Some(ExtractedSymbol {
                    name,
                    kind: "function".to_string(),
                    line_start: line_no,
                    line_end: line_no,
                    is_exported,
                });
            }
            if let Some(rest) = line.strip_prefix("type ") {
                let name = rest.split(|c: char| !c.is_alphanumeric() && c != '_').next()?.to_string();
                if name.is_empty() { return None; }
                return Some(ExtractedSymbol {
                    name,
                    kind: "type".to_string(),
                    line_start: line_no,
                    line_end: line_no,
                    is_exported: true,
                });
            }
        }
        _ => {}
    }
    None
}

fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn file_mtime(path: &Path) -> Result<i64, AppError> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| AppError::Internal(format!("读取文件元数据失败: {}: {}", path.display(), e)))?;
    let modified = metadata.modified()
        .map_err(|e| AppError::Internal(format!("读取 mtime 失败: {}", e)))?;
    let duration = modified.duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| AppError::Internal(format!("时间转换失败: {}", e)))?;
    Ok(duration.as_secs() as i64)
}

fn detect_language(path: &str) -> Option<String> {
    let path_lower = path.to_lowercase();
    let ext = path_lower.rsplit('.').next()?;
    INDEXED_EXTENSIONS.iter().find(|(e, _)| *e == ext).map(|(_, l)| l.to_string())
}
