// ipc/xin.ts — xin 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）
import { ipc } from './core';

// ============================================================================
// D3.4b 小欣多模态类型定义（对齐后端 xin_multimodal_service.rs）
// 规范：功能展望/模块深化/03_小欣_多模态融合_深度.md §2
// ============================================================================

/** 附件输入（前端构造，传给后端 parse_attachment / dialogue_send） */
export interface XinAttachment {
  name: string;
  mime_type: string;
  path?: string;
  data_b64?: string;
  label?: string;
}

/** 解析后的附件（后端 parse_attachment 返回） */
export interface XinParsedAttachment {
  name: string;
  mime_type: string;
  text_content?: string | null;
  image_b64?: string | null;
  is_image: boolean;
  is_text: boolean;
  is_video: boolean;
  label: string;
}

/** 输出增强信息（后端 enhance_output 返回） */
export interface XinOutputEnhancement {
  has_code_blocks: boolean;
  code_languages: string[];
  has_tables: boolean;
  has_lists: boolean;
  content_length: number;
  suggested_mode: string;
}

/** dialogueSend 的强类型 options（替代原 Record<string, unknown>） */
export interface XinDialogueSendOptions {
  model_id?: number;
  persona_id?: string;
  skills_enabled?: boolean;
  stream?: boolean;
  /** D3.4b 多模态附件列表（图片/文本/视频） */
  attachments?: XinAttachment[];
}

