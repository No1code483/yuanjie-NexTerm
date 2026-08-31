//! A5 离线与同步机制 - 传输后端 trait
//!
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.1.3
//!
//! 当前实现（Phase 1 骨架）：
//! - TransportBackend trait：统一传输接口（push/pull/test_connection/list）
//! - SyncTransportConfig：传输配置（backend_type / endpoint / credentials）
//! - 尚未实现具体适配器（WebDAV/S3 在 Phase 2 实现）
//!
//! Phase 2 将实现：
//! - WebDavBackend：通过 HTTP PUT/GET 同步到 WebDAV 服务器
//! - S3Backend：通过 S3 API 同步到 R2/MinIO/阿里云 OSS

use serde::{Deserialize, Serialize};

use crate::error::app_error::AppError;
use crate::services::sync_service::SyncQueueItem;

/// 传输后端类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransportBackendType {
    Webdav,
    S3,
    Http,
    Local,
}

impl TransportBackendType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Webdav => "webdav",
            Self::S3 => "s3",
            Self::Http => "http",
            Self::Local => "local",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "webdav" => Some(Self::Webdav),
            "s3" => Some(Self::S3),
            "http" => Some(Self::Http),
            "local" => Some(Self::Local),
            _ => None,
        }
    }
}

/// 传输配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTransportConfig {
    pub backend_type: TransportBackendType,
    pub endpoint: String,
    pub bucket: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub path_prefix: Option<String>,
}

/// 传输结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResult {
    pub items_pushed: usize,
    pub items_failed: usize,
    pub errors: Vec<String>,
}

/// 拉取结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullResult {
    pub items_pulled: usize,
    pub conflicts: Vec<String>,
}

/// 传输后端 trait
///
/// Phase 2 将为 WebDAV 和 S3 分别实现此 trait。
/// Phase 1 仅定义接口，不提供具体实现。
pub trait TransportBackend: Send + Sync {
    /// 后端名称
    fn name(&self) -> &str;

    /// 测试连接是否可用
    fn test_connection(&self) -> impl std::future::Future<Output = Result<bool, AppError>> + Send;

    /// 推送变更到远端
    fn push(
        &self,
        items: &[SyncQueueItem],
    ) -> impl std::future::Future<Output = Result<PushResult, AppError>> + Send;

    /// 拉取远端变更
    fn pull(
        &self,
        since: Option<&str>,
    ) -> impl std::future::Future<Output = Result<PullResult, AppError>> + Send;

    /// 获取远端版本号（用于检测是否有新变更）
    fn get_remote_version(
        &self,
    ) -> impl std::future::Future<Output = Result<String, AppError>> + Send;
}

/// 本地文件系统后端（用于测试和单设备备份）
///
/// 将同步数据序列化为 JSON 文件存储到本地目录。
/// 不进行真正的跨设备同步，但可用于验证同步管线。
pub struct LocalFileBackend {
    pub storage_dir: std::path::PathBuf,
}

impl LocalFileBackend {
    pub fn new(storage_dir: std::path::PathBuf) -> Self {
        Self { storage_dir }
    }

    fn get_sync_file(&self) -> std::path::PathBuf {
        self.storage_dir.join("sync_data.json")
    }
}

impl TransportBackend for LocalFileBackend {
    fn name(&self) -> &str {
        "local_file"
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        std::fs::create_dir_all(&self.storage_dir).map_err(AppError::FileSystem)?;
        Ok(true)
    }

    async fn push(&self, items: &[SyncQueueItem]) -> Result<PushResult, AppError> {
        std::fs::create_dir_all(&self.storage_dir).map_err(AppError::FileSystem)?;

        let file_path = self.get_sync_file();
        let existing = std::fs::read_to_string(&file_path).unwrap_or_else(|_| "[]".to_string());

        let mut all_items: Vec<serde_json::Value> =
            serde_json::from_str(&existing).unwrap_or_default();

        let mut pushed = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for item in items {
            match serde_json::to_value(item) {
                Ok(v) => {
                    all_items.push(v);
                    pushed += 1;
                }
                Err(e) => {
                    failed += 1;
                    errors.push(format!("序列化失败: {}", e));
                }
            }
        }

        let json = serde_json::to_string_pretty(&all_items)
            .map_err(|e| AppError::Internal(format!("JSON 序列化失败: {}", e)))?;

        std::fs::write(&file_path, json).map_err(AppError::FileSystem)?;

        Ok(PushResult {
            items_pushed: pushed,
            items_failed: failed,
            errors,
        })
    }

    async fn pull(&self, _since: Option<&str>) -> Result<PullResult, AppError> {
        // 本地后端的 pull 操作只是读取文件，不产生冲突
        let file_path = self.get_sync_file();
        let content = std::fs::read_to_string(&file_path).unwrap_or_else(|_| "[]".to_string());
        let items: Vec<serde_json::Value> =
            serde_json::from_str(&content).unwrap_or_default();

        Ok(PullResult {
            items_pulled: items.len(),
            conflicts: Vec::new(),
        })
    }

    async fn get_remote_version(&self) -> Result<String, AppError> {
        let file_path = self.get_sync_file();
        if file_path.exists() {
            let metadata = std::fs::metadata(&file_path).map_err(AppError::FileSystem)?;
            let modified = metadata
                .modified()
                .map_err(|e| AppError::Internal(format!("获取修改时间失败: {}", e)))?;
            Ok(format!(
                "{:?}",
                modified.duration_since(std::time::UNIX_EPOCH).unwrap_or_default()
            ))
        } else {
            Ok("none".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_backend_type_from_str() {
        assert_eq!(
            TransportBackendType::from_str("webdav"),
            Some(TransportBackendType::Webdav)
        );
        assert_eq!(
            TransportBackendType::from_str("s3"),
            Some(TransportBackendType::S3)
        );
        assert_eq!(TransportBackendType::from_str("invalid"), None);
    }

    #[test]
    fn test_transport_backend_type_as_str() {
        assert_eq!(TransportBackendType::Webdav.as_str(), "webdav");
        assert_eq!(TransportBackendType::S3.as_str(), "s3");
    }
}
