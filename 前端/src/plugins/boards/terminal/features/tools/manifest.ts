import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/terminal/features/tools/manifest.rs 逐字段对齐
 *  （L2 Feature，terminal.tools；终端本体「一切皆插件」拆分；parent boards.terminal） */
export const manifest: Manifest = {
  id: 'terminal.tools',
  name: '命令辅助工具',
  level: 'feature',
  parent: 'boards.terminal',
  slot: 'terminal.tools',
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
  i18nNamespace: 'terminal.tools',
};
