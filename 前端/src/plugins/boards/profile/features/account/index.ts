// profile.account L2 前端半体（个人中心「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 必备子插件：承载账号/概览视图（身份卡/统计/成就/活动时间线，组件由 L1 Profile 壳经 props 组合渲染）。
// 侧边栏选项由 Layout 个人中心分支按 pluginId 门控（boards.profile 为 home 的 L2，其子插件不采用 ?tab= 注册表驱动）。
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
