import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/recycle/features/actions/manifest.rs 逐字段对齐
 *  （L2 Feature，recycle.actions；回收站「一切皆插件」拆分） */
export const manifest: Manifest = {
  id: 'recycle.actions',
  name: '回收站操作',
  level: 'feature',
  parent: 'customs.recycle',
  slot: 'recycle.actions',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表与自有 IPC（rc_* 归 L1）；纯前端视图。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'recycle.actions',
};
