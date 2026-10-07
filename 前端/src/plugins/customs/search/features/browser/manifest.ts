import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/search/features/browser/manifest.rs 逐字段对齐
 *  （L2 Feature，search.browser；搜索「一切皆插件」拆分；parent customs.search；必备） */
export const manifest: Manifest = {
  id: 'search.browser',
  name: '浏览器搜索',
  level: 'feature',
  parent: 'customs.search',
  slot: 'search.browser',
  version: '0.1.0',
  kernelApi: '1',
  permissions: { db: [], events: { subscribe: [], publish: [] }, ipc: [], fs: [], net: [] },
  slots: [],
  i18nNamespace: 'search.browser',
};
