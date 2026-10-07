// game.preview L2 前端半体（游戏「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 必备子插件：承载 2D 游戏预览页（原 src/pages/game/，GamePreview 及其组件），物理迁入本插件目录。
// 路由级子插件（/game），不贡献 navItems。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: { routes: [], navItems: [], slotComponents: [] },
});
