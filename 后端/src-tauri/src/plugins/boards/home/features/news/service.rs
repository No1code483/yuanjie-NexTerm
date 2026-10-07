//! home.news 插件服务边界说明。
//!
//! 自有表：news_cache / news_pending_delete / news_sources / news_offline_cache
//! （归属登记见 migrations/0001_baseline.sql）。
//!
//! 跨模块只读消费者（实测仅 1 处）：`commands/intelligence_commands.rs` 的每日简报
//! 服务级只读 `news_service::get_news(&pool, user_id)`；按裁定 3 本批仅登记，
//! 阶段4 由 customs.intelligence 迁移时收口（见档案 §13.6）。
//!
//! 越权面收口（批次 1b-2b-2）：`news_pending_delete` / `news_offline_cache` 已由
//! `migrations/0002_hardening_user_id.sql` 补 `user_id` 列，读写路径按会话主体过滤。
//! 已登记残留（裁定 20-A）：`news_offline_cache.id` 仍为 `source+url+title` 哈希，
//! 跨用户同源同题会哈希相同 → `INSERT OR REPLACE` 可能覆盖他人行；归一属阶段4。

pub const OWNED_TABLES: &[&str] = &[
    "news_cache",
    "news_pending_delete",
    "news_sources",
    "news_offline_cache",
];
