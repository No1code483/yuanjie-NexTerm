import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/knowledge/features/search/manifest.rs 逐字段对齐
 *  （L2 Feature，knowledge.search；本轮知识库「一切皆插件」拆分；可选子插件） */
export const manifest: Manifest = {
  id: 'knowledge.search',
  name: '搜索',
  level: 'feature',
  parent: 'boards.knowledge',
  slot: 'knowledge.search',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'knowledge.search',
};
