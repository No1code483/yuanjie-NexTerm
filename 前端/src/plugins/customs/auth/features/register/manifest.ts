import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/auth/features/register/manifest.rs 逐字段对齐
 *  （L2 Feature，auth.register；认证「一切皆插件」拆分） */
export const manifest: Manifest = {
  id: 'auth.register',
  name: '注册',
  level: 'feature',
  parent: 'customs.auth',
  slot: 'auth.register',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表与自有 IPC（au_* 归 L1）；纯前端视图。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'auth.register',
};
