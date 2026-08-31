//! D4.4 游戏动态剧情服务
//!
//! 设计依据：
//! - 功能展望/模块深化/04_游戏_真实AI接入_深度.md §D4.4
//! - .trae/rules/项目核心设计意图.md §五（游戏使用云端 API 模型，非底层智能）
//!
//! 核心能力：
//! - generate_story()：调用云端 API 生成动态剧情（含分支选择）
//! - advance_story()：根据玩家选择推进剧情到下一节点
//! - get_story()：获取当前剧情状态
//! - list_stories()：列出玩家历史剧情（D4.4b 新增，跨会话恢复）
//!
//! 降级策略：AI 调用失败或 JSON 解析失败时，降级到预设剧情模板，保证游戏可用性。
//! 存储策略（D4.4b 升级）：DB 持久化（game_stories + game_story_nodes 两张表），
//!   替代 MVP 阶段的 OnceCell 内存存储，支持跨会话恢复与历史剧情查询。

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

/// 剧情选择项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryChoice {
    pub id: String,
    pub text: String,
    pub hint: Option<String>,
}

/// 剧情节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryNode {
    pub id: String,
    pub title: String,
    pub description: String,
    pub narration: String,
    pub choices: Vec<StoryChoice>,
    pub is_ending: bool,
}

/// 完整剧情（多节点分支树）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    pub id: String,
    pub world_id: String,
    pub theme: String,
    pub current_node_id: String,
    pub nodes: Vec<StoryNode>,
    /// 是否使用了 AI 生成（false 表示降级到预设模板）
    pub used_ai: bool,
}

/// 剧情列表摘要（D4.4b 新增，用于历史剧情时间线）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorySummary {
    pub id: String,
    pub world_id: String,
    pub theme: String,
    pub current_node_id: String,
    pub used_ai: bool,
    pub is_finished: bool,
    pub node_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 生成剧情请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateStoryRequest {
    pub world_id: String,
    pub player_name: String,
    pub realm: String,
    pub model_id: i64,
    pub user_id: i64,
    pub theme: Option<String>,
}

/// 推进剧情请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvanceStoryRequest {
    pub story_id: String,
    pub choice_id: String,
    pub model_id: i64,
    pub user_id: i64,
}

// ============================================================================
// 服务实现
// ============================================================================

pub struct GameStoryService;

