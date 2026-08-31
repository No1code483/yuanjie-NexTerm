//! D1 v3.1 Task 3.2.9: WebSearch MCP 服务器
//!
//! 网页搜索与抓取（无需 API Key 的基础实现）：
//! - search：通过 DuckDuckGo Lite HTML 端点搜索（解析首条结果摘要）
//! - fetch_page：抓取指定 URL 的 HTML 文本
//! - extract_text：从给定 HTML 文本中剥离标签提取纯文本
//!
//! 如需更高质量搜索，可通过 configure({"api_key": "...", "engine": "serper"})
//! 切换到 Serper/Google API（预留路径）。

use async_trait::async_trait;
use std::net::IpAddr;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::mcp::sdk::{error_result, json_result, make_tool, text_result, BuiltinMcpServer};
use crate::models::mcp::{Tool, ToolCallResult};

pub struct WebSearchMcpServer {
    api_key: RwLock<Option<String>>,
    engine: RwLock<String>,
}

impl WebSearchMcpServer {
    pub fn new() -> Self {
        Self {
            api_key: RwLock::new(None),
            engine: RwLock::new("duckduckgo".into()),
        }
    }

    /// 极简 HTML 标签剥离
    fn strip_html(html: &str) -> String {
        let mut out = String::with_capacity(html.len());
        let mut in_tag = false;
        for ch in html.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => out.push(ch),
                _ => {}
            }
        }
        // 折叠空白
        out.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(4000)
            .collect()
    }
}

/// SSRF 防护：判断 URL 是否安全（安全审计修复发现 11，MEDIUM）
///
/// **背景**：原 `fetch_page` 工具直接 `client.get(url).send()`，无 host/IP 过滤。
/// 攻击者可让 MCP 抓取：
/// - `http://127.0.0.1:8080/admin` 访问本地服务
/// - `http://169.254.169.254/latest/meta-data/` 窃取云元数据
/// - `http://10.0.0.1/internal-api` 横向访问内网
/// - `file:///etc/passwd` 读取本地文件（reqwest 默认不支持，但应显式拦截）
///
/// **防护策略**：
/// 1. 仅允许 http/https scheme（拒绝 file://, ftp://, gopher:// 等）
/// 2. 解析 host 并 DNS 解析
/// 3. 拒绝解析到私有/保留/环回 IP 的请求
/// 4. 拒绝 IP 字面量（如 `http://127.0.0.1`）属同一规则
async fn is_safe_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("URL 解析失败: {}", e))?;

    // Scheme 白名单
    match parsed.scheme() {
        "http" | "https" => {}
        other => return Err(format!("不允许的 scheme: {}（仅 http/https）", other)),
    }

    let host = parsed.host_str().ok_or_else(|| "URL 缺少 host".to_string())?;
    let port = parsed.port_or_known_default().unwrap_or(80);

    // DNS 解析并检查每个 IP
    let target = format!("{}:{}", host, port);
    let socket_addrs = tokio::net::lookup_host(&target)
        .await
        .map_err(|e| format!("DNS 解析失败: {}", e))?;

    for addr in socket_addrs {
        let ip = addr.ip();
        if !is_safe_ip(&ip) {
            return Err(format!(
                "目标主机 {} 解析到不安全的 IP: {}（私有/保留/环回/元数据地址被禁止）",
                host, ip
            ));
        }
    }

    Ok(())
}

/// 判断 IP 是否安全（非私有/保留/环回/元数据地址）
fn is_safe_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            // 环回 127.0.0.0/8
            if v4.is_loopback() {
                return false;
            }
            // 私有 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
            if v4.is_private() {
                return false;
            }
            // 链路本地 169.254.0.0/16（含云元数据 169.254.169.254）
            if v4.is_link_local() {
                return false;
            }
            // 未指定 0.0.0.0/8
            if v4.is_unspecified() {
                return false;
            }
            // 多播 224.0.0.0/4
            if v4.is_multicast() {
                return false;
            }
            // 保留 240.0.0.0/4（含 255.255.255.255 广播）
            // 注：`Ipv4Addr::is_reserved()` 在 stable Rust 中尚未稳定（feature `ip`），
            // 改用手动 octets 检查（首字节 >= 240 即为保留段）
            if v4.octets()[0] >= 240 {
                return false;
            }
            // 显式拦截 AWS/Azure/GCP 元数据端点（双保险，已在 is_link_local 内）
            if v4.octets() == [169, 254, 169, 254] {
                return false;
            }
            true
        }
        IpAddr::V6(v6) => {
            // 环回 ::1
            if v6.is_loopback() {
                return false;
            }
            // 未指定 ::
            if v6.is_unspecified() {
                return false;
            }
            // 多播 ff00::/8
            if v6.is_multicast() {
                return false;
            }
            // 唯一本地地址 fc00::/7（IPv6 私有段）
            if (v6.segments()[0] & 0xfe00) == 0xfc00 {
                return false;
            }
            // 链路本地 fe80::/10
            if (v6.segments()[0] & 0xffc0) == 0xfe80 {
                return false;
            }
            true
        }
    }
}

#[async_trait]
impl BuiltinMcpServer for WebSearchMcpServer {
    fn name(&self) -> &str {
        "websearch"
    }

