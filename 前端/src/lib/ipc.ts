// ipc.ts — IPC 聚合入口（T2.1.6 模块化拆分）
// 核心基础设施在 ./ipc/core.ts，各模块在 ./ipc/ 子目录下
// 此文件作为聚合入口 re-export，保持现有 import { ai, ... } from '@/lib/ipc' 不变
// （auth 已迁出，见下方 Modules 注释）

// Core
export { ipc, USE_MOCK } from './ipc/core';
export type { ApiResponse } from './ipc/core';
export { default } from './ipc/core';

// Modules
// auth 已于阶段3 批次1a-1 迁入插件：@/plugins/customs/auth/ipc/auth
// ai 已于阶段3 批次2b（模型/Agent 2b-1，会话面 2b-2）全量迁入插件：
//   @/plugins/boards/ai/ipc（会话面 L1）+ features/{models,agent,groupchat}/ipc（L2）
// 批次4c（S3）：system/extension 已随 customs.systemtools 插件化，
// 原 `./ipc/system` 聚合导出（system/extension/recycleBin）随之移除；
// recycleBin 由 customs.recycle 插件独享 client。
//   @/plugins/customs/systemtools/ipc/systemtools
// intelligence 已于阶段3 批次6c 迁入插件：@/plugins/customs/intelligence/ipc/intelligence
export { intelligence } from '@/plugins/customs/intelligence/ipc/intelligence';
export type {
  DashboardData,
  RealtimeStats,
  Suggestion,
  BehaviorReport,
  ActivityLog,
  ActivityStats,
} from '@/types';
// recycleBin 已于阶段3 批次6a 迁入插件：@/plugins/customs/recycle/ipc/recycle
export { recycleBin } from '@/plugins/customs/recycle/ipc/recycle';
// terminal/linux 已于阶段3 批次 3a/3b 迁入插件：
//   @/plugins/boards/terminal/ipc（terminal 段，L1 boards.terminal）
//   @/plugins/boards/terminal/features/linux/ipc（linux/docker 段，L2 terminal.linux，
//   前端零消费——唯一消费者 components/Linux.tsx 死文件随 3b 裁定 T9 删除）
// yuanCode / yuanCompact / yuanGoal / yuanMcp 门面对象及 BUILTIN_MCP_SERVERS 常量已于
// 阶段3 批次 3c/5a-5e 迁入插件：@/plugins/boards/terminal/features/yuancode/ipc（yc 门面）；
// 类型 interface 仍保留在 ./ipc/yuan-code 作为 type re-export 来源。
export type { YuanFileNode, YuanExecutionResult, HighlightLine, HighlightResult, CodeCompletionRequest, CompletionItem, ReplaceRange, CodeCompletionResult, CodeAnalysisRequest, CodeIssue, CodeAnalysisResult, CodeSnippet, SaveSnippetRequest, UpdateSnippetRequest, DiffLine, DiffHunk, DiffResult, ReplaceFilesRequest, CopyMoveParams, FileInfoResult, WorkspaceTab, WorkspaceSession, SaveWorkspaceRequest, PromptVariable, PromptTemplateMeta, RenderPromptRequest, RenderPromptResult, AgentsMdFile, HierarchicalInstructions, AssembleSystemPromptRequest, PromptPart, AssembleSystemPromptResult } from './ipc/yuan-code';
// D1 v3.1 Task 3.1 + 3.5: Agent 化 + 云端 API 类型
export type {
  CloudApiKeyInfo,
  UpsertCloudApiKeyRequest,
  CloudProviderInfo,
  CloudConnectionTestResult,
  AgentV3TypeInfo,
  AgentV3Created,
  AgentV3PlanStep,
  AgentV3Plan,
  AgentV3SafetyCheck,
  AgentV3PlanResponse,
  AgentV3FileDiff,
  AgentV3Result,
  AgentV3ReviewRequest,
  AgentV3ReviewResponse,
  AgentV3Status,
} from './ipc/yuan-code';
// D1 v3.1 Task 3.2: MCP 生态类型
export type {
  McpTool,
  McpRegisteredTool,
  McpServerStatus,
  McpServerRegistration,
  McpToolCallResult,
  McpToolCallHistoryEntry,
  McpServerRecord,
} from './ipc/yuan-code';
// BuiltinMcpInfo 类型已随 BUILTIN_MCP_SERVERS 迁入 features/yuancode/ipc，全仓无 @/lib/ipc 消费点
// sync 已于阶段3 批次6b 迁入插件：@/plugins/customs/sync/ipc/sync
export { sync } from '@/plugins/customs/sync/ipc/sync';
export type {
  SyncOperation,
  SyncStatus,
  SyncQueueItem,
  SyncQueueStats,
  SyncDevice,
  SyncRunResult,
  EncryptedPayload,
  SerializedKeyPair,
  NetworkStatus,
} from '@/plugins/customs/sync/ipc/sync';
