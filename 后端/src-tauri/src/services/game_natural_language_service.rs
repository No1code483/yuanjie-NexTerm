//! D4.4 游戏自然语言交互服务
//!
//! 设计依据：
//! - 功能展望/模块深化/04_游戏_真实AI接入_深度.md §D4.4 自然语言交互
//! - .trae/rules/项目核心设计意图.md §五（游戏使用云端 API 模型，非底层智能）
//!
//! 核心能力：
//! - parse_command()：调用云端 API 解析玩家自然语言命令为结构化动作
//!   （例："建造图书馆" → { action: "build", target: "图书馆", ... }）
//!
//! 边界：
//! - 走云端 API（AiModelService::call_model），不使用底层智能模型
//! - AI 失败时降级到关键词规则解析（保证可用性）
//! - 解析后由前端根据 action 分发到既有 game 命令执行（非侵入式，不重复实现建造/突破逻辑）

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::services::ai_model_service::AiModelService;

// ============================================================================
// 数据结构
// ============================================================================

/// 支持的动作类型
///
/// 前端据此分发到既有 game 命令：
/// - build → game_start_building
/// - upgrade → game_upgrade_building
/// - remove → game_remove_building
/// - breakthrough → game_start_breakthrough
/// - query_status → game_get_world_state / game_get_events_and_tasks
/// - chat_npc → game_npc_chat
/// - unknown → 无法解析，提示玩家重新输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NlAction {
    Build,
    Upgrade,
    Remove,
    Breakthrough,
    QueryStatus,
    ChatNpc,
    Unknown,
}

impl Default for NlAction {
    fn default() -> Self {
        NlAction::Unknown
    }
}

/// 解析后的玩家命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    /// 识别出的动作类型
    pub action: NlAction,
    /// 动作目标（建筑名 / NPC 名 / 查询对象），AI 无法识别时为空串
    pub target: String,
    /// 附加参数（JSON 对象，如建筑子类型、NPC id 等），前端按需读取
    pub params: serde_json::Value,
    /// 置信度 0.0-1.0（规则解析固定 0.5，AI 解析由模型给出）
    pub confidence: f64,
    /// 给玩家的自然语言解释（"已理解：您想建造图书馆"）
    pub explanation: String,
    /// 是否走了 AI（false 表示降级到规则解析）
    pub used_ai: bool,
}

/// 解析请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseCommandRequest {
    pub world_id: String,
    pub player_name: String,
    pub realm: String,
    pub message: String,
    pub model_id: i64,
    pub user_id: i64,
}

// ============================================================================
// 服务实现
// ============================================================================

pub struct GameNaturalLanguageService;

impl GameNaturalLanguageService {
    /// 解析玩家自然语言命令
    ///
    /// 调用云端 API 将自然语言映射为结构化动作。AI 失败或 JSON 解析失败时
    /// 降级到关键词规则解析，保证游戏可用性。
    ///
    /// A5 Phase 3 Task 2: 新增 `is_online` 参数，离线时直接走规则解析降级
    /// （设计依据：.trae/rules/项目核心设计意图.md §五 — 游戏 AI 走云端 API，
    /// 离线时不切换本地 ollama；游戏的 fallback 规则解析不依赖网络，保证可玩性）。
    pub async fn parse_command(
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        req: ParseCommandRequest,
        is_online: bool,
    ) -> Result<ParsedCommand, AppError> {
        // A5 Phase 3 Task 2: 离线时直接降级到规则解析（不调用云端 AI）
        if !is_online {
            tracing::info!("[D4.4-NL] 网络离线，命令解析直接降级到规则解析");
            return Ok(Self::fallback_parse(&req.message));
        }

        let ai = AiModelService::new();

        let prompt = format!(
            r#"你是修仙世界游戏命令解析器。请把玩家的自然语言指令解析为结构化动作。

## 玩家信息
- 姓名：{name}
- 境界：{realm}

## 玩家指令
{message}

## 可识别动作
- build：建造建筑（如"建造图书馆"、"造一座书院"）
- upgrade：升级建筑（如"升级藏书阁"）
- remove：拆除建筑（如"拆掉木屋"）
- breakthrough：开始突破考验（如"我要突破"、"渡劫"）
- query_status：查询状态（如"看看我的建筑"、"当前进度"）
- chat_npc：与 NPC 对话（如"和青鸾老人聊聊"）
- unknown：无法识别

## 响应格式（严格 JSON，不要其他文字）
{{
  "action": "build",
  "target": "图书馆",
  "params": {{"building_sub_type": "library"}},
  "confidence": 0.95,
  "explanation": "已理解：您想建造图书馆"
}}"#,
            name = req.player_name,
            realm = req.realm,
            message = req.message,
        );

        let system = "你是游戏命令解析器，只输出严格 JSON 格式，不要任何额外文字。";

        match ai
            .call_model(pool, mek_manager, req.user_id, req.model_id, &prompt, Some(system))
            .await
        {
            Ok(raw) => match serde_json::from_str::<ParsedCommand>(&raw) {
                Ok(mut cmd) => {
                    cmd.used_ai = true;
                    Ok(cmd)
                }
                Err(e) => {
                    tracing::warn!("[D4.4-NL] AI 响应 JSON 解析失败，降级到规则解析: {}", e);
                    Ok(Self::fallback_parse(&req.message))
                }
            },
            Err(e) => {
                tracing::warn!("[D4.4-NL] AI 调用失败，降级到规则解析: {}", e);
                Ok(Self::fallback_parse(&req.message))
            }
        }
    }

