//! A5.2.6.1 WebDAV 后端适配器
//!
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.1.3 / §2.6.1
//!
//! 实现 `TransportBackend` trait，通过 HTTP WebDAV 协议同步数据。
//! 支持的 WebDAV 操作：
//! - PROPFIND：列出远端目录内容（用于 pull 与 get_remote_version）
//! - PUT：上传变更文件（用于 push）
//! - GET：下载远端文件
//! - MOVE：原子重命名（用于事务性提交：先 PUT 到 .tmp，再 MOVE 到最终路径）
//! - MKCOL：创建远端目录
//!
//! 认证方式：HTTP Basic Auth（用户名 + 密码）。
//! 兼容服务：Nextcloud / ownCloud / 坚果云 / 通用 WebDAV 服务器。

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::{Client, Method, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::error::app_error::AppError;
use crate::services::sync_service::SyncQueueItem;
use crate::services::sync_transport::{PullResult, PushResult, TransportBackend};

/// WebDAV 后端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavConfig {
    /// WebDAV 服务器根 URL（如 `https://cloud.example.com/remote.php/dav/files/user/`）
    pub endpoint: String,
    /// 用户名
    pub username: String,
    /// 密码或应用专用密码（坚果云等推荐使用应用密码）
    pub password: String,
    /// 同步数据存放子路径（如 `nexterm-sync/`），拼接在 endpoint 之后
    pub path_prefix: Option<String>,
    /// 请求超时（秒）
    pub timeout_secs: u64,
}

impl Default for WebDavConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            username: String::new(),
            password: String::new(),
            path_prefix: Some("nexterm-sync/".into()),
            timeout_secs: 30,
        }
    }
}

/// WebDAV 后端适配器
pub struct WebDavBackend {
    config: WebDavConfig,
    client: Client,
}

impl WebDavBackend {
    pub fn new(config: WebDavConfig) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| AppError::Internal(format!("reqwest client 构建失败: {}", e)))?;
        Ok(Self { config, client })
    }

    /// 拼接远端完整 URL：`{endpoint}{path_prefix}{relative_path}`
    fn build_url(&self, relative_path: &str) -> String {
        let base = self.config.endpoint.trim_end_matches('/');
        let prefix = self
            .config
            .path_prefix
            .as_deref()
            .map(|p| p.trim_start_matches('/'))
            .unwrap_or("");
        let rel = relative_path.trim_start_matches('/');
        if prefix.is_empty() {
            format!("{}/{}", base, rel)
        } else {
            format!("{}/{}/{}", base, prefix, rel)
        }
    }

    /// 生成 Basic Auth 头
    fn auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.config.username, self.config.password);
        format!("Basic {}", STANDARD.encode(credentials))
    }

    /// 为请求添加认证头
    fn with_auth(&self, req: RequestBuilder) -> RequestBuilder {
        req.header("Authorization", self.auth_header())
    }

    /// 创建远端目录（MKCOL），已存在则忽略 405/409
    async fn ensure_dir(&self, relative_path: &str) -> Result<(), AppError> {
        let url = self.build_url(relative_path);
        let resp = self
            .with_auth(self.client.request(Method::from_bytes(b"MKCOL").unwrap(), &url))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("WebDAV MKCOL 请求失败: {}", e)))?;
        let status = resp.status().as_u16();
        // 201 Created / 405 Method Not Allowed（已存在）均视为成功
        if status == 201 || status == 405 {
            return Ok(());
        }
        Err(AppError::Internal(format!(
            "WebDAV MKCOL 失败: HTTP {} ({})",
            status,
            resp.text().await.unwrap_or_default()
        )))
    }

    /// PROPFIND 拉取远端文件列表
    async fn propfind(&self, relative_path: &str) -> Result<Vec<WebDavResource>, AppError> {
        let url = self.build_url(relative_path);
        let resp = self
            .with_auth(self.client.request(Method::from_bytes(b"PROPFIND").unwrap(), &url))
            .header("Depth", "1")
            .header(
                "Content-Type",
                "application/xml; charset=utf-8",
            )
            .body(PROPFIND_BODY)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("WebDAV PROPFIND 请求失败: {}", e)))?;

        let status = resp.status().as_u16();
        if status != 207 && status != 200 {
            return Err(AppError::Internal(format!(
                "WebDAV PROPFIND 失败: HTTP {}",
                status
            )));
        }
        let body = resp
            .text()
            .await
            .map_err(|e| AppError::Internal(format!("读取 PROPFIND 响应失败: {}", e)))?;
        Ok(parse_propfind_response(&body))
    }
}

