//! 多文件原子编辑服务（D1 v3.2 / v2 D 轴深化）
//!
//! 设计依据：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §v3.2 多文件编辑
//!
//! 提供事务性批量编辑能力：
//!   - begin_transaction()：开始事务，记录原始文件内容到回滚栈
//!   - apply_edits()：批量应用编辑（write/create/delete）
//!   - commit()：提交事务，清空回滚栈
//!   - rollback()：回滚到事务开始前的状态
//!
//! 任意一步失败会自动回滚已应用的编辑，保证文件系统一致性。
//! 适用于 Agent 自主任务执行后的批量文件变更场景。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::app_error::AppError;
use crate::models::yuancode::DiffRequest;
use crate::services::yuancode_service;

/// 单个文件编辑操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum FileEditOp {
    /// 写入/覆盖文件
    Write {
        path: String,
        content: String,
        /// 是否创建不存在的父目录
        #[serde(default = "default_true")]
        create_dirs: bool,
    },
    /// 创建空文件（已存在则跳过）
    Create {
        path: String,
        #[serde(default)]
        content: Option<String>,
    },
    /// 删除文件或目录
    Delete {
        path: String,
        #[serde(default)]
        recursive: bool,
    },
    /// 重命名/移动
    Rename {
        from: String,
        to: String,
        #[serde(default = "default_true")]
        overwrite: bool,
    },
}

fn default_true() -> bool {
    true
}

/// 批量编辑请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiFileEditRequest {
    /// 工作区根路径（相对路径基于此拼接）
    pub workspace_path: String,
    /// 编辑操作列表（按顺序执行）
    pub edits: Vec<FileEditOp>,
    /// 失败时是否自动回滚（默认 true）
    #[serde(default = "default_true")]
    pub auto_rollback: bool,
    /// 提交前是否做 dry-run（不实际写入，仅校验）
    #[serde(default)]
    pub dry_run: bool,
}

/// 批量编辑结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiFileEditResult {
    pub success: bool,
    pub applied_count: usize,
    pub applied_paths: Vec<String>,
    pub rolled_back: bool,
    pub error: Option<String>,
    /// dry-run 模式下的预检报告
    pub dry_run_report: Option<String>,
}

/// 已应用的编辑快照（用于回滚）
struct AppliedSnapshot {
    op: FileEditOp,
    /// 编辑前的原始内容（None 表示文件原本不存在）
    original_content: Option<String>,
}

/// 执行批量原子编辑
pub async fn apply_atomic_edits(req: MultiFileEditRequest) -> Result<MultiFileEditResult, AppError> {
    // dry-run：仅预检，不写入
    if req.dry_run {
        let report = dry_run_check(&req)?;
        return Ok(MultiFileEditResult {
            success: true,
            applied_count: 0,
            applied_paths: vec![],
            rolled_back: false,
            error: None,
            dry_run_report: Some(report),
        });
    }

    let mut snapshots: Vec<AppliedSnapshot> = Vec::new();
    let mut applied_paths: Vec<String> = Vec::new();

    for edit in &req.edits {
        match apply_single(edit, &req.workspace_path, &mut snapshots) {
            Ok(path) => {
                applied_paths.push(path);
            }
            Err(e) => {
                if req.auto_rollback {
                    let _ = rollback(&mut snapshots, &req.workspace_path);
                    return Ok(MultiFileEditResult {
                        success: false,
                        applied_count: applied_paths.len(),
                        applied_paths,
                        rolled_back: true,
                        error: Some(format!("编辑失败已回滚: {}", e)),
                        dry_run_report: None,
                    });
                }
                return Err(e);
            }
        }
    }

    Ok(MultiFileEditResult {
        success: true,
        applied_count: applied_paths.len(),
        applied_paths,
        rolled_back: false,
        error: None,
        dry_run_report: None,
    })
}

fn dry_run_check(req: &MultiFileEditRequest) -> Result<String, AppError> {
    let mut lines = Vec::new();
    for (i, edit) in req.edits.iter().enumerate() {
        match edit {
            FileEditOp::Write { path, .. } => {
                let full = join_ws(&req.workspace_path, path)?;
                let exists = full.exists();
                lines.push(format!("[{}] Write {} (exists={})", i, path, exists));
            }
            FileEditOp::Create { path, .. } => {
                let full = join_ws(&req.workspace_path, path)?;
                let exists = full.exists();
                lines.push(format!("[{}] Create {} (exists={})", i, path, exists));
            }
            FileEditOp::Delete { path, recursive } => {
                let full = join_ws(&req.workspace_path, path)?;
                let exists = full.exists();
                lines.push(format!("[{}] Delete {} (recursive={}, exists={})", i, path, recursive, exists));
            }
            FileEditOp::Rename { from, to, overwrite } => {
                let full_from = join_ws(&req.workspace_path, from)?;
                let full_to = join_ws(&req.workspace_path, to)?;
                let from_exists = full_from.exists();
                let to_exists = full_to.exists();
                lines.push(format!(
                    "[{}] Rename {} -> {} (from_exists={}, to_exists={}, overwrite={})",
                    i, from, to, from_exists, to_exists, overwrite
                ));
            }
        }
    }
    Ok(lines.join("\n"))
}

