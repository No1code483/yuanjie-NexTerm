// plugins/boards/home/features/timer/ipc.ts — home.timer 插件 IPC 客户端（短码 ti）。
// 契约：统一经内核 dispatcher，逻辑名 `ti:plugin:<旧命令名>`；参数形状与迁移前
// 旧 Tauri 命令逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const TIMER_IPC_METHODS = {
  getTimers: { cmd: 'get_timers' },
  createTimer: { cmd: 'create_timer' },
  updateTimerState: { cmd: 'update_timer_state' },
  deleteTimer: { cmd: 'delete_timer' },
  timerAction: { cmd: 'timer_action' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherTimer = defineIpcNamespace('ti', TIMER_IPC_METHODS);
type TimerMethod = keyof typeof TIMER_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeTimer<T = any>(method: TimerMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherTimer[method](args) as Promise<ApiResponse<T>>;
}

export const timer = {
  getTimers: () => invokeTimer<any[]>('getTimers'),
  // 零消费命令（TimerPanel 走 TimerContext + localStorage，前端无调用点），
  // 本批仅完成命名空间声明与归属收归，不接通前端
  createTimer: (timer: any) => invokeTimer('createTimer', { request: timer }),
  updateTimerState: (id: number, isRunning: boolean, elapsed: number) =>
    invokeTimer('updateTimerState', { id, isRunning, elapsed }),
  deleteTimer: (id: number) => invokeTimer('deleteTimer', { id }),
  timerAction: (id: number, action: string) => invokeTimer('timerAction', { id, action }),
};
