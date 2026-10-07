// plugins/customs/search/ipc/search.ts — customs.search L1 IPC 客户端（短码 se，批次6b）。
// 契约：统一经内核 dispatcher，逻辑名 `se:plugin:<旧命令名>`；参数形状与迁移前
// 旧封装逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 15 条 alias：search 15 条命令。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const SEARCH_IPC_METHODS = {
  // ===== 搜索内核命令（5）=====
  global: { cmd: 'search_global' },
  indexDocument: { cmd: 'search_index_document' },
  deleteDocument: { cmd: 'search_delete_document' },
  clearIndex: { cmd: 'search_clear_index' },
  rebuildIndex: { cmd: 'search_rebuild_index' },
  // ===== 索引管理（1）=====
  indexStatus: { cmd: 'search_index_status' },
  // ===== 搜索建议（1）=====
  suggest: { cmd: 'search_suggest' },
  // ===== 高级搜索（1）=====
  advanced: { cmd: 'search_advanced' },
  // ===== 分面统计（1）=====
  facets: { cmd: 'search_facets' },
  // ===== 搜索历史（2）=====
  history: { cmd: 'search_history' },
  clearHistory: { cmd: 'search_clear_history' },
  // ===== 热门查询（1）=====
  hotQueries: { cmd: 'search_hot_queries' },
  // ===== 批量索引（1）=====
  batchIndex: { cmd: 'search_batch_index' },
  // ===== 全站搜索（1）=====
  siteSearch: { cmd: 'global_search' },
  // ===== AI 摘要（1）=====
  aiSummary: { cmd: 'search_ai_summary' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherSe = defineIpcNamespace('se', SEARCH_IPC_METHODS);
type SearchMethod = keyof typeof SEARCH_IPC_METHODS;

function invokeSe<T>(method: SearchMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherSe[method](args) as Promise<ApiResponse<T>>;
}

export const search = {
  // ===== 搜索内核 =====
  global: (query: string, sourceFilter?: unknown, limit?: number, offset?: number) =>
    invokeSe<any[]>('global', { query, sourceFilter, limit, offset }),
  indexDocument: (document: unknown) => invokeSe<void>('indexDocument', { document }),
  deleteDocument: (docId: string) => invokeSe<void>('deleteDocument', { docId }),
  clearIndex: () => invokeSe<void>('clearIndex'),
  rebuildIndex: () => invokeSe<void>('rebuildIndex'),
  // ===== 索引管理 =====
  indexStatus: () => invokeSe<any>('indexStatus'),
  // ===== 搜索建议 =====
  suggest: (request: unknown) => invokeSe<any>('suggest', { request }),
  // ===== 高级搜索 =====
  advanced: (query: unknown) => invokeSe<any>('advanced', { query }),
  // ===== 分面统计 =====
  facets: (query: string) => invokeSe<any>('facets', { query }),
  // ===== 搜索历史 =====
  history: () => invokeSe<any[]>('history'),
  clearHistory: () => invokeSe<void>('clearHistory'),
  // ===== 热门查询 =====
  hotQueries: (limit?: number) => invokeSe<any[]>('hotQueries', { limit }),
  // ===== 批量索引 =====
  batchIndex: (request: unknown) => invokeSe<void>('batchIndex', { request }),
  // ===== 全站搜索 =====
  siteSearch: (request: unknown) => invokeSe<any>('siteSearch', { request }),
  // ===== AI 摘要 =====
  aiSummary: (request: unknown) => invokeSe<any>('aiSummary', { request }),
};