export const xin = {
  // === 基础（xin_basic_commands） ===
  getPersonas: () => ipc.invoke<any[]>('xin_get_personas'),
  getActivePersona: () => ipc.invoke<any>('xin_get_active_persona'),
  setActivePersona: (id: string) => ipc.invoke<void>('xin_set_active_persona', {
    id
  }),
  // === 对话（xin_orchestration_commands / v3） ===
  dialogueList: (limit: number = 50, offset: number = 0) => ipc.invoke<any>('xin_v3_dialogue_list', {
    limit,
    offset
  }),
  dialogueCreate: (personaId?: string) => ipc.invoke<any>('xin_v3_dialogue_create', {
    persona_id: personaId
  }),
  dialogueGet: (convId: string) => ipc.invoke<any>('xin_v3_dialogue_get', {
    conversation_id: convId
  }),
  dialogueDelete: (convId: string) => ipc.invoke<void>('xin_v3_dialogue_delete', {
    conversation_id: convId
  }),
  dialogueSend: (convId: string, content: string, options?: XinDialogueSendOptions) => ipc.invoke<any>('xin_v3_dialogue_send', {
    conversation_id: convId,
    content,
    ...options
  }), // D3.1: options 可传 model_id: number (ai_models.id)，不传则后端用第一个可用模型
      // D3.4b: options.attachments 可传 XinAttachment[] 实现多模态输入（图片/文本/视频）
  dialogueStop: (convId: string) => ipc.invoke<void>('xin_v3_dialogue_stop', {
    conversation_id: convId
  }),
  dialogueSearch: (query: string, limit: number = 20) => ipc.invoke<any>('xin_v3_dialogue_search', {
    query,
    limit
  }),
  // === 压缩（compaction） ===
  compactionGetConfig: () => ipc.invoke<any>('xin_v3_compaction_get_config'),
  compactionUpdateConfig: (partial: Record<string, unknown>) => ipc.invoke<void>('xin_v3_compaction_update_config', {
    config: partial
  }),
  compactionGetRecords: (convId: string, limit: number = 30) => ipc.invoke<any[]>('xin_v3_compaction_get_records', {
    conversation_id: convId,
    limit
  }),
  compactionNeedsCheck: (convId: string) => ipc.invoke<any>('xin_v3_compaction_needs_check', {
    conversation_id: convId
  }),
  compactionAuto: (convId: string) => ipc.invoke<void>('xin_v3_compaction_auto', {
    conversation_id: convId
  }),
  compactionManual: (convId: string, messages: {
    id: string;
    role: string;
    content: string;
  }[]) => ipc.invoke<void>('xin_v3_compaction_manual', {
    conversation_id: convId,
    messages
  }),
  // === 梦境（dream） ===
  dreamDefaultConfig: () => ipc.invoke<any>('xin_v3_dream_default_config'),
  dreamCalcHealth: () => ipc.invoke<any>('xin_v3_dream_calc_health'),
  dreamRunLight: () => ipc.invoke<any>('xin_v3_dream_run_light'),
  dreamRunDeep: (n: number = 20) => ipc.invoke<any>('xin_v3_dream_run_deep', {
    n
  }),
  dreamRunRem: () => ipc.invoke<any>('xin_v3_dream_run_rem'),
  // === 断点（checkpoint） ===
  checkpointList: (convId: string) => ipc.invoke<any[]>('xin_v3_checkpoint_list', {
    conversation_id: convId
  }),
  checkpointSave: (convId: string, label: string) => ipc.invoke<any>('xin_v3_checkpoint_save', {
    conversation_id: convId,
    label
  }),
  checkpointRestore: (checkpointId: string) => ipc.invoke<void>('xin_v3_checkpoint_restore', {
    checkpoint_id: checkpointId
  }),
  checkpointDelete: (checkpointId: string) => ipc.invoke<void>('xin_v3_checkpoint_delete', {
    checkpoint_id: checkpointId
  }),
  // === 复盘（review） ===
  reviewGenerate: (period: string, personaId: string) => ipc.invoke<any>('xin_v3_review_generate', {
    period,
    persona_id: personaId
  }),
  reviewTopicTrends: (personaId: string, period: string, days: number = 7) => ipc.invoke<any>('xin_v3_review_topic_trends', {
    persona_id: personaId,
    period,
    days
  }),
  reviewGrowthTrajectory: (personaId: string, period: string, days: number = 7) => ipc.invoke<any>('xin_v3_review_growth_trajectory', {
    persona_id: personaId,
    period,
    days
  }),
  reviewHeatmap: (personaId: string, period: string) => ipc.invoke<any>('xin_v3_review_heatmap', {
    persona_id: personaId,
    period
  }),
  // === 工具 ===
  estimateTokens: (text: string) => ipc.invoke<number>('xin_v3_estimate_tokens', {
    text
  }),
  listSkills: () => ipc.invoke<any[]>('xin_v3_list_skills'),
  executeSkill: (skillId: string, input: string) => ipc.invoke<any>('xin_v3_execute_skill', {
    skill_id: skillId,
    input
  }),
  listTools: () => ipc.invoke<any[]>('xin_v3_list_tools'),
  parseToolCalls: (text: string) => ipc.invoke<any[]>('xin_v3_parse_tool_calls', {
    text
  }),
  executeTool: (callsJson: string) => ipc.invoke<any[]>('xin_v3_execute_tool', {
    calls: callsJson
  }),
  fusionMemoryQuery: (query: string, limit: number = 10) => ipc.invoke<any[]>('xin_fusion_memory_query', {
    query,
    limit
  }),
  fusionExtractKeywords: (text: string) => ipc.invoke<string[]>('xin_fusion_extract_keywords', {
    text
  }),
  getMemories: (limit: number = 50) => ipc.invoke<any[]>('xin_get_memories', {
    limit
  }),
  getMood: () => ipc.invoke<any>('xin_get_mood'),
  // D3.7: 查询用户情绪历史趋势（基于 xin_moods 表，前端情绪时间线展示）
  emotionTrend: (limit?: number) => ipc.invoke<Array<{
    category: string;
    intensity: number;
    updated_at: string;
    trigger: string | null;
  }>>('xin_emotion_trend', { limit: limit ?? 20 }),
  // D3.8: 查询某人格的历史记忆（交互摘要 + 用户偏好 + 话题标签）
  personaMemory: (personaId: string) => ipc.invoke<{
    persona_id: string;
    interaction_summary: string;
    user_preferences: Record<string, unknown>;
    topic_tags: string[];
    interaction_count: number;
    last_interaction_at: string | null;
    created_at: string;
    updated_at: string;
  }>('xin_persona_memory', { persona_id: personaId }),
  // D3.8: 查询人格切换历史（前端人格切换时间线展示）
  personaSwitchHistory: (limit?: number) => ipc.invoke<Array<{
    id: number;
    from_persona_id: string | null;
    to_persona_id: string;
    switched_at: string;
    trigger: string;
  }>>('xin_persona_switch_history', { limit: limit ?? 50 }),
  // D3.8: 列出所有有记忆记录的人格 ID（按最后交互时间倒序）
  memorizedPersonas: () => ipc.invoke<string[]>('xin_memorized_personas'),
  // D3.8.3: 触发自生长人格生长（接收用户画像，返回生长结果）
  growPersona: (profile: {
    formality: number;
    verbosity: number;
    humor: number;
    technical_depth: number;
    empathy: number;
    interest_domains: string[];
    confidence: number;
  }) => ipc.invoke<{
    formality: number;
    verbosity: number;
    humor: number;
    technical_depth: number;
    empathy: number;
    traits: Array<{ name: string; value: number }>;
    growth_description: string;
  } | null>('xin_grow_persona', { profile }),
  // D3.8.3: 查询自生长人格当前状态（基于已积累记忆估算 confidence）
  selfGrowingPersona: () => ipc.invoke<{
    formality: number;
    verbosity: number;
    humor: number;
    technical_depth: number;
    empathy: number;
    traits: Array<{ name: string; value: number }>;
    growth_description: string;
  } | null>('xin_self_growing_persona'),
  dailyBriefing: () => ipc.invoke<any>('xin_daily_briefing'),
  // D3.3 STT：上传音频文件路径转写为文本
  voiceInput: (audioPath: string, language?: string, modelId?: number) => ipc.invoke<{
    text: string;
    engine: string;
  }>('xin_voice_input', {
    audio_path: audioPath,
    language,
    model_id: modelId
  }),
  // D3.2 TTS：合成语音到文件，返回本地路径（前端用 convertFileSrc 转 URL 后用 <audio> 播放）
  tts: (text: string, voice?: string, speed?: number, pitch?: number) => ipc.invoke<{
    audio_path: string;
    engine: string;
    voices: string[];
  }>('xin_tts', {
    text,
    voice,
    speed,
    pitch
  }),
  ttsListVoices: () => ipc.invoke<{
    engine: string;
    voices: string[];
  }>('xin_tts_list_voices'),
  // === D3.4b 多模态（xin_multimodal_service） ===
  // 规范：功能展望/模块深化/03_小欣_多模态融合_深度.md §2
  // 设计意图（§四）：小欣走云端 API 多模态模型（GPT-4o 等），非本地底层智能模型
  /**
   * 解析附件：图片→base64，文本→字符串，视频→标记（视频由 xin_video_analyze 处理）
   * 前端上传文件后先调用此命令解析，再作为 attachments 传给 dialogueSend
   */
  parseAttachment: (attachment: XinAttachment) => ipc.invoke<XinParsedAttachment>('xin_v3_parse_attachment', {
    attachment
  }),
  /**
   * 增强输出：检测代码块/表格/列表，返回显示提示（suggested_mode）
   * 前端收到小欣回复后调用，用于选择渲染模式（code_view/detailed_view/compact_view）
   */
  enhanceOutput: (text: string) => ipc.invoke<{
    content: string;
    display_hints: XinOutputEnhancement;
  }>('xin_v3_enhance_output', {
    text
  }),
  /**
   * 检测代码语言：从文本中提取 ```lang 标记
   * 用于语法高亮前置检测
   */
  detectCodeLanguages: (text: string) => ipc.invoke<string[]>('xin_v3_detect_code_languages', {
    text
  }),
  // === D3.5 视频输入多模态（xin_video_commands） ===
  // 规范：功能展望/模块深化/03_小欣_多模态融合_深度.md §2.4
  // 设计意图（§四）：小欣走云端 API 多模态模型，非本地底层智能模型
  /**
   * 分析视频：抽取关键帧返回 base64 图片列表
   * 前端拿到帧列表后可作为图片附件走多模态云端 API
   */
  videoAnalyze: (videoPath: string, maxFrames?: number) => ipc.invoke<{
    frames: Array<{
      timestamp_secs: number;
      image_b64: string;
      mime_type: string;
    }>;
    frame_count: number;
    duration_secs: number | null;
    audio_transcript: string | null;
  }>('xin_video_analyze', {
    video_path: videoPath,
    max_frames: maxFrames
  }),
  /**
   * D3.5 视频摘要：抽帧 + 调用云端多模态 LLM 生成文字摘要（一步到位）
   * 完整流程：ffmpeg 抽帧 → 多模态 LLM 生成摘要 + 关键要点
   * model_id 不传则用第一个可用模型
   */
  videoSummarize: (videoPath: string, maxFrames?: number, modelId?: number) => ipc.invoke<{
    summary: string;
    key_points: string[];
    frames: Array<{
      timestamp_secs: number;
      image_b64: string;
      mime_type: string;
    }>;
    frame_count: number;
    duration_secs: number | null;
    model_used: string;
  }>('xin_video_summarize', {
    video_path: videoPath,
    max_frames: maxFrames,
    model_id: modelId
  }),
  /** 检查系统 ffmpeg 是否可用（视频理解前置条件） */
  checkFfmpeg: () => ipc.invoke<boolean>('xin_video_check_ffmpeg')
};


