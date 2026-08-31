use tauri::State;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::file_edit::{
    FileEditContent, FileEditStatus, ReadFileRequest, WriteFileRequest,
    TableDataResult, ReadTableRequest, WriteTableRequest, ExportCsvRequest,
    PptSlidesResult, PptUpdateSlideRequest, PptReorderRequest,
    PdfSaveRequest, ImageSaveRequest, AudioSaveRequest,
};
use crate::services::file_edit_service;

/// 安全审计修复（发现 1，HIGH）：所有 fileedit_*/pdfedit_*/imageedit_*/audioedit_*/
/// tableedit_*/pptedit_* 命令原无认证，任何能注入 IPC 调用的代码均可读写任意文件。
/// 现在每个命令入口强制调用 `require_auth(&state).await?` 校验当前用户 token。
///
/// 注：同步签名改为 async fn 以访问 AppState；Tauri 对前端透明（仍返回 Promise）。

#[tauri::command]
pub async fn fileedit_read(
    state: State<'_, AppState>,
    request: ReadFileRequest,
) -> Result<ApiResponse<FileEditContent>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::read_file_for_edit(&request.path, &request.ext) {
        Ok(content) => Ok(ApiResponse::success(content)),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn pdfedit_save(
    state: State<'_, AppState>,
    request: PdfSaveRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::pdf_save(&request.path, &request.base64_data) {
        Ok(()) => Ok(ApiResponse::success_msg("PDF 已保存")),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn imageedit_save(
    state: State<'_, AppState>,
    request: ImageSaveRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::image_save(&request.path, &request.base64_data) {
        Ok(()) => Ok(ApiResponse::success_msg("图片已保存")),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn audioedit_save(
    state: State<'_, AppState>,
    request: AudioSaveRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::audio_save(&request.path, &request.base64_data) {
        Ok(()) => Ok(ApiResponse::success_msg("音频已保存")),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn fileedit_write(
    state: State<'_, AppState>,
    request: WriteFileRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::write_file_after_edit(&request.path, &request.content, &request.ext) {
        Ok(()) => Ok(ApiResponse::success_msg("保存成功")),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn fileedit_get_status(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<FileEditStatus>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::get_edit_status(&path) {
        Ok(status) => Ok(ApiResponse::success(status)),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn tableedit_read(
    state: State<'_, AppState>,
    request: ReadTableRequest,
) -> Result<ApiResponse<TableDataResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::read_table_for_edit(&request.path, &request.ext) {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn tableedit_write(
    state: State<'_, AppState>,
    request: WriteTableRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::write_table_after_edit(&request.path, &request.ext, &request.sheets) {
        Ok(()) => Ok(ApiResponse::success_msg("保存成功")),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn tableedit_export_csv(
    state: State<'_, AppState>,
    request: ExportCsvRequest,
) -> Result<ApiResponse<String>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::export_table_csv(&request.path, &request.ext, &request.rows) {
        Ok(csv_path) => Ok(ApiResponse {
            code: 0,
            message: "导出成功".into(),
            data: Some(csv_path),
        }),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn pptedit_get_slides(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<PptSlidesResult>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::get_pptx_slides(&path) {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn pptedit_update_slide(
    state: State<'_, AppState>,
    request: PptUpdateSlideRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::update_pptx_slide_text(
        &request.path,
        request.slide_index,
        &request.text_content,
    ) {
        Ok(()) => Ok(ApiResponse::success_msg("保存成功")),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn pptedit_add_slide(
    state: State<'_, AppState>,
    path: String,
) -> Result<ApiResponse<usize>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::add_pptx_slide(&path) {
        Ok(new_index) => Ok(ApiResponse {
            code: 0,
            message: "已添加新幻灯片".into(),
            data: Some(new_index),
        }),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn pptedit_delete_slide(
    state: State<'_, AppState>,
    path: String,
    slide_index: usize,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::delete_pptx_slide(&path, slide_index) {
        Ok(()) => Ok(ApiResponse::success_msg("已删除幻灯片")),
        Err(e) => Err(format!("{}", e)),
    }
}

#[tauri::command]
pub async fn pptedit_reorder(
    state: State<'_, AppState>,
    request: PptReorderRequest,
) -> Result<ApiResponse<()>, String> {
    crate::commands::common::require_auth(&state).await?;
    match file_edit_service::reorder_pptx_slides(
        &request.path,
        request.slide_index,
        request.new_index,
    ) {
        Ok(()) => Ok(ApiResponse::success_msg("已调整顺序")),
        Err(e) => Err(format!("{}", e)),
    }
}
