import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/terminal/manifest.rs 逐字段对齐（L1 Board，终端板块） */
export const manifest: Manifest = {
  id: 'boards.terminal',
  name: '终端',
  level: 'board',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 6 张自有表（5 业务 + FTS5 虚表 terminal_history_fts），见 0001_baseline.sql。
    db: [
      'terminal_sessions',
      'terminal_history',
      'terminal_history_fts',
      'terminal_tab_layout',
      'terminal_config',
      'ssh_profiles',
    ],
    // 本批不发布/订阅内核领域事件；`terminal-output` / `mux:notification` 为 Tauri emit。
    events: { subscribe: [], publish: [] },
    // 29 条 IPC（裁定 T7 全量留 + alias；client 仅收编已消费 10 条）。
    ipc: ['tm_*'],
    fs: [],
    net: [],
  },
  // terminal.linux 插槽（3b 挂载）；terminal.yuancode 插槽（3c 挂载，C2 裁定新增）；
  // terminal.manual 插槽（批C2 挂载，命令手册 L2）。
  // 终端本体「一切皆插件」：terminal.console（必备）/ terminal.mux / terminal.tools
  slots: [
    { id: 'terminal.linux', type: 'panel', description: 'Linux 子系统', capacity: 1 },
    { id: 'terminal.yuancode', type: 'panel', description: 'Yuan Code 编程', capacity: 1 },
    { id: 'terminal.manual', type: 'ui-route', description: '命令手册', capacity: 1 },
    { id: 'terminal.console', type: 'panel', description: '终端命令行', capacity: 1 },
    { id: 'terminal.mux', type: 'panel', description: '标签页与分屏', capacity: 1 },
    { id: 'terminal.tools', type: 'panel', description: '命令辅助工具', capacity: 1 },
  ],
  i18nNamespace: 'terminal',
};
