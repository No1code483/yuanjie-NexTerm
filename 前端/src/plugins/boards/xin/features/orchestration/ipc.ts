// plugins/boards/xin/features/orchestration/ipc.ts — xin.orchestration L2 IPC 客户端（短码 xo，批次4a-2）。
// 契约：统一经内核 dispatcher，逻辑名 `xo:plugin:<旧命令名>`；参数形状与迁移前
// 旧封装逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 87 条编排命令 + 3 条视频命令（4a-3）：context 7 / dialogue 7 / tools 10 / post 8 / fusion 5 / dream 8 /
// commit 9 / eval 4 / evolution 7 / proactive 6 / checkpoint 5 / review 4 / compaction 6 / video 3。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

/** 对话附件输入（4a-2 对话命令 `attachments` 参数形状） */
export interface XinAttachment {
  name: string;
  mime_type: string;
  path?: string;
  data_b64?: string;
  label?: string;
}

/** 解析后的附件（4a-2 `parseAttachment` 返回） */
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

/** 输出增强信息（4a-2 `enhanceOutput` 返回） */
export interface XinOutputEnhancement {
  has_code_blocks: boolean;
  code_languages: string[];
  has_tables: boolean;
  has_lists: boolean;
  content_length: number;
  suggested_mode: string;
}

/** dialogueSend 的强类型 options */
export interface XinDialogueSendOptions {
  model_id?: number;
  persona_id?: string;
  skills_enabled?: boolean;
  stream?: boolean;
  /** D3.4b 多模态附件列表（图片/文本/视频） */
  attachments?: XinAttachment[];
}

