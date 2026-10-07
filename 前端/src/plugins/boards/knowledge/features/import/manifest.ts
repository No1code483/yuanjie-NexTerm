import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/knowledge/features/import/manifest.rs 逐字段对齐
 *  （L2 Feature，knowledge.import；本轮知识库「一切皆插件」拆分；可选子插件） */
export const manifest: Manifest = {
  id: 'knowledge.import',
  name: '导入',
  level: 'feature',
  parent: 'boards.knowledge',
  slot: 'knowledge.import',
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
  i18nNamespace: 'knowledge.import',
};
