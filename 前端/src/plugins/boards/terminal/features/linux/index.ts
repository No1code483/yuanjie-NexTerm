// terminal.linux L2 前端半体（阶段3 批次3b；manifest 与后端 manifest.rs 同源）。
// 贡献 lx 命名空间 IPC（25 条方法 spec，裁定 T9 前端零消费、无便捷封装）；
// 无路由贡献（Linux 面由 Terminal 页 Tab 承载，TERMINAL_LINUX 路由已随 3a 迁入 L1）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { LINUX_IPC_METHODS } from './ipc';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    // L2 不挂板块内菜单：终端侧边栏由 L1 维护
    navItems: [],
    slotComponents: [],
    ipc: {
      namespace: 'lx',
      methods: LINUX_IPC_METHODS,
    },
  },
});
