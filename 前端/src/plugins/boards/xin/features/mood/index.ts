// xin.mood L2 前端半体（小欣「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 心情记录与时间线，组件由 L1 Xin 壳经 props 组合渲染；
// 贡献板块内菜单选项（target=自身 slot id，routePath='?tab=mood'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'xin.mood', labelKey: 'layout.k19', icon: '🌊', order: 30, routePath: '?tab=mood' },
    ],
    slotComponents: [],
  },
});
