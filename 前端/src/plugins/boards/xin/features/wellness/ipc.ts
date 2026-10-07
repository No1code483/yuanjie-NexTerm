// plugins/boards/xin/features/wellness/ipc.ts — xin.wellness L2 IPC 客户端（短码 xw，批次4a-1）。
// 契约：统一经内核 dispatcher，逻辑名 `xw:plugin:<旧命令名>`。
// 17 条 alias：情绪/记忆/会话桥 4 + 提醒 4 + 习惯 3 + 番茄钟 4 + 摘要/演化 2。
// **前端零消费（后端子系统完整、UI 未接线）**：仅登记命名空间，方法保留供后续接线。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const XIN_WELLNESS_IPC_METHODS = {
  moodHistory: { cmd: 'xin_v2_mood_history' },
  consolidateMemories: { cmd: 'xin_v2_consolidate_memories' },
  memoryLinks: { cmd: 'xin_v2_memory_links' },
  conversationBridge: { cmd: 'xin_v2_conversation_bridge' },
  addReminder: { cmd: 'xin_v2_add_reminder' },
  listReminders: { cmd: 'xin_v2_list_reminders' },
  dismissReminder: { cmd: 'xin_v2_dismiss_reminder' },
  deleteReminder: { cmd: 'xin_v2_delete_reminder' },
  registerHabit: { cmd: 'xin_v2_register_habit' },
  checkinHabit: { cmd: 'xin_v2_checkin_habit' },
  listHabits: { cmd: 'xin_v2_list_habits' },
  pomodoroStart: { cmd: 'xin_v2_pomodoro_start' },
  pomodoroCompleteCycle: { cmd: 'xin_v2_pomodoro_complete_cycle' },
  pomodoroStop: { cmd: 'xin_v2_pomodoro_stop' },
  pomodoroStatus: { cmd: 'xin_v2_pomodoro_status' },
  activityDigest: { cmd: 'xin_v2_activity_digest' },
  personalityEvolution: { cmd: 'xin_v2_personality_evolution' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherXinWellness = defineIpcNamespace('xw', XIN_WELLNESS_IPC_METHODS);
type XinWellnessMethod = keyof typeof XIN_WELLNESS_IPC_METHODS;

function invokeXinWellness<T = any>(method: XinWellnessMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherXinWellness[method](args) as Promise<ApiResponse<T>>;
}

export const xinWellness = {
  moodHistory: (triggerText?: string) => invokeXinWellness<any>('moodHistory', { triggerText }),
  consolidateMemories: () => invokeXinWellness<any>('consolidateMemories'),
  memoryLinks: () => invokeXinWellness<any[]>('memoryLinks'),
  conversationBridge: (sessionId?: string) => invokeXinWellness<any>('conversationBridge', { sessionId }),
  addReminder: (title: string, description: string, triggerAt?: string, cronExpr?: string) =>
    invokeXinWellness<any>('addReminder', { title, description, triggerAt, cronExpr }),
  listReminders: () => invokeXinWellness<any[]>('listReminders'),
  dismissReminder: (id: string) => invokeXinWellness<boolean>('dismissReminder', { id }),
  deleteReminder: (id: string) => invokeXinWellness<boolean>('deleteReminder', { id }),
  registerHabit: (name: string, category: string) => invokeXinWellness<any>('registerHabit', { name, category }),
  checkinHabit: (habitId: string) => invokeXinWellness<any>('checkinHabit', { habitId }),
  listHabits: () => invokeXinWellness<any[]>('listHabits'),
  pomodoroStart: (taskName: string, durationMinutes: number, breakMinutes: number, totalCycles: number) =>
    invokeXinWellness<any>('pomodoroStart', { taskName, durationMinutes, breakMinutes, totalCycles }),
  pomodoroCompleteCycle: () => invokeXinWellness<any>('pomodoroCompleteCycle'),
  pomodoroStop: () => invokeXinWellness<any>('pomodoroStop'),
  pomodoroStatus: () => invokeXinWellness<any>('pomodoroStatus'),
  activityDigest: (period?: string) => invokeXinWellness<any>('activityDigest', { period }),
  personalityEvolution: () => invokeXinWellness<any>('personalityEvolution'),
};