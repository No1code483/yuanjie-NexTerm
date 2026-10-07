// ai.agent L2 前端半体（阶段3 批次2b-1；manifest 与后端 manifest.rs 同源）。
// 仅贡献 ag 命名空间 IPC 客户端：Agent 管理面板由 L1 AI 页面直接渲染，不设插槽组件。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { AI_AGENTS_IPC_METHODS } from './ipc';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    // AI会话「一切皆插件」：贡献板块内菜单选项（注册表驱动侧边栏）
    navItems: [
      { target: 'ai.agent', labelKey: 'layout.k3', order: 20, routePath: '?tab=agent', icon: 'cube' },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'ag',
      methods: AI_AGENTS_IPC_METHODS,
    },
  },
});