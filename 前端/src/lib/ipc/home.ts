// ipc/home.ts — home 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）
import { ipc } from './core';

export const home = {
  getTodos: (date: string) => ipc.invoke<any[]>('get_todos', {
    date
  }),
  createTodo: (title: string, date: string) => ipc.invoke('add_todo', {
    title,
    date
  }),
  updateTodo: (id: number) => ipc.invoke('toggle_todo', {
    id
  }),
  deleteTodo: (id: number) => ipc.invoke('delete_todo', {
    id
  }),
  getNews: () => ipc.invoke<any[]>('get_news'),
  getNewsByCategory: (category: string) => ipc.invoke<any[]>('get_news_by_category', {
    category
  }),
  // A5 离线同步 Phase 3 Task 3: 新闻源离线缓存
  newsGetCached: (source?: string, limit?: number) =>
    ipc.invoke<any[]>('news_get_cached', { source: source ?? null, limit: limit ?? 50 }),
  newsCacheStatus: () => ipc.invoke<{
    last_cached_at: string | null;
    total_count: number;
    age_minutes: number | null;
  }>('news_cache_status'),
  fetchNews: () => ipc.invoke<{
    deleted: number;
    inserted: number;
    net_change: number;
    total: number;
  }>('fetch_news'),
  addNews: (news: {
    title: string;
    url?: string;
    source?: string;
    summary?: string;
  }) => ipc.invoke('add_news', news),
  markNewsRead: (id: number) => ipc.invoke('mark_news_read', {
    id
  }),
  clearOldNews: (days?: number) => ipc.invoke('clear_old_news', {
    days: days || 30
  }),
  toggleNewsFavorite: (id: number, isFavorite: boolean) => ipc.invoke('toggle_news_favorite', {
    id,
    isFavorite
  }),
  getPendingDeleteNews: () => ipc.invoke<any[]>('get_pending_delete_news'),
  generateNewsAiSummary: (id: number) => ipc.invoke<any>('generate_news_ai_summary', {
    id
  }),
  getLogs: (date: string) => ipc.invoke<any[]>('get_journal', {
    date
  }),
  saveLog: (date: string, content: string) => ipc.invoke('save_journal', {
    date,
    content
  }),
  deleteLog: (date: string) => ipc.invoke('delete_journal', {
    date
  }),
  getTimers: () => ipc.invoke<any[]>('get_timers'),
  createTimer: (timer: any) => ipc.invoke('create_timer', {
    request: timer
  }),
  updateTimerState: (id: number, isRunning: boolean, elapsed: number) => ipc.invoke('update_timer_state', {
    id,
    is_running: isRunning,
    elapsed
  }),
  deleteTimer: (id: number) => ipc.invoke('delete_timer', {
    id
  }),
  timerAction: (id: number, action: string) => ipc.invoke('timer_action', {
    id,
    action
  }),
  aiSummarizeNews: (newsId: number, title: string, content: string) => ipc.invoke<{
    summary: string;
  }>('home_ai_summarize_news', {
    news_id: newsId,
    title,
    content
  }),
  aiCompleteTodo: () => ipc.invoke<{
    suggestion: string;
  }>('home_ai_complete_todo', {}),
  aiCheckJournal: (date: string, content: string) => ipc.invoke<{
    result: string;
  }>('home_ai_check_journal', {
    date,
    content
  })
};
