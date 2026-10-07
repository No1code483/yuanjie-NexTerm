// boards.knowledge L1 前端半体（阶段3 批次2a-1；manifest 与后端 manifest.rs 同源）。
// 单 L1 无 L2（裁定 33-A）：贡献 1 条知识库路由（携带自身 routePath 供 AuthGuard
// 解析权限资源，/knowledge→knowledge）+ 1 个主干 navItem + kb 命名空间 IPC 客户端。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { KNOWLEDGE_IPC_METHODS } from './ipc';
import { manifest } from './manifest';

const KnowledgePage = lazy(() => import('./Knowledge'));

/** withAuthGuard 的 routePath 决定权限资源，与静态注册期保持同一 routePath */
const GuardedKnowledgePage = lazy(async () => ({
  default: withAuthGuard(KnowledgePage, ROUTES.KNOWLEDGE),
}));

export default definePlugin({
  manifest,
  contributions: {
    routes: [{ path: ROUTES.KNOWLEDGE, component: GuardedKnowledgePage }],
    navItems: [
      {
        target: 'kernel:main',
        labelKey: 'components.intelligence.ActivityPanel.k1',
        order: 30, // 主干第三位（约定稀疏值 10/20/.../70）
        routePath: ROUTES.KNOWLEDGE,
      },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'kb',
      methods: KNOWLEDGE_IPC_METHODS,
    },
  },
});
