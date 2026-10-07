// plugins/boards/home/features/journal/ipc.ts — home.journal 插件 IPC 客户端（短码 jn）。
// 契约：统一经内核 dispatcher，逻辑名 `jn:plugin:<旧命令名>`；参数形状与迁移前
// 旧 Tauri 命令逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const JOURNAL_IPC_METHODS = {
  getJournal: { cmd: 'get_journal' },
  saveJournal: { cmd: 'save_journal' },
  deleteJournal: { cmd: 'delete_journal' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherJournal = defineIpcNamespace('jn', JOURNAL_IPC_METHODS);
type JournalMethod = keyof typeof JOURNAL_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeJournal<T = any>(method: JournalMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherJournal[method](args) as Promise<ApiResponse<T>>;
}

export const journal = {
  getJournal: (date: string) => invokeJournal<any>('getJournal', { date }),
  saveJournal: (date: string, content: string) => invokeJournal<any>('saveJournal', { date, content }),
  // 零消费命令（前端无调用点），保留与后端 alias 的 1:1 映射
  deleteJournal: (date: string) => invokeJournal('deleteJournal', { date }),
};
