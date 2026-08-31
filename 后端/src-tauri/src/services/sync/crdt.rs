//! A5.2.6.3 CRDT 文档合并（接口骨架）
//!
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.3 / §2.6.3
//!
//! ## 当前状态：接口骨架
//!
//! 本模块定义 CRDT（Conflict-free Replicated Data Type）合并的统一接口与基础实现。
//! 当前版本仅提供骨架，未引入完整 CRDT 库。
//!
//! ## 选型评估
//!
//! 评估了两个候选库：
//! 1. **yrs**（Yjs 的 Rust 实现）
//!    - 优点：成熟、与 Yjs 生态互通（前端可用 yjs 库直接协作）
//!    - 缺点：依赖较多（lib0 / rand / serde），引入后包体积增加 ~800KB
//!    - 适用：需要与前端 Yjs 实时协作的场景
//!
//! 2. **automerge**（Automerge 的 Rust 实现）
//!    - 优点：JSON-like 数据模型，更适合结构化文档同步
//!    - 缺点：API 较繁琐，需要包装为 SyncDoc 抽象
//!    - 适用：笔记、配置等 JSON 结构同步
//!
//! ## 决策
//!
//! Phase 2 采用 **LWW + Map CRDT 简化实现**（基于现有 `resolve_lww`），
//! 推迟 yrs/Automerge 集成至 Phase 3（多端实时编辑场景）。
//! 这样可以：
//! - 避免引入重依赖拖慢编译
//! - 满足 80% 的同步场景（笔记、配置、待办等结构化数据）
//! - 实时协作（多人编辑同一文档）在 Phase 3 单独评估
//!
//! ## 后续 TODO
//!
//! - [ ] 引入 yrs 依赖（`yrs = "0.21"`）
//! - [ ] 实现 `YjsCrdtDoc`：封装 yrs::Doc，提供 apply_update / merge / encode 等方法
//! - [ ] 前端集成 yjs + y-websocket，实现真正的实时协作
//! - [ ] 性能基准测试：单文档合并延迟 < 50ms

use serde::{Deserialize, Serialize};

use crate::error::app_error::AppError;

/// CRDT 文档接口
///
/// 所有 CRDT 实现需遵循此接口。当前仅 `LwwMapCrdtDoc` 一个实现。
pub trait CrdtDoc: Send + Sync {
    /// 文档类型名（如 "lww_map" / "yjs" / "automerge"）
    fn doc_type(&self) -> &str;

    /// 合并另一个文档的状态到当前文档
    ///
    /// 参数 `other_state` 为对端序列化后的状态（JSON / 二进制编码）。
    /// 合并后当前文档包含双方所有变更，且无冲突。
    fn merge(&mut self, other_state: &[u8]) -> Result<(), AppError>;

    /// 序列化当前文档状态
    ///
    /// 返回的字节流可用于网络传输或持久化存储。
    fn encode(&self) -> Result<Vec<u8>, AppError>;

    /// 应用一次变更（INSERT/UPDATE/DELETE）
    ///
    /// 参数 `key` 通常是 `{table_name}:{record_id}`，`value` 是 JSON payload。
    fn apply_change(
        &mut self,
        key: &str,
        value: &str,
        timestamp: i64,
    ) -> Result<(), AppError>;
}

/// LWW-Map CRDT 文档（基于 Last-Write-Wins 的简化 CRDT）
///
/// 每个 key 关联一个 `(value, timestamp)` 元组，合并时取 timestamp 较大者。
/// 适用于大多数结构化数据同步场景（笔记、待办、配置等）。
///
/// **优势**：
/// - 实现简单，无外部依赖
/// - 空间复杂度 O(n)，时间复杂度 O(1) per key
/// - 与现有 `resolve_lww` 函数兼容
///
/// **局限**：
/// - 不支持字段级合并（整个 value 原子替换）
/// - 不支持集合类型（list / set）的元素级合并
/// - 不支持撤销（undo）
pub struct LwwMapCrdtDoc {
    entries: std::collections::HashMap<String, LwwEntry>,
}

