import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/terminal/features/console/manifest.rs 逐字段对齐
 *  （L2 Feature，terminal.console；终端本体「一切皆插件」拆分；parent boards.terminal；必备） */
export const manifest: Manifest = {
  id: 'terminal.console',
  name: '终端命令行',
  level: 'feature',
  parent: 'boards.terminal',
  slot: 'terminal.console',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表/命令（复用 L1 boards.terminal 的 tm_* 命名空间）。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'terminal.console',
};
