import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/terminal/features/yuancode/features/skills/manifest.rs 逐字段对齐
 *  （L3 Feature，terminal.yuancode.skills；YuanCode「一切皆插件」拆分；parent terminal.yuancode） */
export const manifest: Manifest = {
  id: 'terminal.yuancode.skills',
  name: '技能与片段',
  level: 'feature',
  parent: 'terminal.yuancode',
  slot: 'terminal.yuancode.skills',
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
  i18nNamespace: 'terminal.yuancode.skills',
};