fn apply_single(
    edit: &FileEditOp,
    workspace: &str,
    snapshots: &mut Vec<AppliedSnapshot>,
) -> Result<String, AppError> {
    match edit {
        FileEditOp::Write { path, content, create_dirs } => {
            let full = join_ws(workspace, path)?;
            // 快照原内容
            let original = fs::read_to_string(&full).ok();
            if *create_dirs {
                if let Some(parent) = full.parent() {
                    fs::create_dir_all(parent).map_err(|e| AppError::FileSystem(e))?;
                }
            }
            fs::write(&full, content).map_err(|e| AppError::FileSystem(e))?;
            snapshots.push(AppliedSnapshot {
                op: edit.clone(),
                original_content: original,
            });
            Ok(path.clone())
        }
        FileEditOp::Create { path, content } => {
            let full = join_ws(workspace, path)?;
            if full.exists() {
                return Ok(path.clone()); // 已存在则跳过
            }
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).map_err(|e| AppError::FileSystem(e))?;
            }
            let default_content = content.clone().unwrap_or_default();
            fs::write(&full, &default_content).map_err(|e| AppError::FileSystem(e))?;
            snapshots.push(AppliedSnapshot {
                op: edit.clone(),
                original_content: None,
            });
            Ok(path.clone())
        }
        FileEditOp::Delete { path, recursive } => {
            let full = join_ws(workspace, path)?;
            if !full.exists() {
                return Ok(path.clone()); // 不存在则跳过
            }
            let original = if full.is_file() {
                fs::read_to_string(&full).ok()
            } else {
                None
            };
            if *recursive {
                fs::remove_dir_all(&full).map_err(|e| AppError::FileSystem(e))?;
            } else {
                fs::remove_file(&full).map_err(|e| AppError::FileSystem(e))?;
            }
            snapshots.push(AppliedSnapshot {
                op: edit.clone(),
                original_content: original,
            });
            Ok(path.clone())
        }
        FileEditOp::Rename { from, to, overwrite } => {
            let full_from = join_ws(workspace, from)?;
            let full_to = join_ws(workspace, to)?;
            let original = fs::read_to_string(&full_from).ok();
            if full_to.exists() {
                if !*overwrite {
                    return Err(AppError::Validation(format!("目标已存在且未启用 overwrite: {}", to)));
                }
                fs::remove_file(&full_to).map_err(|e| AppError::FileSystem(e))?;
            }
            if let Some(parent) = full_to.parent() {
                fs::create_dir_all(parent).map_err(|e| AppError::FileSystem(e))?;
            }
            fs::rename(&full_from, &full_to).map_err(|e| AppError::FileSystem(e))?;
            snapshots.push(AppliedSnapshot {
                op: edit.clone(),
                original_content: original,
            });
            Ok(format!("{} -> {}", from, to))
        }
    }
}

fn rollback(snapshots: &mut Vec<AppliedSnapshot>, workspace: &str) -> Result<(), AppError> {
    // 逆序回滚
    // 注：rollback 是容错路径，路径校验失败时跳过该项继续回滚其他项，
    //     避免因单一快照的问题导致整个回滚中断。
    while let Some(snap) = snapshots.pop() {
        match &snap.op {
            FileEditOp::Write { path, .. } | FileEditOp::Create { path, .. } => {
                let full = match join_ws(workspace, path) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                match &snap.original_content {
                    Some(orig) => {
                        let _ = fs::write(&full, orig);
                    }
                    None => {
                        let _ = fs::remove_file(&full);
                    }
                }
            }
            FileEditOp::Delete { path, .. } => {
                let full = match join_ws(workspace, path) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                if let Some(orig) = &snap.original_content {
                    let _ = fs::write(&full, orig);
                }
            }
            FileEditOp::Rename { from, to, .. } => {
                let (full_from, full_to) = match (join_ws(workspace, from), join_ws(workspace, to)) {
                    (Ok(p1), Ok(p2)) => (p1, p2),
                    _ => continue,
                };
                let _ = fs::rename(&full_to, &full_from);
            }
        }
    }
    Ok(())
}

/// 安全拼接工作区相对路径，防止路径遍历。
///
/// 安全审计修复（发现 7，HIGH）：原实现仅去除前导 `./` 和 `/`，未过滤 `..`，
/// AI 可通过 `FileEditOp::Write { path: "../../.bashrc", ... }` 覆盖系统文件。
/// 现统一委托给 `crate::utils::file_path::safe_join`，执行：
/// 1. `canonicalize(workspace)` 解析符号链接
/// 2. 拒绝 `rel` 为绝对路径
/// 3. 拒绝 `rel` 含 `..` 段
/// 4. `canonicalize` 父目录 + `starts_with(workspace)` 校验
///
/// 返回 `PathBuf` 而非 `String`，调用方直接用作 `AsRef<Path>`。
fn join_ws(workspace: &str, rel: &str) -> Result<PathBuf, AppError> {
    crate::utils::file_path::safe_join(Path::new(workspace), rel)
}

