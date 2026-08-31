use serde::{Deserialize, Serialize};

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub examples: Vec<ToolExample>,
    pub category: ToolCategory,
    pub tags: Vec<String>,
}

/// 工具示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub arguments: serde_json::Value,
}

/// 工具分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolCategory {
    FileEdit,
    Plan,
    Shell,
    Search,
    Agent,
    Info,
    Other,
}

/// 工具调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub session_id: Option<String>,
    pub approval_action: Option<ApprovalAction>,
}

/// 工具调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub success: bool,
    pub content: String,
    pub metadata: Option<ToolCallMetadata>,
    pub error: Option<ToolError>,
}

/// 工具调用元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallMetadata {
    pub duration_ms: u64,
    pub tokens_used: Option<u32>,
    pub files_modified: Vec<String>,
    pub sandbox_used: bool,
}

/// 工具错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

/// 工具信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub definition: ToolDefinition,
    pub permission: ToolPermission,
}

/// 工具权限
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermission {
    pub requires_approval: bool,
    pub approval_level: ApprovalLevel,
    pub sandbox_required: bool,
    pub max_retries: u32,
    pub timeout_ms: u64,
    pub rate_limit_per_minute: Option<u32>,
}

impl Default for ToolPermission {
    fn default() -> Self {
        Self {
            requires_approval: false,
            approval_level: ApprovalLevel::Low,
            sandbox_required: false,
            max_retries: 2,
            timeout_ms: 60_000,
            rate_limit_per_minute: None,
        }
    }
}

/// 审批级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApprovalLevel {
    /// 低风险：自动批准
    Low,
    /// 中风险：需要用户确认
    Medium,
    /// 高风险：需要管理员确认
    High,
    /// 关键：需要多重确认
    Critical,
}

/// 审批动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalAction {
    pub approved: bool,
    pub reason: Option<String>,
    pub timestamp: u64,
}

/// 审批状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalState {
    pub pending: bool,
    pub level: ApprovalLevel,
    pub reason: String,
    pub request_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

/// 并发工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelToolCalls {
    pub calls: Vec<ToolCallRequest>,
    pub max_concurrency: usize,
    pub fail_fast: bool,
}

/// 并行调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelToolResults {
    pub results: Vec<ParallelToolResult>,
    pub total_duration_ms: u64,
    pub success_count: usize,
    pub failure_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelToolResult {
    pub tool_name: String,
    pub result: Result<ToolCallResult, ToolError>,
    pub duration_ms: u64,
}

/// 工具搜索请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchRequest {
    pub query: String,
    pub category: Option<ToolCategory>,
    pub max_results: Option<usize>,
}

/// 工具搜索响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchResponse {
    pub results: Vec<ToolSearchResult>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSearchResult {
    pub tool: ToolDefinition,
    pub relevance_score: f64,
    pub match_type: ToolMatchType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolMatchType {
    ExactName,
    NameContains,
    DescriptionMatch,
    TagMatch,
    SemanticMatch,
}

/// Plan 步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: PlanStepStatus,
    pub dependencies: Vec<String>,
    pub assigned_agent: Option<String>,
    pub result: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlanStepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

/// 计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub title: String,
    pub description: String,
    pub steps: Vec<PlanStep>,
    pub status: PlanStepStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub session_id: String,
}

/// Apply Patch 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchRequest {
    pub file_path: String,
    pub patch_content: String,
    pub description: Option<String>,
    pub create_if_not_exists: Option<bool>,
}

/// Apply Patch 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchResult {
    pub file_path: String,
    pub applied: bool,
    pub changes: Vec<PatchChange>,
    pub original_content: Option<String>,
    pub new_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchChange {
    pub line_start: usize,
    pub line_end: usize,
    pub change_type: PatchChangeType,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatchChangeType {
    Add,
    Remove,
    Modify,
}