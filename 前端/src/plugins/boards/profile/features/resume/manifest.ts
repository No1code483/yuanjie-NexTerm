import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/profile/features/resume/manifest.rs 逐字段对齐
 *  （L2 Feature，profile.resume；个人中心「一切皆插件」拆分；parent boards.profile） */
export const manifest: Manifest = {
  id: 'profile.resume',
  name: '简历',
  level: 'feature',
  parent: 'boards.profile',
  slot: 'profile.resume',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'profile.resume',
};
