import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/knowledge/features/material/manifest.rs 逐字段对齐
 *  （L2 Feature，knowledge.material，批C3；必备子插件） */
export const manifest: Manifest = {
  id: 'knowledge.material',
  name: '资料库',
  level: 'feature',
  parent: 'boards.knowledge',
  slot: 'knowledge.material',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表（资料/学习两库共用 L1 的 kb_* 表）。
    db: [],
    events: { subscribe: [], publish: [] },
    // 无自有 IPC（kb_* 全集归属 L1）。
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'knowledge.material',
};
