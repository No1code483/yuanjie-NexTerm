import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/knowledge/features/graph/manifest.rs 逐字段对齐
 *  （L2 Feature，knowledge.graph，批C3；可选子插件） */
export const manifest: Manifest = {
  id: 'knowledge.graph',
  name: '关系图谱',
  level: 'feature',
  parent: 'boards.knowledge',
  slot: 'knowledge.graph',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表（图谱数据来自 L1 页面已加载状态）。
    db: [],
    events: { subscribe: [], publish: [] },
    // 无自有 IPC。
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'knowledge.graph',
};
