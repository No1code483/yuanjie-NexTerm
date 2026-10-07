import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/sync/features/devices/manifest.rs 逐字段对齐
 *  （L2 Feature，sync.devices；同步「一切皆插件」拆分；必备子插件） */
export const manifest: Manifest = {
  id: 'sync.devices',
  name: '设备管理',
  level: 'feature',
  parent: 'customs.sync',
  slot: 'sync.devices',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表（共用 L1 的 sync_queue / sync_devices）与自有 IPC（sy_* 全集归属 L1）；纯前端视图。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'sync.devices',
};
