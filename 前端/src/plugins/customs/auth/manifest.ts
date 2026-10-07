import type { Manifest } from '@/kernel/types';

/** 与后端 plugins/customs/auth/manifest.rs 严格对齐（阶段3 批次1a-1 / 1a-2a）。 */
export const manifest: Manifest = {
  id: 'customs.auth',
  name: '认证体系',
  level: 'custom',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    db: ['users', 'permissions', 'auth_sessions', 'mek_versions', 'mek_rotation_log'],
    events: { subscribe: [], publish: [] },
    ipc: ['au_*'],
    fs: [],
    net: []
  },
  // 认证「一切皆插件」：4 个纯前端视图 L2 子插件（login 必备，其余可选）
  slots: [
    { id: 'auth.login', type: 'panel', description: '登录', capacity: 1 },
    { id: 'auth.register', type: 'panel', description: '注册', capacity: 1 },
    { id: 'auth.recovery', type: 'panel', description: '找回密码', capacity: 1 },
    { id: 'auth.temp', type: 'panel', description: '临时账号', capacity: 1 },
  ],
  i18nNamespace: 'auth'
};
