// xin.briefing L2 前端半体（小欣「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 每日简报，组件由 L1 Xin 壳经 props 组合渲染；
// 贡献板块内菜单选项（target=自身 slot id，routePath='?tab=briefing'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'xin.briefing', labelKey: 'layout.k21', icon: '📊', order: 50, routePath: '?tab=briefing' },
    ],
    slotComponents: [],
  },
});
