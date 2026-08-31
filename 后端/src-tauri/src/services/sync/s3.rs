//! A5.2.6.2 S3 兼容存储后端适配器
//!
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.1.3 / §2.6.2
//!
//! 实现 `TransportBackend` trait，通过 S3 兼容协议同步数据。
//! 兼容服务：AWS S3 / Cloudflare R2 / MinIO / 阿里云 OSS / 腾讯云 COS / Backblaze B2。
//!
//! 签名：AWS Signature Version 4（SigV4），自实现以避免引入 aws-sdk-s3 重型依赖。
//! 参考：https://docs.aws.amazon.com/AmazonS3/latest/API/sig-v4-header-based-auth.html
//!
//! 支持操作：
//! - PUT Object：上传变更文件
//! - GET Object：下载远端文件
//! - ListObjectsV2：列出 bucket 内对象（用于 pull 与 get_remote_version）
//! - HeadBucket：测试连接可用性

use hmac::{Hmac, Mac};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::app_error::AppError;
use crate::services::sync_service::SyncQueueItem;
use crate::services::sync_transport::{PullResult, PushResult, TransportBackend};

type HmacSha256 = Hmac<Sha256>;

/// S3 后端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// S3 兼容服务端点（如 `https://s3.amazonaws.com` 或 `https://<account>.r2.cloudflarestorage.com`）
    pub endpoint: String,
    /// Region（如 `us-east-1`，R2 用 `auto`）
    pub region: String,
    /// Bucket 名称
    pub bucket: String,
    /// Access Key ID
    pub access_key: String,
    /// Secret Access Key
    pub secret_key: String,
    /// 对象 key 前缀（如 `nexterm-sync/`）
    pub path_prefix: Option<String>,
    /// 是否使用 path-style（MinIO / 自建服务需要 true；AWS S3 / R2 用 false）
    pub path_style: bool,
    /// 请求超时（秒）
    pub timeout_secs: u64,
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            region: "us-east-1".into(),
            bucket: String::new(),
            access_key: String::new(),
            secret_key: String::new(),
            path_prefix: Some("nexterm-sync/".into()),
            path_style: false,
            timeout_secs: 30,
        }
    }
}

/// S3 后端适配器
pub struct S3Backend {
    config: S3Config,
    client: Client,
}

