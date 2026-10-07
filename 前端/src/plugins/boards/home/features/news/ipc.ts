// plugins/boards/home/features/news/ipc.ts — home.news 插件 IPC 客户端（短码 nw）。
// 契约：统一经内核 dispatcher，逻辑名 `nw:plugin:<旧命令名>`；参数形状与迁移前
// 旧 Tauri 命令逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const NEWS_IPC_METHODS = {
  getNews: { cmd: 'get_news' },
  getNewsByCategory: { cmd: 'get_news_by_category' },
  fetchNews: { cmd: 'fetch_news' },
  addNews: { cmd: 'add_news' },
  markNewsRead: { cmd: 'mark_news_read' },
  clearOldNews: { cmd: 'clear_old_news' },
  toggleNewsFavorite: { cmd: 'toggle_news_favorite' },
  getPendingDeleteNews: { cmd: 'get_pending_delete_news' },
  generateNewsAiSummary: { cmd: 'generate_news_ai_summary' },
  newsGetCached: { cmd: 'news_get_cached' },
  newsCacheStatus: { cmd: 'news_cache_status' },
  getNewsSources: { cmd: 'get_news_sources' },
  addNewsSource: { cmd: 'add_news_source' },
  deleteNewsSource: { cmd: 'delete_news_source' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherNews = defineIpcNamespace('nw', NEWS_IPC_METHODS);
type NewsMethod = keyof typeof NEWS_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeNews<T = any>(method: NewsMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherNews[method](args) as Promise<ApiResponse<T>>;
}

export const news = {
  getNews: () => invokeNews<any[]>('getNews'),
  getNewsByCategory: (category: string) => invokeNews<any[]>('getNewsByCategory', { category }),
  // A5 离线同步 Phase 3 Task 3: 新闻源离线缓存
  newsGetCached: (source?: string, limit?: number) =>
    invokeNews<any[]>('newsGetCached', { source: source ?? null, limit: limit ?? 50 }),
  newsCacheStatus: () => invokeNews<{
    last_cached_at: string | null;
    total_count: number;
    age_minutes: number | null;
  }>('newsCacheStatus'),
  fetchNews: () => invokeNews<{
    deleted: number;
    inserted: number;
    net_change: number;
    total: number;
  }>('fetchNews'),
  // 零消费命令（NewsPanel 无调用点），本批仅完成命名空间声明与归属收归，不接通前端
  addNews: (item: {
    title: string;
    url?: string;
    source?: string;
    summary?: string;
  }) => invokeNews('addNews', item),
  markNewsRead: (id: number) => invokeNews('markNewsRead', { id }),
  clearOldNews: (days?: number) => invokeNews('clearOldNews', { days: days || 30 }),
  toggleNewsFavorite: (id: number, isFavorite: boolean) =>
    invokeNews('toggleNewsFavorite', { id, isFavorite }),
  getPendingDeleteNews: () => invokeNews<any[]>('getPendingDeleteNews'),
  generateNewsAiSummary: (id: number) => invokeNews<any>('generateNewsAiSummary', { id }),
  getNewsSources: () => invokeNews<any[]>('getNewsSources'),
  // 裁定 17-A：旧后端形参 `feed_type` 按 Tauri camelCase 绑定取键 `feedType`，
  // 本键名取代迁移前 `lib/ipc/system.ts` 误发的 `feed_type`（v1 可达缺陷）。
  addNewsSource: (name: string, url: string, category: string, feedType: string) =>
    invokeNews('addNewsSource', { name, url, category, feedType }),
  deleteNewsSource: (id: number) => invokeNews('deleteNewsSource', { id }),
};
