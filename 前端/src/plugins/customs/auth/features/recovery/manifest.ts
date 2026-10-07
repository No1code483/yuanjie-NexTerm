import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/auth/features/recovery/manifest.rs 逐字段对齐
 *  （L2 Feature，auth.recovery；认证「一切皆插件」拆分） */
export const manifest: Manifest = {
  id: 'auth.recovery',
  name: '找回密码',
  level: 'feature',
  parent: 'customs.auth',
  slot: 'auth.recovery',
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
  i18nNamespace: 'auth.recovery',
};
