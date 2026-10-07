// ai.chat L2 前端半体（AI会话「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 必备子插件：承载对话面板（ChatPanel，单会话消息区/输入/引用/多选），组件物理迁入本插件目录；
// 由 L1 AI 壳经 props 组合渲染（内嵌于 chat tab）。
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