    /// 规则解析降级（关键词匹配，confidence 固定 0.5）
    fn fallback_parse(message: &str) -> ParsedCommand {
        let msg = message.trim();

        // 突破 / 渡劫
        if msg.contains("突破") || msg.contains("渡劫") || msg.contains("渡天劫") {
            return ParsedCommand {
                action: NlAction::Breakthrough,
                target: String::new(),
                params: serde_json::json!({}),
                confidence: 0.5,
                explanation: "已理解：您想要开始突破考验".into(),
                used_ai: false,
            };
        }

        // 查询状态
        if msg.contains("查看") || msg.contains("看看") || msg.contains("状态") || msg.contains("进度")
        {
            return ParsedCommand {
                action: NlAction::QueryStatus,
                target: String::new(),
                params: serde_json::json!({}),
                confidence: 0.5,
                explanation: "已理解：您想查看当前状态".into(),
                used_ai: false,
            };
        }

        // 与 NPC 对话
        if msg.contains("对话") || msg.contains("聊聊") || msg.contains("说话") {
            let target = Self::extract_target_after(msg, &["和", "跟", "与", "找"]);
            return ParsedCommand {
                action: NlAction::ChatNpc,
                target,
                params: serde_json::json!({}),
                confidence: 0.5,
                explanation: "已理解：您想与 NPC 对话".into(),
                used_ai: false,
            };
        }

        // 拆除
        if msg.contains("拆") || msg.contains("移除") || msg.contains("拆除") {
            let target = Self::extract_building_name(msg);
            let explanation = format!(
                "已理解：您想拆除 {}",
                if target.is_empty() { "建筑" } else { &target }
            );
            return ParsedCommand {
                action: NlAction::Remove,
                target,
                params: serde_json::json!({}),
                confidence: 0.5,
                explanation,
                used_ai: false,
            };
        }

        // 升级
        if msg.contains("升级") || msg.contains("提升") {
            let target = Self::extract_building_name(msg);
            let explanation = format!(
                "已理解：您想升级 {}",
                if target.is_empty() { "建筑" } else { &target }
            );
            return ParsedCommand {
                action: NlAction::Upgrade,
                target,
                params: serde_json::json!({}),
                confidence: 0.5,
                explanation,
                used_ai: false,
            };
        }

        // 建造
        if msg.contains("建造") || msg.contains("建") || msg.contains("造") || msg.contains("修") {
            let target = Self::extract_building_name(msg);
            let explanation = format!(
                "已理解：您想建造 {}",
                if target.is_empty() { "建筑" } else { &target }
            );
            return ParsedCommand {
                action: NlAction::Build,
                target,
                params: serde_json::json!({}),
                confidence: 0.5,
                explanation,
                used_ai: false,
            };
        }

        ParsedCommand {
            action: NlAction::Unknown,
            target: String::new(),
            params: serde_json::json!({}),
            confidence: 0.0,
            explanation: "未能理解您的指令，请尝试「建造图书馆」「我要突破」等".into(),
            used_ai: false,
        }
    }

    /// 从消息中提取关键词之后的 NPC / 对象名（取后续 2-6 个字）
    fn extract_target_after(msg: &str, prefixes: &[&str]) -> String {
        for p in prefixes {
            if let Some(idx) = msg.find(p) {
                let rest = &msg[idx + p.len()..];
                let target: String = rest
                    .chars()
                    .take_while(|c| !matches!(c, '，' | ',' | '。' | '.' | '！' | '!' | '？' | '?'))
                    .take(8)
                    .collect();
                let target = target.trim();
                if !target.is_empty() {
                    return target.to_string();
                }
            }
        }
        String::new()
    }

    /// 粗略提取建筑名（"建造图书馆" → "图书馆"）
    fn extract_building_name(msg: &str) -> String {
        for kw in &["建造", "建造一座", "造一座", "造", "修一座", "修", "升级", "拆除", "拆"] {
            if let Some(idx) = msg.find(kw) {
                let rest = &msg[idx + kw.len()..];
                let name: String = rest
                    .chars()
                    .take_while(|c| !matches!(c, '，' | ',' | '。' | '.' | '！' | '!' | '？' | '?'))
                    .take(8)
                    .collect();
                let name = name.trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_breakthrough() {
        let cmd = GameNaturalLanguageService::fallback_parse("我要突破金丹境");
        assert!(matches!(cmd.action, NlAction::Breakthrough));
        assert!(!cmd.used_ai);
    }

    #[test]
    fn fallback_build_library() {
        let cmd = GameNaturalLanguageService::fallback_parse("建造图书馆");
        assert!(matches!(cmd.action, NlAction::Build));
        assert_eq!(cmd.target, "图书馆");
    }

    #[test]
    fn fallback_query_status() {
        let cmd = GameNaturalLanguageService::fallback_parse("看看我的建筑");
        assert!(matches!(cmd.action, NlAction::QueryStatus));
    }

    #[test]
    fn fallback_unknown() {
        let cmd = GameNaturalLanguageService::fallback_parse("今天天气真好");
        assert!(matches!(cmd.action, NlAction::Unknown));
        assert_eq!(cmd.confidence, 0.0);
    }

    #[test]
    fn fallback_chat_npc() {
        let cmd = GameNaturalLanguageService::fallback_parse("和青鸾老人聊聊");
        assert!(matches!(cmd.action, NlAction::ChatNpc));
        assert_eq!(cmd.target, "青鸾老人");
    }
}
