// plugins/boards/ai/features/agent/ipc.ts — ai.agent 插件 IPC 客户端（短码 ag）。
// 契约：统一经内核 dispatcher，逻辑名 `ag:plugin:<旧命令名>`；参数形状与迁移前
// 旧 Tauri 命令逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 4 条 alias：ai_agents 表 CRUD 4 条。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const AI_AGENTS_IPC_METHODS = {
  getAgents: { cmd: 'get_ai_agents' },
  createAgent: { cmd: 'add_ai_agent' },
  updateAgent: { cmd: 'update_ai_agent' },
  deleteAgent: { cmd: 'delete_ai_agent' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherAiAgents = defineIpcNamespace('ag', AI_AGENTS_IPC_METHODS);
type AiAgentsMethod = keyof typeof AI_AGENTS_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeAiAgents<T = any>(method: AiAgentsMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherAiAgents[method](args) as Promise<ApiResponse<T>>;
}

/** 参数键与迁移前旧封装逐字一致（`request` / `id`），后端 dispatcher 的
 *  `parse_request` 统一从顶层 `request` 键反序列化 AddAgentRequest / UpdateAgentRequest。 */
export const aiAgents = {
  getAgents: () => invokeAiAgents<any[]>('getAgents'),
  createAgent: (request: any) => invokeAiAgents('createAgent', { request }),
  updateAgent: (request: any) => invokeAiAgents('updateAgent', { request }),
  deleteAgent: (id: number) => invokeAiAgents('deleteAgent', { id }),
};