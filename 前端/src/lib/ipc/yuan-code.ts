// ipc/yuan-code.ts — yuan-code 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）
import { ipc } from './core';

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

export const yuanCode = {
  listFiles: (workspacePath: string, excludePatterns?: string[]) => ipc.invoke<YuanFileNode[]>('yuan_list_files', {
    workspace_path: workspacePath,
    exclude_patterns: excludePatterns
  }),
  // ============================================================
  // D1 v3.1 Task 3.5: 云端 API Key 管理（ModelSelector 后端）
  // ============================================================
  cloudApiList: () => ipc.invoke<CloudApiKeyInfo[]>('cloud_api_list'),
  cloudApiUpsert: (request: UpsertCloudApiKeyRequest) => ipc.invoke<CloudApiKeyInfo>('cloud_api_upsert', request),
  cloudApiSetEnabled: (id: number, isEnabled: boolean) => ipc.invoke<void>('cloud_api_set_enabled', {
    id,
    is_enabled: isEnabled
  }),
  cloudApiDelete: (id: number) => ipc.invoke<void>('cloud_api_delete', {
    id
  }),
  cloudApiProviders: () => ipc.invoke<CloudProviderInfo[]>('cloud_api_providers'),
  cloudApiTestConnection: (provider: string, modelName: string) => ipc.invoke<CloudConnectionTestResult>('cloud_api_test_connection', {
    provider,
    model_name: modelName
  }),
  // ============================================================
  // D1 v3.2 Task 3.4.1: 模型路由配置（按任务类型选模型，强制云端 API 用于编程）
  // ============================================================
  modelRoutingList: () => ipc.invoke<ModelRoutingRule[]>('model_routing_list'),
  modelRoutingUpsert: (request: UpsertRoutingRuleRequest) => ipc.invoke<ModelRoutingRule>('model_routing_upsert', {
    request
  }),
  modelRoutingSetEnabled: (id: number, isEnabled: boolean) => ipc.invoke<void>('model_routing_set_enabled', {
    id,
    is_enabled: isEnabled
  }),
  modelRoutingDelete: (id: number) => ipc.invoke<void>('model_routing_delete', {
    id
  }),
  modelRoutingResolve: (taskType: string) => ipc.invoke<ResolvedRouteInfo>('model_routing_resolve', {
    task_type: taskType
  }),
  modelRoutingTaskTypes: () => ipc.invoke<TaskTypeInfo[]>('model_routing_task_types'),
  // ============================================================
  // D1 v3.2 Task 3.4.2: Yuan Code 底层智能监测（非侵入式、可关闭）
  // ============================================================
  yuanCodeMonitorRecordEvent: (eventType: string, detail?: string) => ipc.invoke<void>('yuan_code_monitor_record_event', {
    event_type: eventType,
    detail
  }),
  yuanCodeMonitorStatus: () => ipc.invoke<YuanCodeMonitorStatus>('yuan_code_monitor_status'),
  yuanCodeMonitorSummary: () => ipc.invoke<YuanCodeMonitorSummary>('yuan_code_monitor_summary'),
  // ============================================================
  // D1 v3.2 Task 3.4.3 / 3.4.4: 协作会话管理（Yjs 接口骨架 + 多人协作）
  // ============================================================
  collabSessionCreate: (name: string, workspaceRoot: string) => ipc.invoke<CollabSessionInfo>('collab_session_create', {
    name,
    workspace_root: workspaceRoot
  }),
  collabSessionList: () => ipc.invoke<CollabSessionSummaryInfo[]>('collab_session_list'),
  collabSessionGet: (sessionId: string) => ipc.invoke<CollabSessionInfo | null>('collab_session_get', {
    session_id: sessionId
  }),
  collabSessionJoin: (sessionId: string) => ipc.invoke<CollabSessionInfo>('collab_session_join', {
    session_id: sessionId
  }),
  collabSessionLeave: (sessionId: string) => ipc.invoke<void>('collab_session_leave', {
    session_id: sessionId
  }),
  collabSessionClose: (sessionId: string) => ipc.invoke<void>('collab_session_close', {
    session_id: sessionId
  }),
  collabSessionUpdateCursor: (sessionId: string, cursor: CollabCursor | null) => ipc.invoke<CollabSessionInfo>('collab_session_update_cursor', {
    session_id: sessionId,
    cursor
  }),
  // ============================================================
  // D1 v3.1 Task 3.1: Agent 化（7 种类型 + Plan/Execute/Review）
  // ============================================================
  agentV3Types: () => ipc.invoke<AgentV3TypeInfo[]>('yuan_v3_agent_types'),
  agentV3Create: (agentType: string, taskPrompt: string) => ipc.invoke<AgentV3Created>('yuan_v3_agent_create', {
    request: {
      agent_type: agentType,
      task_prompt: taskPrompt
    }
  }),
  agentV3Plan: (agentId: string, taskPrompt: string) => ipc.invoke<AgentV3PlanResponse>('yuan_v3_agent_plan', {
    request: {
      agent_id: agentId,
      task_prompt: taskPrompt
    }
  }),
  agentV3Execute: (agentId: string) => ipc.invoke<AgentV3Result>('yuan_v3_agent_execute', {
    request: {
      agent_id: agentId
    }
  }),
  agentV3Review: (request: AgentV3ReviewRequest) => ipc.invoke<AgentV3ReviewResponse>('yuan_v3_agent_review', request),
  agentV3SafetyCheck: (agentId: string) => ipc.invoke<AgentV3SafetyCheck>('yuan_v3_agent_safety_check', {
    agent_id: agentId
  }),
  agentV3Status: (agentId: string) => ipc.invoke<AgentV3Status>('yuan_v3_agent_status', {
    agent_id: agentId
  }),
  agentV3List: () => ipc.invoke<AgentV3Status[]>('yuan_v3_agent_list'),
  agentV3Destroy: (agentId: string) => ipc.invoke<void>('yuan_v3_agent_destroy', {
    agent_id: agentId
  }),
  readFile: (path: string) => ipc.invoke<{
    name: string;
    content: string;
    language: string;
  }>('yuan_read_file', {
    path
  }),
  writeFile: (path: string, content: string) => ipc.invoke<void>('yuan_write_file', {
    path,
    content
  }),
  createItem: (parentPath: string, name: string, isDir: boolean) => ipc.invoke<void>('yuan_create_item', {
    parent_path: parentPath,
    name,
    is_dir: isDir
  }),
  execute: (code: string, language: string, timeoutSeconds?: number) => ipc.invoke<YuanExecutionResult>('yuan_execute', {
    code,
    language,
    timeout_seconds: timeoutSeconds ?? 30
  }),
  ioCancel: (executionId: string) => ipc.invoke<void>('yuan_io_cancel', {
    execution_id: executionId
  }),
  ioKill: (executionId: string) => ipc.invoke<void>('yuan_io_kill', {
    execution_id: executionId,
    signal: 'SIGKILL'
  }),
  ioStdin: (executionId: string, data: string) => ipc.invoke<void>('yuan_io_stdin', {
    execution_id: executionId,
    data
  }),
  agentDeploy: (config: Record<string, unknown>) => ipc.invoke<{
    success: boolean;
    message: string;
  }>('yuan_agent_deploy', config),
  sandboxSave: (config: Record<string, unknown>) => ipc.invoke<void>('yuan_sandbox_save', {
    config
  }),
  skillExport: (data: string) => ipc.invoke<void>('yuan_skill_export', {
    data
  }),
  settingsSave: (config: Record<string, unknown>) => ipc.invoke<void>('yuan_settings_save', config),
  // 文件操作 (已实现后端)
  deleteItem: (path: string, permanently?: boolean) => ipc.invoke<void>('yuan_delete_item', {
    path,
    permanently
  }),
  renameItem: (oldPath: string, newPath: string) => ipc.invoke<void>('yuan_rename_item', {
    old_path: oldPath,
    new_path: newPath
  }),
  // 代码分析与补全 (已实现后端)
  highlight: (code: string, language: string) => ipc.invoke<HighlightResult>('yuan_highlight', {
    code,
    language
  }),
  complete: (request: CodeCompletionRequest) => ipc.invoke<CodeCompletionResult>('yuan_complete', request),
  completeStream: (request: CodeCompletionRequest) => ipc.invoke<CodeCompletionResult>('yuan_complete_stream', request),
  analyze: (request: CodeAnalysisRequest) => ipc.invoke<CodeAnalysisResult>('yuan_analyze', request),
  // 代码片段 (已实现后端)
  saveSnippet: (request: SaveSnippetRequest) => ipc.invoke<CodeSnippet>('yuan_save_snippet', request),
  getSnippets: (language?: string) => ipc.invoke<CodeSnippet[]>('yuan_get_snippets', {
    language
  }),
  updateSnippet: (id: string, request: UpdateSnippetRequest) => ipc.invoke<CodeSnippet>('yuan_update_snippet', {
    id,
    ...request
  }),
  deleteSnippet: (id: string) => ipc.invoke<void>('yuan_delete_snippet', {
    id
  }),
  searchSnippets: (query: string, language?: string) => ipc.invoke<CodeSnippet[]>('yuan_search_snippets', {
    query,
    language
  }),
  // Diff 与文件操作 (已实现后端)
  computeDiff: (original: string, modified: string) => ipc.invoke<DiffResult>('yuan_compute_diff', {
    original,
    modified
  }),
  replaceFiles: (request: ReplaceFilesRequest) => ipc.invoke<number>('yuan_replace_files', request),
  copyMove: (request: CopyMoveParams) => ipc.invoke<void>('yuan_copy_move', request),
  getFileInfo: (path: string) => ipc.invoke<FileInfoResult>('yuan_get_file_info', {
    path
  }),
  // 工作区会话 (已实现后端)
  saveWorkspace: (request: SaveWorkspaceRequest) => ipc.invoke<WorkspaceSession>('yuan_save_workspace', request),
  loadWorkspace: (id: string) => ipc.invoke<WorkspaceSession>('yuan_load_workspace', {
    id
  }),
  listWorkspaces: () => ipc.invoke<WorkspaceSession[]>('yuan_list_workspaces'),
  deleteWorkspace: (id: string) => ipc.invoke<void>('yuan_delete_workspace', {
    id
  }),
  // === 提示词系统 (yuan_prompt_commands) ===
  listTemplates: () => ipc.invoke<PromptTemplateMeta[]>('yuan_prompt_list_templates'),
  getTemplate: (templateType: string) => ipc.invoke<PromptTemplateMeta | null>('yuan_prompt_get_template', {
    template_type: templateType
  }),
  renderTemplate: (request: RenderPromptRequest) => ipc.invoke<RenderPromptResult>('yuan_prompt_render', {
    request
  }),
  setCustomTemplate: (templateType: string, content: string) => ipc.invoke<void>('yuan_prompt_set_custom_template', {
    template_type: templateType,
    content
  }),
  removeCustomTemplate: (templateType: string) => ipc.invoke<void>('yuan_prompt_remove_custom_template', {
    template_type: templateType
  }),
  setVariableDefault: (name: string, value: string) => ipc.invoke<void>('yuan_prompt_set_variable_default', {
    name,
    value
  }),
  discoverAgentsMd: (cwd?: string, projectRoot?: string) => ipc.invoke<AgentsMdFile[]>('yuan_agents_discover', {
    cwd,
    project_root: projectRoot
  }),
  getInstructionSources: (cwd?: string, projectRoot?: string) => ipc.invoke<string[]>('yuan_agents_sources', {
    cwd,
    project_root: projectRoot
  }),
  assembleInstructions: (agentsFiles: AgentsMdFile[], userInstructions?: string, maxBytes?: number) => ipc.invoke<HierarchicalInstructions>('yuan_agents_assemble', {
    agents_files: agentsFiles,
    user_instructions: userInstructions,
    max_bytes: maxBytes
  }),
  setMaxBytes: (maxBytes: number) => ipc.invoke<void>('yuan_agents_set_max_bytes', {
    max_bytes: maxBytes
  }),
  getMaxBytes: () => ipc.invoke<number>('yuan_agents_get_max_bytes'),
  assembleSystemPrompt: (request: AssembleSystemPromptRequest) => ipc.invoke<AssembleSystemPromptResult>('yuan_prompt_assemble', {
    request
  })
};

