// plugins/boards/ai/ipc.ts — boards.ai L1 IPC 客户端（短码 ai，批次2b-2）。
// 契约：统一经内核 dispatcher，逻辑名 `ai:plugin:<旧命令名>`；参数形状与迁移前
// 旧 Tauri 命令逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// batchC1 收编后 L1 剩余 5 条 alias：sendMessage 1 + Prompt 模板 4。
// 会话面 13 条命令归 L2 ai.sessions（ss），前端 AI.tsx 切换至 aiSessions 客户端。
// 编排 2 条（runOrchestrator / stopGeneration）归 gc，模型健康归 am，均不在本文件。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const AI_BOARD_IPC_METHODS = {
  sendMessage: { cmd: 'send_message' },
  promptTemplateList: { cmd: 'prompt_template_list' },
  promptTemplateCreate: { cmd: 'prompt_template_create' },
  promptTemplateUpdate: { cmd: 'prompt_template_update' },
  promptTemplateDelete: { cmd: 'prompt_template_delete' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherAiBoard = defineIpcNamespace('ai', AI_BOARD_IPC_METHODS);
type AiBoardMethod = keyof typeof AI_BOARD_IPC_METHODS;

// 未标注泛宽时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeAiBoard<T = any>(method: AiBoardMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherAiBoard[method](args) as Promise<ApiResponse<T>>;
}

/** 参数键与迁移前旧封装（`lib/ipc/ai.ts`）逐字一致；后端 dispatcher 以 snake_case
 *  优先、camelCase 回退，两种口径均可绑定。方法命名沿用旧 `ai.*` 门面对齐调用方最小改动。 */
export const ai = {
  sendMessage: (sessionId: number, content: string) =>
    invokeAiBoard('sendMessage', { request: { conversation_id: sessionId, content } }),
  getPromptTemplates: () => invokeAiBoard<any[]>('promptTemplateList'),
  createPromptTemplate: (title: string, category: string, content: string) =>
    invokeAiBoard<any>('promptTemplateCreate', { request: { title, category, content } }),
  updatePromptTemplate: (id: number, title?: string, category?: string, content?: string) =>
    invokeAiBoard('promptTemplateUpdate', { request: { id, title, category, content } }),
  deletePromptTemplate: (id: number) => invokeAiBoard('promptTemplateDelete', { id }),
};