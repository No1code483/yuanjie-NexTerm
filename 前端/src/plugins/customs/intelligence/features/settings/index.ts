// intelligence.settings L2 前端半体（底层智能「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 必备子插件：承载设置面板（SettingsPanel，底层智能总开关），组件物理迁入本插件目录；
// 贡献 /spyglass 板块内菜单选项（注册表驱动侧边栏）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'intelligence.settings', labelKey: 'common.settings', order: 50, routePath: '?tab=settings' },
    ],
    slotComponents: [],
  },
});
