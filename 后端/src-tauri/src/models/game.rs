//! 游戏 3D 重构 - 数据模型
//!
//! 对应 `07_数据库设计.md` v2.0 的 10 张表 + 配套枚举。
//! change-id: `game-3d-rebuild-refactor`
//!
//! 设计原则：
//! - 结构体字段与 DB 列一一对应（命名保持 snake_case）
//! - 枚举同时实现 `as_str()` / `from_str()` 便于 DB ↔ 内存互转
//! - 9 个枚举：`RealmMajor` / `RealmMinor` / `DaoFoundation` / `BuildingCategory` /
//!   `BuildingStatus` / `BuildHistoryEventType` / `BreakthroughResult` /
//!   `PointsLogResult` / `MappingSource`
//! - 3 个核心方法：`RealmMajor::from_xp` / `dao_foundation_multiplier` /
//!   `building_category_for_domain`

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// 一、枚举定义
// ============================================================================

/// 大境界（10 阶），按 XP 范围递增。对应 `game_worlds.realm_major` 列。
///
/// XP 范围（02 文档 §2.1）：
/// - Mortal 0~99 / QiRefining 100~499 / FoundationBuilding 500~1999
/// - GoldenCore 2000~7999 / NascentSoul 8000~29999 / SpiritTransformation 30000~99999
/// - Unity 100000~299999 / Mahayana 300000~799999 / Tribulation 800000~1999999
/// - Immortal 2000000+
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RealmMajor {
    #[serde(rename = "mortal")]
    Mortal,
    #[serde(rename = "qi_refining")]
    QiRefining,
    #[serde(rename = "foundation_building")]
    FoundationBuilding,
    #[serde(rename = "golden_core")]
    GoldenCore,
    #[serde(rename = "nascent_soul")]
    NascentSoul,
    #[serde(rename = "spirit_transformation")]
    SpiritTransformation,
    #[serde(rename = "unity")]
    Unity,
    #[serde(rename = "mahayana")]
    Mahayana,
    #[serde(rename = "tribulation")]
    Tribulation,
    #[serde(rename = "immortal")]
    Immortal,
}

impl RealmMajor {
    /// 根据总修为（`total_xp`）判定对应大境界。
    pub fn from_xp(total_xp: i64) -> Self {
        if total_xp < 100 {
            RealmMajor::Mortal
        } else if total_xp < 500 {
            RealmMajor::QiRefining
        } else if total_xp < 2000 {
            RealmMajor::FoundationBuilding
        } else if total_xp < 8000 {
            RealmMajor::GoldenCore
        } else if total_xp < 30000 {
            RealmMajor::NascentSoul
        } else if total_xp < 100000 {
            RealmMajor::SpiritTransformation
        } else if total_xp < 300000 {
            RealmMajor::Unity
        } else if total_xp < 800000 {
            RealmMajor::Mahayana
        } else if total_xp < 2_000_000 {
            RealmMajor::Tribulation
        } else {
            RealmMajor::Immortal
        }
    }

    /// 转字符串（与 DB 存储一致）。
    pub fn as_str(&self) -> &'static str {
        match self {
            RealmMajor::Mortal => "mortal",
            RealmMajor::QiRefining => "qi_refining",
            RealmMajor::FoundationBuilding => "foundation_building",
            RealmMajor::GoldenCore => "golden_core",
            RealmMajor::NascentSoul => "nascent_soul",
            RealmMajor::SpiritTransformation => "spirit_transformation",
            RealmMajor::Unity => "unity",
            RealmMajor::Mahayana => "mahayana",
            RealmMajor::Tribulation => "tribulation",
            RealmMajor::Immortal => "immortal",
        }
    }

    /// 从字符串解析。未知值返回 `None`。
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "mortal" => Some(RealmMajor::Mortal),
            "qi_refining" => Some(RealmMajor::QiRefining),
            "foundation_building" => Some(RealmMajor::FoundationBuilding),
            "golden_core" => Some(RealmMajor::GoldenCore),
            "nascent_soul" => Some(RealmMajor::NascentSoul),
            "spirit_transformation" => Some(RealmMajor::SpiritTransformation),
            "unity" => Some(RealmMajor::Unity),
            "mahayana" => Some(RealmMajor::Mahayana),
            "tribulation" => Some(RealmMajor::Tribulation),
            "immortal" => Some(RealmMajor::Immortal),
            _ => None,
        }
    }

    /// 大境界序号（1-10）。
    pub fn ordinal(&self) -> u8 {
        match self {
            RealmMajor::Mortal => 1,
            RealmMajor::QiRefining => 2,
            RealmMajor::FoundationBuilding => 3,
            RealmMajor::GoldenCore => 4,
            RealmMajor::NascentSoul => 5,
            RealmMajor::SpiritTransformation => 6,
            RealmMajor::Unity => 7,
            RealmMajor::Mahayana => 8,
            RealmMajor::Tribulation => 9,
            RealmMajor::Immortal => 10,
        }
    }

    /// 当前大境界的 XP 下限（含）。
    pub fn xp_lower_bound(&self) -> i64 {
        match self {
            RealmMajor::Mortal => 0,
            RealmMajor::QiRefining => 100,
            RealmMajor::FoundationBuilding => 500,
            RealmMajor::GoldenCore => 2_000,
            RealmMajor::NascentSoul => 8_000,
            RealmMajor::SpiritTransformation => 30_000,
            RealmMajor::Unity => 100_000,
            RealmMajor::Mahayana => 300_000,
            RealmMajor::Tribulation => 800_000,
            RealmMajor::Immortal => 2_000_000,
        }
    }

    /// 当前大境界的 XP 上限（不含）。仙境界返回 `i64::MAX`。
    pub fn xp_upper_bound(&self) -> i64 {
        match self {
            RealmMajor::Mortal => 100,
            RealmMajor::QiRefining => 500,
            RealmMajor::FoundationBuilding => 2_000,
            RealmMajor::GoldenCore => 8_000,
            RealmMajor::NascentSoul => 30_000,
            RealmMajor::SpiritTransformation => 100_000,
            RealmMajor::Unity => 300_000,
            RealmMajor::Mahayana => 800_000,
            RealmMajor::Tribulation => 2_000_000,
            RealmMajor::Immortal => i64::MAX,
        }
    }

    /// 下一阶大境界。仙境界返回 `None`。
    pub fn next(&self) -> Option<RealmMajor> {
        match self {
            RealmMajor::Mortal => Some(RealmMajor::QiRefining),
            RealmMajor::QiRefining => Some(RealmMajor::FoundationBuilding),
            RealmMajor::FoundationBuilding => Some(RealmMajor::GoldenCore),
            RealmMajor::GoldenCore => Some(RealmMajor::NascentSoul),
            RealmMajor::NascentSoul => Some(RealmMajor::SpiritTransformation),
            RealmMajor::SpiritTransformation => Some(RealmMajor::Unity),
            RealmMajor::Unity => Some(RealmMajor::Mahayana),
            RealmMajor::Mahayana => Some(RealmMajor::Tribulation),
            RealmMajor::Tribulation => Some(RealmMajor::Immortal),
            RealmMajor::Immortal => None,
        }
    }

    /// 突破考验题目数量（02 文档 §2.1）。
    pub fn breakthrough_question_count(&self) -> u8 {
        match self {
            RealmMajor::Mortal => 0,
            RealmMajor::QiRefining => 3,
            RealmMajor::FoundationBuilding => 4,
            RealmMajor::GoldenCore => 5,
            RealmMajor::NascentSoul => 6,
            RealmMajor::SpiritTransformation => 8,
            RealmMajor::Unity => 10,
            RealmMajor::Mahayana => 12,
            RealmMajor::Tribulation => 15,
            RealmMajor::Immortal => 0,
        }
    }
}

