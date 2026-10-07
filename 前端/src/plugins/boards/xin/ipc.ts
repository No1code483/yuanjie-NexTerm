// plugins/boards/xin/ipc.ts — boards.xin L1 基础面 IPC 客户端（短码 xn，批次4a-1）。
// 契约：统一经内核 dispatcher，逻辑名 `xn:plugin:<旧命令名>`；参数形状与迁移前
// 旧封装逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 28 条 alias：配置 2 + 人格 8 + 记忆 4 + 情绪/多模态 7 + 摘要/洞察 4 + 语音 3。
//
// **v1 既有缺陷随批修复（37-A 同口径）**：旧封装存在参数键与 Tauri v2
// `ArgumentCase::Camel` 口径失配的调用点（`{ id }` / snake_case 多词键），
// 本批统一改 camelCase 修正（详见各方法注释）。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

// 类型来自 xin.orchestration L2 feature（原 lib/ipc/xin.ts 迁入），本处仅 re-export 供 L1 消费。
export type { XinAttachment, XinParsedAttachment, XinOutputEnhancement, XinDialogueSendOptions } from './features/orchestration/ipc';
// 小欣「一切皆插件」拆分：编排面 IPC 客户端经 L1 ipc 门面 re-export，
// 各 L2 面板统一从 `../../ipc` 取用（板块内 L1→L2 直连，规范 §1.4），避免 L2→L2 交叉 import。
export { xinOrchestration } from './features/orchestration/ipc';

export const XIN_BOARD_IPC_METHODS = {
  getConfig: { cmd: 'xin_get_config' },
  setConfig: { cmd: 'xin_set_config' },
  getPersonas: { cmd: 'xin_get_personas' },
  getActivePersona: { cmd: 'xin_get_active_persona' },
  setActivePersona: { cmd: 'xin_set_active_persona' },
  personaMemory: { cmd: 'xin_persona_memory' },
  personaSwitchHistory: { cmd: 'xin_persona_switch_history' },
  memorizedPersonas: { cmd: 'xin_memorized_personas' },
  growPersona: { cmd: 'xin_grow_persona' },
  selfGrowingPersona: { cmd: 'xin_self_growing_persona' },
  saveMemory: { cmd: 'xin_save_memory' },
  getMemories: { cmd: 'xin_get_memories' },
  searchMemories: { cmd: 'xin_search_memories' },
  deleteMemory: { cmd: 'xin_delete_memory' },
  analyzeSentiment: { cmd: 'xin_analyze_sentiment' },
  getTtsStatus: { cmd: 'xin_get_tts_status' },
  ttsSpeak: { cmd: 'xin_tts_speak' },
  processMultimodal: { cmd: 'xin_process_multimodal' },
  getMood: { cmd: 'xin_get_mood' },
  updateMood: { cmd: 'xin_update_mood' },
  emotionTrend: { cmd: 'xin_emotion_trend' },
  addSummary: { cmd: 'xin_add_summary' },
  getSummaries: { cmd: 'xin_get_summaries' },
  dailyBriefing: { cmd: 'xin_daily_briefing' },
  personalityInsights: { cmd: 'xin_personality_insights' },
  voiceInput: { cmd: 'xin_voice_input' },
  tts: { cmd: 'xin_tts' },
  ttsListVoices: { cmd: 'xin_tts_list_voices' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherXinBoard = defineIpcNamespace('xn', XIN_BOARD_IPC_METHODS);
type XinBoardMethod = keyof typeof XIN_BOARD_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeXin<T = any>(method: XinBoardMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherXinBoard[method](args) as Promise<ApiResponse<T>>;
}

export const xin = {
  getConfig: () => invokeXin<any>('getConfig'),
  setConfig: (config: any) => invokeXin<void>('setConfig', { config }),
  getPersonas: () => invokeXin<any[]>('getPersonas'),
  getActivePersona: () => invokeXin<any>('getActivePersona'),
  // 修复：旧封装发 `{ id }`，与后端必填参数 `persona_id` 完全失配（必填键缺失 → 命令恒失败）。
  setActivePersona: (id: string) => invokeXin<void>('setActivePersona', { personaId: id }),
  personaMemory: (personaId: string) => invokeXin<any>('personaMemory', { personaId }),
  // 修复：旧封装发 snake_case `persona_id`，改 Tauri v2 camelCase 口径。
  personaSwitchHistory: (limit?: number) => invokeXin<any[]>('personaSwitchHistory', { limit }),
  memorizedPersonas: () => invokeXin<string[]>('memorizedPersonas'),
  growPersona: (profile: {
    formality: number;
    verbosity: number;
    humor: number;
    technical_depth: number;
    empathy: number;
    interest_domains: string[];
    confidence: number;
  }) => invokeXin<any>('growPersona', { profile }),
  selfGrowingPersona: () => invokeXin<any>('selfGrowingPersona'),
  saveMemory: (memory: any) => invokeXin<void>('saveMemory', { memory }),
  getMemories: (limit: number = 50) => invokeXin<any[]>('getMemories', { limit }),
  searchMemories: (query: string) => invokeXin<any[]>('searchMemories', { query }),
  deleteMemory: (id: string) => invokeXin<void>('deleteMemory', { id }),
  analyzeSentiment: (text: string) => invokeXin<any>('analyzeSentiment', { text }),
  getTtsStatus: () => invokeXin<any>('getTtsStatus'),
  ttsSpeak: (request: any) => invokeXin<void>('ttsSpeak', { request }),
  processMultimodal: (input: any) => invokeXin<string>('processMultimodal', { input }),
  getMood: () => invokeXin<any>('getMood'),
  updateMood: (text: string) => invokeXin<any>('updateMood', { text }),
  emotionTrend: (limit?: number) => invokeXin<Array<{
    category: string;
    intensity: number;
    updated_at: string;
    trigger: string | null;
  }>>('emotionTrend', { limit: limit ?? 20 }),
  addSummary: (summary: any) => invokeXin<void>('addSummary', { summary }),
  getSummaries: (limit?: number) => invokeXin<any[]>('getSummaries', { limit }),
  dailyBriefing: () => invokeXin<any>('dailyBriefing'),
  personalityInsights: () => invokeXin<any>('personalityInsights'),
  // 修复：旧封装发 snake_case `{ audio_path, model_id }`（必填 `audio_path` 缺失 →
  // 命令恒失败），改 camelCase。
  voiceInput: (audioPath: string, language?: string, modelId?: number) => invokeXin<{
    text: string;
    engine: string;
  }>('voiceInput', {
    audioPath,
    language,
    modelId,
  }),
  tts: (text: string, voice?: string, speed?: number, pitch?: number) => invokeXin<{
    audio_path: string;
    engine: string;
    voices: string[];
  }>('tts', {
    text,
    voice,
    speed,
    pitch,
  }),
  ttsListVoices: () => invokeXin<{
    engine: string;
    voices: string[];
  }>('ttsListVoices'),

  // === v3 编排系列（4a-2 迁入 xin.orchestration L2 client）===
  // 编排命令已迁入 features/orchestration/ipc.ts（短码 xo），
  // 本处不再直接 ipc.invoke xin_v3_*；改用 xinOrchestration 客户端。
  // === 视频（4a-3 迁入 xin.orchestration L2 client）===
  // 视频命令已迁入 features/orchestration/ipc.ts（短码 xo），
  // 本处不再直接 ipc.invoke xin_video_*；改用 xinOrchestration 客户端。

  // === 视频（4a-3，待迁）===
};