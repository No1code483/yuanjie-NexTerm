//! boards.ai 的 L2 Feature 插件集合。
//! 批次2b-1：ai.models / ai.agent / ai.groupchat（模型与 Agent 管理面 14 条命令）。
//! batchC1：ai.sessions（会话列表，10 条命令 + 4 张会话表）。
//! 本轮：ai.chat / ai.multimodel / ai.orchestration / ai.prompts（纯前端 L2 骨架）。

pub mod agent;          // 智能体集
pub mod chat;           // 对话
pub mod groupchat;      // 辩论群聊
pub mod models;         // 模型管理
pub mod multimodel;     // 多模型对比
pub mod orchestration;  // 群聊编排
pub mod prompts;        // 提示词模板
pub mod sessions;       // 会话列表