/// 小境界阶位（3 级）。对应 `game_worlds.realm_minor` 列。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RealmMinor {
    #[serde(rename = "early")]
    Early,
    #[serde(rename = "middle")]
    Middle,
    #[serde(rename = "complete")]
    Complete,
}

impl RealmMinor {
    pub fn as_str(&self) -> &'static str {
        match self {
            RealmMinor::Early => "early",
            RealmMinor::Middle => "middle",
            RealmMinor::Complete => "complete",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "early" => Some(RealmMinor::Early),
            "middle" => Some(RealmMinor::Middle),
            "complete" => Some(RealmMinor::Complete),
            _ => None,
        }
    }

    /// 下一阶小境界。`complete` 返回 `None`（应突破大境界）。
    pub fn next(&self) -> Option<RealmMinor> {
        match self {
            RealmMinor::Early => Some(RealmMinor::Middle),
            RealmMinor::Middle => Some(RealmMinor::Complete),
            RealmMinor::Complete => None,
        }
    }
}

/// 道基品质（5 级）。对应 `game_worlds.dao_foundation` 列。
///
/// 系数（02 文档 §2.2）：白 0.5 / 蓝 0.7 / 红 1.0 / 紫 1.3 / 黑 1.6。
/// 修为实际增长 = 知识积分 × 道基系数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaoFoundation {
    #[serde(rename = "white")]
    White,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "black")]
    Black,
}

impl DaoFoundation {
    pub fn as_str(&self) -> &'static str {
        match self {
            DaoFoundation::White => "white",
            DaoFoundation::Blue => "blue",
            DaoFoundation::Red => "red",
            DaoFoundation::Purple => "purple",
            DaoFoundation::Black => "black",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "white" => Some(DaoFoundation::White),
            "blue" => Some(DaoFoundation::Blue),
            "red" => Some(DaoFoundation::Red),
            "purple" => Some(DaoFoundation::Purple),
            "black" => Some(DaoFoundation::Black),
            _ => None,
        }
    }

    /// 道基序号（1-5）。
    pub fn ordinal(&self) -> u8 {
        match self {
            DaoFoundation::White => 1,
            DaoFoundation::Blue => 2,
            DaoFoundation::Red => 3,
            DaoFoundation::Purple => 4,
            DaoFoundation::Black => 5,
        }
    }
}

/// 建筑大类（8 类，对应文明演进）。对应 `game_buildings.building_category` 列。
///
/// 04 文档：房屋 / 城镇 / 城池 / 王国 / 宫殿 / 科技 / 宗门 / 修仙。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingCategory {
    #[serde(rename = "house")]
    House,
    #[serde(rename = "town")]
    Town,
    #[serde(rename = "city")]
    City,
    #[serde(rename = "kingdom")]
    Kingdom,
    #[serde(rename = "palace")]
    Palace,
    #[serde(rename = "technology")]
    Technology,
    #[serde(rename = "sect")]
    Sect,
    #[serde(rename = "immortal")]
    Immortal,
}

impl BuildingCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            BuildingCategory::House => "house",
            BuildingCategory::Town => "town",
            BuildingCategory::City => "city",
            BuildingCategory::Kingdom => "kingdom",
            BuildingCategory::Palace => "palace",
            BuildingCategory::Technology => "technology",
            BuildingCategory::Sect => "sect",
            BuildingCategory::Immortal => "immortal",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "house" => Some(BuildingCategory::House),
            "town" => Some(BuildingCategory::Town),
            "city" => Some(BuildingCategory::City),
            "kingdom" => Some(BuildingCategory::Kingdom),
            "palace" => Some(BuildingCategory::Palace),
            "technology" => Some(BuildingCategory::Technology),
            "sect" => Some(BuildingCategory::Sect),
            "immortal" => Some(BuildingCategory::Immortal),
            _ => None,
        }
    }

    /// 解锁该建筑大类所需的最低文明等级（04 文档表）。
    pub fn min_civilization_level(&self) -> u32 {
        match self {
            BuildingCategory::House => 1,
            BuildingCategory::Town => 2,
            BuildingCategory::City => 3,
            BuildingCategory::Kingdom => 4,
            BuildingCategory::Palace => 5,
            BuildingCategory::Technology => 6,
            BuildingCategory::Sect => 7,
            BuildingCategory::Immortal => 8,
        }
    }
}

/// 建造状态。对应 `game_buildings.status` 列。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingStatus {
    #[serde(rename = "planning")]
    Planning,
    #[serde(rename = "building")]
    Building,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "ruined")]
    Ruined,
}

impl BuildingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BuildingStatus::Planning => "planning",
            BuildingStatus::Building => "building",
            BuildingStatus::Completed => "completed",
            BuildingStatus::Ruined => "ruined",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "planning" => Some(BuildingStatus::Planning),
            "building" => Some(BuildingStatus::Building),
            "completed" => Some(BuildingStatus::Completed),
            "ruined" => Some(BuildingStatus::Ruined),
            _ => None,
        }
    }
}

/// 建造历史事件类型。对应 `game_build_history.event_type` 列。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildHistoryEventType {
    #[serde(rename = "build")]
    Build,
    #[serde(rename = "upgrade")]
    Upgrade,
    #[serde(rename = "demolish")]
    Demolish,
    #[serde(rename = "move")]
    Move,
    #[serde(rename = "complete")]
    Complete,
}

impl BuildHistoryEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BuildHistoryEventType::Build => "build",
            BuildHistoryEventType::Upgrade => "upgrade",
            BuildHistoryEventType::Demolish => "demolish",
            BuildHistoryEventType::Move => "move",
            BuildHistoryEventType::Complete => "complete",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "build" => Some(BuildHistoryEventType::Build),
            "upgrade" => Some(BuildHistoryEventType::Upgrade),
            "demolish" => Some(BuildHistoryEventType::Demolish),
            "move" => Some(BuildHistoryEventType::Move),
            "complete" => Some(BuildHistoryEventType::Complete),
            _ => None,
        }
    }
}

