import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/terminal/features/yuancode/features/sandbox/manifest.rs 逐字段对齐
 *  （L3 Feature，terminal.yuancode.sandbox；YuanCode「一切皆插件」拆分；parent terminal.yuancode） */
export const manifest: Manifest = {
  id: 'terminal.yuancode.sandbox',
  name: '沙箱运行',
  level: 'feature',
  parent: 'terminal.yuancode',
  slot: 'terminal.yuancode.sandbox',
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
  i18nNamespace: 'terminal.yuancode.sandbox',
};
