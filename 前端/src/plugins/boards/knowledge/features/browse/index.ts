// knowledge.browse L2 前端半体（本轮知识库「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 必备子插件：承载目录树浏览/条目·目录 CRUD/拖拽移动/批量操作（组件由 L1 Knowledge 壳经 props 组合渲染）；
// 贡献板块内菜单选项（target=自身 slot id，routePath='?tab=browse'）供全局侧边栏注册表驱动。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'knowledge.browse', labelKey: 'knowledge.features.browse', order: 10, routePath: '?tab=browse' },
    ],
    slotComponents: [],
  },
});
