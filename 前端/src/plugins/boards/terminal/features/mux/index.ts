// terminal.mux L2 前端半体（终端本体「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载标签页（Tab 栏/新建 Tab 菜单/WSL 检测）与分屏（pane/拖拽 resize/布局持久化），
// 组件与逻辑物理迁入本插件目录；由 L1 Terminal 壳经 props 组合渲染。
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