/// 境界突破结果。对应 `game_breakthrough_records.result` 列。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakthroughResult {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "dropped")]
    Dropped,
}

impl BreakthroughResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            BreakthroughResult::Success => "success",
            BreakthroughResult::Failed => "failed",
            BreakthroughResult::Dropped => "dropped",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "success" => Some(BreakthroughResult::Success),
            "failed" => Some(BreakthroughResult::Failed),
            "dropped" => Some(BreakthroughResult::Dropped),
            _ => None,
        }
    }
}

/// 积分入账结果。对应 `game_points_log.result` 列。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointsLogResult {
    #[serde(rename = "accepted")]
    Accepted,
    #[serde(rename = "rejected_daily_limit")]
    RejectedDailyLimit,
    #[serde(rename = "rejected_negative")]
    RejectedNegative,
}

impl PointsLogResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            PointsLogResult::Accepted => "accepted",
            PointsLogResult::RejectedDailyLimit => "rejected_daily_limit",
            PointsLogResult::RejectedNegative => "rejected_negative",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "accepted" => Some(PointsLogResult::Accepted),
            "rejected_daily_limit" => Some(PointsLogResult::RejectedDailyLimit),
            "rejected_negative" => Some(PointsLogResult::RejectedNegative),
            _ => None,
        }
    }
}

/// KB 分类→游戏领域映射来源。对应 `game_kb_category_mapping.mapping_source` 列。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MappingSource {
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "ai_suggest")]
    AiSuggest,
    #[serde(rename = "ai_auto")]
    AiAuto,
}

impl MappingSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            MappingSource::Manual => "manual",
            MappingSource::AiSuggest => "ai_suggest",
            MappingSource::AiAuto => "ai_auto",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "manual" => Some(MappingSource::Manual),
            "ai_suggest" => Some(MappingSource::AiSuggest),
            "ai_auto" => Some(MappingSource::AiAuto),
            _ => None,
        }
    }
}

// ============================================================================
// 二、常量与函数
// ============================================================================

/// 12 个知识领域 id（03 文档 §1）。
pub mod domain {
    pub const CS: &str = "cs";
    pub const MATH: &str = "math";
    pub const PHYSICS: &str = "physics";
    pub const LITERATURE: &str = "literature";
    pub const HISTORY: &str = "history";
    pub const ART: &str = "art";
    pub const ENGINEERING: &str = "engineering";
    pub const MEDICINE: &str = "medicine";
    pub const PHILOSOPHY: &str = "philosophy";
    pub const ECONOMICS: &str = "economics";
    pub const LANGUAGE: &str = "language";
    pub const OTHER: &str = "other";

    /// 全部 12 个领域 id（顺序与 `game_knowledge_domains` 种子数据一致）。
    pub const ALL: [&str; 12] = [
        CS, MATH, PHYSICS, LITERATURE, HISTORY, ART, ENGINEERING, MEDICINE, PHILOSOPHY,
        ECONOMICS, LANGUAGE, OTHER,
    ];
}

/// 11 种积分来源类型（03 文档 §2）。
pub mod source_type {
    pub const KB_ENTRY_CREATE: &str = "kb_entry_create";
    pub const KB_CATEGORY_CREATE: &str = "kb_category_create";
    pub const KB_ENTRY_UPDATE: &str = "kb_entry_update";
    pub const TODO_COMPLETE: &str = "todo_complete";
    pub const TIMER_SHORT_COMPLETE: &str = "timer_short_complete";
    pub const TIMER_LONG_COMPLETE: &str = "timer_long_complete";
    pub const AI_CHAT_TURN: &str = "ai_chat_turn";
    pub const YUANCODE_USE: &str = "yuancode_use";
    pub const JOURNAL_RECORD: &str = "journal_record";
    pub const BUILDING_REFUND: &str = "building_refund";
    pub const MANUAL_ADJUST: &str = "manual_adjust";

    /// 全部 11 个来源类型。
    pub const ALL: [&str; 11] = [
        KB_ENTRY_CREATE,
        KB_CATEGORY_CREATE,
        KB_ENTRY_UPDATE,
        TODO_COMPLETE,
        TIMER_SHORT_COMPLETE,
        TIMER_LONG_COMPLETE,
        AI_CHAT_TURN,
        YUANCODE_USE,
        JOURNAL_RECORD,
        BUILDING_REFUND,
        MANUAL_ADJUST,
    ];
}

/// 知识领域等级阶梯阈值（03 文档 §3）。
///
/// 8 级：1 入门 / 2 初窥 / 3 进阶 / 4 精通 / 5 大师 / 6 宗师 / 7 化境 / 8 问道。
/// 累计积分阈值：0 / 100 / 500 / 2000 / 10000 / 50000 / 200000 / 1000000。
pub const DOMAIN_LEVEL_THRESHOLDS: [i64; 8] = [
    0,      // Lv1 入门
    100,    // Lv2 初窥
    500,    // Lv3 进阶
    2_000,  // Lv4 精通
    10_000, // Lv5 大师
    50_000, // Lv6 宗师
    200_000, // Lv7 化境
    1_000_000, // Lv8 问道
];

/// 根据累计积分计算领域等级（1-8）。
pub fn domain_level_from_earned(total_earned: i64) -> u8 {
    let mut level: u8 = 1;
    for (i, threshold) in DOMAIN_LEVEL_THRESHOLDS.iter().enumerate() {
        if total_earned >= *threshold {
            level = (i + 1) as u8;
        } else {
            break;
        }
    }
    level
}

/// 道基系数（02 文档 §2.2）：白 0.5 / 蓝 0.7 / 红 1.0 / 紫 1.3 / 黑 1.6。
pub fn dao_foundation_multiplier(dao: DaoFoundation) -> f64 {
    match dao {
        DaoFoundation::White => 0.5,
        DaoFoundation::Blue => 0.7,
        DaoFoundation::Red => 1.0,
        DaoFoundation::Purple => 1.3,
        DaoFoundation::Black => 1.6,
    }
}

/// 根据积分得分判定道基品质。
///
/// 规则（02 文档 §2.2，按累计积分阶梯）：
/// - < 1000 → White
/// - 1000~4999 → Blue
/// - 5000~19999 → Red
/// - 20000~99999 → Purple
/// - >= 100000 → Black
pub fn dao_foundation_from_score(score: i64) -> DaoFoundation {
    if score < 1_000 {
        DaoFoundation::White
    } else if score < 5_000 {
        DaoFoundation::Blue
    } else if score < 20_000 {
        DaoFoundation::Red
    } else if score < 100_000 {
        DaoFoundation::Purple
    } else {
        DaoFoundation::Black
    }
}