impl S3Backend {
    pub fn new(config: S3Config) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| AppError::Internal(format!("reqwest client 构建失败: {}", e)))?;
        Ok(Self { config, client })
    }

    /// 构建对象 URL：path-style 用 `{endpoint}/{bucket}/{key}`，virtual-hosted 用 `{bucket}.{endpoint}/{key}`
    fn build_url(&self, key: &str) -> String {
        let endpoint = self.config.endpoint.trim_end_matches('/');
        let prefix = self
            .config
            .path_prefix
            .as_deref()
            .map(|p| p.trim_start_matches('/'))
            .unwrap_or("");
        let full_key = if prefix.is_empty() {
            key.trim_start_matches('/').to_string()
        } else {
            format!("{}/{}", prefix, key.trim_start_matches('/'))
        };

        if self.config.path_style {
            format!("{}/{}/{}", endpoint, self.config.bucket, full_key)
        } else {
            // virtual-hosted-style：{bucket}.{endpoint_host}/{key}
            // 简化处理：从 endpoint 中分离 scheme 与 host
            let endpoint_no_scheme = endpoint
                .strip_prefix("https://")
                .or_else(|| endpoint.strip_prefix("http://"))
                .unwrap_or(endpoint);
            let scheme = if endpoint.starts_with("https") { "https" } else { "http" };
            format!("{}://{}.{}/{}", scheme, self.config.bucket, endpoint_no_scheme, full_key)
        }
    }

    /// SigV4 签名并发出请求
    ///
    /// 参考：https://docs.aws.amazon.com/AmazonS3/latest/API/sig-v4-header-based-auth.html
    async fn sign_and_send(
        &self,
        method: Method,
        key: &str,
        body: Vec<u8>,
        extra_headers: Vec<(String, String)>,
    ) -> Result<reqwest::Response, AppError> {
        let url = self.build_url(key);
        let parsed = reqwest::Url::parse(&url)
            .map_err(|e| AppError::Internal(format!("URL 解析失败: {}", e)))?;

        let host = parsed
            .host_str()
            .ok_or_else(|| AppError::Internal("URL 缺少 host".into()))?;
        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        // 1. 计算请求体 SHA256
        let payload_hash = hex::encode(Sha256::digest(&body));

        // 2. 构建 Canonical 请求
        let canonical_uri = if self.config.path_style {
            format!("/{}/{}", self.config.bucket, key.trim_start_matches('/'))
        } else {
            format!("/{}", key.trim_start_matches('/'))
        };

        let mut canonical_headers = Vec::new();
        let mut signed_headers: Vec<String> = Vec::new();
        // host 头
        canonical_headers.push(format!("host:{}\n", host));
        signed_headers.push("host".to_string());
        // x-amz-content-sha256 头
        canonical_headers.push(format!("x-amz-content-sha256:{}\n", payload_hash));
        signed_headers.push("x-amz-content-sha256".to_string());
        // x-amz-date 头
        canonical_headers.push(format!("x-amz-date:{}\n", amz_date));
        signed_headers.push("x-amz-date".to_string());

        // 额外头
        for (k, v) in &extra_headers {
            let lower_k = k.to_lowercase();
            canonical_headers.push(format!("{}:{}\n", lower_k, v));
            signed_headers.push(lower_k);
        }

        signed_headers.sort();
        let signed_headers_str = signed_headers.join(";");

        let canonical_request = format!(
            "{}\n{}\n\n{}\n{}\n{}",
            method.as_str(),
            canonical_uri,
            canonical_headers.join(""),
            signed_headers_str,
            payload_hash
        );

        // 3. 构建待签字符串
        let credential_scope = format!("{}/{}/s3/aws4_request", date_stamp, self.config.region);
        let hashed_canonical = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date, credential_scope, hashed_canonical
        );

        // 4. 计算签名
        let signing_key = derive_signing_key(
            &self.config.secret_key,
            &date_stamp,
            &self.config.region,
            "s3",
        );
        let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

        // 5. 构建 Authorization 头
        let auth_header = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.config.access_key, credential_scope, signed_headers_str, signature
        );

        // 6. 发送请求
        let mut req = self
            .client
            .request(method, &url)
            .header("Authorization", &auth_header)
            .header("Host", host)
            .header("x-amz-content-sha256", &payload_hash)
            .header("x-amz-date", &amz_date);

        for (k, v) in &extra_headers {
            req = req.header(k, v);
        }

        req = req.body(body);
        req.send()
            .await
            .map_err(|e| AppError::Internal(format!("S3 请求失败: {}", e)))
    }

    /// ListObjectsV2 - 列出 bucket 内的对象
    async fn list_objects(&self) -> Result<Vec<S3Object>, AppError> {
        // ListObjectsV2 走 GET /?list-type=2，签名时需要包含 query string
        let url_str = self.build_url("");
        let parsed = reqwest::Url::parse(&url_str)
            .map_err(|e| AppError::Internal(format!("URL 解析失败: {}", e)))?;

        let host = parsed.host_str().ok_or_else(|| AppError::Internal("URL 缺少 host".into()))?;
        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        let prefix = self
            .config
            .path_prefix
            .as_deref()
            .map(|p| p.trim_start_matches('/'))
            .unwrap_or("");

        // ListObjectsV2 查询参数
        let canonical_querystring = if prefix.is_empty() {
            "list-type=2".to_string()
        } else {
            format!("list-type=2&prefix={}", urlencoding(prefix))
        };

        let canonical_uri = if self.config.path_style {
            format!("/{}", self.config.bucket)
        } else {
            "/".to_string()
        };

        let canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            host,
            EMPTY_PAYLOAD_HASH,
            amz_date
        );
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";

        let canonical_request = format!(
            "GET\n{}\n{}\n{}\n{}\n{}",
            canonical_uri,
            canonical_querystring,
            canonical_headers,
            signed_headers,
            EMPTY_PAYLOAD_HASH
        );

        let credential_scope = format!("{}/{}/s3/aws4_request", date_stamp, self.config.region);
        let hashed_canonical = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date, credential_scope, hashed_canonical
        );
        let signing_key = derive_signing_key(
            &self.config.secret_key,
            &date_stamp,
            &self.config.region,
            "s3",
        );
        let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));
        let auth_header = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.config.access_key, credential_scope, signed_headers, signature
        );

        let full_url = if self.config.path_style {
            format!("{}://{}/{}?{}", parsed.scheme(), host, self.config.bucket, canonical_querystring)
        } else {
            format!("{}://{}?{}", parsed.scheme(), host, canonical_querystring)
        };

        let resp = self
            .client
            .get(&full_url)
            .header("Authorization", &auth_header)
            .header("Host", host)
            .header("x-amz-content-sha256", EMPTY_PAYLOAD_HASH)
            .header("x-amz-date", &amz_date)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("S3 ListObjects 请求失败: {}", e)))?;

        if !resp.status().is_success() {
            return Err(AppError::Internal(format!(
                "S3 ListObjects 失败: HTTP {} {}",
                resp.status(),
                resp.text().await.unwrap_or_default()
            )));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| AppError::Internal(format!("读取 ListObjects 响应失败: {}", e)))?;

        Ok(parse_list_objects_response(&body))
    }
}

