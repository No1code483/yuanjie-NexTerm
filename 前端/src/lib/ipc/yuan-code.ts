// ipc/yuan-code.ts — yuan-code 模块类型定义（值对象已随 terminal.yuancode 插件化迁出，仅保留 type 定义供 re-export）

// ============================================================
// D1 v3.1 Task 3.5: 云端 API Key 类型定义
// ============================================================

/** 云端 API Key 信息（响应形态，不含密文） */
export interface CloudApiKeyInfo {
  id: number;
  provider: string;
  display_name?: string;
  api_url?: string;
  is_cloud_only: boolean;
  is_enabled: boolean;
  has_api_key: boolean;
  last_used_at?: number;
  created_at: number;
  updated_at: number;
}

/** 新增/更新 API Key 请求 */
export interface UpsertCloudApiKeyRequest {
  provider: string;
  display_name?: string;
  /** 明文 API Key（仅传输用，落盘前由后端 AES-GCM 加密） */
  api_key_plain: string;
  api_url?: string;
  is_enabled?: boolean;
}

/** 支持的云端 provider 元信息 */
export interface CloudProviderInfo {
  provider: string;
  display_name: string;
  default_api_url?: string;
  default_model?: string;
  available_models: string[];
}

/** 连通性测试结果 */
export interface CloudConnectionTestResult {
  success: boolean;
  provider: string;
  model_used: string;
  response_snippet: string;
  latency_ms: number;
  error?: string;
}

// ============================================================
// D1 v3.1 Task 3.1: Agent 化类型定义
// ============================================================

/** 7 种 Agent 类型信息 */
export interface AgentV3TypeInfo {
  type_name: 'coding' | 'refactor' | 'test' | 'documentation' | 'debug' | 'migration' | 'review';
  display_name: string;
  description: string;
}

/** 创建 Agent 响应 */
export interface AgentV3Created {
  agent_id: string;
  agent_type: string;
  phase: string;
  created_at: number;
}

/** 计划步骤 */
export interface AgentV3PlanStep {
  step_id: number;
  title: string;
  description: string;
  target_files: string[];
  requires_confirmation: boolean;
  status: string;
}

/** 完整计划 */
export interface AgentV3Plan {
  agent_id: string;
  agent_type: string;
  title: string;
  summary: string;
  steps: AgentV3PlanStep[];
  estimated_steps: number;
  estimated_duration_sec?: number;
  created_at: number;
}

/** 安全检查结果 */
export interface AgentV3SafetyCheck {
  passed: boolean;
  blocked_steps: number[];
  reasons: string[];
  risk_level: 'safe' | 'warning' | 'dangerous';
}

/** Plan 响应（含安全检查结果） */
export interface AgentV3PlanResponse {
  plan: AgentV3Plan;
  safety_check: AgentV3SafetyCheck;
}

/** 单文件 diff */
export interface AgentV3FileDiff {
  path: string;
  unified_diff: string;
  original_content?: string;
  modified_content?: string;
  added_lines: number;
  removed_lines: number;
  is_new_file: boolean;
}

/** Agent 执行结果 */
export interface AgentV3Result {
  agent_id: string;
  agent_type: string;
  plan: AgentV3Plan;
  diffs: AgentV3FileDiff[];
  execution_log: string[];
  provider: string;
  model_used: string;
  tokens_used?: number;
  summary: string;
  completed_at: number;
}

/** Review 请求 */
export interface AgentV3ReviewRequest {
  agent_id: string;
  decision: 'approve' | 'reject' | 'approve_partial';
  selected_files?: string[];
  feedback?: string;
  workspace_path?: string;
}

/** Review 响应 */
export interface AgentV3ReviewResponse {
  decision: string;
  applied_files: string[];
  skipped_files: string[];
  errors: string[];
}

/** Agent 状态 */
export interface AgentV3Status {
  agent_id: string;
  agent_type: string;
  phase: string;
  created_at: number;
}


// ============================================================
// Yuan Code (元界代码编辑器) IPC 方法
// ============================================================

export interface YuanFileNode {
  name: string;
  path: string;
  is_dir: boolean;
  children: YuanFileNode[] | null;
}
export interface YuanExecutionResult {
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
  stdout_truncated?: boolean;
  stderr_truncated?: boolean;
  timed_out?: boolean;
}

// ============================================================
// Yuan Code 补充类型定义
// ============================================================

