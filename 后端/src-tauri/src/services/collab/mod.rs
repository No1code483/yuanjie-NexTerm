//! Yuan Code v3.2 Task 3.4.3 / 3.4.4 — 多人实时协作子模块
//!
//! 设计文档：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.3 / §3.4.4（Phase 6）
//!
//! 模块清单：
//! - `yjs_doc`：Yjs 文档同步接口骨架（评估 yrs，当前为接口骨架 + TODO）
//! - `session`：协作会话管理（创建/加入/退出/在线状态/实时光标）
//!
//! ## 当前状态：接口骨架（不阻塞编译）
//!
//! Yjs / yrs 集成方案：
//! - 当前版本：接口骨架 + 内存态会话管理（无外部 yrs 依赖）
//! - 未来升级：引入 `yrs = "0.21"`（Yjs Rust 实现）后，
//!   将 YjsDocSkeleton 替换为真实 yrs::Doc 封装，提供 apply_update / merge / encode
//! - 前端集成：yjs + y-websocket + y-monaco（Monaco Editor 协作绑定）
//!
//! 架构（接口骨架阶段）：
//! - 会话状态：内存态 HashMap<SessionId, CollabSession>（重启丢失，符合 PoC 定位）
//! - 文档同步：每次更新直接广播（无 CRDT 合并），适合单用户 PoC
//! - 多用户场景：前端 yjs 库直接通过 y-websocket 同步；后端仅做会话路由
//!
//! 与底层智能的边界：
//! - 协作会话是非 AI 功能，不调用云端 API，不依赖底层智能
//! - 底层智能可监测协作行为（通过 yuan_code_monitor），但不替代协作逻辑

pub mod session;
pub mod yjs_doc;

pub use session::{
    CollabSession, CollabSessionManager, CollabSessionSummary, CollabUser, CursorPosition,
};
