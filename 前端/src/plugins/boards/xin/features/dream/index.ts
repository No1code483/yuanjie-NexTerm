// xin.dream L2 前端半体（小欣「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 梦境记忆整理（浅睡 / 深睡 / REM 三阶段），组件由 L1 Xin 壳经 props 组合渲染；
// 贡献板块内菜单选项（target=自身 slot id，routePath='?tab=dream'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'xin.dream', labelKey: 'layout.k23', icon: '🌙', order: 70, routePath: '?tab=dream' },
    ],
    slotComponents: [],
  },
});
