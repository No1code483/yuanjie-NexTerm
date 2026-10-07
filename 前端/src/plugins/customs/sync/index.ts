// plugins/customs/sync/index.ts — customs.sync L1 前端插件入口（批次6b）。
// 定制级插件：贡献 /sync 路由 + 1 个主干 navItem + sy 命名空间 IPC 客户端。
// 「一切皆插件」拆分：2 个纯前端视图 L2（sync.devices / sync.conflicts）挂 slot，由 /sync 壳分发展示。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { SYNC_IPC_METHODS } from './ipc/sync';
import { manifest } from './manifest';

const SyncPage = lazy(() => import('./routes/SyncPage'));
const GuardedSyncPage = lazy(async () => ({ default: withAuthGuard(SyncPage, ROUTES.SYNC) }));

export { sync } from './ipc/sync';
export { SYNC_IPC_METHODS } from './ipc/sync';
export { useSyncStore } from './store';
export type { SyncState } from './store';
export type {
  SyncOperation,
  SyncStatus,
  SyncQueueItem,
  SyncQueueStats,
  SyncDevice,
  SyncRunResult,
  EncryptedPayload,
  SerializedKeyPair,
  NetworkStatus,
} from './ipc/sync';

export default definePlugin({
  manifest,
  contributions: {
    routes: [{ path: ROUTES.SYNC, component: GuardedSyncPage }],
    navItems: [
      {
        target: 'kernel:main',
        labelKey: 'components.Sync.k1',
        icon: '🔄',
        order: 100,
        routePath: ROUTES.SYNC,
      },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'sy',
      methods: SYNC_IPC_METHODS,
    },
  },
});
