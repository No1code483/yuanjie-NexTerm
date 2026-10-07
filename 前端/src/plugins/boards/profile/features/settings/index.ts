// profile.settings L2 前端半体（个人中心「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载设置视图（设置列表 + 账号安全子页 + 主题/窗口/新闻源/数据导出，组件由 L1 Profile 壳经 props 组合渲染）；
// 侧边栏选项由 Layout 个人中心分支按 pluginId 门控。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [],
    slotComponents: [],
  },
});
