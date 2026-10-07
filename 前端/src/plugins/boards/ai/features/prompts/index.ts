// ai.prompts L2 前端半体（AI会话「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载提示词模板（PromptTemplates，被 ChatPanel 内嵌为模态框），组件物理迁入本插件目录；
// 由 L1 AI 壳经 props 组合渲染（停用则隐藏提示词模板入口）。
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
