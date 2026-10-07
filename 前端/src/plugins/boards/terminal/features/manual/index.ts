// terminal.manual L2 前端半体（C2 拆分：命令手册从 L1 routes/ 物理迁出）。
// 贡献 5 条命令手册路由（/terminal/manual 及子路由），纯前端静态页面，无 IPC。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { manifest } from './manifest';

const CommandManualPage = lazy(() => import('./CommandManual'));

/** 每条路由需独立的守卫包装：withAuthGuard 的 routePath 决定权限资源，不可共用 */
const guardCommandManual = (routePath: string) =>
  lazy(async () => ({ default: withAuthGuard(CommandManualPage, routePath) }));

export default definePlugin({
  manifest,
  contributions: {
    routes: [
      // L2 路径必须相对（PluginRegistry.buildRouter 会拼父前缀 /terminal）；
      // 绝对路径 '/terminal/manual' 会注册成 '/terminal/terminal/manual' → 404
      { path: 'manual', component: guardCommandManual(ROUTES.TERMINAL_MANUAL) },
      { path: 'manual/terminal', component: guardCommandManual(ROUTES.TERMINAL_MANUAL_TERMINAL) },
      { path: 'manual/yuancode', component: guardCommandManual(ROUTES.TERMINAL_MANUAL_YUANCODE) },
      { path: 'manual/linux', component: guardCommandManual(ROUTES.TERMINAL_MANUAL_LINUX) },
      { path: 'manual/shortcuts', component: guardCommandManual(ROUTES.TERMINAL_MANUAL_SHORTCUTS) },
    ],
    navItems: [],
    slotComponents: [],
  },
});