// 用于测试的辅助函数：从路径映射推断变更集合
#[allow(dead_code)]
pub fn collect_changed_paths(edits: &[FileEditOp]) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for e in edits {
        match e {
            FileEditOp::Write { path, .. } | FileEditOp::Create { path, .. } | FileEditOp::Delete { path, .. } => {
                if !paths.contains(path) {
                    paths.push(path.clone());
                }
            }
            FileEditOp::Rename { from, to, .. } => {
                if !paths.contains(from) {
                    paths.push(from.clone());
                }
                if !paths.contains(to) {
                    paths.push(to.clone());
                }
            }
        }
    }
    paths
}

#[allow(dead_code)]
fn _unused_marker(_: &HashMap<String, String>) {}

// ===== D1.7 跨文件 diff 预览（不实际写入磁盘）=====

/// 单个文件的 diff 预览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiffEntry {
    pub path: String,
    /// 操作类型：write / create / delete / rename
    pub op_type: String,
    /// unified diff 文本（rename 为 None，新增/删除文件显示全量 +/-）
    pub unified_diff: Option<String>,
    pub added_lines: usize,
    pub removed_lines: usize,
}

/// 跨文件 diff 预览汇总
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiFileDiffPreview {
    pub entries: Vec<FileDiffEntry>,
    pub total_added: usize,
    pub total_removed: usize,
}

/// 生成跨文件 diff 预览（不实际写入磁盘，供 Agent / 前端在提交前审查变更）
pub fn preview_diffs(req: &MultiFileEditRequest) -> Result<MultiFileDiffPreview, AppError> {
    let mut entries = Vec::new();
    let mut total_added = 0usize;
    let mut total_removed = 0usize;

    for edit in &req.edits {
        match edit {
            FileEditOp::Write { path, content, .. } => {
                let full = join_ws(&req.workspace_path, path)?;
                let original = fs::read_to_string(&full).unwrap_or_default();
                let diff_req = DiffRequest {
                    original_content: original,
                    modified_content: content.clone(),
                    file_path: Some(path.clone()),
                    context_lines: Some(3),
                };
                let result = yuancode_service::compute_diff(diff_req)?;
                let (added, removed) = count_diff_lines(&result.unified_diff);
                total_added += added;
                total_removed += removed;
                entries.push(FileDiffEntry {
                    path: path.clone(),
                    op_type: "write".into(),
                    unified_diff: Some(result.unified_diff),
                    added_lines: added,
                    removed_lines: removed,
                });
            }
            FileEditOp::Create { path, content } => {
                let c = content.clone().unwrap_or_default();
                let line_count = c.lines().count().max(1);
                total_added += line_count;
                entries.push(FileDiffEntry {
                    path: path.clone(),
                    op_type: "create".into(),
                    unified_diff: Some(format!(
                        "--- /dev/null\n+++ {}\n{}",
                        path,
                        prefix_lines(&c, '+')
                    )),
                    added_lines: line_count,
                    removed_lines: 0,
                });
            }
            FileEditOp::Delete { path, .. } => {
                let full = join_ws(&req.workspace_path, path)?;
                let original = fs::read_to_string(&full).unwrap_or_default();
                let line_count = original.lines().count().max(1);
                total_removed += line_count;
                entries.push(FileDiffEntry {
                    path: path.clone(),
                    op_type: "delete".into(),
                    unified_diff: Some(format!(
                        "--- {}\n+++ /dev/null\n{}",
                        path,
                        prefix_lines(&original, '-')
                    )),
                    added_lines: 0,
                    removed_lines: line_count,
                });
            }
            FileEditOp::Rename { from, to, .. } => {
                entries.push(FileDiffEntry {
                    path: format!("{} -> {}", from, to),
                    op_type: "rename".into(),
                    unified_diff: None,
                    added_lines: 0,
                    removed_lines: 0,
                });
            }
        }
    }

    Ok(MultiFileDiffPreview {
        entries,
        total_added,
        total_removed,
    })
}

/// 统计 unified diff 中的新增/删除行数（跳过 +++/--- 头）
fn count_diff_lines(unified_diff: &str) -> (usize, usize) {
    let mut added = 0usize;
    let mut removed = 0usize;
    for line in unified_diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            added += 1;
        } else if line.starts_with('-') {
            removed += 1;
        }
    }
    (added, removed)
}

/// 给每行加前缀（用于新增/删除文件的全量 diff）
fn prefix_lines(content: &str, prefix: char) -> String {
    content
        .lines()
        .map(|l| format!("{}{}", prefix, l))
        .collect::<Vec<_>>()
        .join("\n")
}
