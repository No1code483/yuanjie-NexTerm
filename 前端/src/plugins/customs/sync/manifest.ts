import type { Manifest } from '@/kernel/types';

/** 与后端 plugins/customs/sync/manifest.rs 逐字段对齐（L1 Custom，同步）。
 *  2 张自有表 sync_queue + sync_devices（短码 sy + _ = sy_，不命中 → name_prefixed=0）。
 *  21 条 IPC。
 */
export const manifest: Manifest = {
  id: 'customs.sync',
  name: '同步',
  level: 'custom',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    db: ['sync_queue', 'sync_devices'],
    events: { subscribe: [], publish: [] },
    ipc: ['sy_*'],
    fs: [],
    net: []
  },
  // 同步「一切皆插件」：2 个纯前端视图 L2 子插件（devices 必备，其余可选）
  slots: [
    { id: 'sync.devices', type: 'panel', description: '设备管理', capacity: 1 },
    { id: 'sync.conflicts', type: 'panel', description: '冲突解决', capacity: 1 },
  ],
  i18nNamespace: 'sync',
};
