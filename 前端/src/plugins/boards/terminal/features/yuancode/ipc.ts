// plugins/boards/terminal/features/yuancode/ipc.ts — terminal.yuancode L2 IPC 客户端
// （短码 yc，阶段3 批次3c）。契约：统一经内核 dispatcher，逻辑名 `yc:plugin:<旧命令名>`
// （06_Rust代码契约 §8.1）。
// 覆盖 54 条：yuancode_commands 27 + yuan_inline_commands 3 + git_commands 13 +
// lsp_commands 5 + browser_commands 6（Agent/MCP/目标/沙箱等其余 yuan_* 域归 3d/3e，
// 仍由 lib/ipc/yuan-code.ts 旧封装承载，S7 后随各段迁移收编）。
// 便捷封装仅覆盖实际被消费的方法（「有消费才迁」口径）；零消费方法仅登记 spec
//（43-A「有 alias 无前端方法」口径），后续消费时在此补录便捷封装。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const YC_IPC_METHODS = {
  // ===== yuancode_commands（27）=====
  listFiles: { cmd: 'yuan_list_files' },
  readFile: { cmd: 'yuan_read_file' },
  writeFile: { cmd: 'yuan_write_file' },
  createItem: { cmd: 'yuan_create_item' },
  deleteItem: { cmd: 'yuan_delete_item' },
  renameItem: { cmd: 'yuan_rename_item' },
  highlight: { cmd: 'yuan_highlight' },
  execute: { cmd: 'yuan_execute' },
  complete: { cmd: 'yuan_complete' },
  completeStream: { cmd: 'yuan_complete_stream' },
  analyze: { cmd: 'yuan_analyze' },
  saveSnippet: { cmd: 'yuan_save_snippet' },
  getSnippets: { cmd: 'yuan_get_snippets' },
  updateSnippet: { cmd: 'yuan_update_snippet' },
  deleteSnippet: { cmd: 'yuan_delete_snippet' },
  searchSnippets: { cmd: 'yuan_search_snippets' },
  computeDiff: { cmd: 'yuan_compute_diff' },
  searchFiles: { cmd: 'yuan_search_files' },
  replaceFiles: { cmd: 'yuan_replace_files' },
  copyMove: { cmd: 'yuan_copy_move' },
  getFileInfo: { cmd: 'yuan_get_file_info' },
  saveWorkspace: { cmd: 'yuan_save_workspace' },
  loadWorkspace: { cmd: 'yuan_load_workspace' },
  listWorkspaces: { cmd: 'yuan_list_workspaces' },
  deleteWorkspace: { cmd: 'yuan_delete_workspace' },
  formatCode: { cmd: 'yuan_format_code' },
  settingsSave: { cmd: 'yuan_settings_save' },
  // ===== yuan_inline_commands（3）=====
  inlineComplete: { cmd: 'yuan_inline_complete' },
  inlineAvailable: { cmd: 'yuan_inline_available' },
  inlineEdit: { cmd: 'yuan_inline_edit' },
  // ===== git_commands（13）=====
  gitStatus: { cmd: 'git_status' },
  gitDiffFile: { cmd: 'git_diff_file' },
  gitDiffUnstaged: { cmd: 'git_diff_unstaged' },
  gitStageFile: { cmd: 'git_stage_file' },
  gitStageAll: { cmd: 'git_stage_all' },
  gitUnstageFile: { cmd: 'git_unstage_file' },
  gitCommit: { cmd: 'git_commit' },
  gitPush: { cmd: 'git_push' },
  gitPull: { cmd: 'git_pull' },
  gitBranches: { cmd: 'git_branches' },
  gitCheckout: { cmd: 'git_checkout' },
  gitLog: { cmd: 'git_log' },
  gitInit: { cmd: 'git_init' },
  // ===== lsp_commands（5）=====
  lspCompletions: { cmd: 'lsp_completions' },
  lspHover: { cmd: 'lsp_hover' },
  lspDefinition: { cmd: 'lsp_definition' },
  lspDiagnostics: { cmd: 'lsp_diagnostics' },
  lspDetectLanguage: { cmd: 'lsp_detect_language' },
  // ===== browser_commands（6）=====
  browserOpenWindow: { cmd: 'browser_open_window' },
  browserCreateView: { cmd: 'browser_create_view' },
  browserNavigateView: { cmd: 'browser_navigate_view' },
  browserResizeView: { cmd: 'browser_resize_view' },
  browserCloseView: { cmd: 'browser_close_view' },
  browserCleanup: { cmd: 'browser_cleanup' },
  // ===== cloud_api_commands（6，批次5a-5e 迁入）=====
  cloudApiList: { cmd: 'cloud_api_list' },
  cloudApiUpsert: { cmd: 'cloud_api_upsert' },
  cloudApiSetEnabled: { cmd: 'cloud_api_set_enabled' },
  cloudApiDelete: { cmd: 'cloud_api_delete' },
  cloudApiProviders: { cmd: 'cloud_api_providers' },
  cloudApiTestConnection: { cmd: 'cloud_api_test_connection' },
  // ===== model_routing_commands（6，批次5a-5e 迁入）=====
  modelRoutingList: { cmd: 'model_routing_list' },
  modelRoutingUpsert: { cmd: 'model_routing_upsert' },
  modelRoutingSetEnabled: { cmd: 'model_routing_set_enabled' },
  modelRoutingDelete: { cmd: 'model_routing_delete' },
  modelRoutingResolve: { cmd: 'model_routing_resolve' },
  modelRoutingTaskTypes: { cmd: 'model_routing_task_types' },
  // ===== yuan_code_monitor_commands（3，批次5a-5e 迁入；零消费仅登记 spec）=====
  yuanCodeMonitorRecordEvent: { cmd: 'yuan_code_monitor_record_event' },
  yuanCodeMonitorStatus: { cmd: 'yuan_code_monitor_status' },
  yuanCodeMonitorSummary: { cmd: 'yuan_code_monitor_summary' },
  // ===== yuan_v3_agent_commands（9，批次5a-5e 迁入）=====
  agentV3Types: { cmd: 'yuan_v3_agent_types' },
  agentV3Create: { cmd: 'yuan_v3_agent_create' },
  agentV3Plan: { cmd: 'yuan_v3_agent_plan' },
  agentV3Execute: { cmd: 'yuan_v3_agent_execute' },
  agentV3Review: { cmd: 'yuan_v3_agent_review' },
  agentV3SafetyCheck: { cmd: 'yuan_v3_agent_safety_check' },
  agentV3Status: { cmd: 'yuan_v3_agent_status' },
  agentV3List: { cmd: 'yuan_v3_agent_list' },
  agentV3Destroy: { cmd: 'yuan_v3_agent_destroy' },
  // ===== collab_session_commands（7，批次5a-5e 迁入）=====
  collabSessionCreate: { cmd: 'collab_session_create' },
  collabSessionList: { cmd: 'collab_session_list' },
  collabSessionGet: { cmd: 'collab_session_get' },
  collabSessionJoin: { cmd: 'collab_session_join' },
  collabSessionLeave: { cmd: 'collab_session_leave' },
  collabSessionClose: { cmd: 'collab_session_close' },
  collabSessionUpdateCursor: { cmd: 'collab_session_update_cursor' },
  // ===== yuan_goal_commands（14，批次5a-5e 迁入）=====
  goalCreate: { cmd: 'yuan_goal_create' },
  goalGet: { cmd: 'yuan_goal_get' },
  goalList: { cmd: 'yuan_goal_list' },
  goalUpdate: { cmd: 'yuan_goal_update' },
  goalStart: { cmd: 'yuan_goal_start' },
  goalPause: { cmd: 'yuan_goal_pause' },
  goalComplete: { cmd: 'yuan_goal_complete' },
  goalAbort: { cmd: 'yuan_goal_abort' },
  goalDelete: { cmd: 'yuan_goal_delete' },
  goalConsumeTokens: { cmd: 'yuan_goal_consume_tokens' },
  goalUpdateProgress: { cmd: 'yuan_goal_update_progress' },
  goalSaveCheckpoint: { cmd: 'yuan_goal_save_checkpoint' },
  goalBuildContinuation: { cmd: 'yuan_goal_build_continuation' },
  goalCheckpointsCount: { cmd: 'yuan_goal_checkpoints_count' },
  // ===== yuan_compact_commands（12，批次5a-5e 迁入）=====
  compactConfigGet: { cmd: 'yuan_compact_config_get' },
  compactConfigUpdate: { cmd: 'yuan_compact_config_update' },
  compactEstimate: { cmd: 'yuan_compact_estimate' },
  compactCheck: { cmd: 'yuan_compact_check' },
  compactExecute: { cmd: 'yuan_compact_execute' },
  compactSession: { cmd: 'yuan_compact_session' },
  compactReset: { cmd: 'yuan_compact_reset' },
  compactStats: { cmd: 'yuan_compact_stats' },
  compactBudgetVisualization: { cmd: 'yuan_compact_budget_visualization' },
  compactAuto: { cmd: 'yuan_compact_auto' },
  compactManual: { cmd: 'yuan_compact_manual' },
  compactForce: { cmd: 'yuan_compact_force' },
  // ===== yuan_io_commands（3，批次5a-5e 迁入）=====
  ioCancel: { cmd: 'yuan_io_cancel' },
  ioKill: { cmd: 'yuan_io_kill' },
  ioStdin: { cmd: 'yuan_io_stdin' },
  // ===== agent 部署 / 沙箱 / 技能（3，零消费仅登记 spec）=====
  agentDeploy: { cmd: 'yuan_agent_deploy' },
  sandboxSave: { cmd: 'yuan_sandbox_save' },
  skillExport: { cmd: 'yuan_skill_export' },
  // ===== yuan_prompt_commands（12，批次5a-5e 迁入；部分零消费仅登记 spec）=====
  promptListTemplates: { cmd: 'yuan_prompt_list_templates' },
  promptGetTemplate: { cmd: 'yuan_prompt_get_template' },
  promptRender: { cmd: 'yuan_prompt_render' },
  promptSetCustomTemplate: { cmd: 'yuan_prompt_set_custom_template' },
  promptRemoveCustomTemplate: { cmd: 'yuan_prompt_remove_custom_template' },
  promptSetVariableDefault: { cmd: 'yuan_prompt_set_variable_default' },
  agentsDiscover: { cmd: 'yuan_agents_discover' },
  agentsSources: { cmd: 'yuan_agents_sources' },
  agentsAssemble: { cmd: 'yuan_agents_assemble' },
  agentsSetMaxBytes: { cmd: 'yuan_agents_set_max_bytes' },
  agentsGetMaxBytes: { cmd: 'yuan_agents_get_max_bytes' },
  promptAssemble: { cmd: 'yuan_prompt_assemble' },
  // ===== yuan_mcp_commands（12，批次5a-5e 迁入）=====
  mcpListBuiltinServers: { cmd: 'yuan_mcp_list_builtin_servers' },
  mcpListBuiltinTools: { cmd: 'yuan_mcp_list_builtin_tools' },
  mcpCallBuiltinTool: { cmd: 'yuan_mcp_call_builtin_tool' },
  mcpConfigureBuiltin: { cmd: 'yuan_mcp_configure_builtin' },
  mcpListCallHistory: { cmd: 'yuan_mcp_list_call_history' },
  mcpListServers: { cmd: 'yuan_mcp_list_servers' },
  mcpListPersistedServers: { cmd: 'yuan_mcp_list_persisted_servers' },
  mcpSaveServerConfig: { cmd: 'yuan_mcp_save_server_config' },
  mcpDeleteServerConfig: { cmd: 'yuan_mcp_delete_server_config' },
  mcpSetServerEnabled: { cmd: 'yuan_mcp_set_server_enabled' },
  mcpConnectServer: { cmd: 'yuan_mcp_connect_server' },
  mcpDisconnectServer: { cmd: 'yuan_mcp_disconnect_server' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherYc = defineIpcNamespace('yc', YC_IPC_METHODS);
type YcMethod = keyof typeof YC_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeYc<T = any>(method: YcMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherYc[method](args) as Promise<ApiResponse<T>>;
}

/** camelCase 键 → snake_case 键（深层，仅键名转换，不改值）。
 *  settings 域既有消费方以 camelCase 载荷保存（mcpServers 等），后端 YuanCodeSettings
 *  字段为 snake_case 且全部 serde default——不转换时列表字段静默丢失，故在此收口。 */
function toSnakeDeep(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(toSnakeDeep);
  if (value !== null && typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(value as Record<string, unknown>)) {
      out[k.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`)] = toSnakeDeep(v);
    }
    return out;
  }
  return value;
}

/**
 * terminal.yuancode 便捷门面。
 * 请求体命令（后端 `request: T` / `settings: T` 签名）由门面负责包装；
 * 平铺参数命令（git 请求体除外——git_* 均为 request 包装；lsp/browser 为平铺）
 * 直接透传顶层键。返回 ApiResponse<T>（与旧 ipc.invoke 一致）。
 */
export const yc = {
  // ===== 编辑器核心（yuancode_commands，被消费 11 条）=====
  listFiles: <T = any>(request: { workspace_path: string; depth?: number; exclude_patterns?: string[] }) =>
    invokeYc<T>('listFiles', { request }),
  readFile: <T = any>(request: { path: string; offset_line?: number; limit_lines?: number }) =>
    invokeYc<T>('readFile', { request }),
  writeFile: <T = any>(request: { path: string; content: string; create_dirs?: boolean }) =>
    invokeYc<T>('writeFile', { request }),
  createItem: <T = any>(request: { parent_path: string; name: string; is_dir: boolean }) =>
    invokeYc<T>('createItem', { request }),
  execute: <T = any>(request: {
    code: string;
    language: string;
    working_dir?: string;
    timeout_seconds?: number;
  }) => invokeYc<T>('execute', { request }),
  formatCode: <T = any>(request: { file_path: string; language: string; content: string }) =>
    invokeYc<T>('formatCode', { request }),
  analyze: <T = any>(request: { code: string; language: string; analysis_type: string; model_id?: number }) =>
    invokeYc<T>('analyze', { request }),
  completeStream: <T = any>(request: {
    code?: string;
    language?: string;
    cursor_line?: number;
    cursor_column?: number;
    context_before?: string;
    context_after?: string;
    file_path?: string;
    model_id?: number;
  }) => invokeYc<T>('completeStream', { request }),
  searchFiles: <T = any>(request: {
    workspace_path: string;
    query: string;
    case_sensitive?: boolean;
    whole_word?: boolean;
    use_regex?: boolean;
    include_pattern?: string;
    exclude_pattern?: string;
    max_results?: number;
  }) => invokeYc<T>('searchFiles', { request }),
  replaceFiles: <T = any>(request: {
    workspace_path: string;
    query: string;
    replacement: string;
    case_sensitive?: boolean;
    whole_word?: boolean;
    use_regex?: boolean;
    include_pattern?: string;
    exclude_pattern?: string;
  }) => invokeYc<T>('replaceFiles', { request }),
  settingsSave: (config: Record<string, unknown>) =>
    invokeYc<void>('settingsSave', { settings: toSnakeDeep(config) }),

  // ===== 行内补全 / 内联编辑（yuan_inline_commands，被消费 2 条）=====
  inlineComplete: <T = any>(request: {
    file_path: string;
    language: string;
    position: { line: number; character: number };
    context_before: string;
    context_after: string;
    model_id?: number;
  }) => invokeYc<T>('inlineComplete', { request }),
  inlineEdit: <T = any>(request: {
    selected_code: string;
    instruction: string;
    language: string;
    file_path: string;
    model_id?: number;
  }) => invokeYc<T>('inlineEdit', { request }),

  // ===== Git 面板（git_commands，被消费 12 条）=====
  gitStatus: <T = any>(request: { workspace_path: string }) => invokeYc<T>('gitStatus', { request }),
  gitBranches: <T = any>(request: { workspace_path: string }) => invokeYc<T>('gitBranches', { request }),
  gitLog: <T = any>(request: { workspace_path: string; count: number }) => invokeYc<T>('gitLog', { request }),
  gitStageFile: (request: { workspace_path: string; file_path: string }) =>
    invokeYc('gitStageFile', { request }),
  gitUnstageFile: (request: { workspace_path: string; file_path: string }) =>
    invokeYc('gitUnstageFile', { request }),
  gitStageAll: (request: { workspace_path: string }) => invokeYc('gitStageAll', { request }),
  gitCommit: <T = any>(request: { workspace_path: string; message: string }) =>
    invokeYc<T>('gitCommit', { request }),
  gitPush: (request: { workspace_path: string; remote: string; branch: string }) =>
    invokeYc('gitPush', { request }),
  gitPull: (request: { workspace_path: string; remote: string; branch: string }) =>
    invokeYc('gitPull', { request }),
  gitCheckout: (request: { workspace_path: string; branch: string }) =>
    invokeYc('gitCheckout', { request }),
  gitInit: (request: { workspace_path: string }) => invokeYc('gitInit', { request }),
  gitDiffFile: <T = any>(request: { workspace_path: string; file_path: string }) =>
    invokeYc<T>('gitDiffFile', { request }),

  // ===== LSP（lsp_commands，5 条全部被消费，lib/ipc-lsp.ts 承载）=====
  lspCompletions: <T = any>(args: { filePath: string; line: number; character: number; workspaceRoot: string }) =>
    invokeYc<T>('lspCompletions', args),
  lspHover: <T = any>(args: { filePath: string; line: number; character: number; workspaceRoot: string }) =>
    invokeYc<T>('lspHover', args),
  lspDefinition: <T = any>(args: { filePath: string; line: number; character: number; workspaceRoot: string }) =>
    invokeYc<T>('lspDefinition', args),
  lspDiagnostics: <T = any>(args: { filePath: string; workspaceRoot: string }) =>
    invokeYc<T>('lspDiagnostics', args),
  lspDetectLanguage: <T = any>(args: { filePath: string }) => invokeYc<T>('lspDetectLanguage', args),

  // ===== 内嵌浏览器（browser_commands，6 条全部被消费，pages/Search.tsx 承载）=====
  browserOpenWindow: (args: { url: string }) => invokeYc('browserOpenWindow', args),
  browserCreateView: <T = any>(args: { url: string; x: number; y: number; width: number; height: number }) =>
    invokeYc<T>('browserCreateView', args),
  browserNavigateView: (args: { label: string; url: string }) =>
    invokeYc('browserNavigateView', args),
  browserResizeView: (args: { label: string; x: number; y: number; width: number; height: number }) =>
    invokeYc('browserResizeView', args),
  browserCloseView: (args: { label: string }) => invokeYc('browserCloseView', args),
  browserCleanup: <T = any>(args?: Record<string, unknown>) => invokeYc<T>('browserCleanup', args),

  // ===== 云端 API Key 管理（cloud_api_commands，ModelSelector 消费）=====
  // cloudApiUpsert 沿用旧封装的平铺透传（后端签名非 request 包装）。
  cloudApiList: <T = any>() => invokeYc<T>('cloudApiList'),
  cloudApiUpsert: <T = any>(request: object) => invokeYc<T>('cloudApiUpsert', request as Record<string, unknown>),
  cloudApiSetEnabled: (id: number, isEnabled: boolean) =>
    invokeYc<void>('cloudApiSetEnabled', { id, is_enabled: isEnabled }),
  cloudApiDelete: (id: number) => invokeYc<void>('cloudApiDelete', { id }),
  cloudApiProviders: <T = any>() => invokeYc<T>('cloudApiProviders'),
  cloudApiTestConnection: <T = any>(provider: string, modelName: string) =>
    invokeYc<T>('cloudApiTestConnection', { provider, model_name: modelName }),

  // ===== 模型路由配置（model_routing_commands，ModelRoutingConfig 消费）=====
  modelRoutingList: <T = any>() => invokeYc<T>('modelRoutingList'),
  modelRoutingUpsert: <T = any>(request: object) =>
    invokeYc<T>('modelRoutingUpsert', { request }),
  modelRoutingSetEnabled: (id: number, isEnabled: boolean) =>
    invokeYc<void>('modelRoutingSetEnabled', { id, is_enabled: isEnabled }),
  modelRoutingDelete: (id: number) => invokeYc<void>('modelRoutingDelete', { id }),
  modelRoutingResolve: <T = any>(taskType: string) =>
    invokeYc<T>('modelRoutingResolve', { task_type: taskType }),
  modelRoutingTaskTypes: <T = any>() => invokeYc<T>('modelRoutingTaskTypes'),

  // ===== 底层智能监测（yuan_code_monitor_commands，零消费仅登记 spec，暂建门面以备后用）=====
  yuanCodeMonitorRecordEvent: (eventType: string, detail?: string) =>
    invokeYc<void>('yuanCodeMonitorRecordEvent', { event_type: eventType, detail }),
  yuanCodeMonitorStatus: <T = any>() => invokeYc<T>('yuanCodeMonitorStatus'),
  yuanCodeMonitorSummary: <T = any>() => invokeYc<T>('yuanCodeMonitorSummary'),

  // ===== Agent 化 v3（yuan_v3_agent_commands，AgentPlanView/AgentReview 消费）=====
  // agentV3Review 沿用旧封装的平铺透传（后端签名非 request 包装）。
  agentV3Types: <T = any>() => invokeYc<T>('agentV3Types'),
  agentV3Create: <T = any>(agentType: string, taskPrompt: string) =>
    invokeYc<T>('agentV3Create', { request: { agent_type: agentType, task_prompt: taskPrompt } }),
  agentV3Plan: <T = any>(agentId: string, taskPrompt: string) =>
    invokeYc<T>('agentV3Plan', { request: { agent_id: agentId, task_prompt: taskPrompt } }),
  agentV3Execute: <T = any>(agentId: string) =>
    invokeYc<T>('agentV3Execute', { request: { agent_id: agentId } }),
  agentV3Review: <T = any>(request: object) => invokeYc<T>('agentV3Review', request as Record<string, unknown>),
  agentV3SafetyCheck: <T = any>(agentId: string) =>
    invokeYc<T>('agentV3SafetyCheck', { agent_id: agentId }),
  agentV3Status: <T = any>(agentId: string) => invokeYc<T>('agentV3Status', { agent_id: agentId }),
  agentV3List: <T = any>() => invokeYc<T>('agentV3List'),
  agentV3Destroy: (agentId: string) => invokeYc<void>('agentV3Destroy', { agent_id: agentId }),

  // ===== 协作会话（collab_session_commands，CollabSession 消费）=====
  collabSessionCreate: <T = any>(name: string, workspaceRoot: string) =>
    invokeYc<T>('collabSessionCreate', { name, workspace_root: workspaceRoot }),
  collabSessionList: <T = any>() => invokeYc<T>('collabSessionList'),
  collabSessionGet: <T = any>(sessionId: string) =>
    invokeYc<T>('collabSessionGet', { session_id: sessionId }),
  collabSessionJoin: <T = any>(sessionId: string) =>
    invokeYc<T>('collabSessionJoin', { session_id: sessionId }),
  collabSessionLeave: (sessionId: string) =>
    invokeYc<void>('collabSessionLeave', { session_id: sessionId }),
  collabSessionClose: (sessionId: string) =>
    invokeYc<void>('collabSessionClose', { session_id: sessionId }),
  collabSessionUpdateCursor: <T = any>(sessionId: string, cursor: unknown) =>
    invokeYc<T>('collabSessionUpdateCursor', { session_id: sessionId, cursor }),

  // ===== 目标管理（yuan_goal_commands，GoalManager 消费）=====
  // create/update 用 request 包装（后端 request: T 签名），其余平铺透传。
  goalCreate: <T = any>(request: object) => invokeYc<T>('goalCreate', { request }),
  goalGet: <T = any>(goalId: number) => invokeYc<T>('goalGet', { goal_id: goalId }),
  goalList: <T = any>(sessionId: string) => invokeYc<T>('goalList', { session_id: sessionId }),
  goalUpdate: <T = any>(request: object) => invokeYc<T>('goalUpdate', { request }),
  goalStart: <T = any>(goalId: number) => invokeYc<T>('goalStart', { goal_id: goalId }),
  goalPause: <T = any>(goalId: number) => invokeYc<T>('goalPause', { goal_id: goalId }),
  goalComplete: <T = any>(goalId: number, resultJson?: string) =>
    invokeYc<T>('goalComplete', { goal_id: goalId, result_json: resultJson }),
  goalAbort: <T = any>(goalId: number, reason: string) =>
    invokeYc<T>('goalAbort', { goal_id: goalId, reason }),
  goalDelete: (goalId: number) => invokeYc<void>('goalDelete', { goal_id: goalId }),
  goalConsumeTokens: <T = any>(goalId: number, tokens: number, modelId?: string) =>
    invokeYc<T>('goalConsumeTokens', { goal_id: goalId, tokens, model_id: modelId }),
  goalUpdateProgress: <T = any>(goalId: number, progressPct: number) =>
    invokeYc<T>('goalUpdateProgress', { goal_id: goalId, progress_pct: progressPct }),
  goalSaveCheckpoint: <T = any>(
    goalId: number,
    cp: { description: string; snapshot_json?: string; restart_instructions?: string },
  ) => invokeYc<T>('goalSaveCheckpoint', { goal_id: goalId, checkpoint: cp }),
  goalBuildContinuation: <T = any>(goalId: number) =>
    invokeYc<T>('goalBuildContinuation', { goal_id: goalId }),
  goalCheckpointsCount: <T = number>(goalId: number) =>
    invokeYc<T>('goalCheckpointsCount', { goal_id: goalId }),

  // ===== 上下文压缩（yuan_compact_commands，SettingsPanel 消费）=====
  // execute 用 request 包装、configUpdate 用 config 包装，其余 messages/session_id 平铺。
  compactConfigGet: <T = any>() => invokeYc<T>('compactConfigGet'),
  compactConfigUpdate: (config: Record<string, unknown>) =>
    invokeYc<void>('compactConfigUpdate', { config }),
  compactEstimate: <T = any>(messages: { id: string; role: string; content: string }[]) =>
    invokeYc<T>('compactEstimate', { messages }),
  compactCheck: <T = any>(messages: { id: string; role: string; content: string }[]) =>
    invokeYc<T>('compactCheck', { messages }),
  compactExecute: <T = any>(request: object) => invokeYc<T>('compactExecute', { request }),
  compactSession: <T = any>(sessionId: string) =>
    invokeYc<T>('compactSession', { session_id: sessionId }),
  compactReset: (sessionId: string) => invokeYc<void>('compactReset', { session_id: sessionId }),
  compactStats: <T = any>(messages: { id: string; role: string; content: string }[]) =>
    invokeYc<T>('compactStats', { messages }),
  compactBudgetVisualization: <T = any>(messages: { id: string; role: string; content: string }[]) =>
    invokeYc<T>('compactBudgetVisualization', { messages }),
  compactAuto: <T = any>(sessionId: string, messages: { id: string; role: string; content: string }[]) =>
    invokeYc<T>('compactAuto', { session_id: sessionId, messages }),
  compactManual: <T = any>(
    sessionId: string,
    messages: { id: string; role: string; content: string }[],
    config?: Record<string, unknown>,
  ) => invokeYc<T>('compactManual', { session_id: sessionId, messages, config }),
  compactForce: <T = any>(
    sessionId: string,
    messages: { id: string; role: string; content: string }[],
    config?: Record<string, unknown>,
  ) => invokeYc<T>('compactForce', { session_id: sessionId, messages, config }),

  // ===== 运行时 IO（yuan_io_commands，YuanCode 消费）=====
  ioCancel: (executionId: string) => invokeYc<void>('ioCancel', { execution_id: executionId }),
  ioKill: (executionId: string) =>
    invokeYc<void>('ioKill', { execution_id: executionId, signal: 'SIGKILL' }),
  ioStdin: (executionId: string, data: string) =>
    invokeYc<void>('ioStdin', { execution_id: executionId, data }),

  // ===== 提示词 / AGENTS.md（yuan_prompt_commands / yuan_agents_*，SettingsPanel 消费 4 条）=====
  discoverAgentsMd: <T = any>(cwd?: string, projectRoot?: string) =>
    invokeYc<T>('agentsDiscover', { cwd, project_root: projectRoot }),
  assembleInstructions: <T = any>(
    agentsFiles: unknown[],
    userInstructions?: string,
    maxBytes?: number,
  ) =>
    invokeYc<T>('agentsAssemble', {
      agents_files: agentsFiles,
      user_instructions: userInstructions,
      max_bytes: maxBytes,
    }),
  setMaxBytes: (maxBytes: number) => invokeYc<void>('agentsSetMaxBytes', { max_bytes: maxBytes }),
  getMaxBytes: <T = number>() => invokeYc<T>('agentsGetMaxBytes'),

  // ===== MCP 管理（yuan_mcp_commands，McpConfig/McpToolCall 消费）=====
  mcpListBuiltinServers: <T = any>() => invokeYc<T>('mcpListBuiltinServers'),
  mcpListBuiltinTools: <T = any>() => invokeYc<T>('mcpListBuiltinTools'),
  mcpCallBuiltinTool: <T = any>(serverName: string, toolName: string, args?: Record<string, unknown>) =>
    invokeYc<T>('mcpCallBuiltinTool', {
      server_name: serverName,
      tool_name: toolName,
      arguments: args ?? null,
    }),
  mcpConfigureBuiltin: (serverName: string, config: Record<string, unknown>) =>
    invokeYc<void>('mcpConfigureBuiltin', { server_name: serverName, config }),
  mcpListCallHistory: <T = any>() => invokeYc<T>('mcpListCallHistory'),
  mcpListServers: <T = any>() => invokeYc<T>('mcpListServers'),
  mcpListPersistedServers: <T = any>() => invokeYc<T>('mcpListPersistedServers'),
  mcpSaveServerConfig: (server: unknown, autoConnect: boolean) =>
    invokeYc<void>('mcpSaveServerConfig', { server, auto_connect: autoConnect }),
  mcpDeleteServerConfig: (serverId: string) =>
    invokeYc<void>('mcpDeleteServerConfig', { server_id: serverId }),
  mcpSetServerEnabled: (serverId: string, enabled: boolean) =>
    invokeYc<void>('mcpSetServerEnabled', { server_id: serverId, enabled }),
  mcpConnectServer: <T = any>(serverId: string) =>
    invokeYc<T>('mcpConnectServer', { server_id: serverId }),
  mcpDisconnectServer: (serverId: string) =>
    invokeYc<void>('mcpDisconnectServer', { server_id: serverId }),
};

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
