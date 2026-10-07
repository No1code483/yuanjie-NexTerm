// search.browser L2 前端半体（搜索「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 必备子插件：承载浏览器搜索模式（多标签/WebView 视口/导航/引擎），组件物理迁入本插件目录。
// 嵌入型子插件（/search 侧边栏为收藏网站列表而非功能导航），不贡献 navItems，仅插件管理页启停。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: { routes: [], navItems: [], slotComponents: [] },
});
