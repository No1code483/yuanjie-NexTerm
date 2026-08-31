use sqlx::SqlitePool;

use crate::db::repositories::kb_repo;
use crate::db::repositories::recycle_repo;
use crate::error::app_error::AppError;
use crate::models::knowledge::{KbCategory, KbEntry, KbTag, KbTrackedPath, KbSnapshotResult, EntryTagsResult, ScanDirFileInfo, PathCheckResult, TagStats, BacklinkResult};
use crate::services::intelligence_v4_service;
use calamine::Reader;

fn get_mime_type(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "tiff" | "tif" => "image/tiff",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "ogg" => "audio/ogg",
        "opus" => "audio/opus",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

pub async fn get_categories(pool: &SqlitePool, user_id: i64, library: Option<&str>) -> Result<Vec<KbCategory>, AppError> {
    kb_repo::get_categories(pool, user_id, library).await
}

pub async fn get_category_by_id(pool: &SqlitePool, user_id: i64, id: i64) -> Result<KbCategory, AppError> {
    kb_repo::get_category_by_id(pool, user_id, id).await
}

pub async fn add_category(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    parent_id: Option<i64>,
    library: &str,
    sort_order: i32,
) -> Result<KbCategory, AppError> {
    if library != "study" && library != "material" {
        return Err(AppError::Validation(format!("library 必须为 'study' 或 'material'，当前值: '{}'", library)));
    }
    let now = chrono::Utc::now().timestamp_millis();
    kb_repo::add_category(pool, user_id, name, parent_id, library, sort_order, now).await
}

pub async fn delete_category(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    kb_repo::delete_category(pool, user_id, id).await
}

pub async fn recursive_delete_category(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
) -> Result<(i64, i64), AppError> {
    kb_repo::recursive_delete_category(pool, user_id, id).await
}

pub async fn move_category_tree_to_recycle(
    pool: &SqlitePool,
    user_id: i64,
    root_id: i64,
    deleted_by: Option<i64>,
) -> Result<(i64, i64), AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let auto_delete_at = now + 10 * 24 * 3600 * 1000;

    let mut tx = pool.begin().await.map_err(AppError::Database)?;

    let mut all_cat_ids = vec![root_id];
    let mut i = 0;
    while i < all_cat_ids.len() {
        let children: Vec<(i64,)> = sqlx::query_as(
            "SELECT id FROM kb_categories WHERE parent_id = ? AND user_id = ?"
        )
        .bind(all_cat_ids[i])
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(AppError::Database)?;
        for (child_id,) in children {
            all_cat_ids.push(child_id);
        }
        i += 1;
    }

    let total_entries;

    // Phase 3 §2.2.4 批量化：原实现按 cat_id 循环逐个 SELECT + DELETE kb_entries，
    // 在分类树较大时产生 N+1 查询。改为单次批量 SELECT（带 category_id 列）+ 单次 DELETE IN (...)。
    // 多用户隔离批次 3：所有 SQL 添加 user_id 过滤
    {
        #[derive(sqlx::FromRow)]
        struct EntryRow {
            category_id: i64,
            id: i64,
            name: String,
            path_url: String,
            entry_type: String,
            created_at: i64,
            updated_at: i64,
        }

        // 构造 IN (?, ?, ...) 占位符
        let placeholders = std::iter::repeat("?")
            .take(all_cat_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let select_sql = format!(
            "SELECT category_id, id, name, path_url, entry_type, created_at, updated_at \
             FROM kb_entries WHERE user_id = ? AND category_id IN ({})",
            placeholders
        );
        let mut q = sqlx::query_as::<_, EntryRow>(&select_sql);
        q = q.bind(user_id);
        for &cid in &all_cat_ids {
            q = q.bind(cid);
        }
        let entries: Vec<EntryRow> = q
            .fetch_all(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        for e in &entries {
            let metadata = serde_json::json!({
                "category_id": e.category_id,
                "name": e.name,
                "path_url": e.path_url,
                "entry_type": e.entry_type,
                "created_at": e.created_at,
                "updated_at": e.updated_at,
                "user_id": user_id,
            });
            recycle_repo::add_item(
                &mut *tx,
                user_id,
                &format!("kb_entry/{}", e.id),
                "kb_entry",
                Some(e.id),
                Some(&e.name),
                Some(&metadata.to_string()),
                None,
                deleted_by,
                now,
                auto_delete_at,
            )
            .await?;
        }
        total_entries = entries.len() as i64;

        // 批量 DELETE（单条 SQL）
        let delete_sql = format!(
            "DELETE FROM kb_entries WHERE user_id = ? AND category_id IN ({})",
            placeholders
        );
        let mut dq = sqlx::query(&delete_sql);
        dq = dq.bind(user_id);
        for &cid in &all_cat_ids {
            dq = dq.bind(cid);
        }
        dq.execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
    }

    // Phase 3 §2.2.4 批量化：原实现按 cat_id 循环逐个 SELECT + DELETE kb_categories，
    // 改为单次批量 SELECT（带 id 列）+ 单次 DELETE IN (...)。
    // 多用户隔离批次 3：所有 SQL 添加 user_id 过滤
    {
        #[derive(sqlx::FromRow)]
        struct CatRow {
            id: i64,
            name: String,
            parent_id: Option<i64>,
            sort_order: i32,
            created_at: i64,
        }
        let placeholders = std::iter::repeat("?")
            .take(all_cat_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let select_sql = format!(
            "SELECT id, name, parent_id, sort_order, created_at FROM kb_categories WHERE user_id = ? AND id IN ({})",
            placeholders
        );
        let mut q = sqlx::query_as::<_, CatRow>(&select_sql);
        q = q.bind(user_id);
        for &cid in &all_cat_ids {
            q = q.bind(cid);
        }
        let cats: Vec<CatRow> = q
            .fetch_all(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        // 按原逻辑逆序处理（all_cat_ids.iter().rev()），保留删除顺序语义
        let cat_map: std::collections::HashMap<i64, CatRow> =
            cats.into_iter().map(|c| (c.id, c)).collect();
        for &cat_id in all_cat_ids.iter().rev() {
            let cat = cat_map.get(&cat_id).ok_or_else(|| AppError::NotFound)?;

            let metadata = serde_json::json!({
                "name": cat.name,
                "parent_id": cat.parent_id,
                "sort_order": cat.sort_order,
                "created_at": cat.created_at,
                "user_id": user_id,
            });

            recycle_repo::add_item(
                &mut *tx,
                user_id,
                &format!("kb_category/{}", cat_id),
                "kb_category",
                Some(cat_id),
                Some(&cat.name),
                Some(&metadata.to_string()),
                None,
                deleted_by,
                now,
                auto_delete_at,
            )
            .await?;
        }

        // 批量 DELETE（单条 SQL）
        let delete_sql = format!(
            "DELETE FROM kb_categories WHERE user_id = ? AND id IN ({})",
            placeholders
        );
        let mut dq = sqlx::query(&delete_sql);
        dq = dq.bind(user_id);
        for &cid in &all_cat_ids {
            dq = dq.bind(cid);
        }
        dq.execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
    }

    tx.commit().await.map_err(AppError::Database)?;

    tracing::info!(
        root_id = root_id,
        subfolders = all_cat_ids.len() as i64 - 1,
        entries = total_entries,
        "分类树已移入回收站"
    );

    Ok((all_cat_ids.len() as i64 - 1, total_entries))
}

pub async fn update_category(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    name: &str,
) -> Result<KbCategory, AppError> {
    kb_repo::update_category(pool, user_id, id, name).await
}

pub async fn get_entries(pool: &SqlitePool, user_id: i64, category_id: i64) -> Result<Vec<KbEntry>, AppError> {
    kb_repo::get_entries_by_category(pool, user_id, category_id).await
}

/// 为条目列表补充 file_exists 字段：检测 source_path 对应的本地文件是否存在
fn mark_file_existence(entries: &mut Vec<KbEntry>) {
    for entry in entries.iter_mut() {
        if let Some(ref path) = entry.source_path {
            if !path.is_empty() {
                entry.file_exists = std::path::Path::new(path).exists();
            }
        }
    }
}

pub async fn get_all_entries(pool: &SqlitePool, user_id: i64) -> Result<Vec<KbEntry>, AppError> {
    let mut entries = kb_repo::get_all_entries(pool, user_id).await?;
    mark_file_existence(&mut entries);
    Ok(entries)
}

pub async fn get_category_entry_counts(pool: &SqlitePool, user_id: i64) -> Result<Vec<(i64, i64)>, AppError> {
    kb_repo::get_entries_count_by_category(pool, user_id).await
}

pub async fn get_child_category_ids(pool: &SqlitePool, user_id: i64, parent_id: i64) -> Result<Vec<i64>, AppError> {
    kb_repo::get_child_category_ids(pool, user_id, parent_id).await
}

pub async fn add_entry(
    pool: &SqlitePool,
    user_id: i64,
    category_id: i64,
    name: &str,
    path_url: &str,
    entry_type: &str,
    notes_base_dir: &std::path::Path,
) -> Result<KbEntry, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    if !matches!(entry_type, "text" | "link" | "file" | "video" | "image" | "audio" | "document") {
        return Err(AppError::Validation(
            format!("entry_type 非法: '{}'，必须是 text/link/file/video/image/audio/document 之一", entry_type)
        ));
    }

    let actual_path_url = if entry_type == "text" && path_url.starts_with("kb://") {
        let category = kb_repo::get_category_by_id(pool, user_id, category_id).await?;
        let library = &category.library;
        let notes_dir = notes_base_dir.join(library);
        std::fs::create_dir_all(&notes_dir)?;

        let base_name = if name.ends_with(".md") { name.to_string() } else { format!("{}.md", name) };
        let mut file_path = notes_dir.join(&base_name);
        let mut counter = 1;
        while file_path.exists() {
            let stem = name.trim_end_matches(".md");
            file_path = notes_dir.join(format!("{}（{}）.md", stem, counter));
            counter += 1;
        }
        std::fs::write(&file_path, "")?;
        file_path.to_string_lossy().to_string()
    } else {
        path_url.to_string()
    };

    let entry = kb_repo::add_entry(pool, user_id, category_id, name, &actual_path_url, entry_type, None, now).await?;

    let _ = intelligence_v4_service::instrument(
        pool, "system", "knowledge", "add_entry",
        Some(&format!("name: {}, type: {}", name, entry_type)),
    ).await;

    Ok(entry)
}

pub async fn delete_entry(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    kb_repo::delete_entry(pool, user_id, id).await?;

    let _ = intelligence_v4_service::instrument(
        pool, "system", "knowledge", "delete_entry",
        Some(&format!("id: {}", id)),
    ).await;

    Ok(())
}

pub async fn update_entry(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    name: Option<&str>,
    path_url: Option<&str>,
    entry_type: Option<&str>,
    category_id: Option<i64>,
    content: Option<&str>,
    notes_base_dir: &std::path::Path,
) -> Result<KbEntry, AppError> {
    if let Some(et) = entry_type {
        if !matches!(et, "text" | "link" | "file" | "video" | "image" | "audio" | "document") {
            return Err(AppError::Validation(
                format!("entry_type 非法: '{}'，必须是 text/link/file/video/image/audio/document 之一", et)
            ));
        }
    }

    if let Some(content_str) = content {
        if let Ok(entry) = kb_repo::get_entry_by_id(pool, user_id, id).await {
            let existing_content = entry.content.as_deref().unwrap_or("");
            if existing_content != content_str && !existing_content.is_empty() {
                let now = chrono::Utc::now().timestamp_millis();
                kb_repo::create_snapshot(pool, user_id, id, existing_content, now).await?;
            }

            let entry_path = std::path::Path::new(&entry.path_url);
            if entry_path.starts_with(notes_base_dir) {
                if let Some(parent) = entry_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(entry_path, content_str)?;
            }
        }
    }

    let result = kb_repo::update_entry(pool, user_id, id, name, path_url, entry_type, category_id, content).await?;

    if let Some(content_str) = content {
        let ref_targets = parse_wiki_links(content_str);
        kb_repo::rebuild_references(pool, user_id, id, &ref_targets).await?;
    }

    let _ = intelligence_v4_service::instrument(
        pool, "system", "knowledge", "update_entry",
        Some(&format!("id: {}", id)),
    ).await;

    Ok(result)
}

pub async fn search_entries(pool: &SqlitePool, user_id: i64, query: &str) -> Result<Vec<KbEntry>, AppError> {
    let mut entries = kb_repo::search_entries(pool, user_id, query).await?;
    mark_file_existence(&mut entries);
    Ok(entries)
}

pub async fn import_folder(
    pool: &SqlitePool,
    user_id: i64,
    folder_path: &str,
    category_id: i64,
    library: &str,
) -> Result<(Vec<KbCategory>, Vec<KbEntry>), AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let root_dir = std::path::Path::new(folder_path);
    if !root_dir.exists() || !root_dir.is_dir() {
        return Err(AppError::Validation(format!("目录不存在或不是文件夹: {}", folder_path)));
    }

    struct ScanTask {
        dir: std::path::PathBuf,
        parent_cat_id: i64,
        sort_order: i32,
    }

    let mut stack = vec![ScanTask {
        dir: root_dir.to_path_buf(),
        parent_cat_id: category_id,
        sort_order: 0,
    }];
    let mut new_categories: Vec<KbCategory> = Vec::new();
    let mut new_entries: Vec<KbEntry> = Vec::new();

    while let Some(task) = stack.pop() {
        let dir_name = task.dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("未命名目录");

        let cat = kb_repo::add_category(pool, user_id, dir_name, Some(task.parent_cat_id), library, task.sort_order, now).await?;
        let cat_id = cat.id;
        new_categories.push(cat);

        let items: Vec<_> = match std::fs::read_dir(&task.dir) {
            Ok(reader) => reader.filter_map(|e| e.ok()).collect(),
            Err(_) => continue,
        };

        let mut sub_dirs: Vec<std::path::PathBuf> = Vec::new();
        for entry in &items {
            let path = entry.path();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') {
                continue;
            }
            if matches!(name.as_str(), ".DS_Store" | "Thumbs.db" | "desktop.ini") {
                continue;
            }

            if is_dir {
                sub_dirs.push(path);
            } else {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                let entry_type = match ext.as_str() {
                    "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => "file",
                    "odt" | "ods" | "odp" | "rtf" | "epub" | "mobi" | "azw" | "azw3" | "fb2" | "djvu" | "chm" => "document",
                    "md" | "markdown" | "mdown" | "mkd" | "mkdn" | "txt" | "text" | "rst" | "rest" | "restructuredtext" | "asciidoc" | "adoc" | "org" | "wiki" | "mediawiki" | "log" | "nfo" | "diz" |
                    "json" | "jsonc" | "json5" | "xml" | "xaml" | "xsl" | "xslt" | "xsd" | "yaml" | "yml" | "toml" | "cfg" | "conf" | "ini" | "inf" | "cnf" | "env" | "envrc" | "properties" | "prop" | "lock" |
                    "html" | "htm" | "xhtml" | "shtml" | "css" | "scss" | "sass" | "less" | "styl" | "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "vue" | "svelte" | "astro" |
                    "py" | "pyw" | "pyx" | "rs" | "rlib" | "go" | "java" | "kt" | "kts" | "scala" | "sc" | "groovy" | "gvy" | "gy" | "gradle" | "php" | "phtml" | "php3" | "php4" | "php5" | "phps" | "phpt" | "rb" | "rbw" | "rake" | "gemspec" | "pl" | "pm" | "pod" | "t" | "swift" | "dart" | "jl" | "lua" | "r" | "rprofile" | "rmd" | "rnw" |
                    "erl" | "hrl" | "ex" | "exs" | "eex" | "leex" | "heex" | "hs" | "lhs" | "ml" | "mli" | "clj" | "cljs" | "cljc" | "edn" | "elm" | "fs" | "fsx" | "fsi" | "fsscript" | "nim" | "nims" | "zig" |
                    "sh" | "bash" | "zsh" | "fish" | "bat" | "cmd" | "ps1" | "psm1" | "psd1" | "makefile" | "mk" | "dockerfile" | "containerfile" | "cmake" |
                    "sql" | "psql" | "mysql" | "hql" | "prql" | "sparql" | "rq" | "graphql" | "gql" | "ttl" | "nt" | "n3" | "rdf" | "owl" | "cypher" | "cql" |
                    "proto" | "protobuf" | "thrift" | "avsc" | "avdl" | "wsdl" | "wadl" | "raml" | "oas" | "openapi" | "grpc" | "cue" | "dhall" | "nix" | "tf" | "tfvars" | "hcl" | "nomad" | "sentinel" | "smithy" |
                    "handlebars" | "hbs" | "hbrs" | "mustache" | "ejs" | "ect" | "pug" | "jade" | "twig" | "jinja" | "jinja2" | "j2" | "liquid" | "njk" | "nunjucks" | "dust" | "haml" | "slim" | "erb" | "rhtml" | "volt" | "latte" | "blade" | "mjml" |
                    "patch" | "diff" | "rej" | "eml" | "mbox" | "vcard" | "vcf" | "ics" | "ical" | "ifb" | "coffee" | "cson" | "iced" | "litcoffee" | "peg" | "pegjs" | "ohm" | "rnc" | "rng" | "dtd" | "sgml" | "sgm" | "ent" | "g4" | "ebnf" | "bnf" | "abnf" | "wast" | "wat" | "asm" | "sage" | "sagews" | "m" | "mat" | "octave" | "matlab" | "mma" | "nb" | "wl" | "wls" | "cirru" | "idr" | "lidr" | "agda" | "lagda" | "v" | "vhdl" | "vhd" | "sv" | "svh" | "glsl" | "vert" | "frag" | "tesc" | "tese" | "geom" | "comp" | "hlsl" | "fx" | "fxh" | "vsh" | "psh" | "wgsl" | "metal" | "opencl" | "cuh" | "ispc" | "plist" | "strings" | "dic" | "aff" | "po" | "pot" | "mo" | "lang" | "resx" | "resw" | "resjson" | "xliff" | "xlf" | "arb" | "ftl" | "fxml" | "mxml" | "xib" | "storyboard" | "abc" | "ly" | "ily" | "schemas" | "mod" | "pest" | "pomsky" | "nearley" | "a51" | "bsv" | "semgrep" | "smel" => "text",
                    "mp4" | "avi" | "mkv" | "mov" => "video",
                    _ => "file",
                };
                let path_str = path.to_string_lossy().to_string();
                if let Ok(e) = kb_repo::add_entry(pool, user_id, cat_id, &name, &path_str, entry_type, Some(folder_path), now).await {
                    new_entries.push(e);
                }
            }
        }
        sub_dirs.reverse();
        for (idx, sub_dir) in sub_dirs.into_iter().enumerate() {
            stack.push(ScanTask { dir: sub_dir, parent_cat_id: cat_id, sort_order: idx as i32 });
        }
    }

    if let Err(e) = kb_repo::add_tracked_path(pool, user_id, folder_path, category_id, library, now).await {
        tracing::warn!(path = %folder_path, error = %e, "记录追踪路径失败");
    }

    let count = new_entries.len();
    let _ = intelligence_v4_service::instrument(
        pool, "system", "knowledge", "import_folder",
        Some(&format!("path: {}, files: {}", folder_path, count)),
    ).await;

    Ok((new_categories, new_entries))
}

pub async fn get_tags(pool: &SqlitePool, user_id: i64) -> Result<Vec<KbTag>, AppError> {
    kb_repo::get_tags(pool, user_id).await
}

pub async fn add_tag(pool: &SqlitePool, user_id: i64, name: &str, color: &str) -> Result<KbTag, AppError> {
    kb_repo::add_tag(pool, user_id, name, color).await
}

pub async fn update_tag(pool: &SqlitePool, user_id: i64, id: i64, name: &str, color: &str) -> Result<KbTag, AppError> {
    kb_repo::update_tag(pool, user_id, id, name, color).await
}

pub async fn delete_tag(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    kb_repo::delete_tag(pool, user_id, id).await
}

pub async fn get_entry_tags(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<Vec<KbTag>, AppError> {
    kb_repo::get_entry_tags(pool, user_id, entry_id).await
}

pub async fn set_entry_tags(pool: &SqlitePool, user_id: i64, entry_id: i64, tag_ids: &[i64]) -> Result<(), AppError> {
    kb_repo::set_entry_tags(pool, user_id, entry_id, tag_ids).await
}

pub async fn batch_add_tag(pool: &SqlitePool, user_id: i64, entry_ids: &[i64], tag_id: i64) -> Result<u64, AppError> {
    kb_repo::batch_add_tag_to_entries(pool, user_id, entry_ids, tag_id).await
}

pub async fn batch_remove_tag(pool: &SqlitePool, user_id: i64, entry_ids: &[i64], tag_id: i64) -> Result<u64, AppError> {
    kb_repo::batch_remove_tag_from_entries(pool, user_id, entry_ids, tag_id).await
}

pub async fn get_entries_by_tag(pool: &SqlitePool, user_id: i64, tag_id: i64) -> Result<Vec<KbEntry>, AppError> {
    let mut entries = kb_repo::get_entries_by_tag(pool, user_id, tag_id).await?;
    mark_file_existence(&mut entries);
    Ok(entries)
}

pub async fn toggle_favorite(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<bool, AppError> {
    kb_repo::toggle_favorite(pool, user_id, entry_id).await
}

pub async fn get_favorite_entries(pool: &SqlitePool, user_id: i64) -> Result<Vec<KbEntry>, AppError> {
    let mut entries = kb_repo::get_favorite_entries(pool, user_id).await?;
    mark_file_existence(&mut entries);
    Ok(entries)
}

pub async fn record_access(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<(), AppError> {
    kb_repo::record_access(pool, user_id, entry_id).await
}

pub async fn get_recent_entries(pool: &SqlitePool, user_id: i64, limit: i64) -> Result<Vec<KbEntry>, AppError> {
    let mut entries = kb_repo::get_recent_entries(pool, user_id, limit).await?;
    mark_file_existence(&mut entries);
    Ok(entries)
}

pub async fn batch_delete_entries(pool: &SqlitePool, user_id: i64, ids: &[i64]) -> Result<u64, AppError> {
    kb_repo::batch_delete_entries(pool, user_id, ids).await
}

pub async fn batch_move_entries(pool: &SqlitePool, user_id: i64, ids: &[i64], target_category_id: i64) -> Result<u64, AppError> {
    kb_repo::batch_move_entries(pool, user_id, ids, target_category_id).await
}

pub async fn get_all_entry_tags(pool: &SqlitePool, user_id: i64) -> Result<Vec<EntryTagsResult>, AppError> {
    kb_repo::get_all_entry_tags(pool, user_id).await
}

pub async fn get_tag_stats(pool: &SqlitePool, user_id: i64) -> Result<Vec<TagStats>, AppError> {
    kb_repo::get_tag_stats(pool, user_id).await
}

pub async fn move_entry(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    target_category_id: i64,
) -> Result<KbEntry, AppError> {
    kb_repo::get_entry_by_id(pool, user_id, id).await?;
    kb_repo::get_category_by_id(pool, user_id, target_category_id).await?;
    kb_repo::move_entry(pool, user_id, id, target_category_id).await
}

pub async fn move_category(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    target_parent_id: Option<i64>,
    target_library: Option<String>,
) -> Result<KbCategory, AppError> {
    kb_repo::get_category_by_id(pool, user_id, id).await?;
    if let Some(target_id) = target_parent_id {
        if target_id == id {
            return Err(AppError::Validation("不能将分类移动到自身".into()));
        }
        let descendants = kb_repo::get_all_descendant_category_ids(pool, user_id, id).await?;
        if descendants.contains(&target_id) {
            return Err(AppError::Validation("不能将分类移动到其子分类下".into()));
        }
        kb_repo::get_category_by_id(pool, user_id, target_id).await?;
    }
    kb_repo::move_category(pool, user_id, id, target_parent_id, target_library).await
}

pub fn read_external_file(path: &str) -> Result<String, AppError> {
    let file_path = std::path::Path::new(path);
    if !file_path.exists() {
        return Err(AppError::Validation(format!("文件不存在: {}", path)));
    }
    if !file_path.is_file() {
        return Err(AppError::Validation(format!("路径不是文件: {}", path)));
    }
    let metadata = file_path.metadata().map_err(|e| AppError::FileSystem(e))?;
    if metadata.len() > 50 * 1024 * 1024 {
        return Err(AppError::Validation("文件过大（>50MB），不支持直接读取".into()));
    }
    let bytes = std::fs::read(file_path).map_err(|e| AppError::FileSystem(e))?;
    match std::str::from_utf8(&bytes) {
        Ok(s) => Ok(s.to_owned()),
        Err(_) => {
            Err(AppError::Validation("文件不是有效的 UTF-8 文本，请用外部程序打开".into()))
        }
    }
}

pub fn read_file_base64(path: &str) -> Result<(String, String, u64), AppError> {
    let file_path = std::path::Path::new(path);
    if !file_path.exists() {
        return Err(AppError::Validation(format!("文件不存在: {}", path)));
    }
    if !file_path.is_file() {
        return Err(AppError::Validation(format!("路径不是文件: {}", path)));
    }
    let metadata = file_path.metadata().map_err(|e| AppError::FileSystem(e))?;
    let size = metadata.len();
    if size > 500 * 1024 * 1024 {
        return Err(AppError::Validation("文件过大（>500MB），不支持加载".into()));
    }
    let bytes = std::fs::read(file_path).map_err(|e| AppError::FileSystem(e))?;
    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let mime = get_mime_type(&ext).to_string();
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
    Ok((b64, mime, size))
}

pub fn extract_docx_text(path: &str) -> Result<String, AppError> {
    let file_path = std::path::Path::new(path);
    if !file_path.exists() {
        return Err(AppError::Validation(format!("文件不存在: {}", path)));
    }
    let file = std::fs::File::open(file_path).map_err(|e| AppError::FileSystem(e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let doc_xml = archive.by_name("word/document.xml")
        .map_err(|_| AppError::Validation("不是有效的 DOCX 文件".into()))?;

    use std::io::BufReader;
    use quick_xml::Reader;
    use quick_xml::events::Event;
    let buf_reader = BufReader::new(doc_xml);
    let mut reader = Reader::from_reader(buf_reader);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut texts = Vec::new();
    let mut in_paragraph = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == b"w:p" {
                    in_paragraph = true;
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"w:p" && in_paragraph {
                    texts.push("\n".to_string());
                    in_paragraph = false;
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_paragraph {
                    if let Ok(t) = quick_xml::escape::unescape(std::str::from_utf8(e.as_ref()).unwrap_or_default()) {
                        texts.push(t.to_string());
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    let result = texts.join("");
    if result.trim().is_empty() {
        return Err(AppError::Validation("DOCX 文件内容为空或无法解析".into()));
    }
    Ok(result)
}

pub fn extract_doc_text(path: &str) -> Result<String, AppError> {
    let file_path = std::path::Path::new(path);
    tracing::info!("[DOC-DEBUG] 请求路径: {}", path);
    tracing::info!("[DOC-DEBUG] 文件存在: {}", file_path.exists());

    if !file_path.exists() {
        return Err(AppError::Validation(format!("文件不存在: {}", path)));
    }

    let metadata = std::fs::metadata(file_path)
        .map_err(|e| AppError::FileSystem(e))?;

    tracing::info!("[DOC-DEBUG] 文件大小: {} bytes", metadata.len());

    if metadata.len() > 50 * 1024 * 1024 {
        return Err(AppError::Validation("DOC 文件过大（超过 50MB），建议转换为 DOCX 格式后导入".into()));
    }

    let file = std::fs::File::open(file_path)
        .map_err(|e| AppError::FileSystem(e))?;

    use std::io::Read;
    tracing::info!("[DOC-DEBUG] 开始打开 CFB/OLE...");
    let mut compound = cfb::CompoundFile::open(file)
        .map_err(|e| AppError::Validation(format!("无法打开 DOC 文件: {}（可能不是有效的 OLE 格式，建议转为 DOCX）", e)))?;
    tracing::info!("[DOC-DEBUG] CFB 打开成功");

    let mut word_stream = compound
        .open_stream("WordDocument")
        .map_err(|_| AppError::Validation("不是有效的 DOC 文件（缺少 WordDocument 流）".into()))?;
    tracing::info!("[DOC-DEBUG] WordDocument 流打开成功");

    let max_stream_size = 20 * 1024 * 1024;
    let mut buf = Vec::with_capacity(metadata.len().min(max_stream_size) as usize);
    let mut reader = std::io::BufReader::with_capacity(65536, &mut word_stream);
    reader.read_to_end(&mut buf)
        .map_err(|e| AppError::FileSystem(e))?;
    drop(reader);

    if buf.is_empty() {
        return Err(AppError::Validation("DOC 文件的 WordDocument 流为空".into()));
    }

    // 跳过 FIB 文件信息块头部
    let skip = doc_fib_header_size(&buf).min(buf.len());
    let data = &buf[skip..];

    if data.is_empty() {
        return Err(AppError::Validation("DOC 文件内容为空".into()));
    }

    // ---- 编码检测：判断是 UTF-16LE 还是单字节编码 ----
    // 取样前 N 个偶数位置的字节，统计 0x00 的比例
    let sample_len = (data.len() / 2).min(4096) * 2;
    if sample_len < 4 { return Err(AppError::Validation("DOC 文件数据过短".into())); }

    let mut zero_count = 0u32;
    let mut sample_pairs = 0u32;
    for idx in (0..sample_len).step_by(2) {
        sample_pairs += 1;
        if data[idx] == 0x00 { zero_count += 1; }
    }
    let utf16_ratio = zero_count as f32 / sample_pairs.max(1) as f32;

    let result = if utf16_ratio > 0.35f32 {
        // 高比例的 0x00 在偶数位 → 很可能是 UTF-16LE 编码
        extract_doc_utf16le(data)
    } else {
        // 单字节编码（CP1252 / ASCII 等）
        extract_doc_singlebyte(data)
    };

    if result.trim().is_empty() {
        return Err(AppError::Validation("无法从 DOC 文件中提取有效文本（建议转为 DOCX 格式）".into()));
    }

    Ok(result)
}

/// 从 WordDocument 流中提取 UTF-16LE 编码的文本
fn extract_doc_utf16le(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len() / 3);
    let len = data.len();
    let mut i = 0;

    while i + 1 < len {
        let code = u16::from_le_bytes([data[i], data[i + 1]]);

        match code {
            0x000D => result.push('\n'),
            0x0007 | 0x0001 | 0x0002 | 0x0008 => result.push(' '),
            0x0009 => result.push('\t'),
            0x0020..=0xD7FF | 0xE000..=0xFFFF => {
                if let Some(ch) = char::from_u32(code as u32) {
                    if !ch.is_control() || ch == '\n' || ch == '\t' || ch == '\r' {
                        result.push(ch);
                    }
                }
            }
            _ => {}
        }
        i += 2;
    }

    cleanup_doc_text(&result)
}

/// 从 WordDocument 流中提取单字节编码的文本
fn extract_doc_singlebyte(data: &[u8]) -> String {
    let mut result = String::with_capacity(data.len() / 4);
    let len = data.len();
    let mut i = 0;

    // 策略：只提取连续可打印字符序列，过滤零散的二进制垃圾
    // 最小连续可打印长度阈值（低于此值视为垃圾）
    const MIN_RUN: usize = 4;

    while i < len {
        let b = data[i];

        match b {
            0x0D => {
                result.push('\n');
                i += 1;
            }
            0x0A => {
                // 跟随在 \r 后面的 \n 不重复添加
                if !result.ends_with('\n') { result.push('\n'); }
                i += 1;
            }
            0x09 => { result.push('\t'); i += 1; }
            0x20..=0x7E => {
                // ASCII 可打印字符：收集连续运行
                let run_start = i;
                while i < len && matches!(data[i], 0x20..=0x7E | 0xA0..=0xFF) {
                    i += 1;
                }
                let run_len = i - run_start;
                // 只保留足够长的运行（真正的文本段落）
                if run_len >= MIN_RUN {
                    for idx in run_start..i {
                        let b = data[idx];
                        if b >= 0x20 && b <= 0x7E {
                            result.push(b as char);
                        } else if b >= 0xA0 {
                            // 扩展拉丁字符（Windows-1252）
                            result.push(windows1252_to_unicode(b));
                        } else {
                            result.push(' ');
                        }
                    }
                    result.push(' ');
                }
            }
            _ => { i += 1; }
        }
    }

    cleanup_doc_text(&result)
}

/// Windows-1252 → Unicode 映射（常用子集）
fn windows1252_to_unicode(b: u8) -> char {
    match b {
        0xA0 => '\u{00A0}', 0xA1 => '\u{00A1}', 0xA2 => '\u{00A2}',
        0xA3 => '\u{00A3}', 0xA4 => '\u{00A4}', 0xA5 => '\u{00A5}',
        0xA6 => '\u{00A6}', 0xA7 => '\u{00A7}', 0xA8 => '\u{00A8}',
        0xA9 => '\u{00A9}', 0xAA => '\u{00AA}', 0xAB => '\u{00AB}',
        0xAC => '\u{00AC}', 0xAD => '\u{00AD}', 0xAE => '\u{00AE}',
        0xAF => '\u{00AF}', 0xB0 => '\u{00B0}', 0xB1 => '\u{00B1}',
        0xB2 => '\u{00B2}', 0xB3 => '\u{00B3}', 0xB4 => '\u{00B4}',
        0xB5 => '\u{00B5}', 0xB6 => '\u{00B6}', 0xB7 => '\u{00B7}',
        0xB8 => '\u{00B8}', 0xB9 => '\u{00B9}', 0xBA => '\u{00BA}',
        0xBB => '\u{00BB}', 0xBC => '\u{00BC}', 0xBD => '\u{00BD}',
        0xBE => '\u{00BE}', 0xBF => '\u{00BF}', 0xC0..=0xFF => {
            // C0-FF 范围：大部分直接映射到 Unicode Latin-1 Supplement
            char::from_u32(b as u32).unwrap_or('?')
        }
        _ => '?',
    }
}

/// 清理 DOC 提取结果：去多余空白、合并换行、截断尾部二进制垃圾
fn cleanup_doc_text(text: &str) -> String {
    let s: String = text.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t' || *c == ' ')
        .collect();

    // 合并 3 个及以上连续换行为双换行
    let re_multi = regex::Regex::new(r"\n{3,}").unwrap();
    let s = re_multi.replace_all(&s, "\n\n").to_string();

    // 合并多余空格
    let re_space = regex::Regex::new(r" {2,}").unwrap();
    let s = re_space.replace_all(&s, " ").to_string();

    // 去除行首尾空格，按行分割
    let lines: Vec<&str> = s.lines().map(|l| l.trim()).collect();

    // 从末尾倒序删除乱码行（更激进的检测）
    let re_garbage = regex::Regex::new(r"[\\]|[\x00-\x08\x0b\x0c\x0e-\x1f\x7f-\xff]").unwrap();
    let re_binary_garbage = regex::Regex::new(r"[A-Za-z0-9+/=_]{10,}.*[A-Za-z0-9+/=_]{5,}").unwrap();
    // 新增：OLE 对象链接模式 (\o\ \h\ \z\ \u 开头的控制字串)
    let re_ole_pattern = regex::Regex::new(r"^\\\\[ohzu]\\d").unwrap();
    // 新增：纯非打印字符占比过高
    let re_non_printable = regex::Regex::new(r"[\x80-\xff]{3,}").unwrap();

    let mut valid_end = lines.len();
    for i in (0..lines.len()).rev() {
        let line = lines[i];
        if line.is_empty() { continue; }

        let chinese_count = line.chars().filter(|c| matches!(*c, '\u{4E00}'..='\u{9FFF}')).count();
        let ascii_printable_count = line.chars().filter(|c| c.is_ascii_graphic() || *c == ' ').count();
        let garbage_matches = re_garbage.find_iter(line).count();
        let char_count = line.chars().count();

        let has_binary_pattern = re_binary_garbage.is_match(line);
        let has_ole_pattern = re_ole_pattern.is_match(line);
        let has_non_printable_run = re_non_printable.is_match(line);

        // 乱码判定（满足任一即截断）：
        // a) 含反斜杠或控制字符 且 无中文
        // b) 垃圾字符占比 > 30%
        // c) 二进制垃圾模式 且无中文
        // d) OLE 控制字链接模式（\o\ \h\ \z\ \u 数字开头）
        // e) 含长串高字节非打印字符且无中文
        let is_garbage = (garbage_matches > 0 && chinese_count == 0)
            || (char_count > 3 && garbage_matches as f32 / char_count as f32 > 0.3)
            || (has_binary_pattern && chinese_count == 0)
            || has_ole_pattern
            || (has_non_printable_run && chinese_count == 0 && char_count > 5)
            // 额外安全网：如果一行完全没有可读 ASCII/中文 内容
            || (ascii_printable_count == 0 && chinese_count == 0 && char_count > 2);

        if is_garbage {
            valid_end = i;
        } else {
            break;
        }
    }

    let result = lines[..valid_end].join("\n");

    // 最终安全网 #1：截断到最后一个有意义的句号/句尾标点
    if let Some(last_dot) = result.rfind(['。', '！', '？', '.', '!', '?', '」', '』', '"', '”']) {
        let char_len = result[last_dot..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        let after_punct = last_dot + char_len;

        if after_punct < result.len() {
            let tail: String = result[after_punct..].chars()
                .filter(|c| c.is_ascii_alphanumeric() || matches!(*c, '\u{4E00}'..='\u{9FFF}'))
                .collect();
            if tail.trim().is_empty() || tail.chars().count() < 3 {
                return result[..after_punct].to_string();
            }
        } else {
            return result;
        }
    }

    // 最终安全网 #2：删除最后 N 行中看起来像二进制垃圾的行
    // 有些 OLE 残留数据会被错误解码为混合字符，需要逐行检查
    let final_lines: Vec<&str> = result.lines().collect();
    if !final_lines.is_empty() {
        let mut clean_end = final_lines.len();

        for i in (0..final_lines.len()).rev() {
            let line = final_lines[i];
            if line.trim().is_empty() { continue; }

            // 计算该行的"可读性得分"
            let total_chars: Vec<char> = line.chars().collect();
            let normal_cjk: usize = total_chars.iter()
                .filter(|c| matches!(**c, '\u{4E00}'..='\u{9FFF}' | '\u{3000}'..='\u{303F}'))
                .count();
            let normal_punct: usize = total_chars.iter()
                .filter(|c| matches!(**c, '，' | '。' | '、' | '：' | '；' | '"' | '（' | '）' | '！' | '？' | '—' | '…' | '·' | '《' | '》' | '【' | '】'))
                .count();
            let ascii_alpha: usize = total_chars.iter()
                .filter(|c| c.is_ascii_alphabetic())
                .count();
            let weird_chars: usize = total_chars.iter()
                .filter(|c| {
                    !c.is_ascii_alphanumeric()
                    && !c.is_ascii_whitespace()
                    && !matches!(**c,
                        '\u{4E00}'..='\u{9FFF}'   // CJK 统一汉字
                        | '\u{3000}'..='\u{303F}' // CJK 符号和标点
                        | '\u{FF00}'..='\u{FFEF}' // 全角字符
                        | '"' | '(' | ')' // 英文标点
                    )
                })
                .count();

            let total = total_chars.len();
            if total == 0 { continue; }

            let readable_ratio = (normal_cjk + normal_punct + ascii_alpha) as f32 / total as f32;
            let weird_ratio = weird_chars as f32 / total as f32;

            // 乱码判定：
            // a) 可读字符 < 60% 且 有怪异字符 > 10%
            // b) 怪异字符占比 > 30%
            // c) 包含下划线+字母数字交替模式（OLE 链接特征）
            let has_ole_pattern = regex::Regex::new(r"[_]{2,}|[a-zA-Z0-9_À-ÿ]{5,}[a-zA-Z0-9_À-ÿ]{3,}")
                .unwrap()
                .is_match(line);

            let is_bad_line = (readable_ratio < 0.6 && weird_ratio > 0.1)
                || weird_ratio > 0.3
                || has_ole_pattern;

            if is_bad_line && i == final_lines.len() - 1 {
                // 只删除最后一行（避免误删正文）
                clean_end = i;
                break;
            }
        }

        if clean_end < final_lines.len() {
            return final_lines[..clean_end].join("\n");
        }
    }

    result
}

fn doc_fib_header_size(data: &[u8]) -> usize {
    if data.len() < 4 {
        return 0;
    }

    let nfib = u16::from_le_bytes([data[0x02], data[0x03]]);
    let ccp_text = u32::from_le_bytes([data[0x18], data[0x19], data[0x1A], data[0x1B]]);

    if ccp_text == 0 || ccp_text > 10_000_000 {
        return 512;
    }

    match nfib {
        0x0065 | 0x0069 => 512,
        0x006C | 0x006D => 768,
        0x0071 | 0x0072 => 1024,
        0x007D => 1280,
        0x00A5 | 0x00A6 | 0x00A8 | 0x00A9 | 0x00AA | 0x00AB => 1536,
        0x00B2 | 0x00B3 | 0x00B4 | 0x00B6 | 0x00B7 | 0x00B9 | 0x00BA => 1792,
        _ => 2048,
    }
}

/// 使用 rtf-parser-tt 库提取 RTF 纯文本
pub fn extract_rtf_text(path: &str) -> Result<String, AppError> {
    let file_path = std::path::Path::new(path);
    tracing::info!("[RTF-DEBUG] 请求路径: {}", path);
    tracing::info!("[RTF-DEBUG] 文件存在: {}", file_path.exists());

    if !file_path.exists() {
        return Err(AppError::Validation(format!("文件不存在: {}", path)));
    }

    // 读取原始字节
    let raw_bytes = std::fs::read(file_path)
        .map_err(|e| AppError::FileSystem(e))?;

    tracing::info!("[RTF-DEBUG] 文件大小: {} bytes", raw_bytes.len());
    // 打印前 100 字节的 hex 和尝试 UTF-8
    let preview_len = raw_bytes.len().min(100);
    let hex_preview: String = raw_bytes[..preview_len].iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>().join(" ");
    tracing::info!("[RTF-DEBUG] 前{}字节 HEX: {}", preview_len, hex_preview);
    if let Ok(utf8_preview) = std::str::from_utf8(&raw_bytes[..preview_len]) {
        tracing::info!("[RTF-DEBUG] 前{}字节 UTF-8: {:?}", preview_len, utf8_preview);
    }

    // 使用 encoding_rs 检测编码并解码
    // 尝试顺序：BOM检测 → UTF-8 → GBK/GB18030 → UTF-16LE → Latin1(ANSI)
    let raw = detect_and_decode_rtf(&raw_bytes);

    if !raw.trim_start().starts_with("{\\rtf") && !raw.trim_start().starts_with("{\u{feff}\\rtf") {
        return Err(AppError::Validation("不是有效的 RTF 文件".into()));
    }

    // 清理 BOM（如果有）
    let mut raw = raw.trim_start_matches('\u{feff}').to_string();

    // 关键修复：如果 RTF 使用非 UTF-8 编码（如 \ansicpg936 GBK），
    // 需要将 \'xx 十六进制转义序列转换为 Unicode \uNNNN，因为 rtf-parser-tt 不处理代码页编码
    if raw.contains("\\ansicpg") && !raw.contains("\\ansicpg65001") && !raw.contains("\\utf8") {
        raw = convert_rtf_gbk_escapes_to_unicode(&raw);
        tracing::info!("[RTF-DEBUG] 已转换 GBK 转义为 Unicode");
    }

    // 关键修复：rtf-parser-tt 库不处理 \par（段落分隔符），
    // 导致所有段落文本被连成一片。这里将 \par 替换为 \line（换行符），
    // \line 会被库正确处理为 \n
    // 注意：使用 \line\line 模拟段落间距（双换行）
    raw = regex::Regex::new(r"\\par\b").unwrap().replace_all(&raw, "\\line\n").to_string();
    tracing::info!("[RTF-DEBUG] 已替换 \\par 为 \\line");

    // 使用 rtf-parser-tt 库解析
    let doc = rtf_parser_tt::RtfDocument::try_from(raw.as_str())
        .map_err(|e| AppError::Validation(format!("RTF 解析失败: {}", e)))?;
    let text = doc.get_text();

    tracing::info!("[RTF-DEBUG] rtf-parser-tt 输出长度: {} chars", text.chars().count());
    let preview: String = text.chars().take(300).collect();
    tracing::info!("[RTF-DEBUG] rtf-parser-tt 前300字符: {:?}", preview);

    // 后处理清理
    let s = cleanup_rtf_text(&text);

    if s.trim().is_empty() {
        return Err(AppError::Validation("RTF 文件内容为空".into()));
    }
    Ok(s)
}

/// RTF 文本后处理：过滤元数据、保留段落结构
fn cleanup_rtf_text(text: &str) -> String {
    // Step 1: 过滤控制字符（保留 \n 和 \r 作为段落分隔）
    let s: String = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\r')
        .collect();

    // Step 2: 统一换行符
    let s = s.replace("\r\n", "\n").replace('\r', "\n");

    // Step 3: 过滤开头的文档元数据
    // RTF 的 \info 组通常包含 \author（作者）、{\*\company} 等信息
    // 这些会被 rtf-parser-tt 提取到文本最前面
    // 启发式检测：开头短字母/拼音串 + 连续中文无标点分隔 → 元数据
    let s = strip_rtf_metadata_prefix(&s);

    // Step 4: 压缩多余空白（但保留单个空格和换行）
    let re_space = regex::Regex::new(r" {2,}").unwrap();
    let s = re_space.replace_all(&s, " ").to_string();

    // Step 5: 压缩连续空行为双换行（保留段落结构！）
    let re_multi = regex::Regex::new(r"\n{3,}").unwrap();
    let s = re_multi.replace_all(&s, "\n\n").to_string();

    // Step 6: 按行 trim，但保留空行（段落间距）
    let lines: Vec<String> = s.lines().map(|l| l.trim().to_string()).collect();

    // Step 7: 过滤纯空行但保留段落间隔
    let result: Vec<&str> = lines.iter()
        .map(|l| l.as_str())
        .collect();

    result.join("\n")
}

/// 检测并移除 RTF 开头的文档元数据（如 "shiyin惊鸿" 这种作者信息）
/// 启发式规则：如果文本以 [短字母数字串][中文] 开头且没有句号等标点分隔，
/// 则认为是 \info 组泄漏的元数据
fn strip_rtf_metadata_prefix(text: &str) -> String {
    // 策略：找正文的第一个"有意义中文短语"，跳过之前的 ASCII+短名字 元数据
    // 只用多字节中文词作为标记（避免匹配到正文中的逗号等）

    let body_starters = [
        "感谢", "各位", "今天", "大家",
        "首先", "关于", "对于", "通过",
        "本文", "以下", "摘要", "前言",
    ];

    for starter in &body_starters {
        if let Some(pos) = text.find(starter) {
            // 正文标记必须在前 25 个字符内出现（元数据通常很短）
            if pos >= 3 && pos <= 25 {
                let prefix = &text[..pos];
                // 前缀必须是 "ASCII + 可选短中文名" 格式
                let ascii_part: String = prefix.chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                if !ascii_part.is_empty() && ascii_part.len() >= 2 {
                    let after = text[pos..].trim_start();
                    tracing::info!("[RTF-DEBUG] 移除元数据前缀 {:?}，从 {:?} 开始正文", prefix, starter);
                    return after.to_string();
                }
            }
        }
    }

    text.to_string()
}

/// 使用 encoding_rs 检测 RTF 文件编码并解码为 String
fn detect_and_decode_rtf(bytes: &[u8]) -> String {
    // 1. BOM 检测
    if let Some((enc, bom_len)) = encoding_rs::Encoding::for_bom(bytes) {
        let (s, _, _) = enc.decode(&bytes[bom_len..]);
        return s.into_owned();
    }

    // 2. 尝试 UTF-8
    if let Ok(s) = std::str::from_utf8(bytes) {
        if s.contains("{\\rtf") { return s.to_string(); }
    }

    // 3. 尝试 GBK（中文 Windows WPS 常用）
    {
        let (s, _, had_errors) = encoding_rs::GBK.decode(bytes);
        if !had_errors && s.contains("{\\rtf") { return s.into_owned(); }
    }

    // 4. 尝试 GB18030
    {
        let (s, _, _) = encoding_rs::GB18030.decode(bytes);
        if s.contains("{\\rtf") { return s.into_owned(); }
    }

    // 5. 尝试 UTF-16LE（无 BOM）
    if bytes.len() >= 2 {
        let u16_vec: Vec<u16> = bytes.chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let decoded: String = char::decode_utf16(u16_vec.into_iter())
            .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER)).collect();
        if decoded.contains("{\\rtf") || decoded.contains("{\u{feff}\\rtf") {
            return decoded;
        }
    }

    // 6. 兜底：Latin1/ANSI
    let (s, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    s.into_owned()
}

/// 将 RTF 中的 \'xx 十六进制转义（GBK 编码）转换为 \u{NNNN} Unicode 转义
/// rtf-parser-tt 不处理代码页编码，所以必须预先转换
fn convert_rtf_gbk_escapes_to_unicode(rtf: &str) -> String {
    use regex::Regex;

    // 匹配 \'hh 模式（RTF 十六进制字符转义）
    let re = Regex::new(r"\\'([0-9a-fA-F]{2})").unwrap();

    // 先收集所有匹配位置
    let matches: Vec<_> = re.find_iter(rtf).collect();

    let mut result = String::with_capacity(rtf.len());
    let mut last_end = 0;
    let mut i = 0;

    while i < matches.len() {
        let cap = &matches[i];
        // 保留匹配前的普通文本
        result.push_str(&rtf[last_end..cap.start()]);

        let hex_str = &cap.as_str()[2..]; // 跳过 \'
        let byte_val = u8::from_str_radix(hex_str, 16).unwrap_or(0);

        if byte_val <= 0x7F {
            // ASCII 字符，保持原样
            result.push_str(cap.as_str());
            last_end = cap.end();
        } else {
            // 高位字节，尝试 GBK 双字节解码
            let mut consumed_pair = false;

            if i + 1 < matches.len() {
                let next_cap = &matches[i + 1];
                // 检查两个匹配是否相邻（中间没有其他文本）
                if next_cap.start() == cap.end() {
                    let next_hex = &next_cap.as_str()[2..];
                    let next_byte = u8::from_str_radix(next_hex, 16).unwrap_or(0);

                    if next_byte >= 0x40 {
                        // 双字节 GBK 字符
                        let gbk_bytes = [byte_val, next_byte];
                        let (decoded, _, _) = encoding_rs::GBK.decode(&gbk_bytes);
                        if let Some(ch) = decoded.chars().next() {
                            let codepoint = ch as u32;
                            result.push_str(&format!("\\u{} ", codepoint));
                            last_end = next_cap.end();
                            consumed_pair = true;
                            i += 1; // 跳过下一个匹配
                        }
                    }
                }
            }

            if !consumed_pair {
                // 无法配对解码，用替换字符
                result.push('\u{FFFD}');
                last_end = cap.end();
            }
        }

        i += 1;
    }

    result.push_str(&rtf[last_end..]);
    result
}

pub fn extract_pptx_text(path: &str) -> Result<String, AppError> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "odp" {
        return extract_odp_text_inner(path);
    }

    let result = extract_pptx_slides(path)?;
    let mut all_texts: Vec<String> = Vec::new();
    for slide in &result.slides {
        let slide_texts: Vec<String> = slide.texts.iter().map(|t| t.text.clone()).collect();
        let content = slide_texts.join("");
        let trimmed = content.trim().to_string();
        if !trimmed.is_empty() {
            let mut parts = vec![format!("── 幻灯片 {} ──", slide.slide_number)];
            if let Some(ref notes) = slide.notes {
                if !notes.trim().is_empty() {
                    parts.push(format!("[备注] {}", notes.trim()));
                }
            }
            parts.push(trimmed);
            all_texts.push(parts.join("\n"));
        }
    }
    if all_texts.is_empty() {
        return Err(AppError::Validation("PPTX 文件中没有提取到文本内容".into()));
    }
    Ok(all_texts.join("\n\n"))
}

/// 从 ODP (OpenDocument Presentation) 文件中提取纯文本
fn extract_odp_text_inner(path: &str) -> Result<String, AppError> {
    let file = std::fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let content_xml = {
        let mut entry = archive.by_name("content.xml").map_err(|_| {
            AppError::Validation("不是有效的 ODP 文件（缺少 content.xml）".into())
        })?;
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut buf)
            .map_err(|e| AppError::FileSystem(e))?;
        buf
    };

    let content_str = String::from_utf8_lossy(&content_xml);

    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(&content_str);
    let mut buf = Vec::new();
    let mut in_draw_page = false;
    let mut in_text_p = false;
    let mut slide_num = 0u32;
    let mut result = String::new();
    let mut current_slide_text = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().0).to_string();
                match name.as_str() {
                    "draw:page" => {
                        slide_num += 1;
                        in_draw_page = true;
                        current_slide_text.clear();
                    }
                    "text:p" if in_draw_page => {
                        in_text_p = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().0).to_string();
                match name.as_str() {
                    "draw:page" => {
                        in_draw_page = false;
                        if !current_slide_text.trim().is_empty() {
                            if !result.is_empty() {
                                result.push_str("\n\n");
                            }
                            result.push_str(&format!("── 幻灯片 {} ──\n", slide_num));
                            result.push_str(&current_slide_text);
                        }
                        current_slide_text.clear();
                    }
                    "text:p" => {
                        in_text_p = false;
                        current_slide_text.push('\n');
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text_p {
                    let text = quick_xml::escape::unescape(std::str::from_utf8(e.as_ref()).unwrap_or_default()).unwrap_or_default();
                    current_slide_text.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(AppError::Validation(format!("ODP XML 解析错误: {}", e)));
            }
            _ => {}
        }
        buf.clear();
    }

    if result.trim().is_empty() {
        return Err(AppError::Validation("ODP 文件中没有提取到文本内容".into()));
    }
    Ok(result.trim().to_string())
}

/// 从 EPUB 电子书中提取纯文本
pub fn extract_epub_text(path: &str) -> Result<String, AppError> {
    let file = std::fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    // Step 1: 读取 META-INF/container.xml 找到 OPF 文件路径
    let opf_path = {
        let mut entry = archive.by_name("META-INF/container.xml").map_err(|_| {
            AppError::Validation("不是有效的 EPUB 文件（缺少 META-INF/container.xml）".into())
        })?;
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut buf)
            .map_err(|e| AppError::FileSystem(e))?;
        let content = String::from_utf8_lossy(&buf);

        use quick_xml::Reader;
        use quick_xml::events::Event;
        let mut reader = Reader::from_str(&content);
        let mut buf2 = Vec::new();
        let mut opf = String::new();
        loop {
            match reader.read_event_into(&mut buf2) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    if String::from_utf8_lossy(e.name().0) == "rootfile" {
                        for attr in e.attributes().flatten() {
                            if attr.key.0 == b"full-path" {
                                opf = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf2.clear();
        }
        if opf.is_empty() {
            return Err(AppError::Validation("EPUB container.xml 中未找到 rootfile".into()));
        }
        opf
    };

    // Step 2: 解析 OPF 获取 spine 阅读顺序
    let mut spine_ids: Vec<String> = Vec::new();
    let mut manifest: HashMap<String, String> = HashMap::new(); // id -> href

    {
        let mut entry = archive.by_name(&opf_path).map_err(|_| {
            AppError::Validation(format!("EPUB OPF 文件不存在: {}", opf_path))
        })?;
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut buf)
            .map_err(|e| AppError::FileSystem(e))?;
        let content = String::from_utf8_lossy(&buf);

        use quick_xml::Reader;
        use quick_xml::events::Event;
        let mut reader = Reader::from_str(&content);
        let mut buf2 = Vec::new();
        let mut in_manifest = false;
        let mut in_spine = false;

        loop {
            match reader.read_event_into(&mut buf2) {
                Ok(Event::Start(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().0).to_string();
                    match name.as_str() {
                        "manifest" => in_manifest = true,
                        "spine" => in_spine = true,
                        "item" if in_manifest => {
                            let mut id = String::new();
                            let mut href = String::new();
                            for attr in e.attributes().flatten() {
                                match attr.key.0 {
                                    b"id" => id = String::from_utf8_lossy(&attr.value).to_string(),
                                    b"href" => href = String::from_utf8_lossy(&attr.value).to_string(),
                                    _ => {}
                                }
                            }
                            if !id.is_empty() && !href.is_empty() {
                                manifest.insert(id, href);
                            }
                        }
                        "itemref" if in_spine => {
                            for attr in e.attributes().flatten() {
                                if attr.key.0 == b"idref" {
                                    spine_ids.push(String::from_utf8_lossy(&attr.value).to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().0).to_string();
                    match name.as_str() {
                        "manifest" => in_manifest = false,
                        "spine" => in_spine = false,
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf2.clear();
        }
    }

    if spine_ids.is_empty() {
        return Err(AppError::Validation("EPUB spine 中没有找到内容项".into()));
    }

    // Step 3: 计算 OPF 目录前缀，用于拼接相对路径
    let opf_dir = std::path::Path::new(&opf_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // Step 4: 按 spine 顺序提取文本
    let mut result = String::new();
    for id in &spine_ids {
        if let Some(href) = manifest.get(id) {
            let full_path = if opf_dir.is_empty() {
                href.clone()
            } else {
                format!("{}/{}", opf_dir, href)
            };

            let text = match archive.by_name(&full_path) {
                Ok(mut entry) => {
                    let mut buf = Vec::new();
                    if std::io::Read::read_to_end(&mut entry, &mut buf).is_ok() {
                        let html = String::from_utf8_lossy(&buf);
                        strip_html_tags(&html)
                    } else {
                        String::new()
                    }
                }
                Err(_) => String::new(),
            };

            // 如果 full_path 找不到，尝试直接使用 href
            let text = if text.is_empty() {
                match archive.by_name(href) {
                    Ok(mut entry) => {
                        let mut buf = Vec::new();
                        if std::io::Read::read_to_end(&mut entry, &mut buf).is_ok() {
                            let html = String::from_utf8_lossy(&buf);
                            strip_html_tags(&html)
                        } else {
                            String::new()
                        }
                    }
                    Err(_) => String::new(),
                }
            } else {
                text
            };

            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() {
                if !result.is_empty() {
                    result.push_str("\n\n");
                }
                result.push_str(&trimmed);
            }
        }
    }

    if result.trim().is_empty() {
        return Err(AppError::Validation("EPUB 文件中没有提取到文本内容".into()));
    }
    Ok(result)
}

/// 去除 HTML 标签，提取纯文本
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut in_style = false;
    let mut in_script = false;
    let mut tag_name = String::new();

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag_name.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                let tag_lower = tag_name.to_lowercase();
                if tag_lower == "style" {
                    in_style = true;
                } else if tag_lower == "/style" {
                    in_style = false;
                } else if tag_lower == "script" {
                    in_script = true;
                } else if tag_lower == "/script" {
                    in_script = false;
                } else if tag_lower.starts_with("br") || tag_lower == "p" || tag_lower == "/p"
                    || tag_lower.starts_with("h") && tag_lower.len() == 2
                    || tag_lower.starts_with("/h") && tag_lower.len() == 3
                    || tag_lower == "/div" || tag_lower == "/li" || tag_lower == "/blockquote"
                {
                    result.push('\n');
                }
            }
            _ if in_tag => {
                tag_name.push(ch);
            }
            _ if !in_style && !in_script => {
                result.push(ch);
            }
            _ => {}
        }
    }

    // 解码 HTML 实体
    let result = result.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'");

    // 清理多余空白
    let re_multi_newline = regex::Regex::new(r"\n{3,}").unwrap();
    let result = re_multi_newline.replace_all(&result, "\n\n").to_string();

    let lines: Vec<&str> = result.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    lines.join("\n")
}

/// 列出 ZIP 压缩包内容
pub fn list_zip_contents(path: &str) -> Result<String, AppError> {
    let file = std::fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let mut entries: Vec<(String, u64, u64)> = Vec::new(); // (name, compressed_size, uncompressed_size)
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| {
            AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;
        let name = entry.name().to_string();
        let compressed = entry.compressed_size();
        let uncompressed = entry.size();
        entries.push((name, compressed, uncompressed));
    }

    if entries.is_empty() {
        return Err(AppError::Validation("压缩包为空".into()));
    }

    // 排序：目录在前，文件在后
    entries.sort_by(|a, b| {
        let a_is_dir = a.0.ends_with('/');
        let b_is_dir = b.0.ends_with('/');
        b_is_dir.cmp(&a_is_dir).then(a.0.cmp(&b.0))
    });

    fn format_size(size: u64) -> String {
        if size >= 1024 * 1024 {
            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
        } else if size >= 1024 {
            format!("{:.1} KB", size as f64 / 1024.0)
        } else {
            format!("{} B", size)
        }
    }

    let mut result = String::from("压缩包内容列表：\n\n");
    for (name, _, uncompressed) in &entries {
        if name.ends_with('/') {
            result.push_str(&format!("  [目录] {}\n", name.trim_end_matches('/')));
        } else {
            let file_name = std::path::Path::new(name)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(name);
            result.push_str(&format!("  {}  ({})\n", file_name, format_size(*uncompressed)));
        }
    }

    result.push_str(&format!("\n共 {} 个条目", entries.len()));
    Ok(result)
}

/// 提取 PSD 文件的基本信息
pub fn extract_psd_info(path: &str) -> Result<String, AppError> {
    let data = std::fs::read(path).map_err(|e| AppError::FileSystem(e))?;

    let metadata = rawpsd::parse_psd_metadata(&data)
        .map_err(|e| AppError::Validation(format!("PSD 元数据解析失败: {}", e)))?;

    let layers_result = rawpsd::parse_layer_records(&data);
    let layers = match layers_result {
        Ok(layers) => layers,
        Err((partial, _)) => partial, // 即使部分失败也使用已解析的数据
    };

    let color_mode_name = match metadata.color_mode {
        0 => "Bitmap",
        1 => "Grayscale",
        2 => "Indexed",
        3 => "RGB",
        4 => "CMYK",
        7 => "Multichannel",
        8 => "Duotone",
        9 => "Lab",
        _ => "Unknown",
    };

    let mut result = String::new();
    result.push_str("PSD 文件信息\n");
    result.push_str("━━━━━━━━━━━━━━━━\n");
    result.push_str(&format!("尺寸: {} × {} px\n", metadata.width, metadata.height));
    result.push_str(&format!("位深度: {} bit\n", metadata.depth * 8));
    result.push_str(&format!("通道数: {}\n", metadata.channel_count));
    result.push_str(&format!("颜色模式: {}\n", color_mode_name));
    result.push_str(&format!("图层数: {}\n", layers.len()));

    if !layers.is_empty() {
        result.push_str(&format!("\n图层列表 ({}) 个:\n", "─".repeat(30)));

        let mut depth = 0;
        for layer in &layers {
            if layer.group_closer && depth > 0 {
                depth -= 1;
            }
            let indent = "  ".repeat(depth);
            let tag = if layer.group_opener {
                "[组]"
            } else if layer.group_closer {
                "[组结束]"
            } else {
                ""
            };
            let visible = if layer.is_visible { "" } else { " (隐藏)" };
            result.push_str(&format!("{}  {}{}{}\n", indent, layer.name, tag, visible));

            if layer.group_opener {
                depth += 1;
            }
        }
    }

    Ok(result)
}

/// 提取 AI 文件信息（Adobe Illustrator 不支持文本预览）
pub fn extract_ai_info(path: &str) -> Result<String, AppError> {
    let _path = path;
    Ok("Adobe Illustrator (.ai) 文件不支持文本预览。\n\n建议：\n  - 使用 Adobe Illustrator 打开查看完整内容\n  - 导出为 PDF 或 SVG 格式后在知识库中预览\n  - 导出为 PNG/JPG 图片格式后在知识库中查看".to_string())
}

use std::collections::HashMap;

struct TextRunFmt {
    bold: bool,
    italic: bool,
    font_size_pt: Option<f64>,
}

impl Default for TextRunFmt {
    fn default() -> Self {
        TextRunFmt { bold: false, italic: false, font_size_pt: None }
    }
}

fn parse_pptx_rels_xml(xml_bytes: &[u8]) -> Result<HashMap<String, String>, AppError> {
    use std::io::BufReader;
    use quick_xml::Reader;
    use quick_xml::events::Event;
    let mut map = HashMap::new();
    let buf_reader = BufReader::new(xml_bytes);
    let mut reader = Reader::from_reader(buf_reader);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                let name = e.name();
                if name.as_ref() == b"Relationship" {
                    let mut r_id = String::new();
                    let mut target = String::new();
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"Id" {
                            r_id = String::from_utf8_lossy(&attr.value).to_string();
                        } else if attr.key.as_ref() == b"Target" {
                            target = String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                    if !r_id.is_empty() && !target.is_empty() {
                        map.insert(r_id, target);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(map)
}

fn extract_slide_text_blocks(slide_xml: &[u8]) -> Vec<crate::commands::kb_commands::PptxSlideTextBlock> {
    use std::io::BufReader;
    use quick_xml::Reader;
    use quick_xml::events::Event;
    let mut blocks = Vec::new();
    let buf_reader = BufReader::new(slide_xml);
    let mut reader = Reader::from_reader(buf_reader);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut in_paragraph = false;
    let mut in_text_run = false;
    let mut fmt = TextRunFmt::default();
    let mut current_text = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                if name.as_ref() == b"a:p" {
                    in_paragraph = true;
                } else if name.as_ref() == b"a:r" {
                    in_text_run = true;
                    fmt = TextRunFmt::default();
                    current_text.clear();
                } else if name.as_ref() == b"a:rPr" && in_text_run {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"b" {
                            if let Ok(v) = String::from_utf8(attr.value.to_vec()) {
                                fmt.bold = v == "1" || v == "true";
                            }
                        }
                        if attr.key.as_ref() == b"i" {
                            if let Ok(v) = String::from_utf8(attr.value.to_vec()) {
                                fmt.italic = v == "1" || v == "true";
                            }
                        }
                        if attr.key.as_ref() == b"sz" {
                            if let Ok(v) = String::from_utf8(attr.value.to_vec()) {
                                if let Ok(val) = v.parse::<f64>() {
                                    fmt.font_size_pt = Some(val / 100.0);
                                }
                            }
                        }
                    }
                } else if name.as_ref() == b"a:br" && in_text_run {
                    current_text.push('\n');
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                if name.as_ref() == b"a:p" {
                    in_paragraph = false;
                    blocks.push(crate::commands::kb_commands::PptxSlideTextBlock {
                        text: "\n".to_string(),
                        is_bold: false,
                        is_italic: false,
                        font_size_pt: None,
                    });
                } else if name.as_ref() == b"a:r" {
                    if !current_text.is_empty() {
                        blocks.push(crate::commands::kb_commands::PptxSlideTextBlock {
                            text: std::mem::take(&mut current_text),
                            is_bold: fmt.bold,
                            is_italic: fmt.italic,
                            font_size_pt: fmt.font_size_pt,
                        });
                    }
                    in_text_run = false;
                    fmt = TextRunFmt::default();
                } else if name.as_ref() == b"a:rPr" {
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_paragraph && in_text_run {
                    if let Ok(t) = quick_xml::escape::unescape(std::str::from_utf8(e.as_ref()).unwrap_or_default()) {
                        current_text.push_str(&t);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    blocks
}

fn extract_slide_images(
    archive: &mut zip::ZipArchive<std::fs::File>,
    slide_xml: &[u8],
    rels: &HashMap<String, String>,
) -> Vec<crate::commands::kb_commands::PptxSlideImage> {
    use std::io::BufReader;
    use quick_xml::Reader;
    use quick_xml::events::Event;
    use base64::Engine;
    let mut images = Vec::new();
    let buf_reader = BufReader::new(slide_xml);
    let mut reader = Reader::from_reader(buf_reader);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == b"a:blip" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"r:embed" {
                            let embed_id = String::from_utf8_lossy(&attr.value).to_string();
                            if let Some(target) = rels.get(&embed_id) {
                                let media_name = target.trim_start_matches("../");
                                let media_path = format!("ppt/{}", media_name);
                                if let Ok(mut entry) = archive.by_name(&media_path) {
                                    let mut img_data = Vec::new();
                                    if std::io::Read::read_to_end(&mut entry, &mut img_data).is_ok() && !img_data.is_empty() {
                                        let mime = get_mime_type(
                                            target.split('.').last().unwrap_or("png")
                                        );
                                        let b64 = base64::engine::general_purpose::STANDARD.encode(&img_data);
                                        images.push(crate::commands::kb_commands::PptxSlideImage {
                                            base64: b64,
                                            mime_type: mime.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    images
}

fn extract_slide_notes(notes_xml: &[u8]) -> Option<String> {
    use std::io::BufReader;
    use quick_xml::Reader;
    use quick_xml::events::Event;
    let buf_reader = BufReader::new(notes_xml);
    let mut reader = Reader::from_reader(buf_reader);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut in_text_run = false;
    let mut notes_text = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                if name.as_ref() == b"a:r" {
                    in_text_run = true;
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                if name.as_ref() == b"a:r" {
                    in_text_run = false;
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text_run {
                    if let Ok(t) = quick_xml::escape::unescape(std::str::from_utf8(e.as_ref()).unwrap_or_default()) {
                        notes_text.push_str(&t);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    let trimmed = notes_text.trim().to_string();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

pub fn extract_pptx_slides(path: &str) -> Result<crate::commands::kb_commands::PptxExtractResult, AppError> {
    let file_path = std::path::Path::new(path);
    if !file_path.exists() {
        return Err(AppError::Validation(format!("文件不存在: {}", path)));
    }
    let ext = file_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "pptx" {
        return Err(AppError::Validation(format!(
            "不支持 .{} 格式，请使用 .pptx 格式。.ppt 旧格式请用 PowerPoint 另存为 .pptx 后重试",
            ext
        )));
    }

    let file = std::fs::File::open(file_path).map_err(|e| AppError::FileSystem(e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let mut slide_names: Vec<(u32, String)> = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| {
            AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })?;
        let name = entry.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            let num = name
                .replace("ppt/slides/slide", "")
                .replace(".xml", "")
                .parse::<u32>()
                .unwrap_or(0);
            slide_names.push((num, name));
        }
    }
    if slide_names.is_empty() {
        return Err(AppError::Validation("PPTX 文件中没有找到幻灯片".into()));
    }
    slide_names.sort_by_key(|(n, _)| *n);

    let mut slides: Vec<crate::commands::kb_commands::PptxSlide> = Vec::new();

    for (slide_num, slide_name) in &slide_names {
        let slide_xml = {
            let mut entry = archive.by_name(slide_name).map_err(|e| {
                AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut buf)
                .map_err(|e| AppError::FileSystem(e))?;
            buf
        };

        let rels_path = format!("ppt/slides/_rels/slide{}.xml.rels", slide_num);
        let rels = {
            match archive.by_name(&rels_path) {
                Ok(mut entry) => {
                    let mut rels_buf = Vec::new();
                    if std::io::Read::read_to_end(&mut entry, &mut rels_buf).is_ok() {
                        parse_pptx_rels_xml(&rels_buf).unwrap_or_default()
                    } else {
                        HashMap::new()
                    }
                }
                Err(_) => HashMap::new(),
            }
        };

        let texts = extract_slide_text_blocks(&slide_xml);
        let images = extract_slide_images(&mut archive, &slide_xml, &rels);

        let notes_path = format!("ppt/notesSlides/notesSlide{}.xml", slide_num);
        let notes = match archive.by_name(&notes_path) {
            Ok(mut entry) => {
                let mut notes_buf = Vec::new();
                if std::io::Read::read_to_end(&mut entry, &mut notes_buf).is_ok() {
                    extract_slide_notes(&notes_buf)
                } else {
                    None
                }
            }
            Err(_) => None,
        };

        slides.push(crate::commands::kb_commands::PptxSlide {
            slide_number: *slide_num,
            slide_image_base64: None,
            texts,
            images,
            notes,
        });
    }

    let slide_count = slides.len() as u32;
    Ok(crate::commands::kb_commands::PptxExtractResult { slides, slide_count, render_mode: "text".to_string() })
}

pub fn extract_table_data(path: &str) -> Result<crate::commands::kb_commands::TableDataResult, AppError> {
    let file_path = std::path::Path::new(path);
    if !file_path.exists() {
        return Err(AppError::Validation(format!("文件不存在: {}", path)));
    }
    let ext = file_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut workbook = calamine::open_workbook_auto(path)
        .map_err(|e| AppError::Validation(format!("无法打开表格文件: {}", e)))?;

    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err(AppError::Validation("表格文件中没有工作表".into()));
    }
    let sheet_name = sheet_names[0].clone();

    let range = workbook.worksheet_range(&sheet_name)
        .map_err(|e| AppError::Validation(format!("无法读取工作表: {}", e)))?;

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut total_rows = 0usize;
    let max_rows = 500usize;

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx >= max_rows {
            break;
        }
        let row_data: Vec<String> = row.iter()
            .map(|cell: &calamine::Data| cell.to_string())
            .collect();
        let has_content = row_data.iter().any(|s: &String| !s.trim().is_empty());
        if !row_data.is_empty() && has_content {
            rows.push(row_data);
            total_rows += 1;
        }
    }

    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);

    let pad_row = |mut row: Vec<String>| -> Vec<String> {
        while row.len() < max_cols {
            row.push(String::new());
        }
        row
    };

    let padded_rows: Vec<Vec<String>> = rows.into_iter().map(pad_row).collect();

    Ok(crate::commands::kb_commands::TableDataResult {
        headers: Vec::new(),
        rows: padded_rows,
        sheet_name,
        total_rows,
        total_cols: max_cols,
        extension: ext,
    })
}

pub fn scan_directory(folder_path: &str) -> Result<Vec<ScanDirFileInfo>, AppError> {
    let root = std::path::Path::new(folder_path);
    if !root.exists() || !root.is_dir() {
        return Err(AppError::Validation(format!("目录不存在或不是文件夹: {}", folder_path)));
    }
    let mut files = Vec::new();
    scan_recursive(root, &mut files)?;
    Ok(files)
}

fn scan_recursive(dir: &std::path::Path, files: &mut Vec<ScanDirFileInfo>) -> Result<(), AppError> {
    for entry in std::fs::read_dir(dir).map_err(|e| AppError::FileSystem(e))? {
        let entry = entry.map_err(|e| AppError::FileSystem(e))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || matches!(name.as_str(), "Thumbs.db" | "desktop.ini") {
            continue;
        }
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            scan_recursive(&path, files)?;
        } else {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let file_type = classify_file_type(&ext);
            files.push(ScanDirFileInfo {
                name,
                path: path.to_string_lossy().to_string(),
                size_bytes: size,
                extension: ext,
                file_type,
            });
        }
    }
    Ok(())
}

fn classify_file_type(ext: &str) -> String {
    match ext {
        // 文本类 - 标记语言
        "md" | "markdown" | "mdown" | "mkd" | "mkdn" | "rst" | "rest" | "restructuredtext" | "asciidoc" | "adoc" | "textile" | "pod" | "org" | "wiki" | "mediawiki" | "creole" | "typ" | "tex" | "latex" | "ltx" | "sty" | "cls" | "bib" | "bibtex" | "nfo" | "diz" |
        // 纯文本 / 数据
        "txt" | "text" | "log" | "csv" | "tsv" | "psv" | "srt" | "vtt" | "ass" | "ssa" | "sub" | "smi" | "lrc" |
        "json" | "jsonc" | "json5" | "xml" | "xaml" | "xsl" | "xslt" | "xsd" | "yaml" | "yml" | "toml" | "cfg" | "conf" | "ini" | "inf" | "cnf" | "env" | "envrc" | "properties" | "prop" | "lock" | "editorconfig" |
        // 前端
        "html" | "htm" | "xhtml" | "shtml" | "css" | "scss" | "sass" | "less" | "styl" | "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "vue" | "svelte" | "astro" |
        // 后端
        "py" | "pyw" | "pyx" | "rs" | "rlib" | "go" | "java" | "kt" | "kts" | "scala" | "sc" | "groovy" | "gvy" | "gy" | "gradle" | "php" | "phtml" | "php3" | "php4" | "php5" | "phps" | "phpt" | "rb" | "rbw" | "rake" | "gemspec" | "pl" | "pm" | "t" | "swift" | "dart" | "jl" | "lua" | "r" | "rprofile" | "rmd" | "rnw" |
        // 函数式
        "erl" | "hrl" | "ex" | "exs" | "eex" | "leex" | "heex" | "hs" | "lhs" | "ml" | "mli" | "clj" | "cljs" | "cljc" | "edn" | "elm" | "fs" | "fsx" | "fsi" | "fsscript" | "nim" | "nims" | "zig" |
        // 系统/脚本
        "sh" | "bash" | "zsh" | "fish" | "bat" | "cmd" | "ps1" | "psm1" | "psd1" | "makefile" | "mk" | "dockerfile" | "containerfile" | "cmake" |
        // 数据库/查询
        "sql" | "psql" | "mysql" | "hql" | "prql" | "sparql" | "rq" | "graphql" | "gql" | "ttl" | "nt" | "n3" | "rdf" | "owl" | "cypher" | "cql" |
        // 协议/接口
        "proto" | "protobuf" | "thrift" | "avsc" | "avdl" | "wsdl" | "wadl" | "raml" | "oas" | "openapi" | "grpc" | "cue" | "dhall" | "nix" | "tf" | "tfvars" | "hcl" | "nomad" | "sentinel" | "smithy" |
        // 模板
        "handlebars" | "hbs" | "hbrs" | "mustache" | "ejs" | "ect" | "pug" | "jade" | "twig" | "jinja" | "jinja2" | "j2" | "liquid" | "njk" | "nunjucks" | "dust" | "haml" | "slim" | "erb" | "rhtml" | "volt" | "latte" | "blade" | "mjml" |
        // 其他文本
        "patch" | "diff" | "rej" | "eml" | "mbox" | "vcard" | "vcf" | "ics" | "ical" | "ifb" | "coffee" | "cson" | "iced" | "litcoffee" | "peg" | "pegjs" | "ohm" | "rnc" | "rng" | "dtd" | "sgml" | "sgm" | "ent" | "g4" | "ebnf" | "bnf" | "abnf" | "wast" | "wat" | "asm" | "sage" | "sagews" | "m" | "mat" | "octave" | "matlab" | "mma" | "nb" | "wl" | "wls" | "cirru" | "idr" | "lidr" | "agda" | "lagda" | "v" | "vhdl" | "vhd" | "sv" | "svh" | "glsl" | "vert" | "frag" | "tesc" | "tese" | "geom" | "comp" | "hlsl" | "fx" | "fxh" | "vsh" | "psh" | "wgsl" | "metal" | "opencl" | "cuh" | "ispc" | "plist" | "strings" | "dic" | "aff" | "po" | "pot" | "mo" | "lang" | "resx" | "resw" | "resjson" | "xliff" | "xlf" | "arb" | "ftl" | "fxml" | "mxml" | "xib" | "storyboard" | "abc" | "ly" | "ily" | "schemas" | "mod" | "pest" | "pomsky" | "nearley" | "a51" | "bsv" | "semgrep" | "smel" => "text".into(),
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "tiff" | "tif" | "avif" | "heic" | "heif" => "image".into(),
        "mp4" | "avi" | "mkv" | "mov" | "webm" | "flv" | "wmv" | "m4v" | "3gp" => "video".into(),
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" | "opus" | "aiff" => "audio".into(),
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "rtf" => "document".into(),
        _ => "other".into(),
    }
}

pub async fn import_multiple_folders(
    pool: &SqlitePool,
    user_id: i64,
    folder_paths: &[String],
    category_id: i64,
    library: &str,
) -> Result<(Vec<KbCategory>, Vec<KbEntry>, u32, u32), AppError> {
    let mut all_categories: Vec<KbCategory> = Vec::new();
    let mut all_entries: Vec<KbEntry> = Vec::new();
    let mut success_count = 0u32;
    let mut fail_count = 0u32;

    for folder_path in folder_paths {
        match import_folder(pool, user_id, folder_path, category_id, library).await {
            Ok((categories, entries)) => {
                all_categories.extend(categories);
                all_entries.extend(entries);
                success_count += 1;
            }
            Err(e) => {
                tracing::warn!(path = %folder_path, error = %e, "导入文件夹失败");
                fail_count += 1;
            }
        }
    }

    Ok((all_categories, all_entries, success_count, fail_count))
}

pub async fn record_tracked_path(
    pool: &SqlitePool,
    user_id: i64,
    path: &str,
    category_id: i64,
    library: &str,
) -> Result<KbTrackedPath, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    kb_repo::add_tracked_path(pool, user_id, path, category_id, library, now).await
}

pub async fn get_tracked_paths(
    pool: &SqlitePool,
    user_id: i64,
    library: Option<&str>,
) -> Result<Vec<KbTrackedPath>, AppError> {
    kb_repo::get_tracked_paths(pool, user_id, library).await
}

pub async fn remove_tracked_path(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    kb_repo::remove_tracked_path(pool, user_id, id).await
}

pub async fn add_scanned_files(
    pool: &SqlitePool,
    user_id: i64,
    file_paths: &[String],
    category_id: i64,
    source_path: &str,
) -> Result<Vec<KbEntry>, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut added = Vec::new();
    for path in file_paths {
        let p = std::path::Path::new(path);
        if !p.exists() || !p.is_file() {
            continue;
        }
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("未命名").to_string();
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let entry_type = match ext.as_str() {
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => "file",
            "md" | "txt" | "rst" => "text",
            "mp4" | "avi" | "mkv" | "mov" => "video",
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" => "image",
            "mp3" | "wav" | "flac" | "ogg" | "aac" => "audio",
            _ => "file",
        };
        match kb_repo::add_entry(pool, user_id, category_id, &name, path, entry_type, Some(source_path), now).await {
            Ok(entry) => added.push(entry),
            Err(e) => tracing::warn!(path = %path, error = %e, "添加扫描文件失败"),
        }
    }
    Ok(added)
}

pub async fn check_paths(pool: &SqlitePool, user_id: i64) -> Result<PathCheckResult, AppError> {
    let tracked = kb_repo::get_all_tracked_paths(pool, user_id).await?;
    let total_tracked = tracked.len();
    let mut valid_tracked = 0usize;
    let mut invalid_tracked = Vec::new();

    for tp in &tracked {
        let p = std::path::Path::new(&tp.path);
        if p.exists() && p.is_dir() {
            valid_tracked += 1;
        } else {
            invalid_tracked.push(tp.id);
        }
    }

    let external_entries = kb_repo::get_external_entries(pool, user_id).await?;
    let total_external_entries = external_entries.len();
    let mut valid_external_entries = 0usize;
    let mut invalid_external_entries = Vec::new();

    for entry in &external_entries {
        let p = std::path::Path::new(&entry.path_url);
        if p.exists() {
            valid_external_entries += 1;
        } else {
            invalid_external_entries.push(entry.id);
        }
    }

    Ok(PathCheckResult {
        total_tracked,
        valid_tracked,
        invalid_tracked,
        total_external_entries,
        valid_external_entries,
        invalid_external_entries,
    })
}

pub async fn get_templates(pool: &SqlitePool, user_id: i64) -> Result<Vec<crate::models::knowledge::KbTemplate>, AppError> {
    kb_repo::get_templates(pool, user_id).await
}

pub async fn get_template(pool: &SqlitePool, user_id: i64, id: i64) -> Result<crate::models::knowledge::KbTemplate, AppError> {
    kb_repo::get_template(pool, user_id, id).await
}

pub async fn create_template(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    icon: &str,
    description: &str,
    entry_type: &str,
    content: &str,
) -> Result<crate::models::knowledge::KbTemplate, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    kb_repo::create_template(pool, user_id, name, icon, description, entry_type, content, now).await
}

pub async fn update_template(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    name: Option<&str>,
    icon: Option<&str>,
    description: Option<&str>,
    entry_type: Option<&str>,
    content: Option<&str>,
) -> Result<crate::models::knowledge::KbTemplate, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    kb_repo::update_template(pool, user_id, id, name, icon, description, entry_type, content, now).await
}

pub async fn delete_template(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    kb_repo::delete_template(pool, user_id, id).await
}

fn parse_wiki_links(content: &str) -> Vec<String> {
    let re = regex::Regex::new(r"\[\[([^\]|#]+)(?:\|[^\]]*)?\]\]").unwrap();
    re.captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().trim().to_string()))
        .collect()
}

pub async fn get_backlinks(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<Vec<BacklinkResult>, AppError> {
    let rows = kb_repo::get_backlinks(pool, user_id, entry_id).await?;
    Ok(rows
        .into_iter()
        .map(|(entry, snippet)| BacklinkResult { entry, snippet })
        .collect())
}

pub async fn get_outgoing_links(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<Vec<KbEntry>, AppError> {
    kb_repo::get_outgoing_links(pool, user_id, entry_id).await
}

pub async fn get_snapshots(pool: &SqlitePool, user_id: i64, entry_id: i64) -> Result<Vec<KbSnapshotResult>, AppError> {
    let entry = kb_repo::get_entry_by_id(pool, user_id, entry_id).await?;
    let snapshots = kb_repo::get_snapshots(pool, user_id, entry_id).await?;
    Ok(snapshots
        .into_iter()
        .map(|s| KbSnapshotResult {
            id: s.id,
            entry_id: s.entry_id,
            entry_name: entry.name.clone(),
            content_length: s.content.len(),
            content: s.content,
            created_at: s.created_at,
        })
        .collect())
}

pub async fn restore_snapshot(
    pool: &SqlitePool,
    user_id: i64,
    snapshot_id: i64,
    notes_base_dir: &std::path::Path,
) -> Result<KbEntry, AppError> {
    let snapshot = kb_repo::get_snapshot(pool, user_id, snapshot_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let entry = kb_repo::get_entry_by_id(pool, user_id, snapshot.entry_id).await?;

    let existing_content = entry.content.as_deref().unwrap_or("");
    let now = chrono::Utc::now().timestamp_millis();
    if !existing_content.is_empty() && existing_content != &snapshot.content {
        kb_repo::create_snapshot(pool, user_id, entry.id, existing_content, now).await?;
    }

    let entry_path = std::path::Path::new(&entry.path_url);
    if entry_path.starts_with(notes_base_dir) {
        if let Some(parent) = entry_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(entry_path, &snapshot.content)?;
    }

    let updated = kb_repo::update_entry(
        pool, user_id, entry.id, None, None, None, None, Some(&snapshot.content),
    ).await?;

    let ref_targets = parse_wiki_links(&snapshot.content);
    kb_repo::rebuild_references(pool, user_id, entry.id, &ref_targets).await?;

    Ok(updated)
}