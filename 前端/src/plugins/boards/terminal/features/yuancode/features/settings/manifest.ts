import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/terminal/features/yuancode/features/settings/manifest.rs 逐字段对齐
 *  （L3 Feature，terminal.yuancode.settings；YuanCode「一切皆插件」拆分；parent terminal.yuancode） */
export const manifest: Manifest = {
  id: 'terminal.yuancode.settings',
  name: '编辑器设置',
  level: 'feature',
  parent: 'terminal.yuancode',
  slot: 'terminal.yuancode.settings',
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
  i18nNamespace: 'terminal.yuancode.settings',
};
