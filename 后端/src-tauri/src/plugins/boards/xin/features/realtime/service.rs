//! xin.realtime（L2）服务边界说明。
//!
//! 批次4a-1：承载 `xin_realtime_commands` 4 条命令（实时语音状态机）。
//! 无自有表（`XinRealtimeService` 为内存态全局单例）；实现层 `xin_realtime_service`
//! 留主应用，dispatcher 复用（2b-2 裁定 15 同口径）。
//! **登记（裁定 3）**：全局单例会话无主体分片 → 多用户互斥，架构级改造登记阶段4。

pub const OWNED_TABLES: &[&str] = &[];