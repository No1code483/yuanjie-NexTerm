// plugins/customs/intelligence/ipc/intelligence.ts — customs.intelligence L1 IPC 客户端（短码 sp，批次6c）。
// 契约：统一经内核 dispatcher，逻辑名 `sp:plugin:<旧命令名>`；参数形状与迁移前
// 旧封装逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 74 条 alias：intelligence 25 + v4 38 + behavior 6 + anomaly 5。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';
import type {
  DashboardData,
  RealtimeStats,
  Suggestion,
  BehaviorReport,
  ActivityLog,
  ActivityStats,
} from '@/types';

export const INTELLIGENCE_IPC_METHODS = {
  // ===== intelligence v1 (25) =====
  getConfig: { cmd: 'intelligence_get_config' },
  getEnabled: { cmd: 'intelligence_get_enabled' },
  setEnabled: { cmd: 'intelligence_set_enabled' },
  triggerProactiveActions: { cmd: 'intelligence_trigger_proactive_actions' },
  setConfig: { cmd: 'intelligence_set_config' },
  getContext: { cmd: 'intelligence_get_context' },
  trackActivity: { cmd: 'intelligence_track_activity' },
  setActiveFile: { cmd: 'intelligence_set_active_file' },
  addTerminalSession: { cmd: 'intelligence_add_terminal_session' },
  removeTerminalSession: { cmd: 'intelligence_remove_terminal_session' },
  setWindowTitle: { cmd: 'intelligence_set_window_title' },
  getOllamaStatus: { cmd: 'intelligence_get_ollama_status' },
  getSuggestions: { cmd: 'intelligence_get_suggestions' },
  takeSnapshot: { cmd: 'intelligence_take_snapshot' },
  getSnapshots: { cmd: 'intelligence_get_snapshots' },
  queryLocalLlm: { cmd: 'intelligence_query_local_llm' },
  terminalSuggest: { cmd: 'intelligence_terminal_suggest' },
  resumePolish: { cmd: 'intelligence_resume_polish' },
  newsSummary: { cmd: 'intelligence_news_summary' },
  todoSuggest: { cmd: 'intelligence_todo_suggest' },
  timerRemind: { cmd: 'intelligence_timer_remind' },
  kbClassify: { cmd: 'intelligence_kb_classify' },
  dailyBriefing: { cmd: 'intelligence_daily_briefing' },
  chatSuggest: { cmd: 'intelligence_chat_suggest' },
  gameSuggest: { cmd: 'intelligence_game_suggest' },

  // ===== intelligence_v4 (38) =====
  v4LogActivity: { cmd: 'intelligence_v4_log_activity' },
  v4ResumeSpellCheck: { cmd: 'intelligence_v4_resume_spell_check' },
  v4ResumePolish: { cmd: 'intelligence_v4_resume_polish' },
  v4ResumeGenerate: { cmd: 'intelligence_v4_resume_generate' },
  v4QuoteSpellCheck: { cmd: 'intelligence_v4_quote_spell_check' },
  v4QuoteSourceVerify: { cmd: 'intelligence_v4_quote_source_verify' },
  v4QuoteSmartComplete: { cmd: 'intelligence_v4_quote_smart_complete' },
  v4NewsSummarize: { cmd: 'intelligence_v4_news_summarize' },
  v4TodoEnhance: { cmd: 'intelligence_v4_todo_enhance' },
  v4JournalFill: { cmd: 'intelligence_v4_journal_fill' },
  v4TimerRemind: { cmd: 'intelligence_v4_timer_remind' },
  v4KbClassify: { cmd: 'intelligence_v4_kb_classify' },
  v4TerminalComplete: { cmd: 'intelligence_v4_terminal_complete' },
  v4GameRecommend: { cmd: 'intelligence_v4_game_recommend' },
  v4SearchAnalyze: { cmd: 'intelligence_v4_search_analyze' },
  v4GetSettings: { cmd: 'intelligence_v4_get_settings' },
  v4SaveSettings: { cmd: 'intelligence_v4_save_settings' },
  v4ResetSettings: { cmd: 'intelligence_v4_reset_settings' },
  v4TestConnection: { cmd: 'intelligence_v4_test_connection' },
  v4ExportActivityLogs: { cmd: 'intelligence_v4_export_activity_logs' },
  v4GetOperationTemplates: { cmd: 'intelligence_v4_get_operation_templates' },
  v4LogWithTemplate: { cmd: 'intelligence_v4_log_with_template' },
  v4BatchLogActivity: { cmd: 'intelligence_v4_batch_log_activity' },
  v4QueryActivityLogs: { cmd: 'intelligence_v4_query_activity_logs' },
  v4ActivityStats: { cmd: 'intelligence_v4_activity_stats' },
  v4CleanActivityLogs: { cmd: 'intelligence_v4_clean_activity_logs' },
  v4ClearAllActivityLogs: { cmd: 'intelligence_v4_clear_all_activity_logs' },
  v4Dashboard: { cmd: 'intelligence_v4_dashboard' },
  v4RealtimeStats: { cmd: 'intelligence_v4_realtime_stats' },
  v4GenerateSuggestions: { cmd: 'intelligence_v4_generate_suggestions' },
  v4GetSuggestions: { cmd: 'intelligence_v4_get_suggestions' },
  v4MarkSuggestion: { cmd: 'intelligence_v4_mark_suggestion' },
  v4CleanSuggestions: { cmd: 'intelligence_v4_clean_suggestions' },
  v4AnalyzeBehavior: { cmd: 'intelligence_v4_analyze_behavior' },
  v4BehaviorTrend: { cmd: 'intelligence_v4_behavior_trend' },
  v4BehaviorHistory: { cmd: 'intelligence_v4_behavior_history' },
  v4KbSummarize: { cmd: 'intelligence_v4_kb_summarize' },
  v4KbTags: { cmd: 'intelligence_v4_kb_tags' },

  // ===== intelligence_behavior v2 (6) =====
  v2AnalyzeBehavior: { cmd: 'intelligence_v2_analyze_behavior' },
  v2RecommendWorkflows: { cmd: 'intelligence_v2_recommend_workflows' },
  v2DetectTechStack: { cmd: 'intelligence_v2_detect_tech_stack' },
  v2CognitiveLoad: { cmd: 'intelligence_v2_cognitive_load' },
  v2Notifications: { cmd: 'intelligence_v2_notifications' },
  v2Dashboard: { cmd: 'intelligence_v2_dashboard' },

  // ===== intelligence_anomaly v3 (5) =====
  v3DetectAnomaly: { cmd: 'intelligence_v3_detect_anomaly' },
  v3SnapshotResources: { cmd: 'intelligence_v3_snapshot_resources' },
  v3OrganizeKnowledge: { cmd: 'intelligence_v3_organize_knowledge' },
  v3ListScheduledTasks: { cmd: 'intelligence_v3_list_scheduled_tasks' },
  v3ExecuteScheduledTasks: { cmd: 'intelligence_v3_execute_scheduled_tasks' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherSp = defineIpcNamespace('sp', INTELLIGENCE_IPC_METHODS);
type IntelligenceMethod = keyof typeof INTELLIGENCE_IPC_METHODS;

function invokeSp<T>(method: IntelligenceMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherSp[method](args) as Promise<ApiResponse<T>>;
}

export const intelligence = {
  // ===== v4 仪表盘 =====
  getDashboard: (userId: string, period: string) =>
    invokeSp<DashboardData>('v4Dashboard', { userId, period }),
  getRealtimeStats: (userId: string) =>
    invokeSp<RealtimeStats>('v4RealtimeStats', { userId }),

  // ===== v4 智能建议 =====
  generateSuggestions: () => invokeSp<Suggestion[]>('v4GenerateSuggestions'),
  getSuggestions: (userId: string, status?: string, category?: string, limit?: number, offset?: number) =>
    invokeSp<{ suggestions: Suggestion[]; total: number; page: number; page_size: number }>('v4GetSuggestions', {
      userId, status, category, limit, offset,
    }),
  markSuggestion: (suggestionId: number, action: string) =>
    invokeSp<void>('v4MarkSuggestion', { suggestionId, action }),

  // ===== v4 行为分析 =====
  analyzeBehavior: (userId: string, date?: string) =>
    invokeSp<BehaviorReport>('v4AnalyzeBehavior', { userId, date }),
  getBehaviorTrend: (userId: string, days?: number) =>
    invokeSp<{ dates: string[]; focus_scores: number[]; distraction_counts: number[]; consistency_scores: number[] }>('v4BehaviorTrend', { userId, days }),
  getBehaviorHistory: (userId: string, startDate?: string, endDate?: string) =>
    invokeSp<Array<{ date: string; focus_score: number; focus_verdict: string; summary: string }>>('v4BehaviorHistory', { userId, startDate, endDate }),

  // ===== v4 活动日志 =====
  queryActivityLogs: (params: {
    userId: string; module?: string; startTime?: string; endTime?: string; limit?: number; offset?: number;
  }) =>
    invokeSp<{ logs: ActivityLog[]; total: number; page: number; page_size: number }>('v4QueryActivityLogs', {
      userId: params.userId, module: params.module, startTime: params.startTime,
      endTime: params.endTime, limit: params.limit, offset: params.offset,
    }),
  getActivityStats: (userId: string, period?: string) =>
    invokeSp<ActivityStats>('v4ActivityStats', { userId, period }),
  clearAllActivityLogs: () => invokeSp<number>('v4ClearAllActivityLogs'),

  // ===== v4 设置 =====
  getSettings: () =>
    invokeSp<{
      master_switch: boolean; auto_suggest: boolean; context_collection: boolean;
      activity_tracking: boolean; focus_detection: boolean; smart_complete: boolean;
      auto_classify: boolean; news_summary: boolean; quote_check: boolean;
      command_suggest: boolean; game_recommend: boolean; search_enhance: boolean;
      timer_smart: boolean; floating_ball: boolean; llm_provider: string;
      llm_endpoint: string; llm_model: string; llm_api_key: string;
    }>('v4GetSettings'),
  saveSettings: (settings: Record<string, unknown>) =>
    invokeSp<void>('v4SaveSettings', { settings }),
  resetSettings: (userId?: string) => invokeSp<void>('v4ResetSettings', { userId }),
  testConnection: (provider: string, endpoint: string, apiKey?: string, model?: string) =>
    invokeSp<{ success: boolean; latency_ms: number; message: string }>('v4TestConnection', {
      request: { provider, endpoint, api_key: apiKey, model },
    }),

  // ===== v4 跨模块智能 =====
  kbClassify: (entryName: string, content: string, existingCategories?: string[]) =>
    invokeSp<any[]>('v4KbClassify', {
      request: { entry_name: entryName, content, existing_categories: existingCategories },
    }),
  resumeSpellCheck: (content: string) =>
    invokeSp<{ original: string | null; polished: string; changes: string[]; suggestions: string[] }>('v4ResumeSpellCheck', { content }),
  resumePolish: (content: string) =>
    invokeSp<{ original: string | null; polished: string; changes: string[]; suggestions: string[] }>('v4ResumePolish', { content }),
  resumeGenerate: (name?: string, education?: string, skills?: string[], experience?: string[]) =>
    invokeSp<{ original: string | null; polished: string; changes: string[]; suggestions: string[] }>('v4ResumeGenerate', { name, education, skills, experience }),
  quoteSpellCheck: (content: string) =>
    invokeSp<{ field: string; issue: string; suggestion: string | null; severity: string }>('v4QuoteSpellCheck', { content }),
  quoteSourceVerify: (author?: string, source?: string) =>
    invokeSp<{ field: string; issue: string; suggestion: string | null; severity: string }>('v4QuoteSourceVerify', { author, source }),
  quoteSmartComplete: (content: string, author?: string, source?: string) =>
    invokeSp<{ field: string; issue: string; suggestion: string | null; severity: string }>('v4QuoteSmartComplete', { content, author, source }),
  newsSummarize: (title: string, content: string) =>
    invokeSp<{ title: string; summary: string; keywords: string[]; original_word_count: number; summary_word_count: number }>('v4NewsSummarize', { title, content }),
  todoEnhance: (title: string, description?: string) =>
    invokeSp<{ suggest_priority: string; suggest_estimate_minutes: number; suggest_category: string | null; suggestions: string[] }>('v4TodoEnhance', { title, description }),
  journalFill: (todayEvents?: string) =>
    invokeSp<{ template: string; suggestions: string[] }>('v4JournalFill', { today_events: todayEvents }),
  timerRemind: (currentElapsedSecs: number, taskType?: string) =>
    invokeSp<{ suggest_break: boolean; suggest_stop: boolean; suggestions: string[]; optimal_session_minutes: number }>('v4TimerRemind', {
      current_elapsed_secs: currentElapsedSecs, task_type: taskType,
    }),
  terminalComplete: (partialInput?: string, currentDir?: string, recentHistory?: string[]) =>
    invokeSp<any[]>('v4TerminalComplete', {
      context: { partial_input: partialInput, current_dir: currentDir, recent_history: recentHistory || [] },
    }),
  gameRecommend: (playDurationTodaySecs?: number, currentTimeHour?: number, recentGames?: string[]) =>
    invokeSp<any>('v4GameRecommend', {
      play_duration_today_secs: playDurationTodaySecs, current_time_hour: currentTimeHour, recent_games: recentGames,
    }),
  searchAnalyze: (query: string) =>
    invokeSp<any>('v4SearchAnalyze', { query }),

  // ===== v1 D2.1 可关闭性 + 触发执行式 =====
  getEnabled: () => invokeSp<boolean>('getEnabled'),
  setEnabled: (enabled: boolean) => invokeSp<null>('setEnabled', { enabled }),
  triggerProactiveActions: () =>
    invokeSp<any[]>('triggerProactiveActions'),

  // ===== v1 每日简报 =====
  dailyBriefing: () =>
    invokeSp<{
      date: string; briefing: string; todo_count: number; todo_completed: number;
      journal_words: number; news_count: number; timer_sessions: number;
      total_focus_seconds: number; generated_at: string;
    }>('dailyBriefing'),

  // ===== v1 知识库智能体 =====
  kbSummarize: (entryName: string, content: string, llmConfig: {
    provider: string; endpoint: string; api_key?: string; model?: string;
  }, entryId?: string) =>
    invokeSp<{ summary: string; key_points: string[]; original_length: number; summary_length: number }>('v4KbSummarize', {
      request: {
        entry_id: entryId, entry_name: entryName, content,
        provider: llmConfig.provider, endpoint: llmConfig.endpoint,
        api_key: llmConfig.api_key, model: llmConfig.model,
      },
    }),
  kbTags: (content: string, existingTags: string[], llmConfig: {
    provider: string; endpoint: string; api_key?: string; model?: string;
  }) =>
    invokeSp<{ suggested_tags: string[] }>('v4KbTags', {
      request: {
        content, existing_tags: existingTags,
        provider: llmConfig.provider, endpoint: llmConfig.endpoint,
        api_key: llmConfig.api_key, model: llmConfig.model,
      },
    }),
  logActivity: (userId: string, timestamp: string, module: string, operation: string, detail?: string, remark?: string, durationSecs?: number) =>
    invokeSp<ActivityLog>('v4LogActivity', {
      record: {
        userId, timestamp, module, operation, detail, remark, durationSecs,
      },
    }),
  // legacy single-entry classify (intelligence_kb_classify)
  classifyKbEntryById: (entryId: number | null, content: string) =>
    invokeSp<{ category_name: string; confidence: number; reason: string; description: string }>('kbClassify', {
      entry_id: entryId,
      content,
    }),
};