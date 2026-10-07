// recycle.actions L2 前端半体（回收站「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 侧边栏操作区（恢复 / 永久删除 / 清空 / 全选 / 取消选择；原内联于 layouts/Layout.tsx 的 RecycleActions 物理迁入），
// 由 Layout 直接 import 渲染并按本插件启停门控（Layout 已持有 enabledIds）；为嵌入型视图，不贡献 navItems。
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
