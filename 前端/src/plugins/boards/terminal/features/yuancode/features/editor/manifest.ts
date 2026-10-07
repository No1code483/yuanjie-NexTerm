import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/terminal/features/yuancode/features/editor/manifest.rs 逐字段对齐
 *  （L3 Feature，terminal.yuancode.editor；YuanCode「一切皆插件」拆分；parent terminal.yuancode；必备） */
export const manifest: Manifest = {
  id: 'terminal.yuancode.editor',
  name: '编辑器核心',
  level: 'feature',
  parent: 'terminal.yuancode',
  slot: 'terminal.yuancode.editor',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表/命令（复用 L2 terminal.yuancode 的 yc_* 命名空间）。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'terminal.yuancode.editor',
};
