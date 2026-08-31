//! 路径安全工具
//!
//! 提供 `canonicalize` + `starts_with` 校验，防止路径遍历攻击。
//! 配合 `crate::commands::common::require_auth` 使用，构成文件操作的安全基线。
//!
//! ## 设计依据
//!
//! 安全审计报告（`安全审计报告/2026-07-25-代码安全审计报告.md`）发现 4 处路径遍历：
//! - 发现 2：`file_edit_service` 任意文件读写无路径校验（HIGH）
//! - 发现 6：`agent_executor_service::join_workspace` 未过滤 `..`（HIGH）
//! - 发现 7：`multi_file_edit_service::join_ws` 未过滤 `..`（HIGH）
//! - 发现 8：`agent_v3_commands::apply_diff_to_workspace` 绝对路径逃逸（HIGH）
//!
//! 本模块提供统一的安全拼接 / 校验函数，供上述调用点复用。

use std::path::{Component, Path, PathBuf};

use crate::error::app_error::AppError;

/// 安全拼接工作区相对路径，防止路径遍历。
///
/// 适用于：Agent 执行器、多文件编辑、沙箱等场景，
/// 目标路径必须在 `workspace` 内。
///
/// # 安全策略
///
/// 1. `canonicalize(workspace)` 得到工作区绝对路径（解析符号链接）
/// 2. 拒绝 `rel` 为绝对路径（防止 `Path::join("/etc/passwd")` 返回绝对路径覆盖 base）
/// 3. 拒绝 `rel` 含 `..` 段（防止 `workspace.join("../../../etc/passwd")` 逃逸）
/// 4. 拼接后 `canonicalize` 父目录并断言 `starts_with(workspace)`
///    （防止符号链接逃逸；文件可能尚不存在，故仅 canonicalize 父目录）
///
/// # 参数
///
/// - `workspace`：工作区根目录（必须存在）
/// - `rel`：相对路径（不得含 `..`，不得为绝对路径）
///
/// # 返回
///
/// - `Ok(PathBuf)`：规范化后的绝对路径，保证在 `workspace` 内
/// - `Err(AppError::Validation)`：路径逃逸、含 `..`、为绝对路径或父目录无效
///
/// # 示例
///
/// ```ignore
/// use crate::utils::file_path::safe_join;
/// let ws = Path::new("/workspace");
/// let ok = safe_join(ws, "src/main.rs")?;          // ✅ /workspace/src/main.rs
/// let bad = safe_join(ws, "../../etc/passwd")?;    // ❌ 拒绝含 `..`
/// let bad = safe_join(ws, "/etc/passwd")?;         // ❌ 拒绝绝对路径
/// ```
pub fn safe_join(workspace: &Path, rel: &str) -> Result<PathBuf, AppError> {
    // 1. 规范化 workspace（解析符号链接，必须存在）
    let ws_canon = workspace.canonicalize().map_err(|e| {
        AppError::Validation(format!(
            "工作区路径无效: {}: {}",
            workspace.display(),
            e
        ))
    })?;

    // 2. 拒绝绝对路径（Path::join 会用绝对路径覆盖 base，构成逃逸）
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return Err(AppError::Validation(format!(
            "拒绝绝对路径: {}（必须使用工作区相对路径）",
            rel
        )));
    }

    // 3. 拒绝 `..` 段（防止拼接后逃逸工作区）
    for comp in rel_path.components() {
        if let Component::ParentDir = comp {
            return Err(AppError::Validation(format!(
                "拒绝含 `..` 的路径: {}（必须在工作区内）",
                rel
            )));
        }
    }

    // 4. 拼接 + canonicalize 父目录 + starts_with 校验
    //    注：target 文件可能尚不存在（write 场景），canonicalize 会失败
    //    因此仅 canonicalize 父目录，文件名直接拼接
    let target = ws_canon.join(rel_path);
    canonicalize_within(&ws_canon, &target)
}

