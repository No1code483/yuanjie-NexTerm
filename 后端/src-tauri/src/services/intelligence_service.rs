use std::collections::VecDeque;
use std::sync::Arc;

use reqwest::Client;
use serde::Deserialize;
use tokio::sync::Mutex;
use tracing;
use uuid::Uuid;

use crate::error::app_error::AppError;
use crate::models::intelligence::{
    ChatModuleContext, ContextSnapshot, GameModuleContext, IntelligenceConfig, LocalModelInfo,
    OllamaStatus, Suggestion, SuggestionActionType, UserActivity, UserContext,
};

#[derive(Debug, Deserialize)]
struct OllamaListResponse {
    models: Vec<OllamaModelEntry>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelEntry {
    name: String,
    size: u64,
    modified_at: String,
    digest: Option<String>,
    details: Option<OllamaModelDetails>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelDetails {
    parameter_size: Option<String>,
    quantization_level: Option<String>,
}

pub struct IntelligenceService {
    config: Arc<Mutex<IntelligenceConfig>>,
    activities: Arc<Mutex<VecDeque<UserActivity>>>,
    snapshots: Arc<Mutex<VecDeque<ContextSnapshot>>>,
    active_file: Arc<Mutex<Option<String>>>,
    active_language: Arc<Mutex<Option<String>>>,
    terminal_sessions: Arc<Mutex<Vec<String>>>,
    window_title: Arc<Mutex<Option<String>>>,
    client: Client,
}

impl IntelligenceService {
    pub fn new() -> Self {
        Self {
            config: Arc::new(Mutex::new(IntelligenceConfig::default())),
            activities: Arc::new(Mutex::new(VecDeque::with_capacity(200))),
            snapshots: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            active_file: Arc::new(Mutex::new(None)),
            active_language: Arc::new(Mutex::new(None)),
            terminal_sessions: Arc::new(Mutex::new(Vec::new())),
            window_title: Arc::new(Mutex::new(None)),
            client: Client::builder()
                .no_proxy()
                .http1_only()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn get_config(&self) -> IntelligenceConfig {
        self.config.lock().await.clone()
    }

    /// D2.1 可关闭性：查询全局开关状态
    pub async fn is_enabled(&self) -> bool {
        self.config.lock().await.enabled
    }

    /// D2.1 可关闭性：一键开关（非侵入式，关闭后各模块核心功能仍可用）
    pub async fn set_enabled(&self, enabled: bool) {
        let mut cfg = self.config.lock().await;
        cfg.enabled = enabled;
        tracing::info!("[底层智能] 全局开关已设置为: {}", enabled);
    }

    pub async fn get_recent_activities(&self) -> Vec<UserActivity> {
        let activities: tokio::sync::MutexGuard<'_, VecDeque<UserActivity>> = self.activities.lock().await;
        activities.iter().cloned().collect()
    }

    pub async fn update_config(&self, config: IntelligenceConfig) {
        let mut cfg = self.config.lock().await;
        *cfg = config;
    }

    pub async fn track_activity(&self, activity: UserActivity) {
        // D2.1 可关闭性：关闭后仍记录活动（轻量行为，不影响性能），但不触发智能分析
        let mut activities: tokio::sync::MutexGuard<'_, VecDeque<UserActivity>> = self.activities.lock().await;
        activities.push_back(activity);
        while activities.len() > 200 {
            activities.pop_front();
        }
    }

    pub async fn set_active_file(&self, path: Option<String>, language: Option<String>) {
        let mut file = self.active_file.lock().await;
        *file = path;
        let mut lang = self.active_language.lock().await;
        *lang = language;
    }

    pub async fn add_terminal_session(&self, session_id: String) {
        let mut sessions = self.terminal_sessions.lock().await;
        if !sessions.contains(&session_id) {
            sessions.push(session_id);
        }
    }

    pub async fn remove_terminal_session(&self, session_id: &str) {
        let mut sessions = self.terminal_sessions.lock().await;
        sessions.retain(|s| s != session_id);
    }

    pub async fn set_window_title(&self, title: Option<String>) {
        let mut wt = self.window_title.lock().await;
        *wt = title;
    }

    pub async fn get_context(&self) -> UserContext {
        let file = self.active_file.lock().await;
        let lang = self.active_language.lock().await;
        let sessions = self.terminal_sessions.lock().await;
        let title = self.window_title.lock().await;
        let activities: tokio::sync::MutexGuard<'_, VecDeque<UserActivity>> = self.activities.lock().await;

        let recent: Vec<UserActivity> = activities.iter().rev().take(20).cloned().collect();

        UserContext {
            active_file: file.clone(),
            active_file_language: lang.clone(),
            active_terminal_sessions: sessions.clone(),
            recent_activities: recent,
            active_window_title: title.clone(),
            collected_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }

    pub async fn check_ollama_status(&self) -> OllamaStatus {
        let cfg = self.config.lock().await;
        let url = cfg.ollama_url.clone();

        let is_running = self
            .client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false);

        let mut models: Vec<LocalModelInfo> = Vec::new();
        let version: Option<String> = None;

        if is_running {
            if let Ok(resp) = self.client.get(&format!("{}/api/tags", url)).send().await {
                if let Ok(data) = resp.json::<OllamaListResponse>().await {
                    for entry in data.models {
                        models.push(LocalModelInfo {
                            name: entry.name,
                            size_bytes: entry.size,
                            modified_at: Some(entry.modified_at),
                            parameter_size: entry.details.as_ref().and_then(|d| d.parameter_size.clone()),
                            quantization: entry.details.as_ref().and_then(|d| d.quantization_level.clone()),
                            digest: entry.digest,
                        });
                    }
                }
            }
        }

        OllamaStatus {
            is_installed: is_running,
            is_running,
            api_url: url,
            version,
            available_models: models,
        }
    }

    pub async fn generate_suggestions(&self) -> Vec<Suggestion> {
        // D2.1 可关闭性：关闭后返回空建议列表（非阻塞，各模块核心功能仍可用）
        if !self.is_enabled().await {
            return Vec::new();
        }
        let context = self.get_context().await;
        let mut suggestions: Vec<Suggestion> = Vec::new();

        if let Some(ref file) = context.active_file {
            let ext = file.rsplit('.').next().unwrap_or("");

            match ext {
                "rs" => {
                    suggestions.push(Suggestion {
                        id: Uuid::new_v4().to_string(),
                        title: "运行 Rust 项目".into(),
                        description: format!("检测到正在编辑 Rust 文件: {}，需要运行 cargo build 吗？", file),
                        action_type: SuggestionActionType::RunCommand,
                        action_payload: "cargo build".into(),
                        priority: 80,
                        category: "build".into(),
                    });
                    suggestions.push(Suggestion {
                        id: Uuid::new_v4().to_string(),
                        title: "Rust 代码检查".into(),
                        description: "需要运行 clippy 检查代码风格吗？".into(),
                        action_type: SuggestionActionType::RunCommand,
                        action_payload: "cargo clippy".into(),
                        priority: 60,
                        category: "quality".into(),
                    });
                }
                "py" => {
                    suggestions.push(Suggestion {
                        id: Uuid::new_v4().to_string(),
                        title: "运行 Python 脚本".into(),
                        description: format!("检测到正在编辑 Python 文件: {}", file),
                        action_type: SuggestionActionType::RunCommand,
                        action_payload: format!("python \"{}\"", file),
                        priority: 80,
                        category: "run".into(),
                    });
                }
                "js" | "ts" | "tsx" | "jsx" => {
                    suggestions.push(Suggestion {
                        id: Uuid::new_v4().to_string(),
                        title: "运行 Node.js 开发服务器".into(),
                        description: "检测到 JavaScript/TypeScript 项目，需要启动开发服务器吗？".into(),
                        action_type: SuggestionActionType::RunCommand,
                        action_payload: "npm run dev".into(),
                        priority: 70,
                        category: "run".into(),
                    });
                }
                "md" => {
                    suggestions.push(Suggestion {
                        id: Uuid::new_v4().to_string(),
                        title: "Markdown 预览".into(),
                        description: "检测到正在编辑 Markdown 文件，需要预览效果吗？".into(),
                        action_type: SuggestionActionType::Info,
                        action_payload: file.clone(),
                        priority: 50,
                        category: "preview".into(),
                    });
                }
                "toml" | "json" | "yaml" | "yml" => {
                    suggestions.push(Suggestion {
                        id: Uuid::new_v4().to_string(),
                        title: "配置文件编辑".into(),
                        description: format!("检测到配置文件: {}，需要 AI 帮忙分析配置吗？", file),
                        action_type: SuggestionActionType::AiHelp,
                        action_payload: format!("请分析配置文件 {}", file),
                        priority: 40,
                        category: "config".into(),
                    });
                }
                _ => {}
            }

            suggestions.push(Suggestion {
                id: Uuid::new_v4().to_string(),
                title: "AI 代码助手".into(),
                description: "需要 AI 帮你解释、优化或补充这段代码吗？".into(),
                action_type: SuggestionActionType::AiHelp,
                action_payload: format!("请帮我分析文件: {}", file),
                priority: 30,
                category: "assistance".into(),
            });
        }

        if context.active_terminal_sessions.len() >= 3 {
            suggestions.push(Suggestion {
                id: Uuid::new_v4().to_string(),
                title: "清理终端会话".into(),
                description: format!("当前打开了 {} 个终端会话，是否关闭不常用的？", context.active_terminal_sessions.len()),
                action_type: SuggestionActionType::Info,
                action_payload: "manage_terminals".into(),
                priority: 20,
                category: "cleanup".into(),
            });
        }

        suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));
        suggestions
    }

    /// D2.3 模块渗透：群聊智能建议
    ///
    /// 基于群聊上下文（参与 AI 数、消息频率、空闲时长）生成「触发执行式」建议。
    /// 输出是优化建议（如"建议发起话题"），不是对话回复。
    /// 底层智能关闭后返回空列表（非阻塞，群聊核心功能不受影响）。
    pub async fn generate_chat_suggestions(&self, ctx: ChatModuleContext) -> Vec<Suggestion> {
        if !self.is_enabled().await {
            return Vec::new();
        }
        let mut suggestions = Vec::new();

        // 群聊冷场检测：超过 30 分钟无消息
        if ctx.minutes_since_last_message > 30 {
            suggestions.push(Suggestion {
                id: Uuid::new_v4().to_string(),
                title: "群聊冷场提醒".into(),
                description: format!(
                    "群聊已 {} 分钟无消息，是否发起新话题？",
                    ctx.minutes_since_last_message
                ),
                action_type: SuggestionActionType::Info,
                action_payload: "suggest_topic".into(),
                priority: 50,
                category: "chat_engagement".into(),
            });
        }

        // AI 辩论触发建议：多 AI 就绪但消息少
        if ctx.active_ai_count >= 2 && ctx.message_count_last_hour < 3 {
            suggestions.push(Suggestion {
                id: Uuid::new_v4().to_string(),
                title: "AI 辩论建议".into(),
                description: format!(
                    "{} 个 AI 已就绪但消息较少，是否触发 AI 辩论话题？",
                    ctx.active_ai_count
                ),
                action_type: SuggestionActionType::Info,
                action_payload: "trigger_debate".into(),
                priority: 60,
                category: "chat_engagement".into(),
            });
        }

        suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));
        suggestions
    }

