import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/auth/features/login/manifest.rs 逐字段对齐
 *  （L2 Feature，auth.login；认证「一切皆插件」拆分；必备子插件） */
export const manifest: Manifest = {
  id: 'auth.login',
  name: '登录',
  level: 'feature',
  parent: 'customs.auth',
  slot: 'auth.login',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表（共用 L1 的 users 等）与自有 IPC（au_* 全集归属 L1）；纯前端视图。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'auth.login',
};