/// 根据知识领域 id 返回对应建筑大类（04 文档表）。
///
/// 映射关系：
/// - cs / physics / math → Technology
/// - engineering / history → City
/// - economics / language → Town
/// - economics / history / philosophy → Kingdom
/// - art / literature / philosophy → Palace
/// - philosophy / medicine → Sect
/// - medicine / philosophy → Immortal
/// - other → House（默认）
///
/// 注：04 文档中部分领域可对应多个建筑大类，此处取**最高阶**（文明等级要求最高）的大类作为「领域→建筑」的默认推荐。
pub fn building_category_for_domain(domain_id: &str) -> BuildingCategory {
    match domain_id {
        // 科技类（最高阶）
        "cs" | "physics" | "math" => BuildingCategory::Technology,
        // 修仙类（最高阶，全领域）
        "medicine" | "philosophy" => BuildingCategory::Immortal,
        // 宫殿类
        "art" | "literature" => BuildingCategory::Palace,
        // 城池类
        "engineering" | "history" => BuildingCategory::City,
        // 城镇类
        "economics" | "language" => BuildingCategory::Town,
        // 默认（other）
        _ => BuildingCategory::House,
    }
}

/// 计算建造某子类建筑到指定等级所需的总积分。
///
/// 数据源：04 文档 §3 子类清单的「所需领域积分」列（level 1 基准值，按子类给出）。
/// 等级系数（04 文档 §5.3）：`1 + (level - 1) × 0.5`
///
/// 注：
/// - 04 文档 §5.3 公式（大类基础积分 × 子类时间系数 × 等级系数）与 §3 表数据存在不一致，
///   本实现**以 §3 表「所需领域积分」列为权威**，§5.3 公式仅作参考。
/// - 未知子类按 category 兜底（避免除零），并打 warn 日志便于发现脏数据。
/// - 多领域加权公式（03 文档 §6.1）当前 schema 不支持，待未来扩展。
pub fn building_required_points(category: &str, subtype: &str, level: u32) -> i64 {
    let base_lv1: i64 = match subtype {
        // house 房屋（§3.1）
        "thatch_cottage" => 100,
        "wooden_house" => 250,
        "brick_house" => 500,
        "courtyard" => 1000,
        // town 城镇（§3.2）
        "market" => 800,
        "tavern" => 1200,
        "school" => 2000,
        "temple" => 3000,
        "watchtower" => 1500,
        // city 城池（§3.3）
        "city_wall" => 5000,
        "government" => 8000,
        "bank" => 10000,
        "warehouse" => 4000,
        "academy" => 15000,
        // kingdom 王国（§3.4）
        "throne_hall" => 25000,
        "court" => 20000,
        "embassy" => 18000,
        "barracks" => 22000,
        // palace 宫殿（§3.5）
        "grand_hall" => 50000,
        "inner_palace" => 40000,
        "castle" => 60000,
        "treasure_pavilion" => 45000,
        "spirit_garden" => 35000,
        // technology 科技（§3.6）
        "lab" => 70000,
        "factory" => 80000,
        "reactor" => 120000,
        "observatory" => 90000,
        "data_center" => 100000,
        // sect 宗门（§3.7）
        "main_hall" => 150000,
        "cultivation_cave" => 130000,
        "scripture_pavilion" => 160000,
        "alchemy_room" => 140000,
        // immortal 修仙（§3.8）
        "flying_palace" => 500000,
        "spirit_tower" => 400000,
        "ascension_altar" => 600000,
        "immortal_pavilion" => 350000,
        // 兜底：未知子类按 category 给保守默认值
        _ => {
            tracing::warn!(
                category = category,
                subtype = subtype,
                "未知建筑子类，使用 category 兜底默认积分"
            );
            match category {
                "house" => 200,
                "town" => 800,
                "city" => 5000,
                "kingdom" => 20000,
                "palace" => 40000,
                "technology" => 70000,
                "sect" => 130000,
                "immortal" => 350000,
                _ => 200,
            }
        }
    };
    let level_factor = 1.0 + (level.saturating_sub(1) as f64) * 0.5;
    ((base_lv1 as f64) * level_factor) as i64
}

// ============================================================================
// 三、默认值常量
// ============================================================================

pub const DEFAULT_PLAYER_NAME: &str = "无名修士";
pub const DEFAULT_CIVILIZATION_LEVEL: u32 = 1;
pub const DEFAULT_MAP_WIDTH: u32 = 32;
pub const DEFAULT_MAP_HEIGHT: u32 = 32;
pub const DEFAULT_REALM_MAJOR: RealmMajor = RealmMajor::Mortal;
pub const DEFAULT_REALM_MINOR: RealmMinor = RealmMinor::Early;
pub const DEFAULT_DAO_FOUNDATION: DaoFoundation = DaoFoundation::White;
pub const DEFAULT_BUILDING_STATUS: BuildingStatus = BuildingStatus::Completed;
pub const DEFAULT_BUILD_PROGRESS: f64 = 0.0;
pub const DEFAULT_BUILDING_LEVEL: u32 = 1;
pub const DEFAULT_POINTS: i64 = 0;

// ============================================================================
// 四、结构体定义（10 张表对应）
// ============================================================================

/// 对应 `game_worlds` 表。世界/文明主表。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameWorld {
    pub id: String,
    pub user_id: i64,
    pub player_name: String,
    pub civilization_level: u32,
    pub realm_major: String,
    pub realm_minor: String,
    pub dao_foundation: String,
    pub total_xp: i64,
    pub realm_xp: i64,
    pub map_width: u32,
    pub map_height: u32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 对应 `game_buildings` 表。建筑实例。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameBuilding {
    pub id: String,
    pub world_id: String,
    pub building_category: String,
    pub building_subtype: String,
    pub name: String,
    pub level: u32,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub rotation_y: f64,
    pub status: String,
    pub build_progress: f64,
    pub knowledge_domain: String,
    pub built_at: Option<i64>,
    pub completed_at: Option<i64>,
}

/// 对应 `game_knowledge_domains` 表。12 个知识领域静态字典。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameKnowledgeDomain {
    pub id: String,
    pub name: String,
    pub name_en: String,
    pub description: String,
    pub building_category: String,
    pub sort_order: i32,
}

/// 对应 `game_knowledge_progress` 表。世界在各领域的积分进度。
///
/// 约束：`points = total_earned - total_consumed` 且 `points >= 0`（应用层保证）。
/// `UNIQUE(world_id, domain_id)`：每世界每领域仅一条记录。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameKnowledgeProgress {
    pub id: String,
    pub world_id: String,
    pub domain_id: String,
    pub points: i64,
    /// 领域等级 1~8（由 `total_earned` 按阶梯阈值计算，见 `domain_level_from_earned`）。
    pub level: i32,
    pub total_earned: i64,
    pub total_consumed: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 对应 `game_breakthrough_records` 表。境界突破流水（AI 考验全流程快照）。
