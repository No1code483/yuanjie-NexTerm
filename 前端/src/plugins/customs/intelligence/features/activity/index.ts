// intelligence.activity L2 前端半体（底层智能「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载活动日志面板（ActivityPanel），组件物理迁入本插件目录；
// 贡献 /spyglass 板块内菜单选项（注册表驱动侧边栏）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'intelligence.activity', labelKey: 'layout.k30', order: 40, routePath: '?tab=activity' },
    ],
    slotComponents: [],
  },
});
