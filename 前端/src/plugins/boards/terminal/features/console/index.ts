// terminal.console L2 前端半体（终端本体「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 必备子插件：承载终端命令行（命令执行引擎/Block 输出/命令历史与补全/AI 命令建议/错误右键 AI 解释），
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
