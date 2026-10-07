import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/recycle/features/list/manifest.rs 逐字段对齐
 *  （L2 Feature，recycle.list；回收站「一切皆插件」拆分；必备子插件） */
export const manifest: Manifest = {
  id: 'recycle.list',
  name: '回收站列表',
  level: 'feature',
  parent: 'customs.recycle',
  slot: 'recycle.list',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表（共用 L1 的 recycle_bin）与自有 IPC（rc_* 全集归属 L1）；纯前端视图。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'recycle.list',
};
