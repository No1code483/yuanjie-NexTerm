use std::path::PathBuf;

use crate::error::app_error::AppError;
use crate::models::sandbox::{PermissionProfile, SandboxConfig, SandboxType};

pub fn validate_config(config: &SandboxConfig) -> Result<(), AppError> {
    if !PathBuf::from(&config.working_dir).exists() {
        return Err(AppError::NotFound);
    }

    if config.timeout_ms == 0 || config.timeout_ms > 300_000 {
        return Err(AppError::Validation("timeout_ms 必须在 1 ~ 300000 之间".into()));
    }

    Ok(())
}

pub fn validate_file_path(path: &str, profile: &PermissionProfile, is_write: bool) -> Result<String, AppError> {
    let path = path.replace("..", "")
        .replace("\\", "/")
        .trim_start_matches('/')
        .to_string();

    if is_write {
        if !profile.can_write(&path) {
            return Err(AppError::Permission {
                resource: path,
                action: "write".into(),
            });
        }
    } else {
        if !profile.can_read(&path) {
            return Err(AppError::Permission {
                resource: path,
                action: "read".into(),
            });
        }
    }

    Ok(path)
}

pub fn validate_command(command: &str, profile: &PermissionProfile) -> Result<(), AppError> {
    let base_command = command.split_whitespace().next().unwrap_or(command);

    let dangerous: &[&str] = &["rm", "del", "shutdown", "reboot", "format", "mkfs", "dd", "fdisk"];
    if dangerous.contains(&base_command.to_lowercase().as_str()) {
        return Err(AppError::Permission {
            resource: base_command.into(),
            action: "execute".into(),
        });
    }

    if !profile.can_execute(base_command) && !profile.can_execute("*") {
        return Err(AppError::Permission {
            resource: base_command.into(),
            action: "execute".into(),
        });
    }

    Ok(())
}

pub fn sandbox_type_label(t: &SandboxType) -> &str {
    match t {
        SandboxType::FileSystem => "filesystem",
        SandboxType::Process => "process",
        SandboxType::Network => "network",
    }
}

pub fn truncate_output(output: &str, max_size: u64) -> (String, bool) {
    let bytes = output.as_bytes();
    if bytes.len() as u64 > max_size {
        let truncated = String::from_utf8_lossy(&bytes[..max_size as usize]).to_string();
        (truncated, true)
    } else {
        (output.to_string(), false)
    }
}