/// 校验目标路径在指定工作区内（用于已存在的完整路径校验）。
///
/// 与 [`safe_join`] 的差异：本函数接受完整路径（非相对路径），仅做边界校验。
/// 适用于 `agent_v3_commands::apply_diff_to_workspace` 场景，
/// 其中 `diff.path` 已被 AI 生成为完整相对路径或绝对路径。
///
/// # 安全策略
///
/// 1. `canonicalize(workspace)`
/// 2. `canonicalize(target)` 或 `canonicalize(parent) + file_name`（文件不存在时）
/// 3. 断言 `target.starts_with(workspace)`
///
/// # 参数
///
/// - `workspace`：工作区根目录（必须存在）
/// - `target`：待校验的目标完整路径（文件可能不存在）
pub fn ensure_in_workspace(workspace: &Path, target: &Path) -> Result<PathBuf, AppError> {
    let ws_canon = workspace.canonicalize().map_err(|e| {
        AppError::Validation(format!(
            "工作区路径无效: {}: {}",
            workspace.display(),
            e
        ))
    })?;

    canonicalize_within(&ws_canon, target)
}

/// 规范化用户提供的绝对路径（用于 `file_edit_service` 场景）。
///
/// 与 [`safe_join`] 的差异：本函数**不**限制路径在 workspace 内，
/// 因为文件编辑功能允许用户编辑任意位置的文件（如 PDF、图片、音频）。
/// 但仍需防止 `..` 注入与符号链接逃逸。
///
/// # 安全策略
///
/// 1. 拒绝空路径
/// 2. 拒绝含 `..` 段的路径（用户应直接给出绝对路径，无需 `..`）
/// 3. `canonicalize` 父目录（解析符号链接），文件名直接拼接
///
/// # 参数
///
/// - `path`：用户提供的绝对路径（文件可能尚不存在）
///
/// # 返回
///
/// 规范化后的绝对路径。
pub fn canonicalize_user_path(path: &str) -> Result<PathBuf, AppError> {
    if path.trim().is_empty() {
        return Err(AppError::Validation("路径不能为空".into()));
    }

    let p = Path::new(path);

    // 拒绝含 `..` 段的路径
    for comp in p.components() {
        if let Component::ParentDir = comp {
            return Err(AppError::Validation(format!(
                "拒绝含 `..` 的路径: {}（请提供不含 `..` 的绝对路径）",
                path
            )));
        }
    }

    // canonicalize 父目录（文件可能不存在）
    let parent = p.parent().ok_or_else(|| {
        AppError::Validation(format!("路径无父目录: {}", path))
    })?;

    let parent_canon = parent.canonicalize().map_err(|e| {
        AppError::Validation(format!(
            "父目录无效: {}: {}",
            parent.display(),
            e
        ))
    })?;

    let file_name = p.file_name().ok_or_else(|| {
        AppError::Validation(format!("路径无文件名: {}", path))
    })?;

    Ok(parent_canon.join(file_name))
}

