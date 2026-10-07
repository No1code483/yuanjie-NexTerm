import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/terminal/features/linux/manifest.rs 逐字段对齐
 *  （L2 Feature，terminal.linux，批次3b） */
export const manifest: Manifest = {
  id: 'terminal.linux',
  name: 'Linux 子系统',
  level: 'feature',
  parent: 'boards.terminal',
  slot: 'terminal.linux',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 不持有业务表（linux/docker 命令为系统查询/文件系统/docker CLI，无落库）。
    db: [],
    // 不发布/订阅内核领域事件（无 emit 点，S5 零改动实测）。
    events: { subscribe: [], publish: [] },
    // 25 条 IPC（裁定 T9：前端零消费，有 alias 无前端方法，43-A 口径）。
    ipc: ['lx_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'terminal.linux',
};
