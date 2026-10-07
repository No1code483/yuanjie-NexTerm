// knowledge.graph L2 前端半体（批C3；manifest 与后端 manifest.rs 同源）。
// 可选子插件（手稿 20260926）：关系图谱视图（GraphView）物理迁入本目录，
// 由 L1 Knowledge 页以 props 驱动渲染；可见性由 Layout 侧边栏按钮按 pluginId 过滤承载。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    // 板块内菜单选项（本轮注册表驱动侧边栏）：target=自身 slot id，routePath 为 tab 查询串
    navItems: [
      { target: 'knowledge.graph', labelKey: 'layout.k7', order: 100, routePath: '?tab=graph' },
    ],
    slotComponents: [],
  },
});