/// 内部辅助：canonicalize target 并断言在 ws_canon 内。
///
/// 处理 target 文件不存在的情况（仅 canonicalize 父目录）。
fn canonicalize_within(ws_canon: &Path, target: &Path) -> Result<PathBuf, AppError> {
    let target_canon = match target.canonicalize() {
        // target 已存在：直接 canonicalize
        Ok(p) => p,
        // target 不存在（write 场景）：canonicalize 父目录 + 拼接文件名
        Err(_) => {
            let parent = target.parent().ok_or_else(|| {
                AppError::Validation(format!("路径无父目录: {}", target.display()))
            })?;

            let parent_canon = parent.canonicalize().map_err(|e| {
                AppError::Validation(format!(
                    "父目录无效: {}: {}",
                    parent.display(),
                    e
                ))
            })?;

            let file_name = target.file_name().ok_or_else(|| {
                AppError::Validation(format!("路径无文件名: {}", target.display()))
            })?;

            parent_canon.join(file_name)
        }
    };

    // 断言 target 在 workspace 内（防止符号链接逃逸）
    if !target_canon.starts_with(ws_canon) {
        return Err(AppError::Validation(format!(
            "路径逃逸工作区: {} -> {}",
            ws_canon.display(),
            target_canon.display()
        )));
    }

    Ok(target_canon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_workspace() -> TempDir {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        // 创建子目录与文件，便于 canonicalize 测试
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src").join("main.rs"), "fn main() {}").unwrap();
        dir
    }

    #[test]
    fn test_safe_join_normal_relative_path() {
        let dir = make_workspace();
        let ws = dir.path();
        let result = safe_join(ws, "src/main.rs").expect("合法相对路径应通过");
        assert!(result.starts_with(ws.canonicalize().unwrap()));
        assert!(result.ends_with("src/main.rs"));
    }

    #[test]
    fn test_safe_join_reject_absolute_path() {
        let dir = make_workspace();
        let ws = dir.path();
        // Linux 绝对路径
        let err = safe_join(ws, "/etc/passwd").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains("绝对路径"));
    }

    #[test]
    fn test_safe_join_reject_parent_dir() {
        let dir = make_workspace();
        let ws = dir.path();
        let err = safe_join(ws, "../../etc/passwd").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains(".."));
    }

    #[test]
    fn test_safe_join_reject_single_parent_dir() {
        let dir = make_workspace();
        let ws = dir.path();
        let err = safe_join(ws, "../secret.txt").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn test_safe_join_allow_subdir() {
        let dir = make_workspace();
        let ws = dir.path();
        let result = safe_join(ws, "src/main.rs").expect("子目录路径应通过");
        assert!(result.exists());
    }

    #[test]
    fn test_safe_join_nonexistent_target_for_write() {
        let dir = make_workspace();
        let ws = dir.path();
        // 文件不存在但父目录存在（write 场景）
        let result = safe_join(ws, "src/new_file.rs").expect("write 场景应通过");
        assert!(!result.exists());
        assert!(result.starts_with(ws.canonicalize().unwrap()));
    }

    #[test]
    fn test_ensure_in_workspace_normal() {
        let dir = make_workspace();
        let ws = dir.path();
        let target = ws.join("src/main.rs");
        let result = ensure_in_workspace(ws, &target).expect("工作区内路径应通过");
        assert!(result.starts_with(ws.canonicalize().unwrap()));
    }

    #[test]
    fn test_ensure_in_workspace_reject_escape() {
        let dir = make_workspace();
        let ws = dir.path();
        // 构造一个绝对路径逃逸：ws/../../etc/passwd
        let escape = ws.join("..").join("..").join("etc").join("passwd");
        let err = ensure_in_workspace(ws, &escape).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains("逃逸工作区"));
    }

    #[test]
    fn test_canonicalize_user_path_normal() {
        let dir = make_workspace();
        let file = dir.path().join("src").join("main.rs");
        let result = canonicalize_user_path(file.to_str().unwrap()).expect("合法绝对路径应通过");
        assert!(result.exists());
    }

    #[test]
    fn test_canonicalize_user_path_reject_empty() {
        let err = canonicalize_user_path("").unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
    }

    #[test]
    fn test_canonicalize_user_path_reject_parent_dir() {
        let dir = make_workspace();
        let path_with_dotdot = dir
            .path()
            .join("src")
            .join("..")
            .join("main.rs");
        let err = canonicalize_user_path(path_with_dotdot.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, AppError::Validation(_)));
        assert!(err.to_string().contains(".."));
    }

    #[test]
    fn test_canonicalize_user_path_nonexistent_file_in_existing_dir() {
        let dir = make_workspace();
        let file = dir.path().join("src").join("new.rs");
        let result =
            canonicalize_user_path(file.to_str().unwrap()).expect("write 场景应通过");
        assert!(!result.exists());
        assert!(result.parent().unwrap().exists());
    }
}
