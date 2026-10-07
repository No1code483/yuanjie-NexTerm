// ai.multimodel L2 前端半体（AI会话「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载多模型对比面板（MultiModelChatPanel，多窗格分栏），组件物理迁入本插件目录；
// 由 L1 AI 壳经 props 组合渲染（内嵌于 chat tab，停用则回落单模型 ChatPanel）。
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
