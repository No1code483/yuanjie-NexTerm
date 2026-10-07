import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/home/features/todo/manifest.rs 逐字段对齐（L2 Feature） */
export const manifest: Manifest = {
  id: 'home.todo',
  name: '待办',
  level: 'feature',
  parent: 'boards.home',
  slot: 'home.todo',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // todos 为既有旧表，归属登记见后端 migrations/0001_baseline.sql。
    db: ['todos'],
    // 批次1b-2c（裁定 24-A）：领域事件域 = 插件 id，事件名形如 `home.todo:todo.created`。
    events: { subscribe: [], publish: ['home.todo:*'] },
    // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
    ipc: ['td_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'home.todo',
};