export const yuanCompact = {
  getConfig: () => ipc.invoke<any>('yuan_compact_config_get'),
  updateConfig: (config: Record<string, unknown>) => ipc.invoke<void>('yuan_compact_config_update', {
    config
  }),
  estimate: (messages: {
    id: string;
    role: string;
    content: string;
  }[]) => ipc.invoke<any>('yuan_compact_estimate', {
    messages
  }),
  check: (messages: {
    id: string;
    role: string;
    content: string;
  }[]) => ipc.invoke<any>('yuan_compact_check', {
    messages
  }),
  execute: (request: {
    session_id: string;
    messages: {
      id: string;
      role: string;
      content: string;
    }[];
    config?: Record<string, unknown>;
    reason?: string;
  }) => ipc.invoke<any>('yuan_compact_execute', {
    request
  }),
  getSession: (sessionId: string) => ipc.invoke<any>('yuan_compact_session', {
    session_id: sessionId
  }),
  reset: (sessionId: string) => ipc.invoke<void>('yuan_compact_reset', {
    session_id: sessionId
  }),
  stats: (messages: {
    id: string;
    role: string;
    content: string;
  }[]) => ipc.invoke<any>('yuan_compact_stats', {
    messages
  }),
  budgetVisualization: (messages: {
    id: string;
    role: string;
    content: string;
  }[]) => ipc.invoke<any>('yuan_compact_budget_visualization', {
    messages
  }),
  auto: (sessionId: string, messages: {
    id: string;
    role: string;
    content: string;
  }[]) => ipc.invoke<any>('yuan_compact_auto', {
    session_id: sessionId,
    messages
  }),
  manual: (sessionId: string, messages: {
    id: string;
    role: string;
    content: string;
  }[], config?: Record<string, unknown>) => ipc.invoke<any>('yuan_compact_manual', {
    session_id: sessionId,
    messages,
    config
  }),
  force: (sessionId: string, messages: {
    id: string;
    role: string;
    content: string;
  }[], config?: Record<string, unknown>) => ipc.invoke<any>('yuan_compact_force', {
    session_id: sessionId,
    messages,
    config
  })
};

