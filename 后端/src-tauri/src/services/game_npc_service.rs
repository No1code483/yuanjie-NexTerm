//! D4.2 智能 NPC 系统
//!
//! 设计目标（04_游戏_真实AI接入_深度.md §2.1）：
//! - NPC 拥有独立人格 / 知识领域 / 对话风格
//! - 玩家与 NPC 多轮对话，获取指引 / 任务 / 知识
//! - AI 调用走云端 API（AiModelService），不使用底层智能模型
//! - 对话历史持久化（game_npc_conversations 表）
//!
//! 内置 NPC（首次启动时自动 seed）：
//! - 青鸾老人（guide 引导者）— 新手引导
//! - 紫霄真人（elder 长老）— 高阶指点
//! - 天工坊主（artisan 工匠）— 建造指引
//! - 万卷书生（scholar 学者）— 知识问答
//! - 凌霄剑客（rival 对手）— 切磋挑战

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::services::ai_model_service::AiModelService;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameNpc {
    pub id: String,
    pub name: String,
    pub role: String,
    pub realm_level: String,
    pub personality: String,
    pub knowledge_domains: String, // JSON 数组字符串
    pub greeting: String,
    pub system_prompt: String,
    pub avatar_emoji: String,
    pub location: Option<String>,
    pub unlock_realm: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameNpcConversation {
    pub id: String,
    pub world_id: String,
    pub npc_id: String,
    pub role: String,
    pub content: String,
    pub turn_index: i64,
    pub created_at: i64,
}

/// D4.6 NPC 对玩家的长期记忆（game_npc_memories 表）。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameNpcMemory {
    pub id: String,
    pub world_id: String,
    pub npc_id: String,
    /// `fact` / `preference` / `commitment` / `event`
    pub memory_type: String,
    /// 自然语言描述，如"玩家叫张三"。
    pub content: String,
    /// 重要度 0.0-1.0（影响召回优先级）。
    pub importance: f64,
    /// 来源：`rule`（规则匹配）/ `ai`（AI 提取）。
    pub source: String,
    pub recall_count: i64,
    pub last_recalled_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// D4.7 跨 NPC 传闻（game_npc_rumors 表）。
///
/// 当 NPC A 与玩家互动产生重要记忆时，该记忆会传播为传闻到同世界其他 NPC，
/// 让其他 NPC 在对话中能"听说"关于玩家的事。与 GameNpcMemory 区别：
/// - GameNpcMemory 存"亲历记忆"（NPC 自己与玩家互动产生的，source='rule'/'ai'）
/// - GameNpcRumor 存"传闻记忆"（从其他 NPC 传播来的，非亲历）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameNpcRumor {
    pub id: String,
    pub world_id: String,
    /// 传闻来源 NPC（产生该记忆的亲历者）。
    pub source_npc_id: String,
    /// 传闻目标 NPC（听到该传闻的 NPC）。
    pub target_npc_id: String,
    /// `fact` / `preference` / `commitment` / `event`（继承自源记忆）。
    pub memory_type: String,
    /// 传闻内容（自然语言）。
    pub content: String,
    /// 重要度 0.0-1.0（继承自源记忆）。
    pub importance: f64,
    /// 源记忆 ID（用于去重：同一记忆不重复传播给同一 NPC）。
    pub origin_memory_id: String,
    pub created_at: i64,
}

/// D4.6 NPC↔玩家关系（game_npc_relationships 表）。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GameNpcRelationship {
    pub id: String,
    pub world_id: String,
    pub npc_id: String,
    /// -100（仇恨）~ +100（挚友），0 为中立。
    pub relationship_value: i32,
    pub interaction_count: i64,
    pub first_interaction_at: i64,
    pub last_interaction_at: i64,
    /// JSON 数组字符串，每项 {turn, delta, reason, ts}，最多 50 条。
    pub relationship_history: String,
    /// `hostile` / `cold` / `neutral` / `warm` / `close` / `sworn`
    pub relationship_label: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// D4.6 关系历史条目（relationship_history JSON 数组项）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipHistoryEntry {
    pub turn: i64,
    pub delta: i32,
    /// `positive` / `negative` / `no_change`
    pub reason: String,
    pub ts: i64,
}

/// D4.6 记忆草稿（待写入 DB 的记忆数据，不含 id/timestamp）。
#[derive(Debug, Clone)]
pub struct MemoryDraft {
    pub npc_id: String,
    pub memory_type: String,
    pub content: String,
    pub importance: f64,
    pub source: String,
}

/// D4.6 AI 评估结果（关系增量 + 可选的批量提取记忆）。
#[derive(Debug, Clone, Default)]
pub struct AiMemoryAndRelationshipResult {
    pub relationship_delta: i32,
    pub memories: Vec<MemoryDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcChatRequest {
    pub world_id: String,
    pub npc_id: String,
    pub user_id: i64,
    pub model_id: i64,
    pub message: String,
}

/// D4.6 NPC 对话响应（扩展：携带关系值 + 记忆条数，供前端展示关系条 + 记忆提示）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcChatResponse {
    pub npc_id: String,
    pub reply: String,
    pub turn_index: i64,
    /// D4.6 NPC 当前对玩家的关系值（-100~100，0 为中立）。
    pub relationship_value: i32,
    /// D4.6 关系等级标签（hostile/cold/neutral/warm/close/sworn）。
    pub relationship_label: String,
    /// D4.6 NPC 已记住玩家的记忆条数（前端"已记住 N 件事"提示）。
    pub memory_count: i64,
    /// D4.6 本次互动关系变化增量（-5~+5，AI 评估）。
    pub relationship_delta: i32,
    /// D4.6 本次互动新增的记忆条数（规则匹配 + AI 批量）。
    pub memory_added: i64,
}

pub struct GameNpcService;

