// plugins/customs/search/index.ts — 搜索前端插件入口（批次6b；手稿 20260926：归为板块插件）。
// 贡献 1 条搜索路由 + 1 个主干 navItem + se 命名空间 IPC 客户端。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { SEARCH_IPC_METHODS, search } from './ipc/search';
import { manifest } from './manifest';

const SearchPage = lazy(() => import('./routes/Search'));
const GuardedSearchPage = lazy(async () => ({
  default: withAuthGuard(SearchPage, ROUTES.SEARCH),
}));

export { search };
export { SEARCH_IPC_METHODS };

export default definePlugin({
  manifest,
  contributions: {
    routes: [{ path: ROUTES.SEARCH, component: GuardedSearchPage }],
    navItems: [
      {
        target: 'kernel:main',
        labelKey: 'components.Search.k1',
        order: 40,
        routePath: ROUTES.SEARCH,
      },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'se',
      methods: SEARCH_IPC_METHODS,
    },
  },
});