export interface HighlightLine {
  content: string;
  token_type: string;
}
export interface HighlightResult {
  language: string;
  lines: HighlightLine[];
}
export interface CodeCompletionRequest {
  code: string;
  language: string;
  cursor_line: number;
  cursor_column: number;
  context_before?: string;
  context_after?: string;
  file_path?: string;
  /** 指定使用的 AI 模型 ID（来自统一模型管理）。未传则用第一个可用模型 */
  model_id?: number;
}
export interface CompletionItem {
  text: string;
  display_text?: string;
  description?: string;
  replace_range?: ReplaceRange;
}
export interface ReplaceRange {
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
}
export interface CodeCompletionResult {
  completions: CompletionItem[];
  model_used: string;
}
export interface CodeAnalysisRequest {
  code: string;
  language: string;
  analysis_type: string;
  /** 指定使用的 AI 模型 ID（来自统一模型管理）。未传则用第一个可用模型 */
  model_id?: number;
}
export interface CodeIssue {
  line?: number;
  column?: number;
  severity: string;
  message: string;
  rule?: string;
}
export interface CodeAnalysisResult {
  analysis_type: string;
  summary: string;
  issues: CodeIssue[];
  suggestions: string[];
}
export interface CodeSnippet {
  id: number;
  name: string;
  language: string;
  code: string;
  description?: string;
  tags?: string;
  created_at: number;
  updated_at: number;
}
export interface SaveSnippetRequest {
  name: string;
  language: string;
  code: string;
  description?: string;
  tags?: string[];
}
export interface UpdateSnippetRequest {
  name?: string;
  language?: string;
  code?: string;
  description?: string;
  tags?: string[];
}
export interface DiffLine {
  kind: string;
  content: string;
  old_line?: number;
  new_line?: number;
}
export interface DiffHunk {
  old_start: number;
  old_count: number;
  new_start: number;
  new_count: number;
  lines: DiffLine[];
}
export interface DiffResult {
  hunks: DiffHunk[];
  unified_diff: string;
}
export interface ReplaceFilesRequest {
  workspace_path: string;
  query: string;
  replacement: string;
  case_sensitive?: boolean;
  whole_word?: boolean;
  use_regex?: boolean;
  include_pattern?: string;
  exclude_pattern?: string;
}
export interface CopyMoveParams {
  source_path: string;
  dest_path: string;
  is_move: boolean;
  overwrite?: boolean;
}
export interface FileInfoResult {
  path: string;
  name: string;
  is_dir: boolean;
  size: number;
  modified_at?: number;
  created_at?: number;
  is_readonly: boolean;
  line_count?: number;
  language?: string;
}
export interface WorkspaceTab {
  file_path: string;
  cursor_line: number;
  cursor_column: number;
  scroll_top?: number;
}
export interface WorkspaceSession {
  id?: number;
  name: string;
  workspace_path: string;
  tabs: WorkspaceTab[];
  active_tab_index: number;
  created_at?: number;
  updated_at?: number;
}
export interface SaveWorkspaceRequest {
  name: string;
  workspace_path: string;
  tabs: WorkspaceTab[];
  active_tab_index: number;
}

// ============================================================
// Prompt 系统类型定义
// ============================================================

export interface PromptVariable {
  name: string;
  description: string;
  default_value?: string;
  required: boolean;
}
export interface PromptTemplateMeta {
  name: string;
  template_type: string;
  description: string;
  version: string;
  variables: PromptVariable[];
}
export interface RenderPromptRequest {
  template_type: string;
  variables: Record<string, string>;
}
export interface RenderPromptResult {
  content: string;
  estimated_tokens: number;
  template_name: string;
  template_version: string;
}
export interface AgentsMdFile {
  path: string;
  content: string;
  depth: number;
  scope_root: string;
}
export interface HierarchicalInstructions {
  sources: string[];
  assembled: string;
  total_bytes: number;
  truncated: boolean;
}
export interface AssembleSystemPromptRequest {
  project_root?: string;
  cwd?: string;
  user_instructions?: string;
  skill_names?: string[];
  max_bytes?: number;
  include_child_agent_instructions?: boolean;
}
export interface PromptPart {
  name: string;
  source: string;
  content: string;
  bytes: number;
}
export interface AssembleSystemPromptResult {
  system_prompt: string;
  parts: PromptPart[];
  estimated_tokens: number;
  total_bytes: number;
}

// ============================================================
// D1 v3.2 Task 3.4.1: 模型路由配置类型定义
// 规范：项目核心设计意图 §三/§八 — 编程 AI 必须走云端 API 模型
// ============================================================

/** 模型路由规则（对齐后端 ModelRoutingRule） */
export interface ModelRoutingRule {
  id: number;
  task_type: string;
  provider: string;
  model_name: string;
  temperature?: number;
  max_tokens?: number;
  is_enabled: boolean;
  /** 强制云端：true = 不允许将 provider 改为本地底层智能模型（守卫由后端 service 层强制） */
  is_cloud_only: boolean;
  priority: number;
  created_at: number;
  updated_at: number;
}

/** 新增/更新路由规则请求（对齐后端 UpsertRoutingRuleRequest） */
export interface UpsertRoutingRuleRequest {
  task_type: string;
  provider: string;
  model_name: string;
  temperature?: number;
  max_tokens?: number;
  is_enabled?: boolean;
}

/** 路由解析结果（对齐后端 ResolvedRouteResponse） */
export interface ResolvedRouteInfo {
  task_type: string;
  provider: string;
  model_name: string;
  temperature?: number;
  max_tokens?: number;
  /** 来源：rule（用户配置） | default（系统默认，rule 不存在或未启用时） */
  source: 'rule' | 'default';
}

/** TaskType 元信息（前端 UI 用，对齐后端 TaskTypeInfo） */
export interface TaskTypeInfo {
  task_type: string;
  display_name: string;
  description: string;
  default_provider: string;
  default_model: string;
  default_temperature: number;
}

