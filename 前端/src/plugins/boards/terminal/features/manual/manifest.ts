import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/terminal/features/manual/manifest.rs 逐字段对齐
 *  （L2 Feature，terminal.manual，C2 拆分） */
export const manifest: Manifest = {
  id: 'terminal.manual',
  name: '命令手册',
  level: 'feature',
  parent: 'boards.terminal',
  slot: 'terminal.manual',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无业务表（命令手册纯静态文档，无落库）。
    db: [],
    // 无领域事件。
    events: { subscribe: [], publish: [] },
    // 无 IPC 调用（纯前端静态页面）。
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'terminal.manual',
};
