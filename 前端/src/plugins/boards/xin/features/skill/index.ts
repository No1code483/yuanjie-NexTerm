// xin.skill L2 前端半体（小欣「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 技能列表与执行，组件由 L1 Xin 壳经 props 组合渲染；
// 贡献板块内菜单选项（target=自身 slot id，routePath='?tab=skill'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'xin.skill', labelKey: 'layout.k26', icon: '⚡', order: 110, routePath: '?tab=skill' },
    ],
    slotComponents: [],
  },
});