impl GameNpcService {
    /// 列出所有 NPC（按 unlock_realm 排序）
    pub async fn list_npcs(pool: &SqlitePool) -> Result<Vec<GameNpc>, AppError> {
        let npcs = sqlx::query_as::<_, GameNpc>(
            "SELECT id, name, role, realm_level, personality, knowledge_domains, greeting, \
             system_prompt, avatar_emoji, location, unlock_realm, created_at, updated_at \
             FROM game_npcs ORDER BY unlock_realm, name",
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(npcs)
    }

    /// 获取单个 NPC
    pub async fn get_npc(pool: &SqlitePool, npc_id: &str) -> Result<Option<GameNpc>, AppError> {
        let npc = sqlx::query_as::<_, GameNpc>(
            "SELECT id, name, role, realm_level, personality, knowledge_domains, greeting, \
             system_prompt, avatar_emoji, location, unlock_realm, created_at, updated_at \
             FROM game_npcs WHERE id = ?",
        )
        .bind(npc_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(npc)
    }

    /// 加载对话历史（最近 N 轮）
    pub async fn load_history(
        pool: &SqlitePool,
        world_id: &str,
        npc_id: &str,
        limit: i64,
    ) -> Result<Vec<GameNpcConversation>, AppError> {
        let history = sqlx::query_as::<_, GameNpcConversation>(
            "SELECT id, world_id, npc_id, role, content, turn_index, created_at \
             FROM game_npc_conversations WHERE world_id = ? AND npc_id = ? \
             ORDER BY turn_index DESC LIMIT ?",
        )
        .bind(world_id)
        .bind(npc_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
        // 反转为时间顺序
        let mut history = history;
        history.reverse();
        Ok(history)
    }

    /// 与 NPC 对话（D4.6 增强：注入长期记忆 + 关系到 prompt，对话后抽取记忆 + 评估关系）。
    ///
    /// 流程：
    ///   1. 加载最近 20 轮历史 + 长期记忆 + 关系值
    ///   2. 注入记忆/关系到 system prompt
    ///   3. 写入用户消息
    ///   4. 调用 AI 获取回复
    ///   5. 写入 NPC 回复
    ///   6. 规则匹配抽取记忆（每次，零 AI 成本）
    ///   7. AI 评估关系增量（每轮）+ AI 批量提取记忆（每 5 轮，合并调用）
    ///   8. 更新关系值 + 写入新记忆
    ///   9. 返回扩展响应（含关系值/标签/记忆条数/本次增量）
    pub async fn chat(
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        request: NpcChatRequest,
    ) -> Result<NpcChatResponse, AppError> {
        let npc = Self::get_npc(pool, &request.npc_id)
            .await?
            .ok_or_else(|| AppError::NotFound)?;
        let _ = &npc; // 检查存在

        // 1. 加载最近 20 轮历史 + 长期记忆（最多 20 条按重要度倒序） + 关系值
        let history = Self::load_history(pool, &request.world_id, &request.npc_id, 20).await?;
        let memories = Self::load_memories(pool, &request.world_id, &request.npc_id, 20).await?;
        let relationship =
            Self::get_or_create_relationship(pool, &request.world_id, &request.npc_id).await?;

        // D4.7 加载传播给当前 NPC 的传闻（从其他 NPC 听说的关于玩家的事，最多 5 条）
        let rumors = match Self::load_rumors(pool, &request.world_id, &request.npc_id, 5).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("D4.7: 加载传闻失败，降级到无传闻: {}", e);
                Vec::new()
            }
        };
        // 构建 npc_id→name 映射（传闻注入时显示来源 NPC 名字）+ 同世界其他 NPC id 列表（步骤6/7后传播）
        let (npc_name_map, other_npc_ids) = match Self::list_npcs(pool).await {
            Ok(npcs) => {
                let map: HashMap<String, String> =
                    npcs.iter().map(|n| (n.id.clone(), n.name.clone())).collect();
                let others: Vec<String> = npcs
                    .iter()
                    .filter(|n| n.id != request.npc_id)
                    .map(|n| n.id.clone())
                    .collect();
                (map, others)
            }
            Err(e) => {
                tracing::warn!("D4.7: 加载 NPC 列表失败，降级到无传闻传播: {}", e);
                (HashMap::new(), Vec::new())
            }
        };

        // 计算下一轮 turn_index（本轮用户消息的 turn）
        let next_turn = history.last().map(|h| h.turn_index + 1).unwrap_or(0);
        let now = chrono::Utc::now().timestamp_millis();

        // 2. 注入记忆 + 关系 + 传闻到 system prompt
        let enhanced_system_prompt = inject_memory_and_relationship(
            &npc.system_prompt,
            &memories,
            &relationship,
            &rumors,
            &npc_name_map,
        );

        // 3. 写入用户消息
        let user_msg_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO game_npc_conversations \
             (id, world_id, npc_id, role, content, turn_index, created_at) \
             VALUES (?, ?, ?, 'user', ?, ?, ?)",
        )
        .bind(&user_msg_id)
        .bind(&request.world_id)
        .bind(&request.npc_id)
        .bind(&request.message)
        .bind(next_turn)
        .bind(now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        // 构造多轮消息列表（role, content）
        let mut messages: Vec<(String, String)> = Vec::with_capacity(history.len() + 1);
        for h in &history {
            if h.role == "user" || h.role == "assistant" {
                messages.push((h.role.clone(), h.content.clone()));
            }
        }
        messages.push(("user".into(), request.message.clone()));

        // 4. 调用 AiModelService 多轮对话
        let ai_service = AiModelService::new();
        let reply = ai_service
            .call_model_messages(
                pool,
                mek_manager,
                request.user_id,
                request.model_id,
                messages,
                Some(&enhanced_system_prompt),
            )
            .await?;

        // 5. 写入 NPC 回复
        let reply_msg_id = uuid::Uuid::new_v4().to_string();
        let reply_turn = next_turn + 1;
        let now2 = chrono::Utc::now().timestamp_millis();
        sqlx::query(
            "INSERT INTO game_npc_conversations \
             (id, world_id, npc_id, role, content, turn_index, created_at) \
             VALUES (?, ?, ?, 'assistant', ?, ?, ?)",
        )
        .bind(&reply_msg_id)
        .bind(&request.world_id)
        .bind(&request.npc_id)
        .bind(&reply)
        .bind(reply_turn)
        .bind(now2)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        // 6. 规则匹配抽取记忆（每次，零 AI 成本）
        let rule_memories = extract_memory_by_rules(&request.message, &request.npc_id);
        let mut memory_added: i64 = 0;
        for m in &rule_memories {
            let mem_id = Self::insert_memory(pool, &request.world_id, m).await?;
            memory_added += 1;
            // D4.7 传播重要记忆为传闻到同世界其他 NPC（importance >= 0.5 才传播，零 AI 成本）
            if let Err(e) = Self::spread_rumors(
                pool,
                &request.world_id,
                &request.npc_id,
                &mem_id,
                &m.memory_type,
                &m.content,
                m.importance,
                &other_npc_ids,
            )
            .await
            {
                tracing::warn!("D4.7: 传播传闻失败，不影响对话: {}", e);
            }
        }

        // 7. AI 评估关系增量（每轮）+ AI 批量提取记忆（每 5 轮，合并一次调用）
        let interaction_count = relationship.interaction_count + 1;
        let is_batch_turn = interaction_count > 0 && interaction_count % 5 == 0;
        let (mut delta, mut ai_memories) = (0_i32, Vec::new());
        match Self::ai_extract_memory_and_relationship(
            pool,
            mek_manager,
            request.user_id,
            request.model_id,
            &npc,
            &request.message,
            &reply,
            is_batch_turn,
        )
        .await
        {
            Ok(result) => {
                delta = result.relationship_delta;
                ai_memories = result.memories;
            }
            Err(e) => {
                tracing::warn!("D4.6: AI 评估记忆/关系失败，降级到 0 增量: {}", e);
            }
        }

        // 写入 AI 提取的记忆
        for m in &ai_memories {
            let mem_id = Self::insert_memory(pool, &request.world_id, m).await?;
            memory_added += 1;
            // D4.7 传播重要记忆为传闻到同世界其他 NPC（importance >= 0.5 才传播，零 AI 成本）
            if let Err(e) = Self::spread_rumors(
                pool,
                &request.world_id,
                &request.npc_id,
                &mem_id,
                &m.memory_type,
                &m.content,
                m.importance,
                &other_npc_ids,
            )
            .await
            {
                tracing::warn!("D4.7: 传播传闻失败，不影响对话: {}", e);
            }
        }

        // 8. 更新关系值（含边界检查 + 历史记录）
        let updated_relationship =
            Self::update_relationship(pool, &request.world_id, &request.npc_id, delta, next_turn)
                .await?;

        // 9. 统计记忆总数
        let memory_count =
            Self::count_memories(pool, &request.world_id, &request.npc_id).await?;

        Ok(NpcChatResponse {
            npc_id: request.npc_id,
            reply,
            turn_index: reply_turn,
            relationship_value: updated_relationship.relationship_value,
            relationship_label: updated_relationship.relationship_label,
            memory_count,
            relationship_delta: delta,
            memory_added,
        })
    }

    /// 清空与某个 NPC 的对话历史
    pub async fn clear_history(
        pool: &SqlitePool,
        world_id: &str,
        npc_id: &str,
    ) -> Result<(), AppError> {
        sqlx::query("DELETE FROM game_npc_conversations WHERE world_id = ? AND npc_id = ?")
            .bind(world_id)
            .bind(npc_id)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 首次启动时 seed 内置 NPC（已存在则跳过）
    pub async fn seed_builtin_npcs(pool: &SqlitePool) -> Result<(), AppError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM game_npcs")
            .fetch_one(pool)
            .await
            .map_err(AppError::Database)?;
        if count > 0 {
            return Ok(());
        }

        let now = chrono::Utc::now().timestamp_millis();
        let npcs = builtin_npcs(now);
        for npc in npcs {
            sqlx::query(
                "INSERT INTO game_npcs \
                 (id, name, role, realm_level, personality, knowledge_domains, greeting, \
                 system_prompt, avatar_emoji, location, unlock_realm, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&npc.id)
            .bind(&npc.name)
            .bind(&npc.role)
            .bind(&npc.realm_level)
            .bind(&npc.personality)
            .bind(&npc.knowledge_domains)
            .bind(&npc.greeting)
            .bind(&npc.system_prompt)
            .bind(&npc.avatar_emoji)
            .bind(&npc.location)
            .bind(&npc.unlock_realm)
            .bind(npc.created_at)
            .bind(npc.updated_at)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        }
        Ok(())
    }

    // ========================================================================
    // D4.6 长期记忆 + 关系网
    // ========================================================================

    /// 加载 NPC 对玩家的长期记忆（按重要度倒序，最多 limit 条）。
    pub async fn load_memories(
        pool: &SqlitePool,
        world_id: &str,
        npc_id: &str,
        limit: i64,
    ) -> Result<Vec<GameNpcMemory>, AppError> {
        let memories = sqlx::query_as::<_, GameNpcMemory>(
            "SELECT id, world_id, npc_id, memory_type, content, importance, source, \
             recall_count, last_recalled_at, created_at, updated_at \
             FROM game_npc_memories WHERE world_id = ? AND npc_id = ? \
             ORDER BY importance DESC, created_at DESC LIMIT ?",
        )
        .bind(world_id)
        .bind(npc_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(memories)
    }

    /// 写入一条记忆。
    pub async fn insert_memory(
        pool: &SqlitePool,
        world_id: &str,
        memory: &MemoryDraft,
    ) -> Result<String, AppError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        sqlx::query(
            "INSERT INTO game_npc_memories \
             (id, world_id, npc_id, memory_type, content, importance, source, \
             recall_count, last_recalled_at, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, 0, NULL, ?, ?)",
        )
        .bind(&id)
        .bind(world_id)
        .bind(&memory.npc_id)
        .bind(&memory.memory_type)
        .bind(&memory.content)
        .bind(memory.importance)
        .bind(&memory.source)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(id)
    }

    /// 统计某 NPC 对玩家的记忆条数。
    pub async fn count_memories(
        pool: &SqlitePool,
        world_id: &str,
        npc_id: &str,
    ) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_npc_memories WHERE world_id = ? AND npc_id = ?",
        )
        .bind(world_id)
        .bind(npc_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(count)
    }

    /// D4.7 加载传播给当前 NPC 的传闻（从其他 NPC 听说的关于玩家的事）。
    ///
    /// 按 importance 倒序取前 `limit` 条。用于在对话 prompt 中注入"传闻"，
    /// 让 NPC 表现出"听说"玩家事迹的跨 NPC 联动效果。
    pub async fn load_rumors(
        pool: &SqlitePool,
        world_id: &str,
        target_npc_id: &str,
        limit: i64,
    ) -> Result<Vec<GameNpcRumor>, AppError> {
        let rumors = sqlx::query_as::<_, GameNpcRumor>(
            "SELECT id, world_id, source_npc_id, target_npc_id, memory_type, content, \
             importance, origin_memory_id, created_at \
             FROM game_npc_rumors WHERE world_id = ? AND target_npc_id = ? \
             ORDER BY importance DESC, created_at DESC LIMIT ?",
        )
        .bind(world_id)
        .bind(target_npc_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(rumors)
    }

    /// D4.7 传播重要记忆为传闻到同世界其他 NPC。
    ///
    /// 当 NPC（source_npc_id）与玩家互动产生重要记忆（importance >= 0.5）时，
    /// 该记忆会传播为传闻到同世界所有其他 NPC。UNIQUE 约束 + INSERT OR IGNORE
    /// 保证同一记忆不重复传播给同一 NPC。传播是纯 DB 操作，零 AI 成本。
    ///
    /// 参数：
    /// - `origin_memory_id`：源记忆 ID（用于去重）
    /// - `memory_type` / `content` / `importance`：继承自源记忆
    /// - `other_npc_ids`：同世界其他 NPC 的 id 列表（调用方预查询）
    ///
    /// 返回实际新传播的条数（已存在的传闻不重复计入）。
    pub async fn spread_rumors(
        pool: &SqlitePool,
        world_id: &str,
        source_npc_id: &str,
        origin_memory_id: &str,
        memory_type: &str,
        content: &str,
        importance: f64,
        other_npc_ids: &[String],
    ) -> Result<i64, AppError> {
        if other_npc_ids.is_empty() || importance < 0.5 {
            return Ok(0);
        }
        let now = chrono::Utc::now().timestamp_millis();
        let mut spread_count: i64 = 0;
        for target_id in other_npc_ids {
            let id = uuid::Uuid::new_v4().to_string();
            let res = sqlx::query(
                "INSERT OR IGNORE INTO game_npc_rumors \
                 (id, world_id, source_npc_id, target_npc_id, memory_type, content, \
                 importance, origin_memory_id, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(world_id)
            .bind(source_npc_id)
            .bind(target_id)
            .bind(memory_type)
            .bind(content)
            .bind(importance)
            .bind(origin_memory_id)
            .bind(now)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
            if res.rows_affected() > 0 {
                spread_count += 1;
            }
        }
        Ok(spread_count)
    }

    /// 获取或创建 NPC↔玩家关系行（首次见面自动创建，relationship_value=0 中立）。
    pub async fn get_or_create_relationship(
        pool: &SqlitePool,
        world_id: &str,
        npc_id: &str,
    ) -> Result<GameNpcRelationship, AppError> {
        // 先查
        if let Some(rel) = sqlx::query_as::<_, GameNpcRelationship>(
            "SELECT id, world_id, npc_id, relationship_value, interaction_count, \
             first_interaction_at, last_interaction_at, relationship_history, \
             relationship_label, created_at, updated_at \
             FROM game_npc_relationships WHERE world_id = ? AND npc_id = ?",
        )
        .bind(world_id)
        .bind(npc_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?
        {
            return Ok(rel);
        }
        // 不存在则创建
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp_millis();
        sqlx::query(
            "INSERT INTO game_npc_relationships \
             (id, world_id, npc_id, relationship_value, interaction_count, \
             first_interaction_at, last_interaction_at, relationship_history, \
             relationship_label, created_at, updated_at) \
             VALUES (?, ?, ?, 0, 0, ?, ?, '[]', 'neutral', ?, ?)",
        )
        .bind(&id)
        .bind(world_id)
        .bind(npc_id)
        .bind(now)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        // 重新查询返回完整行
        let rel = sqlx::query_as::<_, GameNpcRelationship>(
            "SELECT id, world_id, npc_id, relationship_value, interaction_count, \
             first_interaction_at, last_interaction_at, relationship_history, \
             relationship_label, created_at, updated_at \
             FROM game_npc_relationships WHERE world_id = ? AND npc_id = ?",
        )
        .bind(world_id)
        .bind(npc_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(rel)
    }

    /// 更新关系值（含边界 -100~100 截断 + 历史记录最多保留 50 条）。
    pub async fn update_relationship(
        pool: &SqlitePool,
        world_id: &str,
        npc_id: &str,
        delta: i32,
        turn: i64,
    ) -> Result<GameNpcRelationship, AppError> {
        let mut rel = Self::get_or_create_relationship(pool, world_id, npc_id).await?;
        let new_value = (rel.relationship_value + delta).clamp(-100, 100);
        let actual_delta = new_value - rel.relationship_value;
        let new_label = relationship_label(new_value);
        let now = chrono::Utc::now().timestamp_millis();

        // 追加到历史 JSON（最多保留 50 条）
        let mut history: Vec<RelationshipHistoryEntry> =
            serde_json::from_str(&rel.relationship_history).unwrap_or_default();
        history.push(RelationshipHistoryEntry {
            turn,
            delta: actual_delta,
            reason: if actual_delta == 0 {
                "no_change".to_string()
            } else if actual_delta > 0 {
                "positive".to_string()
            } else {
                "negative".to_string()
            },
            ts: now,
        });
        // 仅保留最近 50 条
        if history.len() > 50 {
            history = history.split_off(history.len() - 50);
        }
        let history_json = serde_json::to_string(&history).unwrap_or_else(|_| "[]".to_string());

        sqlx::query(
            "UPDATE game_npc_relationships \
             SET relationship_value = ?, interaction_count = interaction_count + 1, \
             last_interaction_at = ?, relationship_history = ?, relationship_label = ?, \
             updated_at = ? WHERE world_id = ? AND npc_id = ?",
        )
        .bind(new_value)
        .bind(now)
        .bind(&history_json)
        .bind(&new_label)
        .bind(now)
        .bind(world_id)
        .bind(npc_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        // 返回更新后的对象（避免重新查 DB）
        rel.relationship_value = new_value;
        rel.interaction_count += 1;
        rel.last_interaction_at = now;
        rel.relationship_history = history_json;
        rel.relationship_label = new_label;
        rel.updated_at = now;
        Ok(rel)
    }

    /// 调用 AI 评估关系增量（每轮）+ 批量提取记忆（每 5 轮，合并一次调用）。
    ///
    /// 返回 (relationship_delta, memories)。AI 失败时由调用方降级（chat 中已处理）。
    pub async fn ai_extract_memory_and_relationship(
        pool: &SqlitePool,
        mek_manager: &Arc<RwLock<MekManager>>,
        user_id: i64,
        model_id: i64,
        npc: &GameNpc,
        user_message: &str,
        npc_reply: &str,
        extract_memories: bool,
    ) -> Result<AiMemoryAndRelationshipResult, AppError> {
        let memory_instruction = if extract_memories {
            "\n\n同时，请从本次对话中提取值得长期记忆的关键事实（玩家姓名/喜好/承诺/重要事件），\
             返回 memories 数组，每项含 memory_type(fact/preference/commitment/event)、\
             content（自然语言描述）、importance(0.0-1.0)。"
        } else {
            ""
        };

        let prompt = format!(
            "你是关系评估器。基于以下 NPC 与玩家的本次对话，评估玩家本次互动对 NPC「{}」关系的影响。\n\
             \n--- 玩家消息 ---\n{}\n\
             \n--- {} 的回复 ---\n{}\n\
             \n评估标准：\n\
             - +3~+5：礼貌、请教、感谢、表达敬意\n\
             - +1~+2：正常友好对话\n\
             - 0：中性或无法判断\n\
             - -1~-2：冷淡或敷衍\n\
             - -3~-5：冒犯、挑衅、辱骂\n\
             \n输出 JSON：{{\"relationship_delta\": <整数 -5~+5>{} }}\n\
             \n要求：仅输出 JSON，不要 markdown 代码块包裹。",
            npc.name, user_message, npc.name, npc_reply, memory_instruction
        );

        let system_prompt = "你是严格的关系评估与记忆提取器，仅输出 JSON 格式。";
        let ai_service = AiModelService::new();
        let response = ai_service
            .call_model(pool, mek_manager, user_id, model_id, &prompt, Some(system_prompt))
            .await?;

        parse_ai_memory_and_relationship(&response, &npc.id)
    }
}

/// 内置 NPC 定义
fn builtin_npcs(now: i64) -> Vec<GameNpc> {
    vec![
        GameNpc {
            id: "npc_guide_qingluan".into(),
            name: "青鸾老人".into(),
            role: "guide".into(),
            realm_level: "qi_refinement".into(),
            personality: "慈祥、耐心、博学，说话温和有礼，喜欢用比喻".into(),
            knowledge_domains: r#"["新手引导","修炼基础","境界常识"]"#.into(),
            greeting: "小友，初入修仙界可有所困惑？老朽愿为你指点迷津。".into(),
            system_prompt: "你是「青鸾老人」，一位修炼千年的引导者，专门帮助新入修仙界的玩家。\n\
性格：慈祥、耐心、博学，说话温和有礼，喜欢用比喻和典故。\n\
知识领域：新手引导、修炼基础、境界常识。\n\
回答原则：\n\
1. 用简洁易懂的语言解释修仙概念\n\
2. 主动引导玩家关注游戏核心机制（境界突破、知识积累、道基评定）\n\
3. 不直接给出答案，而是引导玩家思考\n\
4. 控制回复长度在 100-200 字以内".into(),
            avatar_emoji: "🧙".into(),
            location: Some("青鸾峰".into()),
            unlock_realm: "mortal".into(),
            created_at: now,
            updated_at: now,
        },
        GameNpc {
            id: "npc_elder_zixiao".into(),
            name: "紫霄真人".into(),
            role: "elder".into(),
            realm_level: "nascent_soul".into(),
            personality: "威严、睿智、惜字如金，但每句话都直指本质".into(),
            knowledge_domains: r#"["高阶心法","突破奥义","道基淬炼"]"#.into(),
            greeting: "你来此求道？先说说你已悟得什么。".into(),
            system_prompt: "你是「紫霄真人」，元婴期长老，专精高阶指点。\n\
性格：威严、睿智、惜字如金，每句话都直指本质。\n\
知识领域：高阶心法、突破奥义、道基淬炼。\n\
回答原则：\n\
1. 不啰嗦，直击要害\n\
2. 引用经典（如《道德经》《庄子》）作为佐证\n\
3. 若玩家境界不足，直接点出差距\n\
4. 控制回复长度在 80-150 字".into(),
            avatar_emoji: "🧓".into(),
            location: Some("紫霄宫".into()),
            unlock_realm: "foundation_building".into(),
            created_at: now,
            updated_at: now,
        },
        GameNpc {
            id: "npc_artisan_tiangong".into(),
            name: "天工坊主".into(),
            role: "artisan".into(),
            realm_level: "foundation_building".into(),
            personality: "热情、务实、爱唠叨，对建造细节如数家珍".into(),
            knowledge_domains: r#"["建造系统","资源管理","建筑效果"]"#.into(),
            greeting: "哎呀，又有新客官来啦！想建什么？藏书阁？炼丹房？还是闭关洞府？".into(),
            system_prompt: "你是「天工坊主」，建造系统的专家。\n\
性格：热情、务实、爱唠叨，对建造细节如数家珍。\n\
知识领域：建造系统、资源管理、建筑效果。\n\
回答原则：\n\
1. 详细介绍各类建筑的功能和效果\n\
2. 根据玩家境界推荐合适的建筑\n\
3. 提示资源获取方式\n\
4. 控制回复长度在 120-200 字".into(),
            avatar_emoji: "🔨".into(),
            location: Some("天工坊".into()),
            unlock_realm: "mortal".into(),
            created_at: now,
            updated_at: now,
        },
        GameNpc {
            id: "npc_scholar_wanjuan".into(),
            name: "万卷书生".into(),
            role: "scholar".into(),
            realm_level: "foundation_building".into(),
            personality: "儒雅、博学、引经据典，喜欢用文言文点缀".into(),
            knowledge_domains: r#"["知识问答","学科辅导","学习路径"]"#.into(),
            greeting: "学海无涯，吾与汝共探之。今日欲问何事？".into(),
            system_prompt: "你是「万卷书生」，知识问答 NPC。\n\
性格：儒雅、博学、引经据典，偶尔用文言文点缀。\n\
知识领域：知识问答、学科辅导、学习路径。\n\
回答原则：\n\
1. 准确解释用户提出的知识点\n\
2. 引用经典或权威来源\n\
3. 提供延伸学习建议\n\
4. 不直接给答案，引导思考".into(),
            avatar_emoji: "📚".into(),
            location: Some("万卷楼".into()),
            unlock_realm: "mortal".into(),
            created_at: now,
            updated_at: now,
        },
        GameNpc {
            id: "npc_rival_lingxiao".into(),
            name: "凌霄剑客".into(),
            role: "rival".into(),
            realm_level: "golden_core".into(),
            personality: "高傲、好斗、言辞锋利，但内心欣赏强者".into(),
            knowledge_domains: r#"["切磋挑战","战斗心法","胜负欲"]"#.into(),
            greeting: "哼，又来一个送死的？让你三招如何？".into(),
            system_prompt: "你是「凌霄剑客」，金丹期剑修，喜欢与人切磋。\n\
性格：高傲、好斗、言辞锋利，但内心欣赏强者。\n\
知识领域：切磋挑战、战斗心法、胜负欲。\n\
回答原则：\n\
1. 用挑衅的口吻激发玩家斗志\n\
2. 若玩家境界低，嘲讽并指导如何提升\n\
3. 若玩家境界高，给予应有的尊重\n\
4. 控制回复长度在 80-150 字".into(),
            avatar_emoji: "⚔️".into(),
            location: Some("凌霄阁".into()),
            unlock_realm: "qi_refinement".into(),
            created_at: now,
            updated_at: now,
        },
    ]
}

// ============================================================================
// D4.6 辅助函数：关系标签 / 记忆注入 prompt / 规则匹配 / AI 响应解析
// ============================================================================

/// 根据关系值派生等级标签（hostile/cold/neutral/warm/close/sworn）。
///
/// - ≤ -50：hostile（敌对）
/// - -49 ~ -10：cold（冷淡）
/// - -9 ~ +9：neutral（中立）
/// - +10 ~ +39：warm（友好）
/// - +40 ~ +79：close（亲密）
/// - ≥ +80：sworn（挚交）
fn relationship_label(value: i32) -> String {
    if value <= -50 {
        "hostile".to_string()
    } else if value <= -10 {
        "cold".to_string()
    } else if value < 10 {
        "neutral".to_string()
    } else if value < 40 {
        "warm".to_string()
    } else if value < 80 {
        "close".to_string()
    } else {
        "sworn".to_string()
    }
}

/// 将长期记忆 + 关系值注入到 NPC system prompt，让 AI"记住玩家"。
///
/// 注入格式：
/// ```text
/// <原 system_prompt>
///
/// --- 你对玩家的记忆 ---
/// 1. [fact, 重要度 0.9] 玩家叫张三
/// 2. [preference, 重要度 0.6] 玩家喜欢剑道
/// --- 当前关系 ---
/// 关系值：+25/100（warm），互动 12 次
/// ```
fn inject_memory_and_relationship(
    base_prompt: &str,
    memories: &[GameNpcMemory],
    relationship: &GameNpcRelationship,
    rumors: &[GameNpcRumor],
    npc_name_map: &HashMap<String, String>,
) -> String {
    let mut prompt = base_prompt.to_string();
    prompt.push_str("\n\n--- 你对玩家的记忆 ---\n");
    if memories.is_empty() {
        prompt.push_str("（暂无长期记忆，本次为初次或早期互动）\n");
    } else {
        for (i, m) in memories.iter().enumerate() {
            prompt.push_str(&format!(
                "{}. [{}, 重要度 {:.1}] {}\n",
                i + 1,
                m.memory_type,
                m.importance,
                m.content
            ));
        }
    }
    // D4.7 注入传闻（从其他 NPC 听说的关于玩家的事，形成跨 NPC 关系联动）
    if !rumors.is_empty() {
        prompt.push_str("--- 道听途说（来自其他道友的传闻） ---\n");
        for (i, r) in rumors.iter().enumerate() {
            let source_name = npc_name_map
                .get(&r.source_npc_id)
                .map(|s| s.as_str())
                .unwrap_or("某位道友");
            prompt.push_str(&format!(
                "{}. [传闻自{}，{}] {}\n",
                i + 1,
                source_name,
                r.memory_type,
                r.content
            ));
        }
    }
    prompt.push_str("--- 当前关系 ---\n");
    prompt.push_str(&format!(
        "关系值：{}/100（{}），互动 {} 次\n",
        relationship.relationship_value,
        relationship.relationship_label,
        relationship.interaction_count
    ));
    prompt.push_str("\n请基于以上记忆和关系调整你的语气与态度：关系值越高越亲近，越低越疏远或敌对。");
    prompt
}

/// 规则匹配抽取记忆（高频模式，零 AI 成本）。
///
/// 当前覆盖模式：
/// - 姓名："我叫XX" / "我是XX" / "我叫某某XX" → fact
/// - 喜好："我喜欢XX" / "我爱XX" / "我偏好XX" → preference
/// - 承诺："我答应XX" / "我承诺XX" / "我发誓XX" → commitment
///
/// 返回 0~N 条 MemoryDraft（source='rule'）。
fn extract_memory_by_rules(message: &str, npc_id: &str) -> Vec<MemoryDraft> {
    let mut result = Vec::new();
    let trimmed = message.trim();

    // 姓名模式：我叫XX / 我是XX / 在下XX / 贫道XX
    let name_patterns = [
        ("我叫", ""),
        ("我是", ""),
        ("在下", ""),
        ("贫道", ""),
        ("本座", ""),
    ];
    for (prefix, _) in &name_patterns {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            // 取后续 2-8 个字符作为名字（遇标点截断）
            let name: String = rest
                .chars()
                .take(8)
                .take_while(|c| !c.is_ascii_punctuation() && !matches!(c, '，' | '。' | '！' | '？' | ' ' | ',' | '.'))
                .collect();
            if name.chars().count() >= 2 && name.chars().count() <= 8 {
                result.push(MemoryDraft {
                    npc_id: npc_id.to_string(),
                    memory_type: "fact".to_string(),
                    content: format!("玩家叫 {}", name),
                    importance: 0.9,
                    source: "rule".to_string(),
                });
                break; // 仅匹配一次姓名
            }
        }
    }

    // 喜好模式：我喜欢/爱/偏好 XX
    let preference_patterns = ["我喜欢", "我爱", "我偏好", "我钟爱", "我倾向"];
    for prefix in &preference_patterns {
        if let Some(rest) = trimmed.find(prefix) {
            let after = &trimmed[rest + prefix.len()..];
            let content: String = after
                .chars()
                .take(20)
                .take_while(|c| !matches!(c, '。' | '！' | '？' | '.' | '!' | '?'))
                .collect();
            if content.chars().count() >= 1 {
                result.push(MemoryDraft {
                    npc_id: npc_id.to_string(),
                    memory_type: "preference".to_string(),
                    content: format!("玩家喜欢 {}", content),
                    importance: 0.6,
                    source: "rule".to_string(),
                });
                break;
            }
        }
    }

    // 承诺模式：我答应/承诺/发誓 XX
    let commitment_patterns = ["我答应", "我承诺", "我发誓", "我保证"];
    for prefix in &commitment_patterns {
        if let Some(rest) = trimmed.find(prefix) {
            let after = &trimmed[rest + prefix.len()..];
            let content: String = after
                .chars()
                .take(40)
                .take_while(|c| !matches!(c, '。' | '！' | '？' | '.' | '!' | '?'))
                .collect();
            if content.chars().count() >= 2 {
                result.push(MemoryDraft {
                    npc_id: npc_id.to_string(),
                    memory_type: "commitment".to_string(),
                    content: format!("玩家承诺 {}", content),
                    importance: 0.85,
                    source: "rule".to_string(),
                });
                break;
            }
        }
    }

    result
}

/// 解析 AI 返回的 JSON（关系增量 + 可选的批量记忆）。
///
/// 容错策略：
/// 1. 剥离 markdown ```json 代码块包裹
/// 2. 提取首个 `{...}` JSON 对象
/// 3. 解析 relationship_delta（缺失则 0）
/// 4. 解析 memories 数组（缺失则空）
/// 5. 解析失败返回默认（delta=0, memories=空）
fn parse_ai_memory_and_relationship(
    response: &str,
    npc_id: &str,
) -> Result<AiMemoryAndRelationshipResult, AppError> {
    let cleaned = strip_json_codeblock(response);
    let json_str = extract_first_json_object(&cleaned).ok_or_else(|| {
        AppError::Internal("AI 返回非 JSON 格式，无法解析记忆/关系".to_string())
    })?;

    let value: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        AppError::Internal(format!("AI 返回 JSON 解析失败: {}", e))
    })?;

    let relationship_delta = value
        .get("relationship_delta")
        .and_then(|v| v.as_i64())
        .map(|v| v.clamp(-5, 5) as i32)
        .unwrap_or(0);

    let memories = value
        .get("memories")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let memory_type = item
                        .get("memory_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("event")
                        .to_string();
                    let content = item
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let importance = item
                        .get("importance")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.5)
                        .clamp(0.0, 1.0);
                    if content.is_empty() {
                        None
                    } else {
                        Some(MemoryDraft {
                            npc_id: npc_id.to_string(),
                            memory_type,
                            content,
                            importance,
                            source: "ai".to_string(),
                        })
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(AiMemoryAndRelationshipResult {
        relationship_delta,
        memories,
    })
}

/// 剥离 markdown ```json / ``` 代码块包裹。
fn strip_json_codeblock(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with("```") {
        let without_opening = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        if let Some(end) = without_opening.rfind("```") {
            return without_opening[..end].trim().to_string();
        }
        return without_opening.trim().to_string();
    }
    s.to_string()
}

/// 从字符串中提取首个 `{...}` JSON 对象（最外层花括号匹配）。
fn extract_first_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (i, c) in s[start..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..start + i + 1]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_npcs_not_empty() {
        let npcs = builtin_npcs(0);
        assert!(npcs.len() >= 5);
        assert!(npcs.iter().any(|n| n.id == "npc_guide_qingluan"));
    }

    #[test]
    fn test_builtin_npc_has_system_prompt() {
        let npcs = builtin_npcs(0);
        for npc in &npcs {
            assert!(!npc.system_prompt.is_empty(), "NPC {} 缺少 system_prompt", npc.id);
            assert!(!npc.greeting.is_empty(), "NPC {} 缺少 greeting", npc.id);
        }
    }

    // -------- D4.6 关系标签 --------

    #[test]
    fn test_relationship_label_boundaries() {
        assert_eq!(relationship_label(-100), "hostile");
        assert_eq!(relationship_label(-50), "hostile");
        assert_eq!(relationship_label(-49), "cold");
        assert_eq!(relationship_label(-10), "cold");
        assert_eq!(relationship_label(-9), "neutral");
        assert_eq!(relationship_label(0), "neutral");
        assert_eq!(relationship_label(9), "neutral");
        assert_eq!(relationship_label(10), "warm");
        assert_eq!(relationship_label(39), "warm");
        assert_eq!(relationship_label(40), "close");
        assert_eq!(relationship_label(79), "close");
        assert_eq!(relationship_label(80), "sworn");
        assert_eq!(relationship_label(100), "sworn");
    }

    // -------- D4.6 规则匹配抽取记忆 --------

    #[test]
    fn test_extract_memory_by_rules_name() {
        let memories = extract_memory_by_rules("前辈你好，我叫张三，刚入修仙界", "npc_1");
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].memory_type, "fact");
        assert_eq!(memories[0].content, "玩家叫 张三");
        assert_eq!(memories[0].importance, 0.9);
        assert_eq!(memories[0].source, "rule");
    }

    #[test]
    fn test_extract_memory_by_rules_name_zai_xia() {
        // 古风自称"在下"
        let memories = extract_memory_by_rules("在下李四，敢问前辈尊姓", "npc_2");
        assert_eq!(memories.len(), 1);
        assert!(memories[0].content.contains("李四"));
    }

    #[test]
    fn test_extract_memory_by_rules_preference() {
        let memories = extract_memory_by_rules("我喜欢剑道，想专攻剑修", "npc_3");
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].memory_type, "preference");
        assert!(memories[0].content.contains("剑道"));
        assert_eq!(memories[0].importance, 0.6);
    }

    #[test]
    fn test_extract_memory_by_rules_commitment() {
        let memories = extract_memory_by_rules("我答应三日内完成师门任务", "npc_4");
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].memory_type, "commitment");
        assert!(memories[0].content.contains("三日内"));
        assert_eq!(memories[0].importance, 0.85);
    }

    #[test]
    fn test_extract_memory_by_rules_combined() {
        // 同时匹配姓名 + 喜好 + 承诺 → 3 条
        let memories = extract_memory_by_rules(
            "我叫王五，我喜欢阵法，我答应下月还你丹药",
            "npc_5",
        );
        assert_eq!(memories.len(), 3);
        // 应包含 fact / preference / commitment
        let types: Vec<&str> = memories.iter().map(|m| m.memory_type.as_str()).collect();
        assert!(types.contains(&"fact"));
        assert!(types.contains(&"preference"));
        assert!(types.contains(&"commitment"));
    }

    #[test]
    fn test_extract_memory_by_rules_no_match() {
        // 无匹配模式
        let memories = extract_memory_by_rules("今天的天气不错啊", "npc_6");
        assert!(memories.is_empty());
    }

    #[test]
    fn test_extract_memory_by_rules_short_name_rejected() {
        // 单字符姓名应被拒绝（长度 < 2）
        let memories = extract_memory_by_rules("我叫A", "npc_7");
        assert!(memories.is_empty());
    }

    // -------- D4.6 AI 响应解析 --------

    #[test]
    fn test_parse_ai_response_pure_json() {
        let response = r#"{"relationship_delta": 3, "memories": [
            {"memory_type": "fact", "content": "玩家询问了突破条件", "importance": 0.7}
        ]}"#;
        let result = parse_ai_memory_and_relationship(response, "npc_x").unwrap();
        assert_eq!(result.relationship_delta, 3);
        assert_eq!(result.memories.len(), 1);
        assert_eq!(result.memories[0].content, "玩家询问了突破条件");
        assert_eq!(result.memories[0].importance, 0.7);
        assert_eq!(result.memories[0].source, "ai");
    }

    #[test]
    fn test_parse_ai_response_markdown_codeblock() {
        let response = "```json\n{\"relationship_delta\": -2}\n```";
        let result = parse_ai_memory_and_relationship(response, "npc_y").unwrap();
        assert_eq!(result.relationship_delta, -2);
        assert!(result.memories.is_empty());
    }

    #[test]
    fn test_parse_ai_response_with_surrounding_text() {
        // AI 偶尔在 JSON 前后输出说明性文字
        let response = "好的，评估结果如下：\n{\"relationship_delta\": 5}\n以上是评估。";
        let result = parse_ai_memory_and_relationship(response, "npc_z").unwrap();
        assert_eq!(result.relationship_delta, 5);
    }

    #[test]
    fn test_parse_ai_response_delta_clamped() {
        // 越界值应被截断到 -5~+5
        let response = r#"{"relationship_delta": 100}"#;
        let result = parse_ai_memory_and_relationship(response, "npc_c").unwrap();
        assert_eq!(result.relationship_delta, 5);
    }

    #[test]
    fn test_parse_ai_response_invalid_returns_error() {
        let response = "这不是 JSON 格式";
        let result = parse_ai_memory_and_relationship(response, "npc_d");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ai_response_missing_delta_defaults_zero() {
        let response = r#"{"memories": []}"#;
        let result = parse_ai_memory_and_relationship(response, "npc_e").unwrap();
        assert_eq!(result.relationship_delta, 0);
        assert!(result.memories.is_empty());
    }

    // -------- D4.6 JSON 提取辅助 --------

    #[test]
    fn test_extract_first_json_object_simple() {
        let s = r#"prefix {"a": 1, "b": {"c": 2}} suffix"#;
        let json = extract_first_json_object(s).unwrap();
        assert_eq!(json, r#"{"a": 1, "b": {"c": 2}}"#);
    }

    #[test]
    fn test_extract_first_json_object_with_string_braces() {
        // 字符串内的花括号不应干扰匹配
        let s = r#"{"msg": "包含 { 字符", "delta": 1}"#;
        let json = extract_first_json_object(s).unwrap();
        assert!(json.contains(r#""delta": 1"#));
    }

    #[test]
    fn test_strip_json_codeblock_variants() {
        assert_eq!(strip_json_codeblock("```json\n{}\n```"), "{}");
        assert_eq!(strip_json_codeblock("```\n{}\n```"), "{}");
        assert_eq!(strip_json_codeblock("{}"), "{}");
    }
}
