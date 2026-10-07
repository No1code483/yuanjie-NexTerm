// knowledge.tags L2 前端半体（本轮知识库「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载 TagBar / 全局标签 CRUD / 条目标签（组件由 L1 Knowledge 壳经 props 组合渲染）；
// 贡献板块内菜单选项（target=自身 slot id，routePath='?tab=tags'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'knowledge.tags', labelKey: 'knowledge.features.tags', order: 30, routePath: '?tab=tags' },
    ],
    slotComponents: [],
  },
});
