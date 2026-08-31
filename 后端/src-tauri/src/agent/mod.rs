//! Agent 系统 — 对标 Codex agent/ 模块
//!
//! 提供多 Agent 编排能力：
//! - AgentControl: 创建/销毁/查询 Agent
//! - AgentRegistry: Agent 生命周期管理和限制
//! - RoleRegistry: 角色配置和系统提示词注入
//! - AgentStatus: 状态跟踪
//! - AgentCommunication: Agent 间消息传递
//!
//! ## v3.1 扩展（Phase 3 Agent 化）
//! - types: 7 种 AgentType + AgentPlan/AgentResult/AgentReview 数据结构
//! - lifecycle: AgentLifecycle trait + AgentUtils（云端 API 调用共用方法）
//! - specialized: 7 种具体 Agent 类型实现（CodingAgent/RefactorAgent/TestAgent/
//!   DocumentationAgent/DebugAgent/MigrationAgent/ReviewAgent）

pub mod communication;
pub mod control;
pub mod lifecycle;
pub mod multi;
pub mod registry;
pub mod role;
pub mod safety_guard;
pub mod specialized;
pub mod status;
pub mod templates;
pub mod types;

pub use communication::AgentCommunication;
pub use control::AgentControl;
pub use lifecycle::AgentLifecycle;
pub use multi::MultiAgentOrchestrator;
pub use registry::AgentRegistry;
pub use role::RoleRegistry;
pub use specialized::{
    CodingAgent, DebugAgent, DocumentationAgent, MigrationAgent,
    RefactorAgent, ReviewAgent, TestAgent, create_agent,
};
pub use status::AgentStatus;
pub use templates::TemplateManager;
pub use types::{
    AgentFileDiff, AgentPhase, AgentPlan, AgentPlanStep, AgentResult,
    AgentReviewDecision, AgentReviewRequest, AgentType,
};