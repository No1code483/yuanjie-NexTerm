//! D3.8 人格系统补全 — 人格切换 + 记忆核心服务
//!
//! 设计意图（对照 `.trae/rules/项目核心设计意图.md`）：
//! - 静态内置人格（4 种）：人格参数固定，对话走云端 API
//! - 自生长人格（self_growing，D3.8.3）：初始白纸，由底层智能根据用户画像驱动生长
//! - 人格记忆：每个人格积累独立的交互摘要 + 用户偏好，切换回该人格时恢复上下文
//! - 非侵入式：本服务为纯数据层，不修改已有 builtin_personas / 切换命令
//!
//! 数据库表：
//! - xin_persona_memories（migration v100）：人格记忆
//! - xin_persona_switch_log（migration v101）：切换历史

use chrono::Utc;
use sqlx::SqlitePool;

use crate::error::app_error::AppError;

const DEFAULT_SWITCH_TRIGGER: &str = "manual";

/// 人格记忆 — 每个人格积累的独立交互上下文
///
/// 存储于 `xin_persona_memories` 表，persona_id 唯一。
/// 切换回某人格时，从此表恢复该人格"记得"的用户偏好与交互摘要。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersonaMemory {
    pub persona_id: String,
    /// 该人格下的交互摘要（自然语言，供 system prompt 注入）
    pub interaction_summary: String,
    /// 该人格观察到的用户偏好（JSON object，如 {"style": "concise", "lang": "zh"}）
    pub user_preferences: serde_json::Value,
    /// 该人格下用户关注的话题标签（JSON array，如 ["rust", "ai", "terminal"]）
    pub topic_tags: serde_json::Value,
    /// 累计交互次数
    pub interaction_count: i64,
    /// 最后交互时间（RFC3339）
    pub last_interaction_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 人格切换记录 — 一次切换的审计日志
///
/// 存储于 `xin_persona_switch_log` 表。用于：
/// 1) 前端人格切换时间线展示
/// 2) 自生长人格的学习数据源（分析用户切换习惯）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersonaSwitchRecord {
    pub id: i64,
    pub from_persona_id: Option<String>,
    pub to_persona_id: String,
    pub switched_at: String,
    /// 切换触发原因：manual（用户手动）/ auto_grow（自生长触发）/ system（系统初始化）
    pub trigger: String,
}

/// D3.8 人格系统服务（无状态纯逻辑层）
///
/// D3.8.1：数据层 CRUD（人格记忆 + 切换历史）
/// D3.8.2：system prompt 注入（在 xin_context_service 集成）
/// D3.8.3：自生长人格 + 底层智能联动（grow_persona）
pub struct XinPersonalityService;

// ============ D3.8.3 自生长人格：用户画像 + 生长算法 ============

/// 用户画像 — 由底层智能监测数据生成，驱动自生长人格参数调整
///
/// 数据来源（对照 `.trae/rules/项目核心设计意图.md` §二 底层智能）：
/// - 兴趣领域：从 activity_logs / 对话话题提取
/// - 沟通风格偏好：从 behavior_patterns / 对话分析
/// - 情绪模式：从 xin_moods 历史趋势
/// - 技术深度：从用户使用 Yuan Code / 终端频率推断
///
/// 边界：用户画像由底层智能生成，但自生长人格的对话回复走云端 API。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserProfile {
    /// 用户偏好的对话正式度（0.0 随意 ~ 1.0 正式）
    pub formality: f64,
    /// 用户偏好的回复详细度（0.0 简洁 ~ 1.0 详尽）
    pub verbosity: f64,
    /// 用户对幽默的接受度（0.0 严肃 ~ 1.0 喜爱幽默）
    pub humor: f64,
    /// 用户技术深度（0.0 初学者 ~ 1.0 专家）
    pub technical_depth: f64,
    /// 用户对共情的需求（0.0 理性 ~ 1.0 情感导向）
    pub empathy: f64,
    /// 用户主要兴趣领域（用于人格 traits 生成）
    pub interest_domains: Vec<String>,
    /// 画像置信度（0.0~1.0，数据量不足时降低）
    pub confidence: f64,
}

