// plugins/customs/recycle/ipc/recycle.ts — 回收站插件 IPC 客户端（短码 rc，批次6a）。
// 契约：统一经内核 dispatcher，逻辑名 `rc:plugin:<旧命令名>`；参数形状与迁移前
// 旧封装逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 7 条 alias：recycle_list / recycle_move_to / recycle_restore /
// recycle_delete_permanently / recycle_empty_all / cleanup_expired_recycle / recycle_stats
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const RECYCLE_IPC_METHODS = {
  list: { cmd: 'recycle_list' },
  moveTo: { cmd: 'recycle_move_to' },
  restore: { cmd: 'recycle_restore' },
  deletePermanently: { cmd: 'recycle_delete_permanently' },
  empty: { cmd: 'recycle_empty_all' },
  cleanupExpired: { cmd: 'cleanup_expired_recycle' },
  stats: { cmd: 'recycle_stats' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherRc = defineIpcNamespace('rc', RECYCLE_IPC_METHODS);
type RecycleMethod = keyof typeof RECYCLE_IPC_METHODS;

function invokeRc<T>(method: RecycleMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherRc[method](args) as Promise<ApiResponse<T>>;
}

export const recycleBin = {
  getItems: () => invokeRc<any[]>('list'),
  moveToRecycle: (itemType: string, itemIds: number[]) =>
    invokeRc<void>('moveTo', { itemType, itemIds }),
  restore: (ids: number[]) => invokeRc<void>('restore', { ids }),
  permanentDelete: (ids: number[]) =>
    invokeRc<void>('deletePermanently', { ids }),
  empty: () => invokeRc<void>('empty'),
  cleanupExpired: () => invokeRc<void>('cleanupExpired'),
  stats: () => invokeRc<any>('stats'),
};