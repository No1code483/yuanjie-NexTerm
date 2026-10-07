// boards.terminal L1 前端半体（阶段3 批次3a；manifest 与后端 manifest.rs 同源）。
// 贡献 3 条终端路由（/terminal、/terminal/terminal、/terminal/linux —— 后两者为同页 Tab
// 形态，均渲染 Terminal.tsx，各携带自身 routePath 供 AuthGuard 解析权限资源）+ 主干 navItem；
// terminal.linux 插槽由 3b 的 terminal.linux L2 贡献；terminal.yuancode 插槽由 3c 的
// terminal.yuancode L2 贡献；terminal.manual 插槽由批C2 的 terminal.manual L2 贡献。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { manifest } from './manifest';
import { TERMINAL_IPC_METHODS } from './ipc';

const TerminalPage = lazy(() => import('./Terminal'));

/** 每条路由需独立的守卫包装：withAuthGuard 的 routePath 决定权限资源，不可共用 */
const guardTerminal = (routePath: string) =>
  lazy(async () => ({ default: withAuthGuard(TerminalPage, routePath) }));

export default definePlugin({
  manifest,
  contributions: {
    routes: [
      { path: ROUTES.TERMINAL, component: guardTerminal(ROUTES.TERMINAL) },
      { path: ROUTES.TERMINAL_TERMINAL, component: guardTerminal(ROUTES.TERMINAL_TERMINAL) },
      { path: ROUTES.TERMINAL_LINUX, component: guardTerminal(ROUTES.TERMINAL_LINUX) },
      // 命令手册 5 条路由已随批C2 迁入 terminal.manual L2（features/manual/）
    ],
    navItems: [
      {
        target: 'kernel:main',
        labelKey: 'components.PermissionRestricted.k2',
        order: 30, // 主干第三位（约定稀疏值 10/20/.../70）
        routePath: ROUTES.TERMINAL,
      },
    ],
    slotComponents: [],
    // 3a：L1 承载终端 29 条命令（client 收编已消费 10 条）。
    ipc: {
      namespace: 'tm',
      methods: TERMINAL_IPC_METHODS,
    },
  },
});
