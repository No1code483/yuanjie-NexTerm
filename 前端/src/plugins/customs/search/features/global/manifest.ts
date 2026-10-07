import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/search/features/global/manifest.rs 逐字段对齐
 *  （L2 Feature，search.global；搜索「一切皆插件」拆分；parent customs.search） */
export const manifest: Manifest = {
  id: 'search.global',
  name: '全站搜索',
  level: 'feature',
  parent: 'customs.search',
  slot: 'search.global',
  version: '0.1.0',
  kernelApi: '1',
  permissions: { db: [], events: { subscribe: [], publish: [] }, ipc: [], fs: [], net: [] },
  slots: [],
  i18nNamespace: 'search.global',
};
