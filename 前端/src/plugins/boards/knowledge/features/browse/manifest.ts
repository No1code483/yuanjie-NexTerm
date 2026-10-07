import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/knowledge/features/browse/manifest.rs 逐字段对齐
 *  （L2 Feature，knowledge.browse；本轮知识库「一切皆插件」拆分；必备子插件） */
export const manifest: Manifest = {
  id: 'knowledge.browse',
  name: '条目浏览',
  level: 'feature',
  parent: 'boards.knowledge',
  slot: 'knowledge.browse',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表（共用 L1 的 kb_* 表）；无自有 IPC（kb_* 全集归属 L1）。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'knowledge.browse',
};
