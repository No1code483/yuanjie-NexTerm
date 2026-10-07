// ai.sessions L2 前端半体（batchC1；manifest 与后端 manifest.rs 同源）。
// 声明 ss 命名空间 IPC 客户端：会话列表管理面板由 L1 AI 页面直接渲染，不设插槽组件。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { AI_SESSIONS_IPC_METHODS } from './ipc';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    // AI会话「一切皆插件」：贡献板块内菜单选项（注册表驱动侧边栏）
    navItems: [
      { target: 'ai.sessions', labelKey: 'layout.k4', order: 30, routePath: '?tab=chat', icon: 'chat' },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'ss',
      methods: AI_SESSIONS_IPC_METHODS,
    },
  },
});
