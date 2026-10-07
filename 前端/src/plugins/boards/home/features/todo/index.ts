// home.todo L2 前端半体（阶段3 批次1b-1；manifest 与后端 manifest.rs 同源）。
// 仅贡献 home.todo 插槽组件与 td 命名空间 IPC 客户端。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { TODO_IPC_METHODS } from './ipc';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    // L2 不挂板块内菜单：首页侧边栏由 L1 维护，避免出现第二套菜单
    navItems: [],
    slotComponents: [{ slot: 'home.todo', component: lazy(() => import('./TodoPanel')) }],
    ipc: {
      namespace: 'td',
      methods: TODO_IPC_METHODS,
    },
  },
});
