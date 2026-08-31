// ipc.ts — IPC 聚合入口（T2.1.6 模块化拆分）
// 核心基础设施在 ./ipc/core.ts，各模块在 ./ipc/ 子目录下
// 此文件作为聚合入口 re-export，保持现有 import { auth, ai, ... } from '@/lib/ipc' 不变

// Core
export { ipc, USE_MOCK } from './ipc/core';
export type { ApiResponse } from './ipc/core';
export { default } from './ipc/core';

// Modules
export { auth } from './ipc/auth';
export { home } from './ipc/home';
export { ai } from './ipc/ai';
export { knowledge } from './ipc/knowledge';
export { intelligence } from './ipc/intelligence';
export { system, newsSource, extension, recycleBin } from './ipc/system';
export { terminal, linux } from './ipc/terminal';
export { profile } from './ipc/profile';
export { yuanCode, yuanCompact, yuanGoal, yuanMcp, BUILTIN_MCP_SERVERS } from './ipc/yuan-code';
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
  BuiltinMcpInfo,
} from './ipc/yuan-code';
export { xin, realtime } from './ipc/xin';
export type { XinAttachment, XinParsedAttachment, XinOutputEnhancement, XinDialogueSendOptions, RealtimeConfig, RealtimeState } from './ipc/xin';
export { game } from './ipc/game';
export { sync } from './ipc/sync';
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
} from './ipc/sync';
