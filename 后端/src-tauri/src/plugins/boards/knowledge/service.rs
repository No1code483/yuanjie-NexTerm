//! boards.knowledge 插件服务边界说明。
//!
//! 自有表：10 张 `kb_*`（归属登记见 `migrations/0001_baseline.sql`）。
//!
//! 跨模块消费方（登记为跨插件契约点，档案 `插件档案/boards.knowledge.md` §6.5-C）：
//! 全局搜索 / 数据蒸馏 / 回收站 / 小欣（2 处）/ 游戏（多模态出题）/ Profile 导出。
//! 反向直连：`kb_service` 调 `recycle_repo::add_item`（写回收站）与
//! `intelligence_v4_service::instrument`（行为埋点），均登记待阶段4 接口化。
//!
//! ⚠️ ID 稳定性红线（档案 §6.6）：`game_kb_category_mapping.category_id` 已持久化
//! `kb_categories.id` 值且无物理 FK，本插件迁移不得改变该表主键值。

pub const OWNED_TABLES: &[&str] = &[
    "kb_attachment_cache",
    "kb_categories",
    "kb_entries",
    "kb_entry_tags",
    "kb_recent_access",
    "kb_references",
    "kb_snapshots",
    "kb_tags",
    "kb_templates",
    "kb_tracked_paths",
];