impl Default for UserProfile {
    fn default() -> Self {
        // 默认中性画像（confidence=0 表示无数据，不触发生长）
        Self {
            formality: 0.5,
            verbosity: 0.5,
            humor: 0.5,
            technical_depth: 0.5,
            empathy: 0.5,
            interest_domains: vec![],
            confidence: 0.0,
        }
    }
}

/// 自生长人格的生长结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrownPersona {
    /// 生长后的 speaking_style 参数
    pub formality: f64,
    pub verbosity: f64,
    pub humor: f64,
    pub technical_depth: f64,
    pub empathy: f64,
    /// 生长后的 traits（基于兴趣领域生成）
    pub traits: Vec<crate::models::xin::PersonaTrait>,
    /// 生长说明（自然语言，供前端展示 + system prompt 注入）
    pub growth_description: String,
}

impl XinPersonalityService {
    /// D3.8.3: 根据用户画像生长自生长人格
    ///
    /// 算法（轻量级，非模型训练）：
    /// - speaking_style 直接映射自用户画像偏好（加权融合，confidence 控制幅度）
    /// - traits 基于兴趣领域动态生成（如用户常聊 rust → "技术钻研" trait）
    /// - confidence < 0.3 时不生长（数据不足，保持白纸状态）
    ///
    /// 边界：本方法是纯逻辑层（不调用底层智能模型），用户画像由调用方提供。
    /// 底层智能联动：调用方（IPC 命令）负责从 intelligence_v4_* 提取画像数据传入。
    pub fn grow_persona(profile: &UserProfile) -> Option<GrownPersona> {
        // 数据不足，不生长（保持白纸静态状态）
        if profile.confidence < 0.3 {
            return None;
        }

        // confidence 控制生长幅度（低 confidence 时向中性 0.5 收敛）
        let weight = profile.confidence.clamp(0.0, 1.0);
        let neutral = 0.5;

        let blend = |v: f64| -> f64 {
            neutral * (1.0 - weight) + v * weight
        };

        let formality = blend(profile.formality);
        let verbosity = blend(profile.verbosity);
        let humor = blend(profile.humor);
        let technical_depth = blend(profile.technical_depth);
        let empathy = blend(profile.empathy);

        // 基于兴趣领域生成 traits（最多取前 4 个领域，每个生成一个 trait）
        let mut traits: Vec<crate::models::xin::PersonaTrait> = vec![
            crate::models::xin::PersonaTrait {
                name: "适应性".into(),
                value: 0.5 + weight * 0.4, // 适应性随生长提升
            },
            crate::models::xin::PersonaTrait {
                name: "理解力".into(),
                value: 0.5 + weight * 0.3,
            },
        ];

        for domain in profile.interest_domains.iter().take(4) {
            let trait_name = Self::domain_to_trait_name(domain);
            traits.push(crate::models::xin::PersonaTrait {
                name: trait_name,
                value: 0.7 + weight * 0.2, // 兴趣领域对应 trait 较高
            });
        }

        let growth_description = format!(
            "自生长人格已根据用户画像生长（置信度 {:.0}%）：\
             正式度 {:.2}、详细度 {:.2}、幽默感 {:.2}、技术深度 {:.2}、共情 {:.2}。\
             兴趣领域：{}。",
            profile.confidence * 100.0,
            formality, verbosity, humor, technical_depth, empathy,
            if profile.interest_domains.is_empty() {
                "暂无".to_string()
            } else {
                profile.interest_domains.join("、")
            }
        );

        Some(GrownPersona {
            formality,
            verbosity,
            humor,
            technical_depth,
            empathy,
            traits,
            growth_description,
        })
    }

    /// 兴趣领域 → trait 名称映射
    fn domain_to_trait_name(domain: &str) -> String {
        let d = domain.to_lowercase();
        match d.as_str() {
            "rust" | "python" | "typescript" | "java" | "go" | "c++" | "编程" | "代码" => {
                "技术钻研".into()
            }
            "数学" | "算法" | "math" | "algorithm" => "逻辑思维".into(),
            "文学" | "写作" | "literature" | "writing" => "文采".into(),
            "历史" | "history" => "博学".into(),
            "物理" | "physics" | "科学" | "science" => "严谨".into(),
            "游戏" | "game" | "gaming" => "趣味".into(),
            "音乐" | "music" => "艺术感".into(),
            "哲学" | "philosophy" => "思辨".into(),
            _ => format!("{}关注", domain),
        }
    }