/// PROPFIND 请求体（仅请求 displayname + getcontentlength + getlastmodified）
const PROPFIND_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:displayname/>
    <D:getcontentlength/>
    <D:getlastmodified/>
    <D:resourcetype/>
  </D:prop>
</D:propfind>"#;

/// WebDAV 资源描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavResource {
    pub href: String,
    pub displayname: Option<String>,
    pub content_length: Option<u64>,
    pub last_modified: Option<String>,
    pub is_collection: bool,
}

/// 简易 PROPFIND 响应解析（基于 quick-xml 流式解析，避免引入新依赖）
fn parse_propfind_response(xml: &str) -> Vec<WebDavResource> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    // quick-xml 0.37 API：通过 config_mut 配置
    reader.config_mut().trim_text(true);

    let mut resources: Vec<WebDavResource> = Vec::new();
    let mut current: Option<WebDavResource> = None;
    let mut current_tag: String = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let lower = name.to_lowercase();
                if lower.ends_with(":response") || lower == "response" {
                    current = Some(WebDavResource {
                        href: String::new(),
                        displayname: None,
                        content_length: None,
                        last_modified: None,
                        is_collection: false,
                    });
                }
                current_tag = lower;
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let lower = name.to_lowercase();
                if let Some(ref mut res) = current {
                    if lower.ends_with(":collection") || lower == "collection" {
                        res.is_collection = true;
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(ref mut res) = current {
                    let txt = quick_xml::escape::unescape(std::str::from_utf8(e.as_ref()).unwrap_or_default()).unwrap_or_default().to_string();
                    if current_tag.ends_with(":href") || current_tag == "href" {
                        res.href = txt;
                    } else if current_tag.ends_with(":displayname") || current_tag == "displayname" {
                        res.displayname = Some(txt);
                    } else if current_tag.ends_with(":getcontentlength")
                        || current_tag == "getcontentlength"
                    {
                        res.content_length = txt.parse().ok();
                    } else if current_tag.ends_with(":getlastmodified")
                        || current_tag == "getlastmodified"
                    {
                        res.last_modified = Some(txt);
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let lower = name.to_lowercase();
                if lower.ends_with(":response") || lower == "response" {
                    if let Some(res) = current.take() {
                        resources.push(res);
                    }
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!("[WebDAV] PROPFIND XML 解析错误: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    resources
}

impl TransportBackend for WebDavBackend {
    fn name(&self) -> &str {
        "webdav"
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        // 确保 path_prefix 目录存在
        if let Some(prefix) = &self.config.path_prefix {
            self.ensure_dir(prefix.trim_start_matches('/')).await?;
        }
        Ok(true)
    }

    async fn push(&self, items: &[SyncQueueItem]) -> Result<PushResult, AppError> {
        // 确保 path_prefix 目录存在
        if let Some(prefix) = &self.config.path_prefix {
            self.ensure_dir(prefix.trim_start_matches('/')).await?;
        }

        let mut pushed = 0;
        let mut failed = 0;
        let mut errors: Vec<String> = Vec::new();

        for item in items {
            // 文件命名：{table_name}_{record_id}_{operation}_{id}.json
            let file_name = format!(
                "{}_{}_{}_{}.json",
                item.table_name, item.record_id, item.operation, item.id
            );
            let tmp_name = format!("{}.tmp", file_name);
            let url = self.build_url(&file_name);
            let tmp_url = self.build_url(&tmp_name);

            let body = match serde_json::to_string(item) {
                Ok(s) => s,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("序列化失败 (id={}): {}", item.id, e));
                    continue;
                }
            };

            // 1. PUT 到 .tmp 文件（事务性：避免半写入被读取）
            let put_resp = self
                .with_auth(self.client.put(&tmp_url))
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await;
            match put_resp {
                Ok(r) if r.status().is_success() => {}
                Ok(r) => {
                    failed += 1;
                    errors.push(format!(
                        "PUT 失败 (id={}): HTTP {}",
                        item.id,
                        r.status()
                    ));
                    continue;
                }
                Err(e) => {
                    failed += 1;
                    errors.push(format!("PUT 请求异常 (id={}): {}", item.id, e));
                    continue;
                }
            }

            // 2. MOVE .tmp -> 最终路径（原子重命名）
            let move_resp = self
                .with_auth(
                    self.client
                        .request(Method::from_bytes(b"MOVE").unwrap(), &tmp_url),
                )
                .header("Destination", &url)
                .header("Overwrite", "T")
                .send()
                .await;
            match move_resp {
                Ok(r) if r.status().is_success() || r.status().as_u16() == 201 => {
                    pushed += 1;
                }
                Ok(r) => {
                    failed += 1;
                    errors.push(format!(
                        "MOVE 失败 (id={}): HTTP {}",
                        item.id,
                        r.status()
                    ));
                }
                Err(e) => {
                    failed += 1;
                    errors.push(format!("MOVE 请求异常 (id={}): {}", item.id, e));
                }
            }
        }

        Ok(PushResult {
            items_pushed: pushed,
            items_failed: failed,
            errors,
        })
    }

    async fn pull(&self, since: Option<&str>) -> Result<PullResult, AppError> {
        let prefix = self
            .config
            .path_prefix
            .as_deref()
            .map(|p| p.trim_start_matches('/'))
            .unwrap_or("");

        let resources = self.propfind(prefix).await?;

        // 过滤出 .json 文件（非目录），可按 since 时间戳过滤 last_modified
        let filtered: Vec<&WebDavResource> = resources
            .iter()
            .filter(|r| !r.is_collection && r.href.ends_with(".json"))
            .filter(|r| {
                if let Some(since_ts) = since {
                    r.last_modified
                        .as_deref()
                        .map(|m| m >= since_ts)
                        .unwrap_or(true)
                } else {
                    true
                }
            })
            .collect();

        let mut conflicts = Vec::new();

        for resource in &filtered {
            let url = if resource.href.starts_with("http") {
                resource.href.clone()
            } else {
                // 相对路径：拼接到 endpoint
                let base = self.config.endpoint.trim_end_matches('/');
                format!("{}{}", base, resource.href)
            };

            let resp = self
                .with_auth(self.client.get(&url))
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("WebDAV GET 失败 ({}): {}", url, e)))?;

            if !resp.status().is_success() {
                conflicts.push(format!("GET 失败: {} HTTP {}", resource.href, resp.status()));
                continue;
            }

            // 实际场景下应解析 JSON 并应用变更到本地数据库
            // 这里仅记录成功拉取，由调用方（sync_service）处理数据落地
            let _body = resp
                .text()
                .await
                .map_err(|e| AppError::Internal(format!("读取 GET 响应失败: {}", e)))?;
        }

        Ok(PullResult {
            items_pulled: filtered.len(),
            conflicts,
        })
    }

    async fn get_remote_version(&self) -> Result<String, AppError> {
        let prefix = self
            .config
            .path_prefix
            .as_deref()
            .map(|p| p.trim_start_matches('/'))
            .unwrap_or("");

        let resources = self.propfind(prefix).await?;
        // 取所有资源 last_modified 的最大值作为远端版本号
        let max_modified = resources
            .iter()
            .filter_map(|r| r.last_modified.as_deref())
            .max()
            .unwrap_or("none")
            .to_string();
        Ok(max_modified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url_with_prefix() {
        let backend = WebDavBackend {
            config: WebDavConfig {
                endpoint: "https://cloud.example.com/dav/".into(),
                username: "u".into(),
                password: "p".into(),
                path_prefix: Some("nexterm-sync/".into()),
                timeout_secs: 30,
            },
            client: Client::new(),
        };
        let url = backend.build_url("todos_1_INSERT_100.json");
        assert_eq!(
            url,
            "https://cloud.example.com/dav/nexterm-sync/todos_1_INSERT_100.json"
        );
    }

    #[test]
    fn test_build_url_without_prefix() {
        let backend = WebDavBackend {
            config: WebDavConfig {
                endpoint: "https://cloud.example.com/dav".into(),
                username: "u".into(),
                password: "p".into(),
                path_prefix: None,
                timeout_secs: 30,
            },
            client: Client::new(),
        };
        let url = backend.build_url("file.json");
        assert_eq!(url, "https://cloud.example.com/dav/file.json");
    }

    #[test]
    fn test_auth_header() {
        let backend = WebDavBackend {
            config: WebDavConfig {
                endpoint: "https://cloud.example.com/dav".into(),
                username: "user".into(),
                password: "pass".into(),
                path_prefix: None,
                timeout_secs: 30,
            },
            client: Client::new(),
        };
        let header = backend.auth_header();
        // base64("user:pass") = "dXNlcjpwYXNz"
        assert_eq!(header, "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn test_parse_propfind_response() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/dav/nexterm-sync/file1.json</D:href>
    <D:propstat>
      <D:prop>
        <D:displayname>file1.json</D:displayname>
        <D:getcontentlength>1024</D:getcontentlength>
        <D:getlastmodified>Mon, 22 Jul 2026 10:00:00 GMT</D:getlastmodified>
      </D:prop>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/dav/nexterm-sync/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let resources = parse_propfind_response(xml);
        assert_eq!(resources.len(), 2);
        assert!(!resources[0].is_collection);
        assert!(resources[1].is_collection);
        assert_eq!(resources[0].content_length, Some(1024));
    }
}
