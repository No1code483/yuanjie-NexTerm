// ai.groupchat L2 前端半体（阶段3 批次2b-1；manifest 与后端 manifest.rs 同源）。
// 仅贡献 gc 命名空间 IPC 客户端（本批骨架占位，业务实现复用 ai_commands 原函数）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { AI_GROUPCHAT_IPC_METHODS } from './ipc';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    // AI会话「一切皆插件」：贡献板块内菜单选项（注册表驱动侧边栏）
    navItems: [
      { target: 'ai.groupchat', labelKey: 'layout.k5', order: 40, routePath: '?tab=group', icon: 'users' },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'gc',
      methods: AI_GROUPCHAT_IPC_METHODS,
    },
  },
});