export const XIN_ORCHESTRATION_IPC_METHODS = {
  // context（7）+ prompt 构建（2）
  estimateTokens: { cmd: 'xin_v3_estimate_tokens' },
  contextNew: { cmd: 'xin_v3_context_new' },
  contextAddMessage: { cmd: 'xin_v3_context_add_message' },
  contextTrim: { cmd: 'xin_v3_context_trim' },
  contextStats: { cmd: 'xin_v3_context_stats' },
  compact: { cmd: 'xin_v3_compact' },
  recoveryBriefing: { cmd: 'xin_v3_recovery_briefing' },
  buildPrompt: { cmd: 'xin_v3_build_prompt' },
  buildContextHeader: { cmd: 'xin_v3_build_context_header' },
  // tools（10）
  listSkills: { cmd: 'xin_v3_list_skills' },
  executeSkill: { cmd: 'xin_v3_execute_skill' },
  listTools: { cmd: 'xin_v3_list_tools' },
  parseToolCalls: { cmd: 'xin_v3_parse_tool_calls' },
  executeTool: { cmd: 'xin_v3_execute_tool' },
  getToolsPrompt: { cmd: 'xin_v3_get_tools_prompt' },
  parseAttachment: { cmd: 'xin_v3_parse_attachment' },
  enhanceOutput: { cmd: 'xin_v3_enhance_output' },
  detectCodeLanguages: { cmd: 'xin_v3_detect_code_languages' },
  // dialogue（7）
  dialogueCreate: { cmd: 'xin_v3_dialogue_create' },
  dialogueGet: { cmd: 'xin_v3_dialogue_get' },
  dialogueList: { cmd: 'xin_v3_dialogue_list' },
  dialogueDelete: { cmd: 'xin_v3_dialogue_delete' },
  dialogueSearch: { cmd: 'xin_v3_dialogue_search' },
  dialogueSend: { cmd: 'xin_v3_dialogue_send' },
  dialogueStop: { cmd: 'xin_v3_dialogue_stop' },
  // post process —— 分析/情感/话题（3）+ 增强/质量/安全（4）+ pipeline（1）
  analyzeIntent: { cmd: 'xin_v3_analyze_intent' },
  analyzeSentiment: { cmd: 'xin_v3_analyze_sentiment' },
  extractTopics: { cmd: 'xin_v3_extract_topics' },
  postProcess: { cmd: 'xin_v3_post_process' },
  postScoreQuality: { cmd: 'xin_v3_post_score_quality' },
  postAdjustTone: { cmd: 'xin_v3_post_adjust_tone' },
  postCheckFactuality: { cmd: 'xin_v3_post_check_factuality' },
  postSafetyFilter: { cmd: 'xin_v3_post_safety_filter' },
  postEnhancedPipeline: { cmd: 'xin_v3_post_enhanced_pipeline' },
  // knowledge fusion（5）
  knowledgeFusion: { cmd: 'xin_v3_knowledge_fusion' },
  fusionShouldRetrieve: { cmd: 'xin_v3_fusion_should_retrieve' },
  fusionMemoryQuery: { cmd: 'xin_v3_fusion_memory_query' },
  fusionMultiSource: { cmd: 'xin_v3_fusion_multi_source' },
  fusionUnifiedContext: { cmd: 'xin_v3_fusion_unified_context' },
  fusionExtractKeywords: { cmd: 'xin_v3_fusion_extract_keywords' },
  // dream（8）
  dreamCheckDue: { cmd: 'xin_v3_dream_check_due' },
  dreamRunLight: { cmd: 'xin_v3_dream_run_light' },
  dreamRunDeep: { cmd: 'xin_v3_dream_run_deep' },
  dreamRunRem: { cmd: 'xin_v3_dream_run_rem' },
  dreamCalcHealth: { cmd: 'xin_v3_dream_calc_health' },
  dreamAutoPromote: { cmd: 'xin_v3_dream_auto_promote' },
  dreamDefaultConfig: { cmd: 'xin_v3_dream_default_config' },
  // commit（9）
  commitExtract: { cmd: 'xin_v3_commit_extract' },
  commitCreate: { cmd: 'xin_v3_commit_create' },
  commitListPending: { cmd: 'xin_v3_commit_list_pending' },
  commitCheckDue: { cmd: 'xin_v3_commit_check_due' },
  commitMarkDone: { cmd: 'xin_v3_commit_mark_done' },
  commitSnooze: { cmd: 'xin_v3_commit_snooze' },
  commitDismiss: { cmd: 'xin_v3_commit_dismiss' },
  commitStats: { cmd: 'xin_v3_commit_stats' },
  // eval（4）
  evalScore: { cmd: 'xin_v3_eval_score' },
  evalHistory: { cmd: 'xin_v3_eval_history' },
  evalStats: { cmd: 'xin_v3_eval_stats' },
  evalReport: { cmd: 'xin_v3_eval_report' },
  // evolution（7）
  evolutionRules: { cmd: 'xin_v3_evolution_rules' },
  evolutionAnalyze: { cmd: 'xin_v3_evolution_analyze' },
  evolutionApply: { cmd: 'xin_v3_evolution_apply' },
  evolutionVariantCreate: { cmd: 'xin_v3_evolution_variant_create' },
  evolutionVariantCompare: { cmd: 'xin_v3_evolution_variant_compare' },
  evolutionSnapshot: { cmd: 'xin_v3_evolution_snapshot' },
  evolutionTimeline: { cmd: 'xin_v3_evolution_timeline' },
  // proactive（6）
  proactivePending: { cmd: 'xin_v3_proactive_pending' },
  proactiveCareCheck: { cmd: 'xin_v3_proactive_care_check' },
  proactiveUrgency: { cmd: 'xin_v3_proactive_urgency' },
  proactiveQuietHours: { cmd: 'xin_v3_proactive_quiet_hours' },
  proactiveDailyNudge: { cmd: 'xin_v3_proactive_daily_nudge' },
  proactiveMorningContext: { cmd: 'xin_v3_proactive_morning_context' },
  // checkpoint（5）
  checkpointSave: { cmd: 'xin_v3_checkpoint_save' },
  checkpointList: { cmd: 'xin_v3_checkpoint_list' },
  checkpointRestore: { cmd: 'xin_v3_checkpoint_restore' },
  checkpointDelete: { cmd: 'xin_v3_checkpoint_delete' },
  checkpointCleanup: { cmd: 'xin_v3_checkpoint_cleanup' },
  // review（4）
  reviewGenerate: { cmd: 'xin_v3_review_generate' },
  reviewTopicTrends: { cmd: 'xin_v3_review_topic_trends' },
  reviewGrowthTrajectory: { cmd: 'xin_v3_review_growth_trajectory' },
  reviewHeatmap: { cmd: 'xin_v3_review_heatmap' },
  // compaction（6）
  compactionGetConfig: { cmd: 'xin_v3_compaction_get_config' },
  compactionUpdateConfig: { cmd: 'xin_v3_compaction_update_config' },
  compactionGetRecords: { cmd: 'xin_v3_compaction_get_records' },
  compactionNeedsCheck: { cmd: 'xin_v3_compaction_needs_check' },
  compactionAuto: { cmd: 'xin_v3_compaction_auto' },
  compactionManual: { cmd: 'xin_v3_compaction_manual' },
  // video（3）—— 4a-3：D3.5 小欣视频输入多模态
  videoAnalyze: { cmd: 'xin_video_analyze' },
  videoSummarize: { cmd: 'xin_video_summarize' },
  checkFfmpeg: { cmd: 'xin_video_check_ffmpeg' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherXinOrchestration = defineIpcNamespace('xo', XIN_ORCHESTRATION_IPC_METHODS);
type XinOrchestrationMethod = keyof typeof XIN_ORCHESTRATION_IPC_METHODS;

function invokeXinOrchestration<T = any>(
  method: XinOrchestrationMethod,
  args?: Record<string, unknown>,
): Promise<ApiResponse<T>> {
  return dispatcherXinOrchestration[method](args) as Promise<ApiResponse<T>>;
}

export const xinOrchestration = {
  // ===== context / prompt 构建 =====
  estimateTokens: (text: string) => invokeXinOrchestration<number>('estimateTokens', { text }),
  contextNew: (modelId: string) => invokeXinOrchestration<any>('contextNew', { modelId }),
  contextAddMessage: (modelId: string, messagesJson: string, role: string, content: string, name?: string) =>
    invokeXinOrchestration<any>('contextAddMessage', { modelId, messagesJson, role, content, name }),
  contextTrim: (modelId: string, messagesJson: string) =>
    invokeXinOrchestration<any>('contextTrim', { modelId, messagesJson }),
  contextStats: (modelId: string, messagesJson: string) =>
    invokeXinOrchestration<any>('contextStats', { modelId, messagesJson }),
  compact: (messagesJson: string, modelId: string) =>
    invokeXinOrchestration<any>('compact', { messagesJson, modelId }),
  recoveryBriefing: (messagesJson: string) =>
    invokeXinOrchestration<any>('recoveryBriefing', { messagesJson }),
  buildPrompt: (opts?: { includeSkills?: boolean; includeMood?: boolean; includeTime?: boolean; customInstructions?: string }) =>
    invokeXinOrchestration<any>('buildPrompt', opts ?? {}),
  buildContextHeader: (opts?: { summary?: string; recentTopics?: string[] }) =>
    invokeXinOrchestration<any>('buildContextHeader', opts ?? {}),

  // ===== tools / multimodal =====
  listSkills: () => invokeXinOrchestration<any[]>('listSkills'),
  executeSkill: (skillId: string, input: string) =>
    invokeXinOrchestration<any>('executeSkill', { skillId, input }),
  listTools: () => invokeXinOrchestration<any[]>('listTools'),
  parseToolCalls: (text: string) => invokeXinOrchestration<any[]>('parseToolCalls', { text }),
  executeTool: (callsJson: string) => invokeXinOrchestration<any[]>('executeTool', { callsJson }),
  getToolsPrompt: () => invokeXinOrchestration<string>('getToolsPrompt'),
  parseAttachment: (attachment: XinAttachment) =>
    invokeXinOrchestration<XinParsedAttachment>('parseAttachment', { attachment }),
  enhanceOutput: (text: string) =>
    invokeXinOrchestration<{ content: string; displayHints: XinOutputEnhancement }>('enhanceOutput', { text }),
  detectCodeLanguages: (text: string) => invokeXinOrchestration<string[]>('detectCodeLanguages', { text }),

  // ===== dialogue =====
  dialogueCreate: (opts?: { modelId?: string; personaId?: string; title?: string }) =>
    invokeXinOrchestration<any>('dialogueCreate', opts ?? {}),
  dialogueGet: (conversationId: string) =>
    invokeXinOrchestration<any>('dialogueGet', { conversationId }),
  dialogueList: (limit: number = 50, offset: number = 0) =>
    invokeXinOrchestration<any>('dialogueList', { limit, offset }),
  dialogueDelete: (conversationId: string) =>
    invokeXinOrchestration<void>('dialogueDelete', { conversationId }),
  dialogueSearch: (query: any, limit: number = 20) =>
    invokeXinOrchestration<any>('dialogueSearch', { query, limit }),
  dialogueSend: (
    conversationId: string | null,
    content: string,
    options?: XinDialogueSendOptions,
  ) =>
    invokeXinOrchestration<any>('dialogueSend', { conversationId, content, ...options }),
  dialogueStop: (conversationId: string) =>
    invokeXinOrchestration<void>('dialogueStop', { conversationId }),

  // ===== post process =====
  analyzeIntent: (message: string) => invokeXinOrchestration<any>('analyzeIntent', { message }),
  analyzeSentiment: (text: string) => invokeXinOrchestration<any>('analyzeSentiment', { text }),
  extractTopics: (userMessage: string, assistantResponse: string, previousTopics?: any) =>
    invokeXinOrchestration<any>('extractTopics', { userMessage, assistantResponse, previousTopics }),
  postProcess: (userMessage: string, assistantResponse: string, previousTopics?: any) =>
    invokeXinOrchestration<any>('postProcess', { userMessage, assistantResponse, previousTopics }),
  postScoreQuality: (userMessage: string, assistantResponse: string) =>
    invokeXinOrchestration<any>('postScoreQuality', { userMessage, assistantResponse }),
  postAdjustTone: (response: string, warmth?: number, formality?: number, enthusiasm?: number) =>
    invokeXinOrchestration<any>('postAdjustTone', { response, warmth, formality, enthusiasm }),
  postCheckFactuality: (response: string) =>
    invokeXinOrchestration<any>('postCheckFactuality', { response }),
  postSafetyFilter: (response: string) =>
    invokeXinOrchestration<any>('postSafetyFilter', { response }),
  postEnhancedPipeline: (
    userMessage: string,
    assistantResponse: string,
    previousTopics?: any,
    warmth?: number,
    formality?: number,
    enthusiasm?: number,
  ) =>
    invokeXinOrchestration<any>('postEnhancedPipeline', {
      userMessage,
      assistantResponse,
      previousTopics,
      warmth,
      formality,
      enthusiasm,
    }),

  // ===== knowledge fusion =====
  knowledgeFusion: (message: string, maxResults?: number) =>
    invokeXinOrchestration<any>('knowledgeFusion', { message, maxResults }),
  fusionShouldRetrieve: (message: string) =>
    invokeXinOrchestration<boolean>('fusionShouldRetrieve', { message }),
  fusionMemoryQuery: (query: string, maxResults?: number) =>
    invokeXinOrchestration<any[]>('fusionMemoryQuery', { query, maxResults }),
  fusionMultiSource: (kbJson: any, memoryJson: any, maxTotal?: number) =>
    invokeXinOrchestration<any>('fusionMultiSource', { kbJson, memoryJson, maxTotal }),
  fusionUnifiedContext: (resultsJson: any, maxChars?: number) =>
    invokeXinOrchestration<any>('fusionUnifiedContext', { resultsJson, maxChars }),
  fusionExtractKeywords: (message: string) =>
    invokeXinOrchestration<string[]>('fusionExtractKeywords', { message }),

  // ===== dream =====
  dreamCheckDue: (phase: string, dreamState: any, config: any) =>
    invokeXinOrchestration<any>('dreamCheckDue', { phase, dreamState, config }),
  dreamRunLight: (conversationTexts?: string[], config?: any) =>
    invokeXinOrchestration<any>('dreamRunLight', { conversationTexts, config }),
  dreamRunDeep: (candidates: any, memories: any, config: any) =>
    invokeXinOrchestration<any>('dreamRunDeep', { candidates, memories, config }),
  dreamRunRem: (memories: any, config: any) =>
    invokeXinOrchestration<any>('dreamRunRem', { memories, config }),
  dreamCalcHealth: (memories: any) => invokeXinOrchestration<any>('dreamCalcHealth', { memories }),
  dreamAutoPromote: (candidates: any) => invokeXinOrchestration<any>('dreamAutoPromote', { candidates }),
  dreamDefaultConfig: () => invokeXinOrchestration<any>('dreamDefaultConfig'),

  // ===== commit =====
  commitExtract: (userMsg: string, assistantMsg: string, conversationId: string) =>
    invokeXinOrchestration<any>('commitExtract', { userMsg, assistantMsg, conversationId }),
  commitCreate: (candidates: any) => invokeXinOrchestration<any>('commitCreate', { candidates }),
  commitListPending: () => invokeXinOrchestration<any[]>('commitListPending'),
  commitCheckDue: (commitments: any) => invokeXinOrchestration<any>('commitCheckDue', { commitments }),
  commitMarkDone: (commitment: any) => invokeXinOrchestration<void>('commitMarkDone', { commitment }),
  commitSnooze: (commitment: any, hours: number = 24) =>
    invokeXinOrchestration<void>('commitSnooze', { commitment, hours }),
  commitDismiss: (commitment: any) => invokeXinOrchestration<void>('commitDismiss', { commitment }),
  commitStats: (commitments: any) => invokeXinOrchestration<any>('commitStats', { commitments }),

  // ===== eval =====
  evalScore: (userMsg: string, assistantMsg: string, responseTimeMs?: number) =>
    invokeXinOrchestration<any>('evalScore', { userMsg, assistantMsg, responseTimeMs: responseTimeMs ?? 0 }),
  evalHistory: (results: any, count?: number) =>
    invokeXinOrchestration<any[]>('evalHistory', { results, count: count ?? 10 }),
  evalStats: (results: any) => invokeXinOrchestration<any>('evalStats', { results }),
  evalReport: (results: any) => invokeXinOrchestration<any>('evalReport', { results }),

  // ===== evolution =====
  evolutionRules: () => invokeXinOrchestration<any[]>('evolutionRules'),
  evolutionAnalyze: (evalResults: any, styleJson: any) =>
    invokeXinOrchestration<any>('evolutionAnalyze', { evalResults, styleJson }),
  evolutionApply: (styleJson: any, decisionJson: any) =>
    invokeXinOrchestration<any>('evolutionApply', { styleJson, decisionJson }),
  evolutionVariantCreate: (
    basePersonaId: string,
    name: string,
    styleJson: any,
    traitsJson: any,
    adjustmentsJson: any,
    description: string,
  ) =>
    invokeXinOrchestration<any>('evolutionVariantCreate', {
      basePersonaId,
      name,
      styleJson,
      traitsJson,
      adjustmentsJson,
      description,
    }),
  evolutionVariantCompare: (variantA: any, variantB: any, evalA: any, evalB: any) =>
    invokeXinOrchestration<any>('evolutionVariantCompare', { variantA, variantB, evalA, evalB }),
  evolutionSnapshot: (
    personaId: string,
    styleJson: any,
    traitsJson: any,
    label: string,
    variantId?: string,
  ) =>
    invokeXinOrchestration<any>('evolutionSnapshot', { personaId, variantId, styleJson, traitsJson, label }),
  evolutionTimeline: (
    personaId: string,
    previousJson: any,
    newJson: any,
    reason: string,
    evalResults?: any,
  ) =>
    invokeXinOrchestration<any>('evolutionTimeline', {
      personaId,
      previousJson,
      newJson,
      reason,
      evalResults,
    }),

  // ===== proactive =====
  proactivePending: (opts?: {
    careCheckIn?: string;
    commitments?: any;
    habits?: any;
    persona?: any;
  }) => invokeXinOrchestration<any>('proactivePending', opts ?? {}),
  proactiveCareCheck: (resultsJson: any) =>
    invokeXinOrchestration<any>('proactiveCareCheck', { resultsJson }),
  proactiveUrgency: (commitmentsJson: any) =>
    invokeXinOrchestration<any>('proactiveUrgency', { commitmentsJson }),
  proactiveQuietHours: (quietHours?: any) =>
    invokeXinOrchestration<any>('proactiveQuietHours', { quietHours }),
  proactiveDailyNudge: (personaJson: any, commitmentsJson: any) =>
    invokeXinOrchestration<any>('proactiveDailyNudge', { personaJson, commitmentsJson }),
  proactiveMorningContext: (personaJson: any, commitmentsJson: any) =>
    invokeXinOrchestration<any>('proactiveMorningContext', { personaJson, commitmentsJson }),

  // ===== checkpoint =====
  checkpointSave: (conversationId: string, title?: string) =>
    invokeXinOrchestration<any>('checkpointSave', { conversationId, title }),
  checkpointList: (conversationId: string) =>
    invokeXinOrchestration<any[]>('checkpointList', { conversationId }),
  checkpointRestore: (checkpointId: string) =>
    invokeXinOrchestration<void>('checkpointRestore', { checkpointId }),
  checkpointDelete: (checkpointId: string) =>
    invokeXinOrchestration<void>('checkpointDelete', { checkpointId }),
  checkpointCleanup: () => invokeXinOrchestration<void>('checkpointCleanup'),

  // ===== review =====
  reviewGenerate: (request: any) => invokeXinOrchestration<any>('reviewGenerate', { request }),
  reviewTopicTrends: (personaId?: string, periodType?: string, buckets?: any) =>
    invokeXinOrchestration<any>('reviewTopicTrends', { personaId, periodType, buckets }),
  reviewGrowthTrajectory: (personaId?: string, periodType?: string, buckets?: any, evalResultsJson?: any) =>
    invokeXinOrchestration<any>('reviewGrowthTrajectory', { personaId, periodType, buckets, evalResultsJson }),
  reviewHeatmap: (personaId?: string, periodType?: string) =>
    invokeXinOrchestration<any>('reviewHeatmap', { personaId, periodType }),

  // ===== compaction =====
  compactionGetConfig: () => invokeXinOrchestration<any>('compactionGetConfig'),
  compactionUpdateConfig: (config: Record<string, unknown>) =>
    invokeXinOrchestration<void>('compactionUpdateConfig', { config }),
  compactionGetRecords: (conversationId: string, limit?: number) =>
    invokeXinOrchestration<any[]>('compactionGetRecords', { conversationId, limit }),
  compactionNeedsCheck: (contextJson: string) =>
    invokeXinOrchestration<any>('compactionNeedsCheck', { contextJson }),
  compactionAuto: (conversationId: string, contextJson: string, modelId: string) =>
    invokeXinOrchestration<void>('compactionAuto', { conversationId, contextJson, modelId }),
  compactionManual: (
    conversationId: string,
    contextJson: string,
    modelId: string,
    guidance?: string,
  ) =>
    invokeXinOrchestration<void>('compactionManual', { conversationId, contextJson, modelId, guidance }),

  // ===== video（3）—— 4a-3：D3.5 小欣视频输入多模态 =====
  videoAnalyze: (videoPath: string, maxFrames?: number) =>
    invokeXinOrchestration<{
      frames: Array<{ timestamp_secs: number; image_b64: string; mime_type: string }>;
      frame_count: number;
      duration_secs: number | null;
      audio_transcript: string | null;
    }>('videoAnalyze', { videoPath, maxFrames }),
  videoSummarize: (videoPath: string, maxFrames?: number, modelId?: number) =>
    invokeXinOrchestration<{
      summary: string;
      key_points: string[];
      frames: Array<{ timestamp_secs: number; image_b64: string; mime_type: string }>;
      frame_count: number;
      duration_secs: number | null;
      model_used: string;
    }>('videoSummarize', { videoPath, maxFrames, modelId }),
  checkFfmpeg: () => invokeXinOrchestration<boolean>('checkFfmpeg'),
};

