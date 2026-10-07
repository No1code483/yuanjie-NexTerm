import type { Manifest } from '@/kernel/types';

export const manifest: Manifest = {
  id: 'boards.profile',
  name: '个人中心',
  // 手稿 20260926：个人中心为首页板块必备子插件（L1 board → L2 feature 归位）
  level: 'feature',
  parent: 'boards.home',
  slot: 'home.profile',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    db: ['user_profiles', 'resumes', 'quotes'],
    events: { subscribe: [], publish: ['boards.profile:*'] },
    ipc: ['pf_*'],
    fs: [],
    net: []
  },
  // 个人中心「一切皆插件」：4 个 L2 子插件插槽（account 必备；resume/quote/settings 可选）
  slots: [
    { id: 'profile.account', type: 'panel', description: '账号', capacity: 1 },
    { id: 'profile.resume', type: 'panel', description: '简历', capacity: 1 },
    { id: 'profile.quote', type: 'panel', description: '语录', capacity: 1 },
    { id: 'profile.settings', type: 'panel', description: '设置', capacity: 1 },
  ],
  i18nNamespace: 'profile'
};