/**
 * D3.6 实时对话 - IPC 命名空间
 *
 * 对齐后端 `commands/xin_realtime_commands.rs`。
 * 前端通过 AudioWorklet 采集 PCM 16kHz 单声道 → push_chunk 推送后端
 * 后端状态机：Idle → Listening → Thinking → Speaking → Listening
 *
 * 规范：功能展望/模块深化/03_小欣_多模态融合_深度.md §2.5
 * 设计意图（§四）：小欣走云端 API，实时对话 STT/LLM/TTS 均走云端流式 endpoint
 */
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

export const realtime = {
  /** 启动实时会话（状态 → listening） */
  start: (config: RealtimeConfig) => ipc.invoke<RealtimeState>('xin_realtime_start', { config }),
  /** 停止实时会话（状态 → idle） */
  stop: () => ipc.invoke<void>('xin_realtime_stop'),
  /** 推送 PCM 音频块（samples 为 i16 数组，每块 20-30ms） */
  pushChunk: (samples: number[], timestampMs: number) => ipc.invoke<RealtimeState>('xin_realtime_push_chunk', {
    samples,
    timestamp_ms: timestampMs
  }),
  /** 查询当前会话状态 */
  getState: () => ipc.invoke<RealtimeState>('xin_realtime_get_state')
};
