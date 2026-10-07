// terminal.tools L2 前端半体（终端本体「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载命令辅助工具（命令 Launcher、QuickSelect、Copy Mode、命令搜索、终端内文本搜索），
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
