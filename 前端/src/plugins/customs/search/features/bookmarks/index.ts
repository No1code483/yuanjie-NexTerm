// search.bookmarks L2 前端半体（搜索「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载收藏站点管理（localStorage nexterm_bookmarks），组件/逻辑物理迁入本插件目录。
// 嵌入型子插件，不贡献 navItems，仅插件管理页启停（/search 侧边栏由 Layout 收藏列表承载）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: { routes: [], navItems: [], slotComponents: [] },
});
