// recycle.list L2 前端半体（回收站「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 必备子插件：回收站页面本体（列表 / 分类筛选 / 搜索 / 统计 / 确认弹窗；原 routes/Recycle.tsx 物理迁入），
// 由 L1 路由指向本组件（板块内 L1→L2 直连，规范 §1.4）；为嵌入型视图，不贡献 navItems。
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