export const yuanGoal = {
  create: (request: {
    session_id: string;
    title: string;
    description?: string;
    total_token_budget?: number;
    priority?: number;
  }) => ipc.invoke<any>('yuan_goal_create', {
    request
  }),
  get: (goalId: number) => ipc.invoke<any>('yuan_goal_get', {
    goal_id: goalId
  }),
  list: (sessionId: string) => ipc.invoke<any[]>('yuan_goal_list', {
    session_id: sessionId
  }),
  update: (request: {
    id: number;
    title?: string;
    description?: string;
    total_token_budget?: number;
    priority?: number;
  }) => ipc.invoke<any>('yuan_goal_update', {
    request
  }),
  start: (goalId: number) => ipc.invoke<any>('yuan_goal_start', {
    goal_id: goalId
  }),
  pause: (goalId: number) => ipc.invoke<any>('yuan_goal_pause', {
    goal_id: goalId
  }),
  complete: (goalId: number, resultJson?: string) => ipc.invoke<any>('yuan_goal_complete', {
    goal_id: goalId,
    result_json: resultJson
  }),
  abort: (goalId: number, reason: string) => ipc.invoke<any>('yuan_goal_abort', {
    goal_id: goalId,
    reason
  }),
  delete: (goalId: number) => ipc.invoke<void>('yuan_goal_delete', {
    goal_id: goalId
  }),
  consumeTokens: (goalId: number, tokens: number, modelId?: string) => ipc.invoke<any>('yuan_goal_consume_tokens', {
    goal_id: goalId,
    tokens,
    model_id: modelId
  }),
  updateProgress: (goalId: number, progressPct: number) => ipc.invoke<any>('yuan_goal_update_progress', {
    goal_id: goalId,
    progress_pct: progressPct
  }),
  saveCheckpoint: (goalId: number, cp: {
    description: string;
    snapshot_json?: string;
    restart_instructions?: string;
  }) => ipc.invoke<any>('yuan_goal_save_checkpoint', {
    goal_id: goalId,
    checkpoint: cp
  }),
  buildContinuation: (goalId: number) => ipc.invoke<any>('yuan_goal_build_continuation', {
    goal_id: goalId
  }),
  checkpointsCount: (goalId: number) => ipc.invoke<number>('yuan_goal_checkpoints_count', {
    goal_id: goalId
  })
};

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

