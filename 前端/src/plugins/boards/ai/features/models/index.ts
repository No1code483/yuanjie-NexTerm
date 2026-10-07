// ai.models L2 前端半体（阶段3 批次2b-1；manifest 与后端 manifest.rs 同源）。
// 仅贡献 am 命名空间 IPC 客户端：模型管理面板由 L1 AI 页面直接渲染，不设插槽组件。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { AI_MODELS_IPC_METHODS } from './ipc';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    // AI会话「一切皆插件」：贡献板块内菜单选项（注册表驱动侧边栏），
    // target=自身 slot、routePath 为 ?tab= 查询串（AI 为单页 ?tab= 模型）
    navItems: [
      { target: 'ai.models', labelKey: 'layout.k2', order: 10, routePath: '?tab=model', icon: 'bot' },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'am',
      methods: AI_MODELS_IPC_METHODS,
    },
  },
});