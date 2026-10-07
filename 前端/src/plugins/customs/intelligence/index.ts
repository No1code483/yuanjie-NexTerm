// plugins/customs/intelligence/index.ts — customs.intelligence L1 前端插件入口（批次6c）。
// 横切定制级插件：贡献 1 条 spyglass 路由 + 1 个主干 navItem + sp 命名空间 IPC 客户端。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { INTELLIGENCE_IPC_METHODS, intelligence } from './ipc/intelligence';
import { manifest } from './manifest';

const IntelligencePage = lazy(() => import('./routes/Intelligence'));
const GuardedIntelligencePage = lazy(async () => ({
  default: withAuthGuard(IntelligencePage, ROUTES.SPYGLASS),
}));

export { intelligence };
export { INTELLIGENCE_IPC_METHODS };

export default definePlugin({
  manifest,
  contributions: {
    routes: [{ path: ROUTES.SPYGLASS, component: GuardedIntelligencePage }],
    navItems: [
      {
        target: 'kernel:main',
        labelKey: 'components.intelligence.spyglass',
        order: 20,
        routePath: ROUTES.SPYGLASS,
      },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'sp',
      methods: INTELLIGENCE_IPC_METHODS,
    },
  },
});
