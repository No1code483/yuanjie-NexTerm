// plugins/boards/ai/features/models/ipc.ts — ai.models 插件 IPC 客户端（短码 am）。
// 契约：统一经内核 dispatcher，逻辑名 `am:plugin:<旧命令名>`；参数形状与迁移前
// 旧 Tauri 命令逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 8 条 alias：ai_models 表 CRUD 4 条 + provider info + 3 条健康检测命令。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const AI_MODELS_IPC_METHODS = {
  getModels: { cmd: 'get_ai_models' },
  createModel: { cmd: 'add_ai_model' },
  updateModel: { cmd: 'update_ai_model' },
  deleteModel: { cmd: 'delete_ai_model' },
  getProviderInfo: { cmd: 'get_ai_provider_info' },
  checkAllModelsHealth: { cmd: 'check_all_models_health' },
  getModelHealthStatus: { cmd: 'get_model_health_status' },
  setHealthCheckInterval: { cmd: 'set_health_check_interval' },
  // 2b-2（裁定 11）：单模型健康检测（写 ai_models 表）归 am 域。
  checkModelHealth: { cmd: 'check_model_health' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherAiModels = defineIpcNamespace('am', AI_MODELS_IPC_METHODS);
type AiModelsMethod = keyof typeof AI_MODELS_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeAiModels<T = any>(method: AiModelsMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherAiModels[method](args) as Promise<ApiResponse<T>>;
}

/** 参数键与迁移前旧封装逐字一致（`request` / `id` / `provider` / `modelId` / `minutes`），
 *  后端 dispatcher 以 snake_case 优先、camelCase 回退，两种口径均可绑定。 */
export const aiModels = {
  getModels: <T = any[]>() => invokeAiModels<T>('getModels'),
  createModel: (model: any) => invokeAiModels('createModel', { request: model }),
  updateModel: (model: any) => invokeAiModels('updateModel', { request: model }),
  deleteModel: (id: number) => invokeAiModels('deleteModel', { id }),
  getProviderInfo: (provider: string) => invokeAiModels('getProviderInfo', { provider }),
  checkAllModelsHealth: () => invokeAiModels<any[]>('checkAllModelsHealth'),
  getModelHealthStatus: (modelId: number) => invokeAiModels<any>('getModelHealthStatus', { modelId }),
  setHealthCheckInterval: (minutes: number) => invokeAiModels('setHealthCheckInterval', { minutes }),
  checkModelHealth: (modelId: number) => invokeAiModels<any>('checkModelHealth', { modelId }),
};