///
/// `from_realm` / `to_realm` 格式为 `"{realm_major}:{realm_minor}"`，如 `qi_refining:complete`。
/// 道基评定：90+ black / 75+ purple / 60+ red / 45+ blue / <45 失败。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameBreakthroughRecord {
    pub id: String,
    pub world_id: String,
    /// 突破前境界，格式 `{realm_major}:{realm_minor}`。
    pub from_realm: String,
    /// 突破后境界，格式同上；失败时为 `None`（停留原境界）。
    pub to_realm: Option<String>,
    /// AI 考验得分 0~100；未答题时为 `None`。
    pub score: Option<i32>,
    /// 评定道基：`white`/`blue`/`red`/`purple`/`black`；失败时为 `None`。
    pub dao_foundation_awarded: Option<String>,
    /// 突破结果：`success` / `failed` / `dropped`。
    pub result: String,
    /// 考题快照（JSON 字符串）。
    pub questions_json: Option<String>,
    /// 用户答案快照（JSON 字符串）。
    pub answers_json: Option<String>,
    /// AI 批改评语。
    pub ai_review: Option<String>,
    /// 暴露的弱点列表（JSON 字符串），用于下次突破强化出题。
    pub weakness_json: Option<String>,
    /// 发生时间戳（ms）。
    pub created_at: i64,
}

/// 对应 `game_build_history` 表。建造/升级/拆除/移动/完成事件流水。
///
/// 单坐标快照设计：`pos_x/y/z` + `level` + `progress` 记录事件发生时的建筑状态。
/// `snapshot_json` 存该时间点世界状态快照（JSON），用于时间轴回放重建场景。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameBuildHistory {
    pub id: String,
    pub world_id: String,
    pub event_type: String,
    /// 关联建筑 ID（逻辑引用，建筑删除后仍保留历史）。
    pub building_id: String,
    /// 建筑名称快照（建筑被删除后仍保留用于展示）。
    pub building_name: String,
    /// 事件发生时建筑 3D 坐标快照。
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,
    pub pos_z: Option<f64>,
    /// 事件发生时建筑等级快照。
    pub level: Option<i32>,
    /// 事件发生时建造进度快照。
    pub progress: Option<f64>,
    /// 该时间点世界状态快照（JSON 字符串，用于时间轴回放重建场景）。
    pub snapshot_json: Option<String>,
    /// 事件时间戳（ms）。
    pub created_at: i64,
}

/// 对应 `game_kb_category_mapping` 表。KB 分类 → 游戏领域映射（一对一）。
///
/// `category_id` 为主键，天然保证一对一。未映射的 KB 分类条目默认归入 `other` 领域（应用层兜底）。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameKbCategoryMapping {
    /// KB 分类 ID，主键，引用 `kb_categories.id`（逻辑引用，不加物理外键）。
    pub category_id: i64,
    pub domain_id: String,
    pub mapped_at: i64,
    /// 映射来源：`manual` / `ai_suggest` / `ai_auto`。
    pub mapped_by: String,
    /// AI 映配置信度 0.0~1.0；`manual` 时为 `None`。
    pub confidence: Option<f64>,
}

/// 对应 `game_points_log` 表。积分变更日志（幂等 event_id）。
///
/// `event_id` 设 `UNIQUE` 保证同一业务事件不会重复入账。
/// `points_after` 为变动后该领域可用积分快照。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GamePointsLog {
    pub id: String,
    pub world_id: String,
    pub source_type: String,
    pub domain_id: String,
    /// 积分变动值，正数入账，负数扣除。
    pub points_delta: i64,
    /// 变动后该领域可用积分快照。
    pub points_after: i64,
    /// 处理结果：`accepted` / `rejected_daily_limit` / `rejected_negative`。
    pub result: String,
    /// 业务事件唯一 ID，`UNIQUE` 约束保证幂等。
    pub event_id: String,
    /// 附加元数据（JSON 字符串，如条目 ID、会话 ID 等）。
    pub metadata_json: Option<String>,
    pub created_at: i64,
}

/// 对应 `game_daily_limit_counter` 表。每日上限计数器（按领域+来源）。
///
/// `UNIQUE(world_id, counter_date, domain_id, source_type)` 复合唯一约束。
/// `current_count` 为当日已入账**次数**（配合 `game_points_source_config.daily_limit` 限流）。
/// 跨日自动新建记录（次日首次入账时 INSERT 新行）。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameDailyLimitCounter {
    pub id: String,
    pub world_id: String,
    /// 计数日期，格式 `'YYYY-MM-DD'`（Asia/Shanghai 时区）。
    pub counter_date: String,
    pub domain_id: String,
    pub source_type: String,
    /// 当日已入账次数。
    pub current_count: i64,
    pub updated_at: i64,
}

/// 对应 `game_points_source_config` 表。11 种积分来源配置。
///
/// `daily_limit` 为 `None` 表示无上限；`default_domain_id` 为 `None` 表示由调用方指定。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GamePointsSourceConfig {
    /// 积分来源类型，主键（11 种）。
    pub source_type: String,
    /// 默认入账领域 id；`None` 表示由调用方指定（如 KB 条目按分类映射决定）。
    pub default_domain_id: Option<String>,
    /// 单次事件积分数。
    pub points_per_event: i64,
    /// 每日上限次数；`None` 表示无上限。
    pub daily_limit: Option<i64>,
    pub description: Option<String>,
    /// 启用开关（DB 存 INTEGER 0/1，sqlx 自动转换）。
    pub enabled: bool,
    pub updated_at: i64,
}

// ============================================================================
// 四-补、时间轴回放数据模型（T1.1，11_时间轴回放.md §6.1）
// ============================================================================

/// 快照触发原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotTriggerReason {
    /// 周期性快照（每 10 个事件一次）。
    #[serde(rename = "periodic_10")]
    Periodic10,
    /// 建造完成事件触发。
    #[serde(rename = "complete")]
    Complete,
    /// 升级事件触发。
    #[serde(rename = "upgrade")]
    Upgrade,
    /// 拆除事件触发。
    #[serde(rename = "remove")]
    Remove,
    /// 世界初始化首条事件触发。
    #[serde(rename = "world_init")]
    WorldInit,
    /// 文明等级变化触发。
    #[serde(rename = "civ_level_change")]
    CivLevelChange,
}

