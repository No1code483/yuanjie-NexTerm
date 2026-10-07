// plugins/boards/ai/features/sessions/ipc.ts — ai.sessions 插件 IPC 客户端（短码 ss）。
// batchC1：会话列表功能从 L1 boards.ai 收编到 L2 ai.sessions。
// 契约：统一经内核 dispatcher，逻辑名 `ss:plugin:<旧命令名>`；参数形状与迁移前
// 旧 Tauri 命令逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 10 条 alias：会话 CRUD 6 + 参与者 1 + 模板 4 - 1 (promptTemplateList 归 L1) = 会话 10。
// 编排 2 条（run_orchestrator / stop_generation）归 gc，模型健康归 am，均不在本文件。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const AI_SESSIONS_IPC_METHODS = {
  createConversation: { cmd: 'create_conversation' },
  getConversations: { cmd: 'get_conversations' },
  updateConversation: { cmd: 'update_conversation' },
  deleteConversation: { cmd: 'delete_conversation' },
  markConversationRead: { cmd: 'mark_conversation_read' },
  reorderConversations: { cmd: 'reorder_conversations' },
  searchConversations: { cmd: 'search_conversations' },
  toggleStarConversation: { cmd: 'toggle_star_conversation' },
  branchConversation: { cmd: 'branch_conversation' },
  exportConversation: { cmd: 'export_conversation' },
  getMessages: { cmd: 'get_messages' },
  deleteMessage: { cmd: 'delete_message' },
  getParticipants: { cmd: 'get_participants' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherAiSessions = defineIpcNamespace('ss', AI_SESSIONS_IPC_METHODS);
type AiSessionsMethod = keyof typeof AI_SESSIONS_IPC_METHODS;

function invokeAiSessions<T = any>(method: AiSessionsMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherAiSessions[method](args) as Promise<ApiResponse<T>>;
}

/** 参数键与 L1 boards.ai IPC 客户端逐字一致；后端 dispatcher 以 snake_case
 * 优先、camelCase 回退，两种口径均可绑定。 */
export const aiSessions = {
  createConversation: (request: any) => invokeAiSessions('createConversation', { request }),
  getSessions: () => invokeAiSessions<any[]>('getConversations'),
  getMessages: (conversationId: number) => invokeAiSessions<any[]>('getMessages', { conversationId }),
  getParticipants: (conversationId: number) => invokeAiSessions<any[]>('getParticipants', { conversationId }),
  // sendMessage 归 L1 boards.ai 承载（ai.sendMessage），此处不重复声明
  deleteConversation: (id: number) => invokeAiSessions('deleteConversation', { id }),
  deleteMessage: (id: number) => invokeAiSessions('deleteMessage', { id }),
  searchConversations: (query: string) => invokeAiSessions<any[]>('searchConversations', { request: { query } }),
  toggleStar: (id: number) => invokeAiSessions<boolean>('toggleStarConversation', { id }),
  branchConversation: (conversationId: number, messageId: number) =>
    invokeAiSessions<any>('branchConversation', { request: { conversation_id: conversationId, message_id: messageId } }),
  exportConversation: (conversationId: number, format: string) =>
    invokeAiSessions<string>('exportConversation', { request: { conversation_id: conversationId, format } }),
  markConversationRead: (conversationId: number) => invokeAiSessions('markConversationRead', { conversationId }),
  reorderConversations: (items: Array<{ id: number; sort_order: number }>) =>
    invokeAiSessions('reorderConversations', { items }),
  updateConversation: (request: { id: number; title?: string; token_budget?: number }) =>
    invokeAiSessions('updateConversation', { request }),
};
