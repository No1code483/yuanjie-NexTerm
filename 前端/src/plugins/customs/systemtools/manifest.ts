import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/systemtools/manifest.rs 逐字段对齐（L1 Custom，系统工具）。
 *  10 条 system 命令（短码 `st`）；extension 2 + adapter 3 裁定零消费判删。
 */
export const manifest: Manifest = {
  id: 'customs.systemtools',
  name: '系统工具',
  level: 'custom',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 1 张自有表 system_config（短码 st + _ = st_，不命中 → name_prefixed=0）。
    db: ['system_config'],
    events: { subscribe: [], publish: [] },
    // 10 条 IPC。
    ipc: ['st_*'],
    fs: [],
    net: []
  },
  slots: [],
  i18nNamespace: 'systemtools'
};