impl SnapshotTriggerReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            SnapshotTriggerReason::Periodic10 => "periodic_10",
            SnapshotTriggerReason::Complete => "complete",
            SnapshotTriggerReason::Upgrade => "upgrade",
            SnapshotTriggerReason::Remove => "remove",
            SnapshotTriggerReason::WorldInit => "world_init",
            SnapshotTriggerReason::CivLevelChange => "civ_level_change",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "periodic_10" => Some(SnapshotTriggerReason::Periodic10),
            "complete" => Some(SnapshotTriggerReason::Complete),
            "upgrade" => Some(SnapshotTriggerReason::Upgrade),
            "remove" => Some(SnapshotTriggerReason::Remove),
            "world_init" => Some(SnapshotTriggerReason::WorldInit),
            "civ_level_change" => Some(SnapshotTriggerReason::CivLevelChange),
            _ => None,
        }
    }
}

/// 快照元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub event_id: String,
    /// 该事件在当前世界事件流中的序号（从 1 开始）。
    pub event_seq: u32,
    pub world_id: String,
    pub captured_at: i64,
    pub trigger_reason: String,
}

/// 世界状态快照（不含建筑列表）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateSnapshot {
    pub civilization_level: i32,
    pub realm_major: String,
    pub realm_minor: String,
    pub dao_foundation: String,
    pub total_xp: i64,
    pub realm_xp: i64,
}

/// 单建筑快照（用于 `WorldSnapshot.buildings` 列表项）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingSnapshot {
    pub id: String,
    pub building_subtype: String,
    pub name: String,
    pub level: i32,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub rotation_y: f64,
    pub status: String,
    pub build_progress: f64,
    pub knowledge_domain: String,
}

/// 快照统计信息（冗余字段，便于回放时快速展示）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotStats {
    pub building_count: u32,
    pub total_levels: u32,
}

/// 完整世界状态快照（存储到 `game_build_history.snapshot_json`，zstd 压缩）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub schema_version: u32,
    pub snapshot_meta: SnapshotMeta,
    pub world_state: WorldStateSnapshot,
    pub buildings: Vec<BuildingSnapshot>,
    pub stats: SnapshotStats,
}

impl WorldSnapshot {
    pub const SCHEMA_VERSION: u32 = 1;
}

/// 场景重建结果（`game_get_world_snapshot` 命令返回）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuiltScene {
    pub world_state: WorldStateSnapshot,
    pub buildings: Vec<BuildingSnapshot>,
    pub stats: SnapshotStats,
    pub target_timestamp: i64,
    /// 基准快照的事件 ID；`None` 表示从空世界重建。
    pub source_snapshot_event_id: Option<String>,
    /// 重放的增量事件数。
    pub replayed_event_count: u32,
}

