// boards.xin L1 前端半体（阶段3 批次4a-1；manifest 与后端 manifest.rs 同源）。
// 贡献 1 条小欣路由（/xin，携带自身 routePath 供 AuthGuard 解析权限资源，/xin→xin）
// + 1 个主干 navItem + xn 命名空间 IPC 客户端（28 方法）；
// xin.wellness / xin.realtime 两个插槽由 L2 Feature 插件贡献。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { XIN_BOARD_IPC_METHODS } from './ipc';
import { manifest } from './manifest';

const XinPage = lazy(() => import('./Xin'));

/** withAuthGuard 的 routePath 决定权限资源，与静态注册期保持同一 routePath */
const GuardedXinPage = lazy(async () => ({
  default: withAuthGuard(XinPage, ROUTES.XIN),
}));

export default definePlugin({
  manifest,
  contributions: {
    routes: [{ path: ROUTES.XIN, component: GuardedXinPage }],
    navItems: [
      {
        target: 'kernel:main',
        labelKey: 'components.FloatingXin.k26',
        order: 40, // 主干第四位（约定稀疏值 10/20/.../70）
        routePath: ROUTES.XIN,
      },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'xn',
      methods: XIN_BOARD_IPC_METHODS,
    },
  },
});