// plugins/customs/recycle/index.ts — 回收站前端插件入口（批次6a；手稿 20260926 归位为首页必备子插件）。
// 贡献 1 条回收站路由（相对路径，由 buildRouter 拼父前缀 /home）+ 1 个 home.recycle 插槽菜单 + rc 命名空间 IPC 客户端。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { RECYCLE_IPC_METHODS, recycleBin } from './ipc/recycle';
import { manifest } from './manifest';

const RecyclePage = lazy(() => import('./features/list/RecyclePage'));
/** 守卫参数用绝对路径（决定权限资源），路由注册用相对路径由 buildRouter 拼父前缀 /home */
const GuardedRecyclePage = lazy(async () => ({
  default: withAuthGuard(RecyclePage, ROUTES.RECYCLE),
}));

export { recycleBin };
export { RECYCLE_IPC_METHODS };

export default definePlugin({
  manifest,
  contributions: {
    routes: [{ path: 'recycle', component: GuardedRecyclePage }],
    navItems: [
      {
        target: 'home.recycle',
        labelKey: 'components.recycle.Bin.k1',
        order: 60,
        routePath: 'recycle',
      },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'rc',
      methods: RECYCLE_IPC_METHODS,
    },
  },
});