// ============================================================================
// 五、单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ----- RealmMajor 测试 -----

    #[test]
    fn test_realm_major_from_xp_boundaries() {
        // 下界（含）
        assert_eq!(RealmMajor::from_xp(0), RealmMajor::Mortal);
        assert_eq!(RealmMajor::from_xp(100), RealmMajor::QiRefining);
        assert_eq!(RealmMajor::from_xp(500), RealmMajor::FoundationBuilding);
        assert_eq!(RealmMajor::from_xp(2_000), RealmMajor::GoldenCore);
        assert_eq!(RealmMajor::from_xp(8_000), RealmMajor::NascentSoul);
        assert_eq!(RealmMajor::from_xp(30_000), RealmMajor::SpiritTransformation);
        assert_eq!(RealmMajor::from_xp(100_000), RealmMajor::Unity);
        assert_eq!(RealmMajor::from_xp(300_000), RealmMajor::Mahayana);
        assert_eq!(RealmMajor::from_xp(800_000), RealmMajor::Tribulation);
        assert_eq!(RealmMajor::from_xp(2_000_000), RealmMajor::Immortal);

        // 上界（不含）- 取下一阶下界 - 1
        assert_eq!(RealmMajor::from_xp(99), RealmMajor::Mortal);
        assert_eq!(RealmMajor::from_xp(499), RealmMajor::QiRefining);
        assert_eq!(RealmMajor::from_xp(1_999), RealmMajor::FoundationBuilding);
        assert_eq!(RealmMajor::from_xp(7_999), RealmMajor::GoldenCore);
        assert_eq!(RealmMajor::from_xp(29_999), RealmMajor::NascentSoul);
        assert_eq!(RealmMajor::from_xp(99_999), RealmMajor::SpiritTransformation);
        assert_eq!(RealmMajor::from_xp(299_999), RealmMajor::Unity);
        assert_eq!(RealmMajor::from_xp(799_999), RealmMajor::Mahayana);
        assert_eq!(RealmMajor::from_xp(1_999_999), RealmMajor::Tribulation);

        // 仙境界上限
        assert_eq!(RealmMajor::from_xp(i64::MAX), RealmMajor::Immortal);
    }

    #[test]
    fn test_realm_major_as_str_roundtrip() {
        let all = [
            RealmMajor::Mortal,
            RealmMajor::QiRefining,
            RealmMajor::FoundationBuilding,
            RealmMajor::GoldenCore,
            RealmMajor::NascentSoul,
            RealmMajor::SpiritTransformation,
            RealmMajor::Unity,
            RealmMajor::Mahayana,
            RealmMajor::Tribulation,
            RealmMajor::Immortal,
        ];
        for r in all.iter() {
            let s = r.as_str();
            assert_eq!(RealmMajor::from_str(s), Some(*r));
        }
        assert_eq!(RealmMajor::from_str("invalid"), None);
    }

    #[test]
    fn test_realm_major_ordinal() {
        assert_eq!(RealmMajor::Mortal.ordinal(), 1);
        assert_eq!(RealmMajor::Immortal.ordinal(), 10);
    }

    #[test]
    fn test_realm_major_xp_bounds_consistency() {
        // 每个境界的下界 < 上界
        let all = [
            RealmMajor::Mortal,
            RealmMajor::QiRefining,
            RealmMajor::FoundationBuilding,
            RealmMajor::GoldenCore,
            RealmMajor::NascentSoul,
            RealmMajor::SpiritTransformation,
            RealmMajor::Unity,
            RealmMajor::Mahayana,
            RealmMajor::Tribulation,
            RealmMajor::Immortal,
        ];
        for r in all.iter() {
            assert!(r.xp_lower_bound() <= r.xp_upper_bound());
        }
        // Mortal 下界为 0
        assert_eq!(RealmMajor::Mortal.xp_lower_bound(), 0);
        // Immortal 上界为 i64::MAX
        assert_eq!(RealmMajor::Immortal.xp_upper_bound(), i64::MAX);
    }

    #[test]
    fn test_realm_major_next() {
        assert_eq!(RealmMajor::Mortal.next(), Some(RealmMajor::QiRefining));
        assert_eq!(RealmMajor::Tribulation.next(), Some(RealmMajor::Immortal));
        assert_eq!(RealmMajor::Immortal.next(), None);
    }

    #[test]
    fn test_realm_major_breakthrough_question_count() {
        assert_eq!(RealmMajor::Mortal.breakthrough_question_count(), 0);
        assert_eq!(RealmMajor::QiRefining.breakthrough_question_count(), 3);
        assert_eq!(RealmMajor::Tribulation.breakthrough_question_count(), 15);
        assert_eq!(RealmMajor::Immortal.breakthrough_question_count(), 0);
    }

    // ----- RealmMinor 测试 -----

    #[test]
    fn test_realm_minor_roundtrip() {
        for r in [RealmMinor::Early, RealmMinor::Middle, RealmMinor::Complete] {
            let s = r.as_str();
            assert_eq!(RealmMinor::from_str(s), Some(r));
        }
        assert_eq!(RealmMinor::from_str("invalid"), None);
    }

    #[test]
    fn test_realm_minor_next() {
        assert_eq!(RealmMinor::Early.next(), Some(RealmMinor::Middle));
        assert_eq!(RealmMinor::Middle.next(), Some(RealmMinor::Complete));
        assert_eq!(RealmMinor::Complete.next(), None);
    }

    // ----- DaoFoundation 测试 -----

    #[test]
    fn test_dao_foundation_roundtrip() {
        for d in [
            DaoFoundation::White,
            DaoFoundation::Blue,
            DaoFoundation::Red,
            DaoFoundation::Purple,
            DaoFoundation::Black,
        ] {
            let s = d.as_str();
            assert_eq!(DaoFoundation::from_str(s), Some(d));
        }
        assert_eq!(DaoFoundation::from_str("invalid"), None);
    }

    #[test]
    fn test_dao_foundation_multiplier() {
        assert_eq!(dao_foundation_multiplier(DaoFoundation::White), 0.5);
        assert_eq!(dao_foundation_multiplier(DaoFoundation::Blue), 0.7);
        assert_eq!(dao_foundation_multiplier(DaoFoundation::Red), 1.0);
        assert_eq!(dao_foundation_multiplier(DaoFoundation::Purple), 1.3);
        assert_eq!(dao_foundation_multiplier(DaoFoundation::Black), 1.6);
    }

    #[test]
    fn test_dao_foundation_from_score() {
        assert_eq!(dao_foundation_from_score(0), DaoFoundation::White);
        assert_eq!(dao_foundation_from_score(999), DaoFoundation::White);
        assert_eq!(dao_foundation_from_score(1_000), DaoFoundation::Blue);
        assert_eq!(dao_foundation_from_score(4_999), DaoFoundation::Blue);
        assert_eq!(dao_foundation_from_score(5_000), DaoFoundation::Red);
        assert_eq!(dao_foundation_from_score(19_999), DaoFoundation::Red);
        assert_eq!(dao_foundation_from_score(20_000), DaoFoundation::Purple);
        assert_eq!(dao_foundation_from_score(99_999), DaoFoundation::Purple);
        assert_eq!(dao_foundation_from_score(100_000), DaoFoundation::Black);
        assert_eq!(dao_foundation_from_score(i64::MAX), DaoFoundation::Black);
    }

    #[test]
    fn test_dao_foundation_ordinal() {
        assert_eq!(DaoFoundation::White.ordinal(), 1);
        assert_eq!(DaoFoundation::Black.ordinal(), 5);
    }

    // ----- BuildingCategory 测试 -----

    #[test]
    fn test_building_category_roundtrip() {
        for c in [
            BuildingCategory::House,
            BuildingCategory::Town,
            BuildingCategory::City,
            BuildingCategory::Kingdom,
            BuildingCategory::Palace,
            BuildingCategory::Technology,
            BuildingCategory::Sect,
            BuildingCategory::Immortal,
        ] {
            let s = c.as_str();
            assert_eq!(BuildingCategory::from_str(s), Some(c));
        }
        assert_eq!(BuildingCategory::from_str("invalid"), None);
    }

    #[test]
    fn test_building_category_min_civilization_level() {
        assert_eq!(BuildingCategory::House.min_civilization_level(), 1);
        assert_eq!(BuildingCategory::Town.min_civilization_level(), 2);
        assert_eq!(BuildingCategory::City.min_civilization_level(), 3);
        assert_eq!(BuildingCategory::Kingdom.min_civilization_level(), 4);
        assert_eq!(BuildingCategory::Palace.min_civilization_level(), 5);
        assert_eq!(BuildingCategory::Technology.min_civilization_level(), 6);
        assert_eq!(BuildingCategory::Sect.min_civilization_level(), 7);
        assert_eq!(BuildingCategory::Immortal.min_civilization_level(), 8);
    }

    #[test]
    fn test_building_category_for_domain() {
        // 科技类
        assert_eq!(
            building_category_for_domain(domain::CS),
            BuildingCategory::Technology
        );
        assert_eq!(
            building_category_for_domain(domain::PHYSICS),
            BuildingCategory::Technology
        );
        assert_eq!(
            building_category_for_domain(domain::MATH),
            BuildingCategory::Technology
        );
        // 修仙类
        assert_eq!(
            building_category_for_domain(domain::MEDICINE),
            BuildingCategory::Immortal
        );
        assert_eq!(
            building_category_for_domain(domain::PHILOSOPHY),
            BuildingCategory::Immortal
        );
        // 宫殿类
        assert_eq!(
            building_category_for_domain(domain::ART),
            BuildingCategory::Palace
        );
        assert_eq!(
            building_category_for_domain(domain::LITERATURE),
            BuildingCategory::Palace
        );
        // 城池类
        assert_eq!(
            building_category_for_domain(domain::ENGINEERING),
            BuildingCategory::City
        );
        assert_eq!(
            building_category_for_domain(domain::HISTORY),
            BuildingCategory::City
        );
        // 城镇类
        assert_eq!(
            building_category_for_domain(domain::ECONOMICS),
            BuildingCategory::Town
        );
        assert_eq!(
            building_category_for_domain(domain::LANGUAGE),
            BuildingCategory::Town
        );
        // 默认
        assert_eq!(
            building_category_for_domain(domain::OTHER),
            BuildingCategory::House
        );
        assert_eq!(
            building_category_for_domain("unknown"),
            BuildingCategory::House
        );
    }

    // ----- building_required_points 测试 -----

    #[test]
    fn test_building_required_points_house_subtypes() {
        // house 4 个子类的 level 1 基准积分（04 文档 §3.1）
        assert_eq!(building_required_points("house", "thatch_cottage", 1), 100);
        assert_eq!(building_required_points("house", "wooden_house", 1), 250);
        assert_eq!(building_required_points("house", "brick_house", 1), 500);
        assert_eq!(building_required_points("house", "courtyard", 1), 1000);
    }

    #[test]
    fn test_building_required_points_level_factor() {
        // 等级系数 = 1 + (level - 1) × 0.5
        // thatch_cottage base=100
        assert_eq!(building_required_points("house", "thatch_cottage", 1), 100); // 1.0
        assert_eq!(building_required_points("house", "thatch_cottage", 2), 150); // 1.5
        assert_eq!(building_required_points("house", "thatch_cottage", 3), 200); // 2.0
        assert_eq!(building_required_points("house", "thatch_cottage", 4), 250); // 2.5
        // level=0 防御性：saturating_sub 让 0-1=0，系数=1.0
        assert_eq!(building_required_points("house", "thatch_cottage", 0), 100);
    }

    #[test]
    fn test_building_required_points_all_categories_level1() {
        // 8 大类各取一个子类验证 level 1 基准值
        assert_eq!(building_required_points("house", "thatch_cottage", 1), 100);
        assert_eq!(building_required_points("town", "market", 1), 800);
        assert_eq!(building_required_points("city", "warehouse", 1), 4000);
        assert_eq!(building_required_points("kingdom", "embassy", 1), 18000);
        assert_eq!(building_required_points("palace", "spirit_garden", 1), 35000);
        assert_eq!(building_required_points("technology", "lab", 1), 70000);
        assert_eq!(building_required_points("sect", "cultivation_cave", 1), 130000);
        assert_eq!(building_required_points("immortal", "immortal_pavilion", 1), 350000);
    }

    #[test]
    fn test_building_required_points_unknown_subtype_fallback() {
        // 未知子类按 category 兜底（避免除零）
        assert_eq!(building_required_points("house", "unknown_xyz", 1), 200);
        assert_eq!(building_required_points("immortal", "unknown_xyz", 1), 350000);
        // 未知大类按 house 兜底
        assert_eq!(building_required_points("unknown_cat", "unknown_xyz", 1), 200);
        // 等级系数同样应用
        assert_eq!(building_required_points("house", "unknown_xyz", 2), 300); // 200 × 1.5
    }

    // ----- 其他枚举 roundtrip 测试 -----

    #[test]
    fn test_building_status_roundtrip() {
        for s in [
            BuildingStatus::Planning,
            BuildingStatus::Building,
            BuildingStatus::Completed,
            BuildingStatus::Ruined,
        ] {
            let str_val = s.as_str();
            assert_eq!(BuildingStatus::from_str(str_val), Some(s));
        }
        assert_eq!(BuildingStatus::from_str("invalid"), None);
    }

    #[test]
    fn test_build_history_event_type_roundtrip() {
        for e in [
            BuildHistoryEventType::Build,
            BuildHistoryEventType::Upgrade,
            BuildHistoryEventType::Demolish,
            BuildHistoryEventType::Move,
            BuildHistoryEventType::Complete,
        ] {
            let s = e.as_str();
            assert_eq!(BuildHistoryEventType::from_str(s), Some(e));
        }
    }

    #[test]
    fn test_breakthrough_result_roundtrip() {
        for r in [
            BreakthroughResult::Success,
            BreakthroughResult::Failed,
            BreakthroughResult::Dropped,
        ] {
            let s = r.as_str();
            assert_eq!(BreakthroughResult::from_str(s), Some(r));
        }
    }

    #[test]
    fn test_points_log_result_roundtrip() {
        for r in [
            PointsLogResult::Accepted,
            PointsLogResult::RejectedDailyLimit,
            PointsLogResult::RejectedNegative,
        ] {
            let s = r.as_str();
            assert_eq!(PointsLogResult::from_str(s), Some(r));
        }
    }

    #[test]
    fn test_mapping_source_roundtrip() {
        for m in [
            MappingSource::Manual,
            MappingSource::AiSuggest,
            MappingSource::AiAuto,
        ] {
            let s = m.as_str();
            assert_eq!(MappingSource::from_str(s), Some(m));
        }
    }

    // ----- domain / source_type 模块测试 -----

    #[test]
    fn test_domain_all_count() {
        assert_eq!(domain::ALL.len(), 12);
        // 确保所有 id 唯一
        let mut sorted = domain::ALL.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 12);
    }

    #[test]
    fn test_source_type_all_count() {
        assert_eq!(source_type::ALL.len(), 11);
        let mut sorted = source_type::ALL.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 11);
    }

    // ----- domain_level_from_earned 测试 -----

    #[test]
    fn test_domain_level_from_earned_boundaries() {
        // Lv1 入门（0 ~ 99）
        assert_eq!(domain_level_from_earned(0), 1);
        assert_eq!(domain_level_from_earned(99), 1);
        // Lv2 初窥（100 ~ 499）
        assert_eq!(domain_level_from_earned(100), 2);
        assert_eq!(domain_level_from_earned(499), 2);
        // Lv3 进阶（500 ~ 1999）
        assert_eq!(domain_level_from_earned(500), 3);
        assert_eq!(domain_level_from_earned(1_999), 3);
        // Lv4 精通（2000 ~ 9999）
        assert_eq!(domain_level_from_earned(2_000), 4);
        assert_eq!(domain_level_from_earned(9_999), 4);
        // Lv5 大师（10000 ~ 49999）
        assert_eq!(domain_level_from_earned(10_000), 5);
        assert_eq!(domain_level_from_earned(49_999), 5);
        // Lv6 宗师（50000 ~ 199999）
        assert_eq!(domain_level_from_earned(50_000), 6);
        assert_eq!(domain_level_from_earned(199_999), 6);
        // Lv7 化境（200000 ~ 999999）
        assert_eq!(domain_level_from_earned(200_000), 7);
        assert_eq!(domain_level_from_earned(999_999), 7);
        // Lv8 问道（1000000+）
        assert_eq!(domain_level_from_earned(1_000_000), 8);
        assert_eq!(domain_level_from_earned(i64::MAX), 8);
    }

    #[test]
    fn test_domain_level_thresholds_count() {
        assert_eq!(DOMAIN_LEVEL_THRESHOLDS.len(), 8);
        assert_eq!(DOMAIN_LEVEL_THRESHOLDS[0], 0);
        assert_eq!(DOMAIN_LEVEL_THRESHOLDS[7], 1_000_000);
    }

    // ----- 默认值常量测试 -----

    #[test]
    fn test_default_constants() {
        assert_eq!(DEFAULT_PLAYER_NAME, "无名修士");
        assert_eq!(DEFAULT_CIVILIZATION_LEVEL, 1);
        assert_eq!(DEFAULT_MAP_WIDTH, 32);
        assert_eq!(DEFAULT_MAP_HEIGHT, 32);
        assert_eq!(DEFAULT_REALM_MAJOR, RealmMajor::Mortal);
        assert_eq!(DEFAULT_REALM_MINOR, RealmMinor::Early);
        assert_eq!(DEFAULT_DAO_FOUNDATION, DaoFoundation::White);
        assert_eq!(DEFAULT_BUILDING_STATUS, BuildingStatus::Completed);
        assert_eq!(DEFAULT_BUILD_PROGRESS, 0.0);
    }
}
