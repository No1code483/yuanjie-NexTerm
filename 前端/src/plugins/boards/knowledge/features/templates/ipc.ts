// plugins/boards/knowledge/features/templates/ipc.ts — knowledge.templates L2 IPC 客户端
//（短码 kt，批C3 自 L1 kb 命名空间迁入）。
// 契约：统一经内核 dispatcher，逻辑名 `kt:plugin:<旧命令名>`（06_Rust代码契约 §8.1）；
// 返回形态与 L1 kb 客户端同构（ApiResponse 包装，Knowledge.tsx 消费端零改动）。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const KT_IPC_METHODS = {
  kbGetTemplates: { cmd: 'kb_get_templates' },
  kbCreateTemplate: { cmd: 'kb_create_template' },
  kbUpdateTemplate: { cmd: 'kb_update_template' },
  kbDeleteTemplate: { cmd: 'kb_delete_template' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherKt = defineIpcNamespace('kt', KT_IPC_METHODS);
type KtMethod = keyof typeof KT_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致（与 L1 invokeKb 同口径）
function invokeKt<T = any>(method: KtMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherKt[method](args) as Promise<ApiResponse<T>>;
}

export const kt = {
  kbGetTemplates: () => invokeKt<any[]>('kbGetTemplates'),
  kbCreateTemplate: (args: any) => invokeKt<any>('kbCreateTemplate', args),
  kbUpdateTemplate: (args: any) => invokeKt<any>('kbUpdateTemplate', args),
  kbDeleteTemplate: (args: any) => invokeKt<any>('kbDeleteTemplate', args),
};