    /// D2.3 模块渗透：游戏智能建议
    ///
    /// 基于游戏数据（修为、境界、建筑、近期行为）生成「触发执行式」建议。
    /// 输出是优化建议（如"建议闭关修炼"），不是对话回复。
    /// 底层智能关闭后返回空列表（非阻塞，游戏核心功能不受影响）。
    pub async fn generate_game_suggestions(&self, ctx: GameModuleContext) -> Vec<Suggestion> {
        if !self.is_enabled().await {
            return Vec::new();
        }
        let mut suggestions = Vec::new();

        // 修为停滞检测：长时间未获得修为
        if ctx.minutes_since_last_xp_gain > 60 {
            suggestions.push(Suggestion {
                id: Uuid::new_v4().to_string(),
                title: "修炼停滞提醒".into(),
                description: format!(
                    "已 {} 分钟未获得修为（当前境界：{}），是否进入闭关修炼？",
                    ctx.minutes_since_last_xp_gain, ctx.realm
                ),
                action_type: SuggestionActionType::Info,
                action_payload: "suggest_meditation".into(),
                priority: 70,
                category: "game_progression".into(),
            });
        }

        // 建筑闲置检测
        if ctx.idle_building_count > 0 {
            suggestions.push(Suggestion {
                id: Uuid::new_v4().to_string(),
                title: "建筑升级提醒".into(),
                description: format!(
                    "有 {} 个建筑可升级，是否查看？",
                    ctx.idle_building_count
                ),
                action_type: SuggestionActionType::Info,
                action_payload: "view_buildings".into(),
                priority: 50,
                category: "game_building".into(),
            });
        }

        suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));
        suggestions
    }

    pub async fn take_snapshot(&self) -> Result<ContextSnapshot, AppError> {
        // D2.1 可关闭性：关闭后返回默认快照（非阻塞）
        if !self.is_enabled().await {
            return Ok(ContextSnapshot {
                id: Uuid::new_v4().to_string(),
                context: UserContext {
                    active_file: None,
                    active_file_language: None,
                    active_terminal_sessions: Vec::new(),
                    recent_activities: Vec::new(),
                    active_window_title: None,
                    collected_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                },
                captured_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                session_id: "disabled".into(),
            });
        }
        let context = self.get_context().await;
        let max = {
            let cfg = self.config.lock().await;
            cfg.max_snapshots
        };

        let snapshot = ContextSnapshot {
            id: Uuid::new_v4().to_string(),
            context,
            captured_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            session_id: "default".into(),
        };

        let mut snapshots: tokio::sync::MutexGuard<'_, VecDeque<ContextSnapshot>> = self.snapshots.lock().await;
        snapshots.push_back(snapshot.clone());
        while snapshots.len() > max {
            snapshots.pop_front();
        }

        Ok(snapshot)
    }

    pub async fn get_snapshots(&self) -> Vec<ContextSnapshot> {
        let snapshots: tokio::sync::MutexGuard<'_, VecDeque<ContextSnapshot>> = self.snapshots.lock().await;
        snapshots.iter().cloned().collect()
    }

    pub async fn query_local_llm(&self, prompt: &str) -> Result<String, AppError> {
        // D2.1 可关闭性：关闭后返回错误，各模块调用方已有 unwrap_or_default/unwrap_or_else 降级处理
        if !self.is_enabled().await {
            return Err(AppError::Internal("底层智能已关闭，本地 LLM 不可用".into()));
        }
        let cfg = self.config.lock().await;
        let base_url = cfg.ollama_url.trim_end_matches('/').to_string();
        let mut model = cfg.ollama_model.clone();
        tracing::info!("[LLM] 配置模型: {}, Ollama URL: {}", model, base_url);
        drop(cfg);

        // 自动检测：如果默认模型不可用，从 Ollama 获取可用模型列表，选第一个
        if model.is_empty() || model == "llama3.2" {
            tracing::info!("[LLM] 默认模型为 llama3.2，尝试自动检测可用模型...");
            match self.detect_first_available_model(&base_url).await {
                Ok(detected) => {
                    if !detected.is_empty() {
                        tracing::info!("[LLM] 自动检测到模型: {}", detected);
                        model = detected;
                        let mut cfg = self.config.lock().await;
                        cfg.ollama_model = model.clone();
                    }
                }
                Err(e) => {
                    tracing::warn!("[LLM] 自动检测模型失败: {:?}", e);
                }
            }
        }

        let url = format!("{}/api/generate", base_url);
        tracing::info!("[LLM] 发送请求到 {}, 模型: {}", url, model);

        // 预检：先用 TcpStream 测试连接
        let tcp_test = tokio::net::TcpStream::connect("localhost:11434").await;
        match tcp_test {
            Ok(_) => tracing::info!("[LLM] TcpStream 直连 localhost:11434 成功"),
            Err(e) => tracing::error!("[LLM] TcpStream 直连 localhost:11434 失败: {:?}", e),
        }

        let request = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.7,
                "num_predict": 512
            }
        });

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("[LLM] 无法连接到 Ollama ({}): {:?}", base_url, e);
                AppError::AiApi(format!("无法连接到 Ollama ({}): {:?}", base_url, e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            // 如果模型不存在，尝试自动检测可用模型并重试一次
            if status.as_u16() == 404 || status.as_u16() == 400 {
                if let Ok(detected) = self.detect_first_available_model(&base_url).await {
                    if !detected.is_empty() && detected != model {
                        tracing::warn!(model = %model, fallback = %detected, "原模型不可用，切换到自动检测的模型");
                        let retry_req = serde_json::json!({
                            "model": detected,
                            "prompt": prompt,
                            "stream": false,
                            "options": {
                                "temperature": 0.7,
                                "num_predict": 512
                            }
                        });
                        let retry_resp = self
                            .client
                            .post(&url)
                            .json(&retry_req)
                            .send()
                            .await
                            .map_err(|e| AppError::AiApi(format!("无法连接到 Ollama ({}): {}", base_url, e)))?;
                        if retry_resp.status().is_success() {
                            let body: serde_json::Value = retry_resp.json().await
                                .map_err(|e| AppError::AiApi(format!("解析 Ollama 响应失败: {}", e)))?;
                            let result = body["response"].as_str()
                                .map(|s| s.to_string())
                                .ok_or_else(|| AppError::AiApi("Ollama 响应格式异常".into()))?;
                            // 缓存检测到的模型
                            let mut cfg = self.config.lock().await;
                            cfg.ollama_model = detected;
                            return Ok(result);
                        }
                    }
                }
            }
            tracing::error!("[LLM] Ollama 返回错误: HTTP {} (模型: {})", status, model);
            return Err(AppError::AiApi(format!(
                "Ollama 返回错误: HTTP {} (模型: {})",
                status, model
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::AiApi(format!("解析 Ollama 响应失败: {}", e)))?;

        body["response"]
            .as_str()
            .map(|s| {
                let result = s.to_string();
                tracing::info!("[LLM] 成功获取响应，长度: {} 字符", result.len());
                result
            })
            .ok_or_else(|| AppError::AiApi("Ollama 响应格式异常".into()))
    }

    /// 从 Ollama /api/tags 获取第一个可用模型名
    async fn detect_first_available_model(&self, base_url: &str) -> Result<String, AppError> {
        let url = format!("{}/api/tags", base_url);
        tracing::info!("[LLM] 获取模型列表: {}", url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("[LLM] 获取模型列表网络错误: {}", e);
                AppError::AiApi(format!("获取 Ollama 模型列表失败: {}", e))
            })?;

        if !resp.status().is_success() {
            tracing::error!("[LLM] 获取模型列表 HTTP 错误: {}", resp.status());
            return Err(AppError::AiApi(format!(
                "获取 Ollama 模型列表失败: HTTP {}",
                resp.status()
            )));
        }

        let body_text = resp.text().await
            .map_err(|e| AppError::AiApi(format!("读取模型列表响应失败: {}", e)))?;
        tracing::info!("[LLM] 模型列表响应: {}", &body_text[..body_text.len().min(500)]);

        let data: OllamaListResponse = serde_json::from_str(&body_text)
            .map_err(|e| {
                tracing::error!("[LLM] 解析模型列表 JSON 失败: {} - 原始响应: {}", e, &body_text[..body_text.len().min(200)]);
                AppError::AiApi(format!("解析模型列表失败: {}", e))
            })?;

        let model_name = data.models
            .into_iter()
            .next()
            .map(|m| m.name)
            .ok_or_else(|| AppError::AiApi("Ollama 中没有可用模型，请先拉取模型".into()))?;
        tracing::info!("[LLM] 检测到第一个可用模型: {}", model_name);
        Ok(model_name)
    }

    /// D2.2 触发执行式：主动检测上下文 → 决策 → 返回可执行动作列表
    ///
    /// 设计意图（§2.3）：底层智能的输出是「优化建议/自动执行动作」，不是「对话回复」。
    /// 此函数是触发执行式的核心入口：
    /// 1. 主动采集当前上下文（不等待用户提问）
    /// 2. 基于上下文检测可优化点（文件类型/终端数量/活动模式）
    /// 3. 返回带 action_type + action_payload 的可执行动作列表
    ///
    /// 关闭底层智能时返回空列表（非阻塞）。
    pub async fn trigger_proactive_actions(&self) -> Vec<Suggestion> {
        if !self.is_enabled().await {
            return Vec::new();
        }

        let context = self.get_context().await;
        let mut actions: Vec<Suggestion> = Vec::new();

        // 检测点1：活动文件 → 建议运行/构建
        if let Some(ref file) = context.active_file {
            let ext = file.rsplit('.').next().unwrap_or("");
            let py_cmd = format!("python \"{}\"", file);
            let (title, cmd, category, priority): (&str, &str, &str, i32) = match ext {
                "rs" => ("构建 Rust 项目", "cargo build", "build", 80),
                "py" => ("运行 Python 脚本", py_cmd.as_str(), "run", 80),
                "js" | "ts" | "tsx" | "jsx" => ("启动开发服务器", "npm run dev", "run", 70),
                "md" => ("预览 Markdown", "", "preview", 50),
                _ => ("", "", "", 0),
            };
            if !title.is_empty() {
                actions.push(Suggestion {
                    id: Uuid::new_v4().to_string(),
                    title: title.into(),
                    description: format!("检测到正在编辑 {} 文件: {}", ext, file),
                    action_type: SuggestionActionType::RunCommand,
                    action_payload: cmd.into(),
                    priority,
                    category: category.into(),
                });
            }
        }

        // 检测点2：终端会话过多 → 建议清理
        if context.active_terminal_sessions.len() >= 3 {
            actions.push(Suggestion {
                id: Uuid::new_v4().to_string(),
                title: "清理终端会话".into(),
                description: format!("当前打开了 {} 个终端会话，是否关闭不常用的？", context.active_terminal_sessions.len()),
                action_type: SuggestionActionType::Info,
                action_payload: "manage_terminals".into(),
                priority: 20,
                category: "cleanup".into(),
            });
        }

        // 检测点3：长时间未活动 → 建议休息
        if let Some(last) = context.recent_activities.first() {
            if let Ok(ts) = chrono::DateTime::parse_from_str(&last.timestamp, "%Y-%m-%d %H:%M:%S") {
                let elapsed = chrono::Utc::now().signed_duration_since(ts.with_timezone(&chrono::Utc));
                if elapsed.num_minutes() > 30 {
                    actions.push(Suggestion {
                        id: Uuid::new_v4().to_string(),
                        title: "建议短暂休息".into(),
                        description: "检测到您已持续工作超过 30 分钟，建议短暂休息以保持专注力。".into(),
                        action_type: SuggestionActionType::Info,
                        action_payload: "take_break".into(),
                        priority: 40,
                        category: "wellness".into(),
                    });
                }
            }
        }

        actions.sort_by(|a, b| b.priority.cmp(&a.priority));
        actions
    }
}