    /// D3.8.3: 从活动日志统计生成用户画像（底层智能联动入口）
    ///
    /// 当前实现（V4 数据源）：从 intelligence_v4 行为分析数据提取画像。
    /// 未来扩展（V5）：底层智能完善后，直接调用本地模型生成更精准画像。
    ///
    /// 这是预留的联动接口，实际调用方（IPC 命令）负责传入统计数据。
    pub fn build_profile_from_stats(
        tech_activity_ratio: f64,   // 技术类活动占比 0.0~1.0
        avg_formality_score: f64,   // 用户消息平均正式度 0.0~1.0
        avg_verbosity_score: f64,   // 用户消息平均长度倾向 0.0~1.0
        humor_signals_ratio: f64,   // 幽默信号占比 0.0~1.0
        emotion_empathy_ratio: f64,// 情感类交互占比 0.0~1.0
        interest_domains: Vec<String>,
        total_interactions: i64,    // 总交互数（决定 confidence）
    ) -> UserProfile {
        // confidence 基于交互数量：50 次交互达 0.5，200 次达 1.0
        let confidence = (total_interactions as f64 / 200.0).clamp(0.0, 1.0);

        UserProfile {
            formality: avg_formality_score.clamp(0.0, 1.0),
            verbosity: avg_verbosity_score.clamp(0.0, 1.0),
            humor: humor_signals_ratio.clamp(0.0, 1.0),
            technical_depth: tech_activity_ratio.clamp(0.0, 1.0),
            empathy: emotion_empathy_ratio.clamp(0.0, 1.0),
            interest_domains,
            confidence,
        }
    }
}

impl XinPersonalityService {
    // ============ 人格记忆 CRUD ============

    /// 获取人格记忆（不存在则返回默认空记忆，不报错）
    ///
    /// 设计：人格首次使用时无记忆记录，返回空白 PersonaMemory，
    /// 让上层逻辑无感知地处理"新人格"场景。
    pub async fn get_persona_memory(
        pool: &SqlitePool,
        user_id: i64,
        persona_id: &str,
    ) -> Result<PersonaMemory, AppError> {
        let row = sqlx::query_as::<
            _,
            (String, String, String, String, i64, Option<String>, String, String),
        >(
            "SELECT persona_id, interaction_summary, user_preferences, topic_tags, \
             interaction_count, last_interaction_at, created_at, updated_at \
             FROM xin_persona_memories WHERE user_id = ? AND persona_id = ?",
        )
        .bind(user_id)
        .bind(persona_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row
            .map(
                |(pid, summary, prefs, tags, count, last_at, created, updated)| PersonaMemory {
                    persona_id: pid,
                    interaction_summary: summary,
                    user_preferences: serde_json::from_str(&prefs).unwrap_or(serde_json::json!({})),
                    topic_tags: serde_json::from_str(&tags).unwrap_or(serde_json::json!([])),
                    interaction_count: count,
                    last_interaction_at: last_at,
                    created_at: created,
                    updated_at: updated,
                },
            )
            .unwrap_or_else(|| Self::default_memory(persona_id)))
    }

