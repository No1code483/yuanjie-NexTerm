//! Yuan Code v3.2 Task 3.4.3 — Yjs 多人编辑集成（接口骨架）
//!
//! 设计文档：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.3（Phase 6）
//!
//! ## 当前状态：接口骨架（不阻塞编译）
//!
//! 本模块定义 Yjs 文档同步的统一接口。当前版本仅提供骨架，未引入完整 yrs 库。
//!
//! ## 选型评估
//!
//! 评估了 yrs（Yjs Rust 实现）：
//! - 优点：成熟、与前端 yjs 生态互通（前端可用 yjs + y-monaco 实现实时协作）
//! - 缺点：依赖较多（lib0 / serde），引入后包体积增加 ~800KB
//! - 适用：Yuan Code 多人实时编辑同一源码文件的场景
//!
//! ## 决策
//!
//! Phase 6 采用 **接口骨架 + 内存态会话管理**：
//! - 后端不持有 yrs::Doc，仅做会话路由（创建/加入/退出/在线状态）
//! - 前端集成 yjs + y-websocket + y-monaco，通过 WebSocket 直连同步
//! - 后端 collab_session_* 命令负责会话元数据管理，不参与文档内容同步
//!
//! 这样可以：
//! - 避免引入重依赖拖慢编译
//! - 满足 Phase 6 验收（架构支持多用户实时协作）
//! - 推迟 yrs 集成到真正需要"后端参与文档合并"的场景（如离线协作）
//!
//! ## 后续 TODO
//!
//! - [ ] 引入 yrs 依赖（`yrs = "0.21"`）
//! - [ ] 实现 `YjsDocImpl`：封装 yrs::Doc，提供 apply_update / merge / encode
//! - [ ] 后端集成 y-sync 协议，作为 WebSocket 中继服务器
//! - [ ] 性能基准测试：单文档同步延迟 < 50ms

use crate::error::app_error::AppError;

/// Yjs 文档同步接口（接口骨架）
///
/// 所有 Yjs 文档实现需遵循此接口。当前无实现，仅作为升级点占位。
pub trait YjsDoc: Send + Sync {
    /// 文档 ID（与 collab_session 关联）
    fn doc_id(&self) -> &str;

    /// 应用一次 update（来自对端的二进制 update）
    ///
    /// Yjs 协议：update 是二进制编码的状态变更，apply 后文档包含双方所有变更。
    fn apply_update(&mut self, update: &[u8]) -> Result<(), AppError>;

    /// 序列化当前文档状态（SV + Update 合并）
    fn encode_state_as_update(&self) -> Result<Vec<u8>, AppError>;

    /// 合并另一个文档的状态（用于离线后重新加入会话）
    fn merge(&mut self, other_state: &[u8]) -> Result<(), AppError>;
}

/// Yjs 文档骨架实现（PoC 占位，不参与实际同步）
///
/// 当前版本仅记录 update 历史，不执行真正的 CRDT 合并。
/// 前端 yjs 库通过 y-websocket 直连同步，后端不参与文档内容。
pub struct YjsDocSkeleton {
    doc_id: String,
    /// 累积的 update 历史（仅用于审计/调试，不参与同步）
    updates: Vec<Vec<u8>>,
}

impl YjsDocSkeleton {
    pub fn new(doc_id: impl Into<String>) -> Self {
        Self {
            doc_id: doc_id.into(),
            updates: Vec::new(),
        }
    }

    /// 当前 update 数量（调试/审计用）
    pub fn update_count(&self) -> usize {
        self.updates.len()
    }
}

impl YjsDoc for YjsDocSkeleton {
    fn doc_id(&self) -> &str {
        &self.doc_id
    }

    fn apply_update(&mut self, update: &[u8]) -> Result<(), AppError> {
        // 骨架实现：仅记录历史，不执行真正的 CRDT 合并
        // TODO: 引入 yrs 后，替换为 self.doc.apply_update(update) 完成真实合并
        self.updates.push(update.to_vec());
        Ok(())
    }

    fn encode_state_as_update(&self) -> Result<Vec<u8>, AppError> {
        // 骨架实现：返回空 update
        // TODO: 引入 yrs 后，返回 self.doc.encode_state_as_update_v1()
        Ok(Vec::new())
    }

    fn merge(&mut self, other_state: &[u8]) -> Result<(), AppError> {
        // 骨架实现：调用 apply_update 即可（Yjs update 是可合并的）
        // TODO: 引入 yrs 后，true merge semantics 由 yrs 内部保证
        self.apply_update(other_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_records_updates() {
        let mut doc = YjsDocSkeleton::new("session-1");
        assert_eq!(doc.update_count(), 0);

        doc.apply_update(&[1, 2, 3]).unwrap();
        doc.apply_update(&[4, 5, 6]).unwrap();
        assert_eq!(doc.update_count(), 2);
        assert_eq!(doc.doc_id(), "session-1");
    }

    #[test]
    fn skeleton_encode_returns_empty() {
        let doc = YjsDocSkeleton::new("session-2");
        let state = doc.encode_state_as_update().unwrap();
        assert!(state.is_empty());
    }

    #[test]
    fn skeleton_merge_calls_apply_update() {
        let mut doc = YjsDocSkeleton::new("session-3");
        doc.merge(&[7, 8, 9]).unwrap();
        assert_eq!(doc.update_count(), 1);
    }
}