/** 8 个内置 MCP 服务器元信息（前端展示用） */
export interface BuiltinMcpInfo {
  name: string;
  description: string;
  /** 配置字段 schema（前端动态渲染参数输入） */
  config_fields: Array<{
    key: string;
    label: string;
    type: 'password' | 'text' | 'path';
    required: boolean;
    placeholder?: string;
  }>;
}

/** 内置 MCP 服务器清单（与后端 builtin/ 模块对应） */
export const BUILTIN_MCP_SERVERS: BuiltinMcpInfo[] = [
  {
    name: 'github',
    description: 'GitHub — 操作仓库 PR/Issue/分支',
    config_fields: [{ key: 'token', label: 'API Token', type: 'password', required: true, placeholder: 'ghp_xxx' }]
  },
  {
    name: 'gitlab',
    description: 'GitLab — 操作项目/MR/Issue',
    config_fields: [
      { key: 'token', label: 'API Token', type: 'password', required: true, placeholder: 'glpat-xxx' },
      { key: 'api_url', label: 'API URL', type: 'text', required: false, placeholder: 'https://gitlab.com/api/v4' }
    ]
  },
  {
    name: 'sqlite',
    description: 'SQLite — 查询本地数据库',
    config_fields: [{ key: 'db_path', label: '数据库路径', type: 'path', required: true, placeholder: '/path/to/db.sqlite' }]
  },
  {
    name: 'postgres',
    description: 'Postgres — 查询 Postgres 数据库',
    config_fields: [{ key: 'connection_string', label: '连接串', type: 'text', required: true, placeholder: 'postgresql://user:pass@host/db' }]
  },
  {
    name: 'slack',
    description: 'Slack — 发送/读取消息',
    config_fields: [{ key: 'token', label: 'Bot Token', type: 'password', required: true, placeholder: 'xoxb-xxx' }]
  },
  {
    name: 'jira',
    description: 'Jira — 操作 Issue',
    config_fields: [
      { key: 'base_url', label: 'Base URL', type: 'text', required: true, placeholder: 'https://your-domain.atlassian.net' },
      { key: 'email', label: 'Email', type: 'text', required: true },
      { key: 'api_token', label: 'API Token', type: 'password', required: true }
    ]
  },
  {
    name: 'memory',
    description: 'Memory — 进程内键值记忆存储',
    config_fields: [{ key: 'namespace', label: '命名空间', type: 'text', required: false, placeholder: 'default' }]
  },
  {
    name: 'websearch',
    description: 'WebSearch — 网页搜索与抓取',
    config_fields: [
      { key: 'engine', label: '搜索引擎', type: 'text', required: false, placeholder: 'duckduckgo | serper' },
      { key: 'api_key', label: 'API Key（Serper）', type: 'password', required: false }
    ]
  }
];

