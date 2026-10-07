//! A5 离线与同步机制 - Phase 2-4 子模块
//!
//! 本目录包含同步机制的传输后端适配器、CRDT 集成、退避重试调度器与端到端加密。
//! 设计文档：功能展望/平台级增强/04_离线与同步机制.md
//!
//! 模块清单：
//! - `webdav`：WebDAV 后端适配器（HTTP PUT/GET/PROPFIND/MOVE）
//! - `s3`：S3 兼容存储后端适配器（SigV4 签名 + PUT/GET/LIST）
//! - `crdt`：CRDT 文档合并骨架（评估 yrs/Automerge，当前为接口骨架）
//! - `scheduler`：退避重试调度器（指数退避 + 抖动）
//! - `e2ee`：端到端加密（ECDH 协商密钥 + AES-GCM 加密同步数据）
//! - `connectivity`：网络状态检测层（Phase 3，周期探测 + 事件广播）

pub mod connectivity;
pub mod crdt;
pub mod e2ee;
pub mod s3;
pub mod scheduler;
pub mod webdav;