// ============================================================
// D1 v3.2 Task 3.4.2: Yuan Code 底层智能监测类型定义
// 规范：项目核心设计意图 §二/§三/§8.2 — 非侵入式监测，可关闭
// ============================================================

/** 监测状态（对齐后端 YuanCodeMonitorStatus） */
export interface YuanCodeMonitorStatus {
  /** 底层智能全局开关是否启用（关闭时本钩子完全 no-op） */
  enabled: boolean;
  /** 近 7 天 Yuan Code 事件数 */
  recent_event_count: number;
  /** 近 7 天云端 API 调用次数 */
  recent_cloud_api_calls: number;
  /** 近 7 天 Agent 执行次数 */
  recent_agent_executions: number;
}

/** 监测数据汇总（对齐后端 YuanCodeMonitorSummary） */
export interface YuanCodeMonitorSummary {
  enabled: boolean;
  total_events: number;
  by_operation: Array<{ operation: string; count: number }>;
  daily_counts: Array<{ date: string; count: number }>;
}

// ============================================================
// D1 v3.2 Task 3.4.3 / 3.4.4: 协作会话管理类型定义
// 规范：Yjs 接口骨架 + 多人协作（PoC：内存态会话管理）
// ============================================================

/** 光标位置（对齐后端 CursorPosition / Monaco Editor Range） */
export interface CollabCursor {
  file_path: string;
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
}

/** 协作会话参与者（对齐后端 CollabUser） */
export interface CollabUserInfo {
  user_id: number;
  display_name: string;
  avatar_url?: string;
  joined_at: number;
  last_active_at: number;
  cursor?: CollabCursor | null;
}

/** 协作会话详情（对齐后端 CollabSession） */
export interface CollabSessionInfo {
  session_id: string;
  name: string;
  workspace_root: string;
  created_by: number;
  created_at: number;
  participants: CollabUserInfo[];
  is_closed: boolean;
}

/** 协作会话摘要（列表展示用，对齐后端 CollabSessionSummary） */
export interface CollabSessionSummaryInfo {
  session_id: string;
  name: string;
  workspace_root: string;
  created_by: number;
  created_at: number;
  participant_count: number;
  is_closed: boolean;
}

/** 创建协作会话请求 */
export interface CreateCollabSessionRequest {
  name: string;
  workspace_root: string;
}

// 阶段3 批次3c/5a-5e（S7）：yuanCode / yuanCompact / yuanGoal 三对象及其全部方法封装
// 已随 terminal.yuancode 插件化，统一迁至 plugins/boards/terminal/features/yuancode/ipc
// 的 yc 门面（cloudApi/modelRouting/monitor/collab/agentV3/io/mcp/prompt/agents/goal/
// compact 等各域）；agentDeploy/sandboxSave/skillExport 及多数 prompt 方法全仓零消费，
// 仅在 yc 门面登记 spec、未建门面方法。以下仅保留仍作类型来源的 interface 定义。

// ============================================================
// D1 v3.1 Task 3.2.1-3.2.12: MCP 生态 IPC 封装
// ============================================================

/** MCP 工具定义（对齐后端 crate::models::mcp::Tool） */
export interface McpTool {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
}

/** 已注册工具（含归属服务器信息） */
export interface McpRegisteredTool {
  server_id: string;
  server_name: string;
  tool: McpTool;
}

/** MCP 服务器状态 */
export interface McpServerStatus {
  id: string;
  name: string;
  connected: boolean;
  tools_count: number;
  resources_count: number;
  prompts_count: number;
  error?: string;
  lifecycle_state?: string;
}

/** MCP 服务器注册信息 */
export interface McpServerRegistration {
  id: string;
  name: string;
  command: string;
  args: string[];
  env?: Record<string, string>;
  enabled: boolean;
}

/** 工具调用结果（对齐后端 ToolCallResult） */
export interface McpToolCallResult {
  content: Array<{
    type: string;
    text?: string;
    data?: string;
    mimeType?: string;
  }>;
  isError?: boolean;
}

/** 工具调用历史条目（对齐后端 McpToolCallHistoryEntry） */
export interface McpToolCallHistoryEntry {
  server_id: string;
  server_name: string;
  tool_name: string;
  arguments?: unknown;
  success: boolean;
  latency_ms: number;
  error?: string;
  result_preview?: string;
  called_at: number;
}

/** 持久化的 MCP 服务器记录（对齐后端 McpServerRecord） */
export interface McpServerRecord {
  id: string;
  name: string;
  command: string;
  args: string;
  env: string;
  working_dir?: string;
  auto_connect: boolean;
  lifecycle_config: string;
  enabled: boolean;
  is_builtin: boolean;
  created_at: string;
  updated_at: string;
}

// BuiltinMcpInfo interface 与 BUILTIN_MCP_SERVERS 常量已随 terminal.yuancode 插件化，
// 迁至 plugins/boards/terminal/features/yuancode/ipc（唯一消费者 McpConfig.tsx 已改指新路径）。


