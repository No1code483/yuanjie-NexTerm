//! 字体管理命令（C1.2 / v1.51.5）
//!
//! 提供 4 个 Tauri 命令用于管理用户上传的自定义字体：
//! - `font_list_custom`：列出已上传字体
//! - `font_upload`：上传字体文件到 %APPDATA%/NexTerm/fonts/
//! - `font_delete`：删除指定字体文件
//! - `font_get_dir`：获取字体目录路径
//!
//! 关联文档：功能展望/体验深化/01_主题自定义系统_未来展望.md §2.2

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

/// 自定义字体元数据
#[derive(Debug, Serialize, Clone)]
pub struct CustomFont {
    /// 文件名（含扩展名，如 "JetBrainsMono.ttf"）
    pub filename: String,
    /// 字体家族名（去掉扩展名，用作 @font-face font-family）
    pub family_name: String,
    /// 文件大小（字节）
    pub file_size: u64,
    /// 格式（ttf / otf / woff / woff2）
    pub format: String,
}

/// 字体扩展名 → @font-face format() 值
fn get_font_format(ext: &str) -> Option<&'static str> {
    match ext.to_lowercase().as_str() {
        "ttf" => Some("truetype"),
        "otf" => Some("opentype"),
        "woff" => Some("woff"),
        "woff2" => Some("woff2"),
        _ => None,
    }
}

/// 获取字体目录路径（不存在则创建）
fn get_fonts_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {}", e))?;
    let fonts_dir = data_dir.join("fonts");
    if !fonts_dir.exists() {
        fs::create_dir_all(&fonts_dir).map_err(|e| format!("无法创建字体目录: {}", e))?;
    }
    Ok(fonts_dir)
}

/// 列出用户上传的所有自定义字体
#[tauri::command]
pub async fn font_list_custom(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<ApiResponse<Vec<CustomFont>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let fonts_dir = get_fonts_dir(&app)?;

    let mut fonts = Vec::new();
    if let Ok(entries) = fs::read_dir(&fonts_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    let filename = entry.file_name().to_string_lossy().to_string();
                    let ext = Path::new(&filename)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    if get_font_format(ext).is_some() {
                        let family_name = Path::new(&filename)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        fonts.push(CustomFont {
                            filename: filename.clone(),
                            family_name,
                            file_size: metadata.len(),
                            format: ext.to_lowercase(),
                        });
                    }
                }
            }
        }
    }

    // 按文件名排序，保证顺序稳定
    fonts.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(ApiResponse::success(fonts))
}

/// 上传字体文件到字体目录
///
/// `src_path`：源文件绝对路径（由前端 Tauri dialog 选择）
/// 复制到 %APPDATA%/NexTerm/fonts/<原文件名>
#[tauri::command]
pub async fn font_upload(
    state: State<'_, AppState>,
    app: AppHandle,
    src_path: String,
) -> Result<ApiResponse<CustomFont>, String> {
    crate::commands::common::require_auth(&state).await?;
    let src = PathBuf::from(&src_path);
    if !src.exists() {
        return Err("源文件不存在".to_string());
    }

    let filename = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("无效的文件名")?
        .to_string();
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if get_font_format(&ext).is_none() {
        return Err(format!(
            "不支持的字体格式: .{}（仅支持 .ttf / .otf / .woff / .woff2）",
            ext
        ));
    }

    let fonts_dir = get_fonts_dir(&app)?;
    let dest = fonts_dir.join(&filename);

    // 如果已存在同名文件，覆盖
    if dest.exists() {
        fs::remove_file(&dest).map_err(|e| format!("覆盖旧字体失败: {}", e))?;
    }

    fs::copy(&src, &dest).map_err(|e| format!("复制字体文件失败: {}", e))?;

    let metadata =
        fs::metadata(&dest).map_err(|e| format!("读取字体元数据失败: {}", e))?;
    let family_name = Path::new(&filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    tracing::info!(
        "[font_commands] 字体上传成功: {} ({} bytes)",
        filename,
        metadata.len()
    );

    Ok(ApiResponse::success(CustomFont {
        filename,
        family_name,
        file_size: metadata.len(),
        format: ext,
    }))
}

/// 删除指定字体文件
///
/// 安全校验：filename 不能包含路径分隔符或 ".."
#[tauri::command]
pub async fn font_delete(
    state: State<'_, AppState>,
    app: AppHandle,
    filename: String,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    // 路径遍历攻击防护
    if filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
        || filename.is_empty()
    {
        return Err("无效的文件名".to_string());
    }

    let fonts_dir = get_fonts_dir(&app)?;
    let target = fonts_dir.join(&filename);

    if !target.exists() {
        return Err(format!("字体文件不存在: {}", filename));
    }

    fs::remove_file(&target).map_err(|e| format!("删除字体文件失败: {}", e))?;

    tracing::info!("[font_commands] 字体删除成功: {}", filename);
    Ok(ApiResponse::success(()))
}

/// 获取字体目录绝对路径（前端用于构建 @font-face URL）
#[tauri::command]
pub async fn font_get_dir(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    let fonts_dir = get_fonts_dir(&app)?;
    Ok(ApiResponse::success(
        fonts_dir.to_string_lossy().to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_font_format() {
        assert_eq!(get_font_format("ttf"), Some("truetype"));
        assert_eq!(get_font_format("TTF"), Some("truetype"));
        assert_eq!(get_font_format("otf"), Some("opentype"));
        assert_eq!(get_font_format("woff"), Some("woff"));
        assert_eq!(get_font_format("woff2"), Some("woff2"));
        assert_eq!(get_font_format("exe"), None);
        assert_eq!(get_font_format(""), None);
    }
}
