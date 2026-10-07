// plugins/boards/ai/features/groupchat/ipc.ts — ai.groupchat 插件 IPC 客户端（短码 gc）。
// 契约：统一经内核 dispatcher，逻辑名 `gc:plugin:<旧命令名>`；参数形状与迁移前
// 旧 Tauri 命令逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 2 条 alias：群聊编排状态查询 + 强制结束。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const AI_GROUPCHAT_IPC_METHODS = {
  getOrchestrationStatus: { cmd: 'ai_get_orchestration_status' },
  endGroupChat: { cmd: 'ai_end_group_chat' },
  // 2b-2（裁定 11）：会话编排两条归入 gc 域（与既有编排同实现）。
  runOrchestrator: { cmd: 'run_orchestrator' },
  stopGeneration: { cmd: 'stop_generation' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherAiGroupChat = defineIpcNamespace('gc', AI_GROUPCHAT_IPC_METHODS);
type AiGroupChatMethod = keyof typeof AI_GROUPCHAT_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeAiGroupChat<T = any>(method: AiGroupChatMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherAiGroupChat[method](args) as Promise<ApiResponse<T>>;
}

/** 参数键与迁移前旧封装逐字一致（camelCase `conversationId`），后端 dispatcher
 *  以 snake_case 优先、camelCase 回退，两种口径均可绑定。 */
export const aiGroupChat = {
  getOrchestrationStatus: (conversationId: number) => invokeAiGroupChat('getOrchestrationStatus', { conversationId }),
  endGroupChat: (conversationId: number) => invokeAiGroupChat('endGroupChat', { conversationId }),
  // 参数键与迁移前逐字一致（camelCase `conversationId` / `userMessage` + 展开的 config 字段）。
  runOrchestrator: (conversationId: number, userMessage: string, config?: Record<string, unknown>) =>
    invokeAiGroupChat('runOrchestrator', { conversationId, userMessage, ...(config ?? {}) }),
  stopGeneration: (conversationId: number) => invokeAiGroupChat('stopGeneration', { conversationId }),
};