    /// 构造默认空记忆（新人格首次使用）
    pub fn default_memory(persona_id: &str) -> PersonaMemory {
        let now = Utc::now().to_rfc3339();
        PersonaMemory {
            persona_id: persona_id.into(),
            interaction_summary: String::new(),
            user_preferences: serde_json::json!({}),
            topic_tags: serde_json::json!([]),
            interaction_count: 0,
            last_interaction_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// 更新人格记忆（UPSERT 语义）
    ///
    /// 用于 D3.8.2 在对话结束后持久化该人格的交互摘要与用户偏好。
    pub async fn upsert_persona_memory(
        pool: &SqlitePool,
        user_id: i64,
        memory: &PersonaMemory,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let prefs = serde_json::to_string(&memory.user_preferences).unwrap_or_else(|_| "{}".into());
        let tags = serde_json::to_string(&memory.topic_tags).unwrap_or_else(|_| "[]".into());
        // 多用户隔离：先 UPDATE，未命中再 INSERT（避免跨用户 ON CONFLICT 误命中）
        let affected = sqlx::query(
            "UPDATE xin_persona_memories \
             SET interaction_summary = ?, user_preferences = ?, topic_tags = ?, \
                 interaction_count = ?, last_interaction_at = ?, updated_at = ? \
             WHERE user_id = ? AND persona_id = ?",
        )
        .bind(&memory.interaction_summary)
        .bind(&prefs)
        .bind(&tags)
        .bind(memory.interaction_count)
        .bind(&memory.last_interaction_at)
        .bind(&now)
        .bind(user_id)
        .bind(&memory.persona_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?
        .rows_affected();

        if affected == 0 {
            sqlx::query(
                "INSERT INTO xin_persona_memories \
                 (user_id, persona_id, interaction_summary, user_preferences, topic_tags, \
                  interaction_count, last_interaction_at, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(user_id)
            .bind(&memory.persona_id)
            .bind(&memory.interaction_summary)
            .bind(&prefs)
            .bind(&tags)
            .bind(memory.interaction_count)
            .bind(&memory.last_interaction_at)
            .bind(&memory.created_at)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        }
        Ok(())
    }

    /// 记录一次交互（interaction_count +1，更新 last_interaction_at）
    ///
    /// 轻量调用：对话发送时触发，不阻塞主流程。
    /// 不存在记录时自动初始化。
    pub async fn record_interaction(
        pool: &SqlitePool,
        user_id: i64,
        persona_id: &str,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "UPDATE xin_persona_memories \
             SET interaction_count = interaction_count + 1, \
                 last_interaction_at = ?, updated_at = ? \
             WHERE user_id = ? AND persona_id = ?",
        )
        .bind(&now)
        .bind(&now)
        .bind(user_id)
        .bind(persona_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        if result.rows_affected() == 0 {
            // 人格首次交互，初始化记忆记录
            sqlx::query(
                "INSERT INTO xin_persona_memories \
                 (user_id, persona_id, interaction_summary, user_preferences, topic_tags, \
                  interaction_count, last_interaction_at, created_at, updated_at) \
                 VALUES (?, ?, '', '{}', '[]', 1, ?, ?, ?)",
            )
            .bind(user_id)
            .bind(persona_id)
            .bind(&now)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        }
        Ok(())
    }

    // ============ 人格切换历史 ============

    /// 记录一次人格切换
    ///
    /// `trigger`：manual（用户手动切换）/ auto_grow（自生长人格触发）/ system（初始化）
    /// `from_id` 为 None 表示首次激活（无前序人格）。
    pub async fn log_switch(
        pool: &SqlitePool,
        user_id: i64,
        from_id: Option<&str>,
        to_id: &str,
        trigger: Option<&str>,
    ) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let trigger = trigger.unwrap_or(DEFAULT_SWITCH_TRIGGER);
        sqlx::query(
            "INSERT INTO xin_persona_switch_log \
             (user_id, from_persona_id, to_persona_id, switched_at, trigger) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(user_id)
        .bind(from_id)
        .bind(to_id)
        .bind(&now)
        .bind(trigger)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    /// 获取人格切换历史（倒序，最近优先）
    ///
    /// 用于前端人格切换时间线展示 + 自生长学习数据。
    pub async fn get_switch_history(
        pool: &SqlitePool,
        user_id: i64,
        limit: i64,
    ) -> Result<Vec<PersonaSwitchRecord>, AppError> {
        let rows = sqlx::query_as::<_, (i64, Option<String>, String, String, String)>(
            "SELECT id, from_persona_id, to_persona_id, switched_at, trigger \
             FROM xin_persona_switch_log WHERE user_id = ? ORDER BY switched_at DESC LIMIT ?",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows
            .into_iter()
            .map(|(id, from, to, at, trigger)| PersonaSwitchRecord {
                id,
                from_persona_id: from,
                to_persona_id: to,
                switched_at: at,
                trigger,
            })
            .collect())
    }

    /// 列出所有有记忆记录的 persona_id（按最后交互时间倒序）
    ///
    /// 用于自生长人格分析：哪些人格被用户实际使用过。
    pub async fn list_memorized_personas(
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT persona_id FROM xin_persona_memories \
             WHERE user_id = ? AND interaction_count > 0 \
             ORDER BY last_interaction_at DESC NULLS LAST",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
        Ok(rows)
    }

    /// 统计某人格的交互次数（快速查询，不加载全部记忆）
    pub async fn get_interaction_count(
        pool: &SqlitePool,
        user_id: i64,
        persona_id: &str,
    ) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COALESCE(interaction_count, 0) FROM xin_persona_memories \
             WHERE user_id = ? AND persona_id = ?",
        )
        .bind(user_id)
        .bind(persona_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?
        .unwrap_or(0);
        Ok(count)
    }

    // ============ 记忆摘要生成（纯函数，供 system prompt 注入）============

    /// 将人格记忆转换为 system prompt 注入文本
    ///
    /// 返回 None 的场景：
    ///   - interaction_count == 0（首次使用该人格，无历史记忆）
    ///   - 摘要为空且无偏好且无话题标签（无有效信息可注入）
    ///
    /// 调用点：xin_dialogue_service.rs::send_message 构建 system prompt 前
    pub fn build_memory_prompt(memory: &PersonaMemory, persona_name: &str) -> Option<String> {
        // 首次使用，无历史记忆
        if memory.interaction_count == 0 {
            return None;
        }

        let mut parts: Vec<String> = Vec::new();
        parts.push(format!(
            "【人格记忆】你在「{}」人格下与用户交互过 {} 次。",
            persona_name, memory.interaction_count
        ));

        // 注入交互摘要
        if !memory.interaction_summary.is_empty() {
            parts.push(format!("交互摘要：{}", memory.interaction_summary));
        }

        // 注入用户偏好
        if let Some(prefs) = memory.user_preferences.as_object() {
            if !prefs.is_empty() {
                let prefs_str: Vec<String> = prefs
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                parts.push(format!("用户偏好：{}", prefs_str.join("、")));
            }
        }

        // 注入话题标签
        if let Some(tags) = memory.topic_tags.as_array() {
            if !tags.is_empty() {
                let tags_str: Vec<String> = tags
                    .iter()
                    .filter_map(|t| t.as_str().map(String::from))
                    .collect();
                if !tags_str.is_empty() {
                    parts.push(format!("关注话题：{}", tags_str.join("、")));
                }
            }
        }

        // 全部为空则不注入（仅有 interaction_count 无实质内容）
        if parts.len() <= 1 {
            return None;
        }

        Some(parts.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_memory_fields() {
        let mem = XinPersonalityService::default_memory("test_persona");
        assert_eq!(mem.persona_id, "test_persona");
        assert!(mem.interaction_summary.is_empty());
        assert_eq!(mem.user_preferences, serde_json::json!({}));
        assert_eq!(mem.topic_tags, serde_json::json!([]));
        assert_eq!(mem.interaction_count, 0);
        assert!(mem.last_interaction_at.is_none());
        assert!(!mem.created_at.is_empty());
        assert!(!mem.updated_at.is_empty());
    }

    #[test]
    fn test_default_memory_unique_timestamps() {
        // 同一 persona_id 两次构造应产生不同时间戳（极小概率相同，验证格式即可）
        let m1 = XinPersonalityService::default_memory("p1");
        let m2 = XinPersonalityService::default_memory("p1");
        assert!(m1.created_at.contains('T')); // RFC3339 格式
        assert!(m2.created_at.contains('T'));
    }

    #[test]
    fn test_persona_memory_serialize() {
        let mem = PersonaMemory {
            persona_id: "code_assistant".into(),
            interaction_summary: "用户偏好简洁的代码示例".into(),
            user_preferences: serde_json::json!({"style": "concise", "lang": "zh"}),
            topic_tags: serde_json::json!(["rust", "tauri"]),
            interaction_count: 42,
            last_interaction_at: Some("2026-07-23T10:00:00+00:00".into()),
            created_at: "2026-07-01T00:00:00+00:00".into(),
            updated_at: "2026-07-23T10:00:00+00:00".into(),
        };
        let json = serde_json::to_string(&mem).unwrap();
        assert!(json.contains("code_assistant"));
        assert!(json.contains("concise"));
        assert!(json.contains("rust"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_persona_switch_record_serialize() {
        let rec = PersonaSwitchRecord {
            id: 1,
            from_persona_id: Some("code_assistant".into()),
            to_persona_id: "caring_friend".into(),
            switched_at: "2026-07-23T10:00:00+00:00".into(),
            trigger: "manual".into(),
        };
        let json = serde_json::to_string(&rec).unwrap();
        assert!(json.contains("manual"));
        assert!(json.contains("caring_friend"));
    }

    #[test]
    fn test_persona_switch_record_first_activation() {
        // 首次激活：from_persona_id 为 None
        let rec = PersonaSwitchRecord {
            id: 1,
            from_persona_id: None,
            to_persona_id: "code_assistant".into(),
            switched_at: "2026-07-23T10:00:00+00:00".into(),
            trigger: "system".into(),
        };
        let json = serde_json::to_string(&rec).unwrap();
        assert!(json.contains("\"from_persona_id\":null"));
        assert!(json.contains("\"trigger\":\"system\""));
    }

    #[test]
    fn test_default_switch_trigger_constant() {
        assert_eq!(DEFAULT_SWITCH_TRIGGER, "manual");
    }

    #[test]
    fn test_persona_memory_user_preferences_deserialize() {
        // 验证 user_preferences JSON 反序列化（模拟数据库读取场景）
        let json_str = r#"{"style":"detailed","tone":"friendly","tech_level":0.8}"#;
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert_eq!(parsed["style"], "detailed");
        assert_eq!(parsed["tone"], "friendly");
        assert!((parsed["tech_level"].as_f64().unwrap() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn test_persona_memory_topic_tags_deserialize() {
        // 验证 topic_tags JSON array 反序列化
        let json_str = r#"["rust","tauri","ai","terminal"]"#;
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 4);
        assert_eq!(parsed[0], "rust");
    }

    #[test]
    fn test_invalid_preferences_json_fallback() {
        // 模拟数据库中 user_preferences 字段损坏时的容错
        let bad_json = "not a valid json";
        let fallback: serde_json::Value =
            serde_json::from_str(bad_json).unwrap_or(serde_json::json!({}));
        assert_eq!(fallback, serde_json::json!({}));
    }

    #[test]
    fn test_build_memory_prompt_first_use_returns_none() {
        // 首次使用（interaction_count == 0），不注入记忆
        let mem = XinPersonalityService::default_memory("new_persona");
        let prompt = XinPersonalityService::build_memory_prompt(&mem, "新人格");
        assert!(prompt.is_none());
    }

    #[test]
    fn test_build_memory_prompt_count_only_returns_none() {
        // 仅有 interaction_count 但无摘要/偏好/话题，不注入（无实质内容）
        let mut mem = XinPersonalityService::default_memory("p1");
        mem.interaction_count = 5;
        let prompt = XinPersonalityService::build_memory_prompt(&mem, "代码助手");
        assert!(prompt.is_none());
    }

    #[test]
    fn test_build_memory_prompt_with_summary() {
        let mut mem = XinPersonalityService::default_memory("code_assistant");
        mem.interaction_count = 10;
        mem.interaction_summary = "用户偏好简洁的代码示例".into();
        let prompt = XinPersonalityService::build_memory_prompt(&mem, "代码助手");
        assert!(prompt.is_some());
        let p = prompt.unwrap();
        assert!(p.contains("代码助手"));
        assert!(p.contains("10 次"));
        assert!(p.contains("用户偏好简洁的代码示例"));
    }

    #[test]
    fn test_build_memory_prompt_with_preferences_and_tags() {
        let mut mem = XinPersonalityService::default_memory("code_assistant");
        mem.interaction_count = 42;
        mem.user_preferences = serde_json::json!({"style": "concise", "lang": "zh"});
        mem.topic_tags = serde_json::json!(["rust", "tauri", "ai"]);
        let prompt = XinPersonalityService::build_memory_prompt(&mem, "代码助手");
        assert!(prompt.is_some());
        let p = prompt.unwrap();
        assert!(p.contains("style=concise"));
        assert!(p.contains("lang=zh"));
        assert!(p.contains("rust"));
        assert!(p.contains("tauri"));
        assert!(p.contains("42 次"));
    }

    // ============ D3.8.3 自生长人格测试 ============

    #[test]
    fn test_user_profile_default_no_confidence() {
        let p = UserProfile::default();
        assert_eq!(p.confidence, 0.0);
        assert!(p.interest_domains.is_empty());
        assert!((p.formality - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_grow_persona_low_confidence_returns_none() {
        // confidence < 0.3，数据不足，不生长（保持白纸）
        let profile = UserProfile {
            confidence: 0.2,
            ..UserProfile::default()
        };
        let grown = XinPersonalityService::grow_persona(&profile);
        assert!(grown.is_none());
    }

    #[test]
    fn test_grow_persona_high_confidence() {
        let profile = UserProfile {
            formality: 0.8,
            verbosity: 0.3,
            humor: 0.7,
            technical_depth: 0.9,
            empathy: 0.6,
            interest_domains: vec!["rust".into(), "数学".into()],
            confidence: 0.9,
        };
        let grown = XinPersonalityService::grow_persona(&profile);
        assert!(grown.is_some());
        let g = grown.unwrap();
        // 高 confidence 时结果接近 profile 值
        assert!(g.formality > 0.7); // blend(0.8) with weight 0.9
        assert!(g.technical_depth > 0.8);
        assert!(g.traits.len() >= 4); // 2 基础 + 2 兴趣领域
        assert!(g.growth_description.contains("90%"));
        assert!(g.growth_description.contains("rust"));
    }

    #[test]
    fn test_grow_persona_medium_confidence_blends_to_neutral() {
        // 中等 confidence 时向中性 0.5 收敛
        let profile = UserProfile {
            formality: 1.0,
            verbosity: 0.0,
            humor: 0.5,
            technical_depth: 0.5,
            empathy: 0.5,
            interest_domains: vec![],
            confidence: 0.5,
        };
        let grown = XinPersonalityService::grow_persona(&profile).unwrap();
        // weight=0.5, blend(1.0) = 0.5*0.5 + 1.0*0.5 = 0.75
        assert!((grown.formality - 0.75).abs() < 1e-9);
        // blend(0.0) = 0.5*0.5 + 0.0*0.5 = 0.25
        assert!((grown.verbosity - 0.25).abs() < 1e-9);
    }

    #[test]
    fn test_domain_to_trait_name_mapping() {
        // domain_to_trait_name 是私有方法，通过 grow_persona 间接验证
        let profile = UserProfile {
            interest_domains: vec!["rust".into(), "音乐".into(), "未知领域".into()],
            confidence: 0.8,
            ..UserProfile::default()
        };
        let grown = XinPersonalityService::grow_persona(&profile).unwrap();
        let trait_names: Vec<&str> = grown.traits.iter().map(|t| t.name.as_str()).collect();
        assert!(trait_names.contains(&"技术钻研"));
        assert!(trait_names.contains(&"艺术感"));
        assert!(trait_names.contains(&"未知领域关注"));
    }

    #[test]
    fn test_build_profile_from_stats_confidence_scaling() {
        // 0 次交互 → confidence 0
        let p = XinPersonalityService::build_profile_from_stats(
            0.5, 0.5, 0.5, 0.5, 0.5, vec![], 0,
        );
        assert_eq!(p.confidence, 0.0);

        // 50 次交互 → confidence 0.25
        let p = XinPersonalityService::build_profile_from_stats(
            0.8, 0.6, 0.4, 0.7, 0.5, vec!["rust".into()], 50,
        );
        assert!((p.confidence - 0.25).abs() < 1e-9);

        // 200+ 次交互 → confidence 1.0
        let p = XinPersonalityService::build_profile_from_stats(
            0.9, 0.7, 0.6, 0.8, 0.4, vec!["rust".into(), "ai".into()], 300,
        );
        assert!((p.confidence - 1.0).abs() < 1e-9);
        assert_eq!(p.interest_domains.len(), 2);
    }

    #[test]
    fn test_grown_persona_serialize() {
        let profile = UserProfile {
            formality: 0.7,
            verbosity: 0.6,
            humor: 0.4,
            technical_depth: 0.8,
            empathy: 0.7,
            interest_domains: vec!["rust".into()],
            confidence: 0.85,
        };
        let grown = XinPersonalityService::grow_persona(&profile).unwrap();
        let json = serde_json::to_string(&grown).unwrap();
        assert!(json.contains("growth_description"));
        assert!(json.contains("技术钻研"));
    }
}
