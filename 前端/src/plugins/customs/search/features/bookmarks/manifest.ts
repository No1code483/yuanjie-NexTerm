import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/search/features/bookmarks/manifest.rs 逐字段对齐
 *  （L2 Feature，search.bookmarks；搜索「一切皆插件」拆分；parent customs.search） */
export const manifest: Manifest = {
  id: 'search.bookmarks',
  name: '收藏站点',
  level: 'feature',
  parent: 'customs.search',
  slot: 'search.bookmarks',
  version: '0.1.0',
  kernelApi: '1',
  permissions: { db: [], events: { subscribe: [], publish: [] }, ipc: [], fs: [], net: [] },
  slots: [],
  i18nNamespace: 'search.bookmarks',
};
