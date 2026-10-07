// terminal.yuancode.sandbox L3 前端半体（YuanCode「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载沙箱与运行（SandboxPanel/RunPanel），组件物理迁入本插件目录；由 L2 YuanCode 壳经 props 组合渲染。
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
