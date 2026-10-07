// plugins/customs/systemtools/index.ts — customs.systemtools L1 前端插件入口（批次4c）。
// 横切定制级插件：不挂路由/导航/插槽，只贡献 IPC 命名空间客户端。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { SYSTEMTOOLS_IPC_METHODS, systemtools } from './ipc/systemtools';
import { manifest } from './manifest';

export { systemtools };

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [],
    slotComponents: [],
    ipc: {
      namespace: 'st',
      methods: SYSTEMTOOLS_IPC_METHODS
    }
  }
});