/// LWW 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LwwEntry {
    pub value: String,
    pub timestamp: i64,
    pub deleted: bool,
}

impl LwwMapCrdtDoc {
    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }

    /// 获取某个 key 的当前值
    pub fn get(&self, key: &str) -> Option<&LwwEntry> {
        self.entries.get(key)
    }

    /// 列出所有未删除的条目
    pub fn alive_entries(&self) -> Vec<(&String, &LwwEntry)> {
        self.entries
            .iter()
            .filter(|(_, e)| !e.deleted)
            .collect()
    }

    /// 条目数（含已删除）
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for LwwMapCrdtDoc {
    fn default() -> Self {
        Self::new()
    }
}

impl CrdtDoc for LwwMapCrdtDoc {
    fn doc_type(&self) -> &str {
        "lww_map"
    }

    fn merge(&mut self, other_state: &[u8]) -> Result<(), AppError> {
        let other: std::collections::HashMap<String, LwwEntry> = serde_json::from_slice(other_state)
            .map_err(|e| AppError::Internal(format!("CRDT 状态反序列化失败: {}", e)))?;

        for (key, entry) in other {
            match self.entries.get_mut(&key) {
                Some(existing) if entry.timestamp > existing.timestamp => {
                    *existing = entry;
                }
                None => {
                    self.entries.insert(key, entry);
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn encode(&self) -> Result<Vec<u8>, AppError> {
        serde_json::to_vec(&self.entries)
            .map_err(|e| AppError::Internal(format!("CRDT 状态序列化失败: {}", e)))
    }

    fn apply_change(
        &mut self,
        key: &str,
        value: &str,
        timestamp: i64,
    ) -> Result<(), AppError> {
        let entry = LwwEntry {
            value: value.to_string(),
            timestamp,
            deleted: value.is_empty(),
        };

        match self.entries.get_mut(key) {
            Some(existing) if timestamp >= existing.timestamp => {
                *existing = entry;
            }
            None => {
                self.entries.insert(key.to_string(), entry);
            }
            _ => {}
        }

        Ok(())
    }
}

/// Yjs CRDT 文档（骨架，待引入 yrs 实现）
///
/// TODO: 引入 yrs 依赖后实现：
/// ```ignore
/// use yrs::Doc;
/// pub struct YjsCrdtDoc {
///     doc: Doc,
/// }
/// impl YjsCrdtDoc {
///     pub fn new() -> Self { Self { doc: Doc::new() } }
/// }
/// impl CrdtDoc for YjsCrdtDoc {
///     fn doc_type(&self) -> &str { "yjs" }
///     fn merge(&mut self, other: &[u8]) -> Result<(), AppError> {
///         let update = yrs::updates::decoder::Decode::decode(other)
///             .map_err(|e| AppError::Internal(format!("yrs decode 失败: {}", e)))?;
///         self.doc.apply_update(update);
///         Ok(())
///     }
///     // ... 其他方法
/// }
/// ```
pub struct YjsCrdtDoc {
    _placeholder: (),
}

impl YjsCrdtDoc {
    pub fn new() -> Self {
        Self { _placeholder: () }
    }
}

impl CrdtDoc for YjsCrdtDoc {
    fn doc_type(&self) -> &str {
        "yjs_skeleton"
    }

    fn merge(&mut self, _other_state: &[u8]) -> Result<(), AppError> {
        Err(AppError::Internal(
            "YjsCrdtDoc 暂未实现，待引入 yrs 依赖（见模块顶部 TODO）".into(),
        ))
    }

    fn encode(&self) -> Result<Vec<u8>, AppError> {
        Err(AppError::Internal(
            "YjsCrdtDoc 暂未实现，待引入 yrs 依赖（见模块顶部 TODO）".into(),
        ))
    }

    fn apply_change(
        &mut self,
        _key: &str,
        _value: &str,
        _timestamp: i64,
    ) -> Result<(), AppError> {
        Err(AppError::Internal(
            "YjsCrdtDoc 暂未实现，待引入 yrs 依赖（见模块顶部 TODO）".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lww_map_apply_change() {
        let mut doc = LwwMapCrdtDoc::new();
        doc.apply_change("todos:1", r#"{"title":"hello"}"#, 1000).unwrap();
        assert_eq!(doc.len(), 1);
        assert_eq!(doc.get("todos:1").unwrap().timestamp, 1000);
    }

    #[test]
    fn test_lww_map_newer_wins() {
        let mut doc = LwwMapCrdtDoc::new();
        doc.apply_change("todos:1", r#"{"v":"old"}"#, 1000).unwrap();
        doc.apply_change("todos:1", r#"{"v":"new"}"#, 2000).unwrap();
        assert_eq!(doc.get("todos:1").unwrap().value, r#"{"v":"new"}"#);
    }

    #[test]
    fn test_lww_map_older_ignored() {
        let mut doc = LwwMapCrdtDoc::new();
        doc.apply_change("todos:1", r#"{"v":"new"}"#, 2000).unwrap();
        doc.apply_change("todos:1", r#"{"v":"old"}"#, 1000).unwrap();
        assert_eq!(doc.get("todos:1").unwrap().value, r#"{"v":"new"}"#);
    }

    #[test]
    fn test_lww_map_merge() {
        let mut doc1 = LwwMapCrdtDoc::new();
        doc1.apply_change("todos:1", r#"{"v":"a"}"#, 1000).unwrap();
        doc1.apply_change("todos:2", r#"{"v":"b"}"#, 1500).unwrap();

        let mut doc2 = LwwMapCrdtDoc::new();
        doc2.apply_change("todos:1", r#"{"v":"a2"}"#, 2000).unwrap();
        doc2.apply_change("todos:3", r#"{"v":"c"}"#, 1800).unwrap();

        let state2 = doc2.encode().unwrap();
        doc1.merge(&state2).unwrap();

        // todos:1 应取较新版本（timestamp=2000）
        assert_eq!(doc1.get("todos:1").unwrap().value, r#"{"v":"a2"}"#);
        // todos:2 保持不变
        assert_eq!(doc1.get("todos:2").unwrap().value, r#"{"v":"b"}"#);
        // todos:3 新增
        assert_eq!(doc1.get("todos:3").unwrap().value, r#"{"v":"c"}"#);
    }

    #[test]
    fn test_lww_map_delete_via_empty_value() {
        let mut doc = LwwMapCrdtDoc::new();
        doc.apply_change("todos:1", r#"{"v":"a"}"#, 1000).unwrap();
        doc.apply_change("todos:1", "", 2000).unwrap();
        assert!(doc.get("todos:1").unwrap().deleted);
        assert_eq!(doc.alive_entries().len(), 0);
    }

    #[test]
    fn test_yjs_skeleton_returns_error() {
        let mut doc = YjsCrdtDoc::new();
        assert!(doc.merge(b"{}").is_err());
        assert!(doc.encode().is_err());
        assert!(doc.apply_change("k", "v", 1).is_err());
        assert_eq!(doc.doc_type(), "yjs_skeleton");
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut doc = LwwMapCrdtDoc::new();
        doc.apply_change("k1", "v1", 100).unwrap();
        doc.apply_change("k2", "v2", 200).unwrap();
        let encoded = doc.encode().unwrap();

        let mut doc2 = LwwMapCrdtDoc::new();
        doc2.merge(&encoded).unwrap();
        assert_eq!(doc2.len(), 2);
        assert_eq!(doc2.get("k1").unwrap().value, "v1");
        assert_eq!(doc2.get("k2").unwrap().value, "v2");
    }
}
