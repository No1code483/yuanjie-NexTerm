import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/sync/features/conflicts/manifest.rs 逐字段对齐
 *  （L2 Feature，sync.conflicts；同步「一切皆插件」拆分） */
export const manifest: Manifest = {
  id: 'sync.conflicts',
  name: '冲突解决',
  level: 'feature',
  parent: 'customs.sync',
  slot: 'sync.conflicts',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表与自有 IPC（sy_* 归 L1）；纯前端视图。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'sync.conflicts',
};
