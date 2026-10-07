// plugins/boards/xin/features/realtime/ipc.ts — xin.realtime L2 IPC 客户端（短码 xr，批次4a-1）。
// 契约：统一经内核 dispatcher，逻辑名 `xr:plugin:<旧命令名>`；
// 对齐后端 commands/xin_realtime_commands.rs（状态机 Idle → Listening → Thinking → Speaking）。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

/** 实时会话配置（对齐后端 RealtimeConfig 结构体字段） */
export interface RealtimeConfig {
  stt_model_id: number;
  llm_model_id: number;
  tts_model_id?: number | null;
  persona_id: string;
  title?: string;
  vad_enabled?: boolean;
  tts_voice?: string;
}

export type RealtimeState = 'idle' | 'listening' | 'thinking' | 'speaking';

export const XIN_REALTIME_IPC_METHODS = {
  start: { cmd: 'xin_realtime_start' },
  stop: { cmd: 'xin_realtime_stop' },
  pushChunk: { cmd: 'xin_realtime_push_chunk' },
  getState: { cmd: 'xin_realtime_get_state' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherXinRealtime = defineIpcNamespace('xr', XIN_REALTIME_IPC_METHODS);
type XinRealtimeMethod = keyof typeof XIN_REALTIME_IPC_METHODS;

function invokeXinRealtime<T = any>(method: XinRealtimeMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherXinRealtime[method](args) as Promise<ApiResponse<T>>;
}

export const realtime = {
  /** 启动实时会话（状态 → listening） */
  start: (config: RealtimeConfig) => invokeXinRealtime<RealtimeState>('start', { config }),
  /** 停止实时会话（状态 → idle） */
  stop: () => invokeXinRealtime<void>('stop'),
  /** 推送 PCM 音频块（samples 为 i16 数组，每块 20-30ms）；
   *  修复：旧封装发 snake_case `timestamp_ms`（必填键缺失 → 命令恒失败），改 camelCase。 */
  pushChunk: (samples: number[], timestampMs: number) =>
    invokeXinRealtime<RealtimeState>('pushChunk', {
      samples,
      timestampMs,
    }),
  /** 查询当前会话状态 */
  getState: () => invokeXinRealtime<RealtimeState>('getState'),
};