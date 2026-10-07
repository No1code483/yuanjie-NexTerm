import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/knowledge/features/tags/manifest.rs 逐字段对齐
 *  （L2 Feature，knowledge.tags；本轮知识库「一切皆插件」拆分；可选子插件） */
export const manifest: Manifest = {
  id: 'knowledge.tags',
  name: '标签',
  level: 'feature',
  parent: 'boards.knowledge',
  slot: 'knowledge.tags',
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
  i18nNamespace: 'knowledge.tags',
};
