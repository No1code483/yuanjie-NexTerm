import { definePlugin } from '@/kernel/registry/definePlugin';
import { AUTH_IPC_METHODS } from './ipc/auth';
import { manifest } from './manifest';

/** L3 定制级插件（横切）：不挂路由/导航/插槽，只贡献 IPC 命名空间。 */
export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [],
    slotComponents: [],
    ipc: {
      namespace: 'au',
      methods: AUTH_IPC_METHODS
    }
  }
});
