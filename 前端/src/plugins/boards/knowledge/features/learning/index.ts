// knowledge.learning L2 前端半体（批C3；manifest 与后端 manifest.rs 同源）。
// 必备子插件（手稿 20260926）：学习库视图（library='study'，历史字段值不改）
// 启停登记，无路由/菜单/IPC 贡献；可见性由 Layout 侧边栏库切换按钮按 pluginId 过滤承载。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    // 板块内菜单选项（本轮注册表驱动侧边栏）：target=自身 slot id，routePath 为 tab 查询串
    navItems: [
      { target: 'knowledge.learning', labelKey: 'layout.k38', order: 120, routePath: '?tab=learning' },
    ],
    slotComponents: [],
  },
});
