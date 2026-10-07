// search.global L2 前端半体（搜索「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载全站搜索模式（结果按模块分组/AI 摘要/一键归档 KB），组件物理迁入本插件目录。
// 嵌入型子插件，不贡献 navItems，仅插件管理页启停。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: { routes: [], navItems: [], slotComponents: [] },
});
