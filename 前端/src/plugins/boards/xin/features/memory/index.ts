// xin.memory L2 前端半体（小欣「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 长期记忆浏览（记忆列表 / 搜索 / 分类筛选），组件由 L1 Xin 壳经 props 组合渲染；
// 贡献板块内菜单选项（target=自身 slot id，routePath='?tab=memory'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'xin.memory', labelKey: 'layout.k18', icon: '🧠', order: 20, routePath: '?tab=memory' },
    ],
    slotComponents: [],
  },
});
