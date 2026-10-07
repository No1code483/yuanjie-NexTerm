import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/profile/features/account/manifest.rs 逐字段对齐
 *  （L2 Feature，profile.account；个人中心「一切皆插件」拆分；parent boards.profile；必备子插件） */
export const manifest: Manifest = {
  id: 'profile.account',
  name: '账号',
  level: 'feature',
  parent: 'boards.profile',
  slot: 'profile.account',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表（共用 L1 的 user_profiles/resumes/quotes）；无自有 IPC（pf_* 全集归属 L1）。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'profile.account',
};
