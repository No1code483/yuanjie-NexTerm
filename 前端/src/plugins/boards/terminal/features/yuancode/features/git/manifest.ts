import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/terminal/features/yuancode/features/git/manifest.rs 逐字段对齐
 *  （L3 Feature，terminal.yuancode.git；YuanCode「一切皆插件」拆分；parent terminal.yuancode） */
export const manifest: Manifest = {
  id: 'terminal.yuancode.git',
  name: '版本控制',
  level: 'feature',
  parent: 'terminal.yuancode',
  slot: 'terminal.yuancode.git',
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
  i18nNamespace: 'terminal.yuancode.git',
};