/// 空 payload 的 SHA256（"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"）
const EMPTY_PAYLOAD_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// S3 对象描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Object {
    pub key: String,
    pub size: u64,
    pub last_modified: String,
    pub etag: Option<String>,
}

/// HMAC-SHA256 计算
fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key 长度任意");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// SigV4 派生签名密钥
fn derive_signing_key(secret_key: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("AWS4{}", secret_key).as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

/// 简易 URL 编码（仅对 ListObjectsV2 的 prefix 参数使用）
fn urlencoding(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b'/' => out.push_str("%2F"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// 简易 ListObjectsV2 响应解析
fn parse_list_objects_response(xml: &str) -> Vec<S3Object> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    // quick-xml 0.37 API：通过 config_mut 配置
    reader.config_mut().trim_text(true);

    let mut objects: Vec<S3Object> = Vec::new();
    let mut current: Option<S3Object> = None;
    let mut current_tag: String = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let lower = name.to_lowercase();
                if lower.ends_with(":contents") || lower == "contents" {
                    current = Some(S3Object {
                        key: String::new(),
                        size: 0,
                        last_modified: String::new(),
                        etag: None,
                    });
                }
                current_tag = lower;
            }
            Ok(Event::Text(e)) => {
                if let Some(ref mut obj) = current {
                    let txt = quick_xml::escape::unescape(std::str::from_utf8(e.as_ref()).unwrap_or_default()).unwrap_or_default().to_string();
                    if current_tag.ends_with(":key") || current_tag == "key" {
                        obj.key = txt;
                    } else if current_tag.ends_with(":size") || current_tag == "size" {
                        obj.size = txt.parse().unwrap_or(0);
                    } else if current_tag.ends_with(":lastmodified")
                        || current_tag == "lastmodified"
                    {
                        obj.last_modified = txt;
                    } else if current_tag.ends_with(":etag") || current_tag == "etag" {
                        obj.etag = Some(txt);
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let lower = name.to_lowercase();
                if lower.ends_with(":contents") || lower == "contents" {
                    if let Some(obj) = current.take() {
                        objects.push(obj);
                    }
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!("[S3] ListObjects XML 解析错误: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    objects
}

impl TransportBackend for S3Backend {
    fn name(&self) -> &str {
        "s3"
    }

    async fn test_connection(&self) -> Result<bool, AppError> {
        // HeadBucket：通过 ListObjectsV2 验证凭证与 bucket 可达性
        let objects = self.list_objects().await?;
        tracing::info!("[S3] 连接测试成功，bucket 内已有 {} 个对象", objects.len());
        Ok(true)
    }

    async fn push(&self, items: &[SyncQueueItem]) -> Result<PushResult, AppError> {
        let mut pushed = 0;
        let mut failed = 0;
        let mut errors: Vec<String> = Vec::new();

        for item in items {
            let key = format!(
                "{}_{}_{}_{}.json",
                item.table_name, item.record_id, item.operation, item.id
            );
            let body = match serde_json::to_string(item) {
                Ok(s) => s.into_bytes(),
                Err(e) => {
                    failed += 1;
                    errors.push(format!("序列化失败 (id={}): {}", item.id, e));
                    continue;
                }
            };

            let extra_headers: Vec<(String, String)> = vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Content-Length".to_string(), body.len().to_string()),
            ];

            match self
                .sign_and_send(Method::PUT, &key, body, extra_headers)
                .await
            {
                Ok(r) if r.status().is_success() => {
                    pushed += 1;
                }
                Ok(r) => {
                    failed += 1;
                    errors.push(format!(
                        "PUT 失败 (id={}): HTTP {}",
                        item.id,
                        r.status()
                    ));
                }
                Err(e) => {
                    failed += 1;
                    errors.push(format!("PUT 异常 (id={}): {}", item.id, e));
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
        let objects = self.list_objects().await?;
        let filtered: Vec<&S3Object> = objects
            .iter()
            .filter(|o| o.key.ends_with(".json"))
            .filter(|o| {
                if let Some(since_ts) = since {
                    o.last_modified.as_str() >= since_ts
                } else {
                    true
                }
            })
            .collect();

        let mut conflicts = Vec::new();

        for obj in &filtered {
            match self
                .sign_and_send(Method::GET, &obj.key, Vec::new(), Vec::new())
                .await
            {
                Ok(r) if r.status().is_success() => {
                    // 实际场景下应解析 JSON 并应用变更到本地数据库
                    let _body = r.text().await.unwrap_or_default();
                }
                Ok(r) => {
                    conflicts.push(format!("GET 失败: {} HTTP {}", obj.key, r.status()));
                }
                Err(e) => {
                    conflicts.push(format!("GET 异常: {} {}", obj.key, e));
                }
            }
        }

        Ok(PullResult {
            items_pulled: filtered.len(),
            conflicts,
        })
    }

    async fn get_remote_version(&self) -> Result<String, AppError> {
        let objects = self.list_objects().await?;
        let max_modified = objects
            .iter()
            .map(|o| o.last_modified.as_str())
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
    fn test_derive_signing_key() {
        // AWS 标准测试向量（来自 AWS 文档）
        // https://docs.aws.amazon.com/general/latest/gr/signature-v4-test-suite.html
        let key = derive_signing_key(
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "20150830",
            "us-east-1",
            "iam",
        );
        let expected_hex = "c4afb1cc5771d871763a393e44b703571b55cc28424d1a5e86da6ed3c154a4b9";
        assert_eq!(hex::encode(&key), expected_hex);
    }

    #[test]
    fn test_urlencoding() {
        assert_eq!(urlencoding("nexterm-sync/"), "nexterm-sync%2F");
        assert_eq!(urlencoding("abc 123"), "abc%20123");
        assert_eq!(urlencoding("safe-_.~"), "safe-_.~");
    }

    #[test]
    fn test_parse_list_objects_response() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Name>example-bucket</Name>
  <Prefix>nexterm-sync/</Prefix>
  <Contents>
    <Key>nexterm-sync/todos_1_INSERT_100.json</Key>
    <LastModified>2026-07-22T10:00:00.000Z</LastModified>
    <ETag>"abc123"</ETag>
    <Size>1024</Size>
  </Contents>
  <Contents>
    <Key>nexterm-sync/journals_5_UPDATE_200.json</Key>
    <LastModified>2026-07-22T11:00:00.000Z</LastModified>
    <ETag>"def456"</ETag>
    <Size>2048</Size>
  </Contents>
</ListBucketResult>"#;
        let objects = parse_list_objects_response(xml);
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0].key, "nexterm-sync/todos_1_INSERT_100.json");
        assert_eq!(objects[0].size, 1024);
        assert_eq!(objects[1].size, 2048);
    }

    #[test]
    fn test_empty_payload_hash() {
        // 验证空字符串的 SHA256
        let hash = hex::encode(Sha256::digest(b""));
        assert_eq!(hash, EMPTY_PAYLOAD_HASH);
    }

    #[test]
    fn test_hmac_sha256() {
        // RFC 4231 测试向量 #1
        let key = b"\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b";
        let data = b"Hi There";
        let mac = hmac_sha256(key, data);
        let expected = "b0344c61d8db38535ca8afceaf0bf12b881dc200c98c3da78493e83b9e1d2e5b";
        assert_eq!(hex::encode(mac), expected);
    }

    #[test]
    fn test_s3_config_default() {
        let config = S3Config::default();
        assert_eq!(config.region, "us-east-1");
        assert_eq!(config.path_prefix.as_deref(), Some("nexterm-sync/"));
        assert!(!config.path_style);
    }
}
