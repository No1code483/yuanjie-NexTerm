use std::path::PathBuf;
use std::time::Instant;

use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;

use crate::error::app_error::AppError;
use crate::models::sandbox::{
    ExecuteRequest, ExecuteResult, FileOperationRequest, FileOperationResult, PermissionProfile,
};
use crate::sandbox::primitives::{truncate_output, validate_command, validate_file_path};

pub struct SandboxExecutor;

impl SandboxExecutor {
    pub async fn execute_command(
        req: &ExecuteRequest,
        profile: &PermissionProfile,
        timeout_ms: u64,
    ) -> Result<ExecuteResult, AppError> {
        validate_command(&req.command, profile)?;

        let start = Instant::now();

        let mut cmd = TokioCommand::new(&req.command);
        cmd.args(&req.args);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(dir) = &req.working_dir {
            if profile.can_read(dir) {
                cmd.current_dir(dir);
            }
        }

        for (key, value) in req.env.as_ref().unwrap_or(&vec![]) {
            cmd.env(key, value);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| AppError::Internal(format!("命令执行失败: {}", e)))?;

        if let Some(stdin_data) = &req.stdin {
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(stdin_data.as_bytes())
                    .await
                    .map_err(|e| AppError::Internal(format!("写入stdin失败: {}", e)))?;
            }
        }

        let timeout_dur = std::time::Duration::from_millis(timeout_ms);

        let output = timeout(timeout_dur, child.wait_with_output())
            .await
            .map_err(|_| AppError::Internal("命令执行超时".into()))?
            .map_err(|e| AppError::Internal(format!("等待进程退出失败: {}", e)))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let raw_stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let max_size = profile.max_output_size;
        let (stdout, truncated_out) = truncate_output(&raw_stdout, max_size);
        let (stderr, truncated_err) = truncate_output(&raw_stderr, max_size);

        Ok(ExecuteResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            duration_ms,
            truncated: truncated_out || truncated_err,
        })
    }

    pub async fn read_file(
        req: &FileOperationRequest,
        profile: &PermissionProfile,
        working_dir: &str,
    ) -> Result<FileOperationResult, AppError> {
        let clean_path = validate_file_path(&req.path, profile, false)?;
        let full_path = PathBuf::from(working_dir).join(&clean_path);

        let metadata = fs::metadata(&full_path)
            .await
            .map_err(|_| AppError::NotFound)?;

        let file_size = metadata.len();

        if file_size > profile.max_file_size {
            return Err(AppError::Validation(format!(
                "文件过大: {} bytes (限制 {})",
                file_size, profile.max_file_size
            )));
        }

        let mut file = fs::File::open(&full_path)
            .await
            .map_err(|e| AppError::Internal(format!("打开文件失败: {}", e)))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(|e| AppError::Internal(format!("读取文件失败: {}", e)))?;

        Ok(FileOperationResult {
            success: true,
            content: Some(content),
            error: None,
            file_size: Some(file_size),
        })
    }

    pub async fn write_file(
        req: &FileOperationRequest,
        profile: &PermissionProfile,
        working_dir: &str,
    ) -> Result<FileOperationResult, AppError> {
        let clean_path = validate_file_path(&req.path, profile, true)?;
        let full_path = PathBuf::from(working_dir).join(&clean_path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::Internal(format!("创建目录失败: {}", e)))?;
        }

        let content = req.content.as_ref().ok_or(AppError::Validation("缺少内容".into()))?;

        let content_size = content.as_bytes().len() as u64;
        if content_size > profile.max_file_size {
            return Err(AppError::Validation(format!(
                "内容过大: {} bytes (限制 {})",
                content_size, profile.max_file_size
            )));
        }

        fs::write(&full_path, content)
            .await
            .map_err(|e| AppError::Internal(format!("写入文件失败: {}", e)))?;

        Ok(FileOperationResult {
            success: true,
            content: None,
            error: None,
            file_size: Some(content_size),
        })
    }

    pub async fn delete_file(
        path: &str,
        profile: &PermissionProfile,
        working_dir: &str,
    ) -> Result<FileOperationResult, AppError> {
        let clean_path = validate_file_path(path, profile, true)?;
        let full_path = PathBuf::from(working_dir).join(&clean_path);

        fs::remove_file(&full_path)
            .await
            .map_err(|e| AppError::Internal(format!("删除文件失败: {}", e)))?;

        Ok(FileOperationResult {
            success: true,
            content: None,
            error: None,
            file_size: None,
        })
    }

    pub async fn list_files(
        path: &str,
        profile: &PermissionProfile,
        working_dir: &str,
    ) -> Result<Vec<String>, AppError> {
        let clean_path = validate_file_path(path, profile, false)?;
        let full_path = PathBuf::from(working_dir).join(&clean_path);

        let mut entries = fs::read_dir(&full_path)
            .await
            .map_err(|e| AppError::Internal(format!("读取目录失败: {}", e)))?;

        let mut names = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| AppError::Internal(format!("遍历目录失败: {}", e)))?
        {
            names.push(entry.file_name().to_string_lossy().to_string());
        }
        names.sort();
        Ok(names)
    }
}