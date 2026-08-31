// ipc/intelligence.ts — intelligence 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）
import { ipc } from './core';
import type { DashboardData, RealtimeStats, Suggestion, BehaviorReport, ActivityLog, ActivityStats } from '@/types';

export const intelligence = {
  // ===== 仪表盘 =====
  getDashboard: (userId: string, period: string) => ipc.invoke<DashboardData>('intelligence_v4_dashboard', {
    userId,
    period
  }),
  getRealtimeStats: (userId: string) => ipc.invoke<RealtimeStats>('intelligence_v4_realtime_stats', {
    userId
  }),
  // ===== 智能建议 =====
  generateSuggestions: (userId: string) => ipc.invoke<Suggestion[]>('intelligence_v4_generate_suggestions', {
    userId
  }),
  getSuggestions: (userId: string, status?: string, category?: string, limit?: number, offset?: number) => ipc.invoke<{
    suggestions: Suggestion[];
    total: number;
    page: number;
    page_size: number;
  }>('intelligence_v4_get_suggestions', {
    userId,
    status,
    category,
    limit,
    offset
  }),
  markSuggestion: (suggestionId: number, action: string) => ipc.invoke<void>('intelligence_v4_mark_suggestion', {
    suggestion_id: suggestionId,
    action
  }),
  // ===== 行为分析 =====
  analyzeBehavior: (userId: string, date?: string) => ipc.invoke<BehaviorReport>('intelligence_v4_analyze_behavior', {
    userId,
    date
  }),
  getBehaviorTrend: (userId: string, days?: number) => ipc.invoke<{
    dates: string[];
    focus_scores: number[];
    distraction_counts: number[];
    consistency_scores: number[];
  }>('intelligence_v4_behavior_trend', {
    userId,
    days
  }),
  getBehaviorHistory: (userId: string, startDate?: string, endDate?: string) => ipc.invoke<Array<{
    date: string;
    focus_score: number;
    focus_verdict: string;
    summary: string;
  }>>('intelligence_v4_behavior_history', {
    userId,
    start_date: startDate,
    end_date: endDate
  }),
  // ===== 活动日志 =====
  queryActivityLogs: (params: {
    userId: string;
    module?: string;
    startTime?: string;
    endTime?: string;
    limit?: number;
    offset?: number;
  }) => ipc.invoke<{
    logs: ActivityLog[];
    total: number;
    page: number;
    page_size: number;
  }>('intelligence_v4_query_activity_logs', {
    user_id: params.userId,
    module: params.module,
    start_time: params.startTime,
    end_time: params.endTime,
    limit: params.limit,
    offset: params.offset
  }),
  getActivityStats: (userId: string, period?: string) => ipc.invoke<ActivityStats>('intelligence_v4_activity_stats', {
    userId,
    period
  }),
  // ===== 设置 =====
  getSettings: () => ipc.invoke<{
    master_switch: boolean;
    auto_suggest: boolean;
    context_collection: boolean;
    activity_tracking: boolean;
    focus_detection: boolean;
    smart_complete: boolean;
    auto_classify: boolean;
    news_summary: boolean;
    quote_check: boolean;
    command_suggest: boolean;
    game_recommend: boolean;
    search_enhance: boolean;
    timer_smart: boolean;
    floating_ball: boolean;
    llm_provider: string;
    llm_endpoint: string;
    llm_model: string;
    llm_api_key: string;
  }>('intelligence_v4_get_settings'),
  saveSettings: (settings: Record<string, unknown>) => ipc.invoke<void>('intelligence_v4_save_settings', {
    settings
  }),
  resetSettings: () => ipc.invoke<void>('intelligence_v4_reset_settings'),
  testConnection: (provider: string, endpoint: string, apiKey?: string, model?: string) => ipc.invoke<{
    success: boolean;
    latency_ms: number;
    message: string;
  }>('intelligence_v4_test_connection', {
    request: {
      provider,
      endpoint,
      api_key: apiKey,
      model
    }
  }),
  // ===== 跨模块智能（P2阶段）=====
  classifyKbEntry: (entryName: string, content: string, existingCategories?: string[]) => ipc.invoke<Array<{
    category_name: string;
    confidence: number;
    reason: string;
    description: string;
  }>>('intelligence_v4_kb_classify', {
    request: {
      entry_name: entryName,
      content,
      existing_categories: existingCategories
    }
  }),
  resumeSpellCheck: (content: string) => ipc.invoke<{
    original: string | null;
    polished: string;
    changes: string[];
    suggestions: string[];
  }>('intelligence_v4_resume_spell_check', {
    content
  }),
  resumePolish: (content: string) => ipc.invoke<{
    original: string | null;
    polished: string;
    changes: string[];
    suggestions: string[];
  }>('intelligence_v4_resume_polish', {
    content
  }),
  resumeGenerate: (name?: string, education?: string, skills?: string[], experience?: string[]) => ipc.invoke<{
    original: string | null;
    polished: string;
    changes: string[];
    suggestions: string[];
  }>('intelligence_v4_resume_generate', {
    name,
    education,
    skills,
    experience
  }),
  quoteSpellCheck: (content: string) => ipc.invoke<{
    field: string;
    issue: string;
    suggestion: string | null;
    severity: string;
  }>('intelligence_v4_quote_spell_check', {
    content
  }),
  quoteSourceVerify: (author?: string, source?: string) => ipc.invoke<{
    field: string;
    issue: string;
    suggestion: string | null;
    severity: string;
  }>('intelligence_v4_quote_source_verify', {
    author,
    source
  }),
  quoteSmartComplete: (content: string, author?: string, source?: string) => ipc.invoke<{
    field: string;
    issue: string;
    suggestion: string | null;
    severity: string;
  }>('intelligence_v4_quote_smart_complete', {
    content,
    author,
    source
  }),
  newsSummarize: (title: string, content: string) => ipc.invoke<{
    title: string;
    summary: string;
    keywords: string[];
    original_word_count: number;
    summary_word_count: number;
  }>('intelligence_v4_news_summarize', {
    title,
    content
  }),
  todoEnhance: (title: string, description?: string) => ipc.invoke<{
    suggest_priority: string;
    suggest_estimate_minutes: number;
    suggest_category: string | null;
    suggestions: string[];
  }>('intelligence_v4_todo_enhance', {
    title,
    description
  }),
  journalFill: (todayEvents?: string) => ipc.invoke<{
    template: string;
    suggestions: string[];
  }>('intelligence_v4_journal_fill', {
    today_events: todayEvents
  }),
  timerRemind: (currentElapsedSecs: number, taskType?: string) => ipc.invoke<{
    suggest_break: boolean;
    suggest_stop: boolean;
    suggestions: string[];
    optimal_session_minutes: number;
  }>('intelligence_v4_timer_remind', {
    current_elapsed_secs: currentElapsedSecs,
    task_type: taskType
  }),
  terminalComplete: (partialInput?: string, currentDir?: string, recentHistory?: string[]) => ipc.invoke<Array<{
    command: string;
    description: string;
    category: string;
    match_score: number;
  }>>('intelligence_v4_terminal_complete', {
    context: {
      partial_input: partialInput,
      current_dir: currentDir,
      recent_history: recentHistory || []
    }
  }),
  // ===== Task 5 跨模块智能入口 =====
  kbClassify: (entryId: number | null, content: string) => ipc.invoke<{
    category_name: string;
    confidence: number;
    reason: string;
    description: string;
  }>('intelligence_kb_classify', {
    entry_id: entryId,
    content
  }),
  gameRecommend: (playDurationTodaySecs?: number, currentTimeHour?: number, recentGames?: string[]) => ipc.invoke<{
    recommended_games: string[];
    addiction_risk: string;
    anti_addiction_warnings: string[];
    today_total_minutes: number;
    suggest_daily_limit_minutes: number;
  }>('intelligence_v4_game_recommend', {
    play_duration_today_secs: playDurationTodaySecs,
    current_time_hour: currentTimeHour,
    recent_games: recentGames
  }),
  searchAnalyze: (query: string) => ipc.invoke<{
    original_query: string;
    corrected_query: string | null;
    corrections: string[];
    suggestions: string[];
    related_terms: string[];
  }>('intelligence_v4_search_analyze', {
    query
  }),
  // ===== 每日简报 =====
  dailyBriefing: () => ipc.invoke<{
    date: string;
    briefing: string;
    todo_count: number;
    todo_completed: number;
    journal_words: number;
    news_count: number;
    timer_sessions: number;
    total_focus_seconds: number;
    generated_at: string;
  }>('intelligence_daily_briefing'),
  // ===== D2.1 可关闭性 + D2.2 触发执行式 =====
  getEnabled: () => ipc.invoke<boolean>('intelligence_get_enabled'),
  setEnabled: (enabled: boolean) => ipc.invoke<null>('intelligence_set_enabled', { enabled }),
  triggerProactiveActions: () => ipc.invoke<Array<{
    id: string;
    title: string;
    description: string;
    action_type: string;
    action_payload: string;
    priority: number;
    category: string;
  }>>('intelligence_trigger_proactive_actions'),
  // ===== 知识库智能体 =====

  kbSummarize: (entryName: string, content: string, llmConfig: {
    provider: string;
    endpoint: string;
    api_key?: string;
    model?: string;
  }, entryId?: string) => ipc.invoke<{
    summary: string;
    key_points: string[];
    original_length: number;
    summary_length: number;
  }>('intelligence_v4_kb_summarize', {
    request: {
      entry_id: entryId,
      entry_name: entryName,
      content,
      provider: llmConfig.provider,
      endpoint: llmConfig.endpoint,
      api_key: llmConfig.api_key,
      model: llmConfig.model
    }
  }),
  kbTags: (content: string, existingTags: string[], llmConfig: {
    provider: string;
    endpoint: string;
    api_key?: string;
    model?: string;
  }) => ipc.invoke<{
    suggested_tags: string[];
  }>('intelligence_v4_kb_tags', {
    request: {
      content,
      existing_tags: existingTags,
      provider: llmConfig.provider,
      endpoint: llmConfig.endpoint,
      api_key: llmConfig.api_key,
      model: llmConfig.model
    }
  }),
  logActivity: (userId: string, timestamp: string, module: string, operation: string, detail?: string, remark?: string, durationSecs?: number) => ipc.invoke<ActivityLog>('intelligence_v4_log_activity', {
    record: {
      userId,
      timestamp,
      module,
      operation,
      detail,
      remark,
      durationSecs
    }
  }),
  clearAllActivityLogs: () => ipc.invoke<number>('intelligence_v4_clear_all_activity_logs')
};