impl GameStoryService {
    /// 生成动态剧情
    ///
    /// 调用云端 API 生成初始剧情节点，AI 失败时降级到预设模板。
    /// 持久化：写入 game_stories + game_story_nodes 两张表。
    ///
    /// A5 Phase 3 Task 2: 新增 `is_online` 参数，离线时直接走预设模板降级
    /// （设计依据：.trae/rules/项目核心设计意图.md §五 — 游戏 AI 走云端 API，
    /// 离线时不切换本地 ollama；预设模板不依赖网络，保证游戏可用性）。
    pub async fn generate_story(
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        req: GenerateStoryRequest,
        is_online: bool,
    ) -> Result<Story, AppError> {
        let theme = req.theme.unwrap_or_else(|| "修仙历练".into());

        // A5 Phase 3 Task 2: 离线时直接降级到预设模板（不调用云端 AI）
        let (mut node, used_ai) = if !is_online {
            tracing::info!("[D4.4] 网络离线，剧情生成直接降级到预设模板");
            (Self::fallback_start_node(&theme), false)
        } else {
            let ai = AiModelService::new();
            let prompt = format!(
                r#"你是修仙世界剧情生成器。请为玩家生成一段动态剧情。

## 玩家信息
- 姓名：{name}
- 境界：{realm}
- 主题：{theme}

## 要求
生成一个剧情节点，包含：
1. 剧情标题（简短，4-8字）
2. 场景描述（2-3句，交代环境）
3. 旁白叙述（3-5句，营造氛围）
4. 2-3个选择项，每个选择有简短文字和提示

## 响应格式（严格 JSON，不要其他文字）
{{
  "title": "剧情标题",
  "description": "场景描述",
  "narration": "旁白叙述",
  "choices": [
    {{"id": "a", "text": "选择文字", "hint": "选择提示"}},
    {{"id": "b", "text": "选择文字", "hint": "选择提示"}}
  ]
}}"#,
                name = req.player_name,
                realm = req.realm,
                theme = theme,
            );

            let system = "你是修仙世界剧情生成器，只输出严格 JSON 格式，不要任何额外文字。";

            match ai
                .call_model(pool, mek_manager, req.user_id, req.model_id, &prompt, Some(system))
                .await
            {
                Ok(raw) => match Self::parse_story_node(&raw, "start") {
                    Ok(n) => (n, true),
                    Err(e) => {
                        tracing::warn!("[D4.4] 剧情 JSON 解析失败，降级到预设模板: {}", e);
                        (Self::fallback_start_node(&theme), false)
                    }
                },
                Err(e) => {
                    tracing::warn!("[D4.4] 剧情生成 AI 调用失败，降级到预设模板: {}", e);
                    (Self::fallback_start_node(&theme), false)
                }
            }
        };

        let story_id = format!("story_{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let now = chrono::Utc::now().to_rfc3339();

        // 持久化剧情主表
        sqlx::query(
            "INSERT INTO game_stories (id, world_id, user_id, theme, current_node_id, used_ai, is_finished, node_count, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, 0, 1, ?, ?)",
        )
        .bind(&story_id)
        .bind(&req.world_id)
        .bind(req.user_id)
        .bind(&theme)
        .bind(&node.id)
        .bind(used_ai as i64)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        // 持久化首节点
        Self::insert_node(pool, &story_id, &mut node, 0, &now).await?;

        // 内存组装返回
        let story = Story {
            id: story_id,
            world_id: req.world_id,
            theme,
            current_node_id: node.id.clone(),
            nodes: vec![node],
            used_ai,
        };
        Ok(story)
    }

    /// 推进剧情：根据玩家选择生成下一节点
    ///
    /// 从 DB 读取当前剧情+节点，调用 AI 生成后续节点，写回 DB。
    /// AI 失败时返回错误（推进阶段不降级，前端可重试）。
    ///
    /// A5 Phase 3 Task 2: 新增 `is_online` 参数，离线时直接返回 `Offline` 错误
    /// （推进阶段本就不降级；前端可提示「📴 降级模式」并允许玩家重试）。
    pub async fn advance_story(
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        req: AdvanceStoryRequest,
        is_online: bool,
    ) -> Result<Story, AppError> {
        // A5 Phase 3 Task 2: 离线守卫（不调用云端 AI）
        if !is_online {
            tracing::info!("[D4.4] 网络离线，剧情推进被拦截（不降级，前端可重试）");
            return Err(AppError::Offline(
                "AI 服务不可用，剧情推进需要联网，请连接网络后重试".into(),
            ));
        }

        // 从 DB 读取完整剧情
        let mut story = Self::load_full_story(pool, &req.story_id)
            .await?
            .ok_or_else(|| AppError::NotFound)?;

        // 找到当前节点和选择
        let current_node = story
            .nodes
            .iter()
            .find(|n| n.id == story.current_node_id)
            .ok_or_else(|| AppError::NotFound)?;

        let choice = current_node
            .choices
            .iter()
            .find(|c| c.id == req.choice_id)
            .ok_or_else(|| AppError::Validation("无效的选择 ID".into()))?
            .clone();

        let ai = AiModelService::new();
        let history: Vec<String> = story
            .nodes
            .iter()
            .map(|n| format!("《{}》: {}", n.title, n.description))
            .collect();

        let prompt = format!(
            r#"玩家选择了：{choice_text}

## 剧情历史
{history}

## 生成要求
根据玩家选择生成下一个剧情节点。
- 如果剧情应该结束（达成结局），设置 is_ending=true 且 choices 为空数组
- 否则生成 2-3 个新选择

## 响应格式（严格 JSON）
{{
  "title": "新剧情标题",
  "description": "新场景描述",
  "narration": "旁白叙述",
  "choices": [
    {{"id": "a", "text": "选择文字", "hint": "提示"}}
  ],
  "is_ending": false
}}"#,
            choice_text = choice.text,
            history = history.join("\n"),
        );

        let system = "你是修仙世界剧情生成器，只输出严格 JSON 格式。";

        let raw = ai
            .call_model(pool, mek_manager, req.user_id, req.model_id, &prompt, Some(system))
            .await
            .map_err(|e| AppError::AiApi(format!("剧情推进 AI 调用失败: {}", e)))?;

        let sequence = story.nodes.len() as i64;
        let new_node_id = format!("node_{}", sequence);
        let mut new_node = Self::parse_story_node(&raw, &new_node_id)
            .map_err(|e| AppError::AiApi(format!("剧情推进 JSON 解析失败: {}", e)))?;

        // ending 节点清空选择
        if new_node.is_ending {
            new_node.choices = vec![];
        }

        let now = chrono::Utc::now().to_rfc3339();

        // 持久化新节点
        Self::insert_node(pool, &story.id, &mut new_node, sequence, &now).await?;

        // 更新剧情主表 current_node_id / node_count / is_finished / updated_at
        // IDOR 修复：添加 user_id 过滤，防止跨用户推进他人剧情
        let is_finished = new_node.is_ending;
        let new_node_count = sequence + 1;
        sqlx::query(
            "UPDATE game_stories
             SET current_node_id = ?, node_count = ?, is_finished = ?, updated_at = ?
             WHERE id = ? AND user_id = ?",
        )
        .bind(&new_node.id)
        .bind(new_node_count)
        .bind(is_finished as i64)
        .bind(&now)
        .bind(&story.id)
        .bind(req.user_id)
        .execute(pool)
        .await?;

        // 返回更新后的完整剧情
        story.current_node_id = new_node.id.clone();
        story.nodes.push(new_node);
        Ok(story)
    }

    /// 获取当前剧情状态（从 DB 读取完整剧情 + 所有节点）
    pub async fn get_story(pool: &SqlitePool, story_id: &str) -> Result<Option<Story>, AppError> {
        Self::load_full_story(pool, story_id).await
    }

    /// 列出玩家历史剧情摘要（D4.4b 新增，按最后推进时间倒序）
    pub async fn list_stories(
        pool: &SqlitePool,
        user_id: i64,
        limit: i64,
    ) -> Result<Vec<StorySummary>, AppError> {
        let rows = sqlx::query_as::<_, StorySummaryRow>(
            "SELECT id, world_id, theme, current_node_id, used_ai, is_finished, node_count, created_at, updated_at
             FROM game_stories
             WHERE user_id = ?
             ORDER BY updated_at DESC
             LIMIT ?",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    // ===== 内部辅助 =====

    /// 从 DB 加载完整剧情（主表 + 所有节点，按 sequence 排序）
    async fn load_full_story(pool: &SqlitePool, story_id: &str) -> Result<Option<Story>, AppError> {
        let main: Option<StoryMainRow> = sqlx::query_as::<_, StoryMainRow>(
            "SELECT id, world_id, theme, current_node_id, used_ai
             FROM game_stories
             WHERE id = ?",
        )
        .bind(story_id)
        .fetch_optional(pool)
        .await?;

        let main = match main {
            Some(m) => m,
            None => return Ok(None),
        };

        let node_rows = sqlx::query_as::<_, StoryNodeRow>(
            "SELECT node_id, title, description, narration, choices, is_ending
             FROM game_story_nodes
             WHERE story_id = ?
             ORDER BY sequence ASC",
        )
        .bind(story_id)
        .fetch_all(pool)
        .await?;

        let nodes: Vec<StoryNode> = node_rows.into_iter().map(Into::into).collect();

        Ok(Some(Story {
            id: main.id,
            world_id: main.world_id,
            theme: main.theme,
            current_node_id: main.current_node_id,
            nodes,
            used_ai: main.used_ai != 0,
        }))
    }

    /// 写入一个节点到 DB（choices 序列化为 JSON）
    async fn insert_node(
        pool: &SqlitePool,
        story_id: &str,
        node: &mut StoryNode,
        sequence: i64,
        now: &str,
    ) -> Result<(), AppError> {
        // 若 AI 返回的 node.id 为空，用 sequence 兜底
        if node.id.is_empty() {
            node.id = format!("node_{}", sequence);
        }
        let choices_json = serde_json::to_string(&node.choices).unwrap_or_else(|_| "[]".into());
        sqlx::query(
            "INSERT INTO game_story_nodes (story_id, node_id, title, description, narration, choices, is_ending, sequence, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(story_id)
        .bind(&node.id)
        .bind(&node.title)
        .bind(&node.description)
        .bind(&node.narration)
        .bind(&choices_json)
        .bind(node.is_ending as i64)
        .bind(sequence)
        .bind(now)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// 解析 LLM 返回的 JSON 为 StoryNode
    fn parse_story_node(raw: &str, default_id: &str) -> Result<StoryNode, String> {
        let json_str = Self::extract_json(raw);
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| format!("JSON 解析失败: {}", e))?;

        let id = parsed["id"].as_str().unwrap_or(default_id).to_string();
        let title = parsed["title"].as_str().unwrap_or("未知剧情").to_string();
        let description = parsed["description"].as_str().unwrap_or("").to_string();
        let narration = parsed["narration"].as_str().unwrap_or("").to_string();
        let is_ending = parsed["is_ending"].as_bool().unwrap_or(false);

        let choices = parsed["choices"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .enumerate()
                    .map(|(i, c)| StoryChoice {
                        id: c["id"].as_str().unwrap_or(&format!("c{}", i)).to_string(),
                        text: c["text"].as_str().unwrap_or("继续").to_string(),
                        hint: c["hint"].as_str().map(String::from),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(StoryNode {
            id,
            title,
            description,
            narration,
            choices,
            is_ending,
        })
    }

    /// 从 LLM 响应中提取 JSON（去除 markdown 代码块包裹）
    fn extract_json(raw: &str) -> String {
        let trimmed = raw.trim();
        // ```json ... ```
        if let Some(start) = trimmed.find("```json") {
            if let Some(end) = trimmed[start + 7..].find("```") {
                return trimmed[start + 7..start + 7 + end].trim().to_string();
            }
        }
        // ``` ... ```
        if let Some(start) = trimmed.find("```") {
            if let Some(end) = trimmed[start + 3..].find("```") {
                return trimmed[start + 3..start + 3 + end].trim().to_string();
            }
        }
        // 裸 JSON：第一个 { 到最后一个 }
        if let Some(start) = trimmed.find('{') {
            if let Some(end) = trimmed.rfind('}') {
                return trimmed[start..=end].to_string();
            }
        }
        trimmed.to_string()
    }

    /// 降级预设起始节点（AI 失败时使用，保证游戏可用性）
    fn fallback_start_node(theme: &str) -> StoryNode {
        StoryNode {
            id: "start".into(),
            title: format!("{}·初遇", theme),
            description: "你行至一处幽静山谷，云雾缭绕间似有灵气波动。".into(),
            narration: "山谷深处传来阵阵清音，似有人在修炼。你驻足聆听，心神为之一清。前方有两条路径，一条通往山巅，一条通往溪谷。".into(),
            choices: vec![
                StoryChoice {
                    id: "a".into(),
                    text: "攀登山巅，寻访高人".into(),
                    hint: Some("可能遇到前辈指点".into()),
                },
                StoryChoice {
                    id: "b".into(),
                    text: "沿溪而行，探索幽谷".into(),
                    hint: Some("可能发现灵药".into()),
                },
            ],
            is_ending: false,
        }
    }
}

// ============================================================================
// DB Row 映射（内部用，sqlx::FromRow）
// ============================================================================

#[derive(sqlx::FromRow)]
struct StoryMainRow {
    id: String,
    world_id: String,
    theme: String,
    current_node_id: String,
    used_ai: i64,
}

#[derive(sqlx::FromRow)]
struct StoryNodeRow {
    node_id: String,
    title: String,
    description: String,
    narration: String,
    choices: String,
    is_ending: i64,
}

impl From<StoryNodeRow> for StoryNode {
    fn from(row: StoryNodeRow) -> Self {
        let choices: Vec<StoryChoice> =
            serde_json::from_str(&row.choices).unwrap_or_default();
        StoryNode {
            id: row.node_id,
            title: row.title,
            description: row.description,
            narration: row.narration,
            choices,
            is_ending: row.is_ending != 0,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StorySummaryRow {
    id: String,
    world_id: String,
    theme: String,
    current_node_id: String,
    used_ai: i64,
    is_finished: i64,
    node_count: i64,
    created_at: String,
    updated_at: String,
}

impl From<StorySummaryRow> for StorySummary {
    fn from(row: StorySummaryRow) -> Self {
        StorySummary {
            id: row.id,
            world_id: row.world_id,
            theme: row.theme,
            current_node_id: row.current_node_id,
            used_ai: row.used_ai != 0,
            is_finished: row.is_finished != 0,
            node_count: row.node_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_story_node_full() {
        let raw = r#"```json
        {
          "title": "云山初遇",
          "description": "云雾缭绕的山谷",
          "narration": "你驻足聆听，心神为之一清。",
          "choices": [
            {"id": "a", "text": "登山", "hint": "可能遇高人"},
            {"id": "b", "text": "探谷", "hint": "可能寻灵药"}
          ]
        }
        ```"#;
        let node = GameStoryService::parse_story_node(raw, "start").unwrap();
        assert_eq!(node.title, "云山初遇");
        assert_eq!(node.id, "start"); // AI 未返回 id 时用 default_id
        assert_eq!(node.choices.len(), 2);
        assert_eq!(node.choices[0].id, "a");
        assert_eq!(node.choices[1].hint.as_deref(), Some("可能寻灵药"));
        assert!(!node.is_ending);
    }

    #[test]
    fn test_parse_story_node_naked_json() {
        let raw = r#"{"title":"裸JSON","description":"d","narration":"n","choices":[]}"#;
        let node = GameStoryService::parse_story_node(raw, "n0").unwrap();
        assert_eq!(node.title, "裸JSON");
        assert!(node.choices.is_empty());
    }

    #[test]
    fn test_parse_story_node_ending() {
        let raw = r#"{"title":"结局","description":"d","narration":"n","choices":[],"is_ending":true}"#;
        let node = GameStoryService::parse_story_node(raw, "end").unwrap();
        assert!(node.is_ending);
        assert!(node.choices.is_empty());
    }

    #[test]
    fn test_parse_story_node_invalid_json() {
        let raw = "not a json";
        let result = GameStoryService::parse_story_node(raw, "x");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_json_codeblock() {
        let raw = "前缀\n```json\n{\"a\":1}\n```\n后缀";
        assert_eq!(GameStoryService::extract_json(raw), r#"{"a":1}"#);
    }

    #[test]
    fn test_extract_json_plain() {
        let raw = r#"{"a":1}"#;
        assert_eq!(GameStoryService::extract_json(raw), r#"{"a":1}"#);
    }

    #[test]
    fn test_fallback_start_node_has_two_choices() {
        let node = GameStoryService::fallback_start_node("修仙历练");
        assert_eq!(node.id, "start");
        assert_eq!(node.choices.len(), 2);
        assert!(!node.is_ending);
        assert!(node.title.contains("修仙历练"));
    }

    #[test]
    fn test_story_summary_serialization() {
        let s = StorySummary {
            id: "story_abc".into(),
            world_id: "w1".into(),
            theme: "修仙历练".into(),
            current_node_id: "node_2".into(),
            used_ai: true,
            is_finished: false,
            node_count: 3,
            created_at: "2026-07-23T00:00:00Z".into(),
            updated_at: "2026-07-23T01:00:00Z".into(),
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("story_abc"));
        assert!(json.contains("\"used_ai\":true"));
        let back: StorySummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node_count, 3);
    }
}