export const yuanMcp = {
  // ===== 内置 MCP 服务器 =====
  listBuiltinServers: () => ipc.invoke<string[]>('yuan_mcp_list_builtin_servers'),
  listBuiltinTools: () => ipc.invoke<McpRegisteredTool[]>('yuan_mcp_list_builtin_tools'),
  callBuiltinTool: (serverName: string, toolName: string, args?: Record<string, unknown>) =>
    ipc.invoke<McpToolCallResult>('yuan_mcp_call_builtin_tool', {
      server_name: serverName,
      tool_name: toolName,
      arguments: args ?? null
    }),
  configureBuiltin: (serverName: string, config: Record<string, unknown>) =>
    ipc.invoke<void>('yuan_mcp_configure_builtin', {
      server_name: serverName,
      config
    }),

  // ===== 工具调用历史 =====
  listCallHistory: () => ipc.invoke<McpToolCallHistoryEntry[]>('yuan_mcp_list_call_history'),

  // ===== 外部 MCP 服务器（已有命令的封装，供 McpConfig 使用） =====
  listServers: () => ipc.invoke<McpServerStatus[]>('yuan_mcp_list_servers'),
  listPersistedServers: () => ipc.invoke<McpServerRecord[]>('yuan_mcp_list_persisted_servers'),
  saveServerConfig: (server: McpServerRegistration, autoConnect: boolean) =>
    ipc.invoke<void>('yuan_mcp_save_server_config', { server, auto_connect: autoConnect }),
  deleteServerConfig: (serverId: string) =>
    ipc.invoke<void>('yuan_mcp_delete_server_config', { server_id: serverId }),
  setServerEnabled: (serverId: string, enabled: boolean) =>
    ipc.invoke<void>('yuan_mcp_set_server_enabled', { server_id: serverId, enabled }),
  connectServer: (serverId: string) =>
    ipc.invoke<McpServerStatus>('yuan_mcp_connect_server', { server_id: serverId }),
  disconnectServer: (serverId: string) =>
    ipc.invoke<void>('yuan_mcp_disconnect_server', { server_id: serverId })
};

