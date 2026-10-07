// home.focus L2 前端半体（批C4；manifest 与后端 manifest.rs 同源）。
// 可选子插件（手稿 20260926 首页可选子插件：专注）：FocusMode 组件物理迁入本目录，
// 由 L1 Home 页以整页视图渲染（focusMode 状态切换，非路由）；入口按钮按本插件
// 启停状态门控（Home.tsx 拉取 kernel:plugin:get_enabled 判断）。
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