    fn description(&self) -> &str {
        "WebSearch MCP — 网页搜索与抓取（search/fetch_page/extract_text）"
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "search",
                "搜索网页（返回结果标题/摘要/链接）",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "limit": {"type": "integer", "default": 5}
                    },
                    "required": ["query"]
                }),
            ),
            make_tool(
                "fetch_page",
                "抓取指定 URL 的页面文本",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string"},
                        "max_chars": {"type": "integer", "default": 4000}
                    },
                    "required": ["url"]
                }),
            ),
            make_tool(
                "extract_text",
                "从给定 HTML 提取纯文本",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "html": {"type": "string"}
                    },
                    "required": ["html"]
                }),
            ),
        ]
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        args: Option<serde_json::Value>,
    ) -> Result<ToolCallResult, AppError> {
        let args = args.unwrap_or(serde_json::json!({}));
        match tool_name {
            "search" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 query".into()))?;
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(5) as usize;
                let engine = self.engine.read().await.clone();
                let api_key = self.api_key.read().await.clone();
                if engine == "serper" && api_key.is_some() {
                    // 预留：Serper API 高质量搜索路径
                    let client = reqwest::Client::new();
                    let resp = client
                        .post("https://google.serper.dev/search")
                        .header("X-API-KEY", api_key.unwrap())
                        .json(&serde_json::json!({"q": query, "num": limit}))
                        .send()
                        .await
                        .map_err(|e| AppError::Internal(format!("Serper 请求失败: {}", e)))?;
                    let body: serde_json::Value = resp
                        .json()
                        .await
                        .map_err(|e| AppError::Internal(format!("解析失败: {}", e)))?;
                    let results: Vec<serde_json::Value> = body
                        .get("organic")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .take(limit)
                                .map(|r| {
                                    serde_json::json!({
                                        "title": r.get("title"),
                                        "link": r.get("link"),
                                        "snippet": r.get("snippet")
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    return Ok(json_result(&serde_json::json!({
                        "engine": "serper",
                        "results": results,
                        "count": results.len()
                    })));
                }
                // 默认 DuckDuckGo Lite（无需 API Key）
                let client = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::limited(5))
                    .build()
                    .map_err(|e| AppError::Internal(format!("构建客户端失败: {}", e)))?;
                let resp = client
                    .get("https://lite.duckduckgo.com/lite/")
                    .query(&[("q", query)])
                    .header("User-Agent", "NexTerm-YuanCode-MCP")
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("搜索请求失败: {}", e)))?;
                let html = resp
                    .text()
                    .await
                    .map_err(|e| AppError::Internal(format!("读取响应失败: {}", e)))?;
                // 极简结果解析：抽取 <a class="result-link" href="...">标题</a>
                let mut results: Vec<serde_json::Value> = Vec::new();
                for cap in regex::Regex::new(r#"<a[^>]*class="result-link"[^>]*href="([^"]+)"[^>]*>([^<]+)</a>"#)
                    .unwrap()
                    .captures_iter(&html)
                {
                    results.push(serde_json::json!({
                        "title": cap[2].trim(),
                        "link": cap[1].trim(),
                        "snippet": ""
                    }));
                    if results.len() >= limit {
                        break;
                    }
                }
                Ok(json_result(&serde_json::json!({
                    "engine": "duckduckgo",
                    "query": query,
                    "results": results,
                    "count": results.len()
                })))
            }
            "fetch_page" => {
                let url = args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 url".into()))?;

                // SSRF 防护：抓取前先校验 URL（安全审计修复发现 11）
                if let Err(reason) = is_safe_url(url).await {
                    return Ok(error_result(&format!(
                        "URL 安全检查失败: {}（SSRF 防护：禁止访问私有/保留/环回/元数据地址）",
                        reason
                    )));
                }

                let max_chars = args
                    .get("max_chars")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(4000) as usize;
                let client = reqwest::Client::builder()
                    // 自定义重定向策略：每次重定向都重新做 SSRF 校验
                    .redirect(reqwest::redirect::Policy::custom(move |attempt| {
                        // 同步上下文：用 std::net 解析重定向 URL 的 host
                        // 注：异步 DNS 解析无法在此回调内执行，降级为 host 字符串 + IP 字面量检查
                        if let Ok(parsed) = reqwest::Url::parse(attempt.url().as_str()) {
                            if let Some(host) = parsed.host_str() {
                                // 拒绝 IP 字面量（最常见的 SSRF 重定向攻击向量）
                                if let Ok(ip) = host.parse::<IpAddr>() {
                                    if !is_safe_ip(&ip) {
                                        return attempt.stop();
                                    }
                                }
                            }
                        }
                        if attempt.previous().len() >= 5 {
                            attempt.stop()
                        } else {
                            attempt.follow()
                        }
                    }))
                    .build()
                    .map_err(|e| AppError::Internal(format!("构建客户端失败: {}", e)))?;
                let resp = client
                    .get(url)
                    .header("User-Agent", "NexTerm-YuanCode-MCP")
                    .send()
                    .await
                    .map_err(|e| AppError::Internal(format!("抓取失败: {}", e)))?;
                let status = resp.status();
                let html = resp
                    .text()
                    .await
                    .map_err(|e| AppError::Internal(format!("读取响应失败: {}", e)))?;
                if !status.is_success() {
                    return Ok(error_result(&format!("HTTP {}", status)));
                }
                let text = Self::strip_html(&html);
                let truncated = text.chars().take(max_chars).collect::<String>();
                Ok(text_result(&truncated))
            }
            "extract_text" => {
                let html = args
                    .get("html")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::Validation("缺少 html".into()))?;
                Ok(text_result(&Self::strip_html(html)))
            }
            _ => Ok(error_result(&format!("未知工具: {}", tool_name))),
        }
    }

    async fn configure(&self, config: serde_json::Value) -> Result<(), AppError> {
        if let Some(engine) = config.get("engine").and_then(|v| v.as_str()) {
            *self.engine.write().await = engine.to_string();
        }
        if let Some(key) = config.get("api_key").and_then(|v| v.as_str()) {
            *self.api_key.write().await = Some(key.to_string());
        }
        Ok(())
    }
}
