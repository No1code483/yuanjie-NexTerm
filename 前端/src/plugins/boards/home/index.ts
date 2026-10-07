// boards.home L1 前端半体（阶段3 批次1b-1；manifest 与后端 manifest.rs 同源）。
// 7 条首页路由各携带自身 routePath（AuthGuard 据此解析权限资源：
// /home→home、/home/news→home_news、/home/todo→home_todo、/home/log→home_log、
// /home/timer*→home_timer）；todo / journal 面板由 L2 插件经插槽贡献。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { manifest } from './manifest';

const HomePage = lazy(() => import('./Home'));

/** 每条路由需独立的守卫包装：withAuthGuard 的 routePath 决定权限资源，不可共用 */
const guardHome = (routePath: string) =>
  lazy(async () => ({ default: withAuthGuard(HomePage, routePath) }));

export default definePlugin({
  manifest,
  contributions: {
    routes: [
      { path: ROUTES.HOME, component: guardHome(ROUTES.HOME) },
      { path: ROUTES.HOME_NEWS, component: guardHome(ROUTES.HOME_NEWS) },
      { path: ROUTES.HOME_TODO, component: guardHome(ROUTES.HOME_TODO) },
      { path: ROUTES.HOME_LOG, component: guardHome(ROUTES.HOME_LOG) },
      { path: ROUTES.HOME_TIMER, component: guardHome(ROUTES.HOME_TIMER) },
      { path: ROUTES.HOME_TIMER_SHORT, component: guardHome(ROUTES.HOME_TIMER_SHORT) },
      { path: ROUTES.HOME_TIMER_LONG, component: guardHome(ROUTES.HOME_TIMER_LONG) },
    ],
    navItems: [
      {
        target: 'kernel:main',
        labelKey: 'hooks.useModuleTheme.home',
        order: 10, // 主干首位（约定稀疏值 10/20/.../70）
        routePath: ROUTES.HOME,
      },
    ],
    slotComponents: [],
  },
});
