import type { Manifest } from '@/kernel/types';

/** 与后端 plugins/customs/recycle/manifest.rs 逐字段对齐（回收站）。
 *  手稿 20260926：回收站为首页板块必备子插件（L1 custom → L2 feature 归位）。
 *  1 张自有表 recycle_bin（短码 rc + _ = rc_，不命中 → name_prefixed=0）。
 */
export const manifest: Manifest = {
  id: 'customs.recycle',
  name: '回收站',
  level: 'feature',
  parent: 'boards.home',
  slot: 'home.recycle',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    db: ['recycle_bin'],
    events: { subscribe: [], publish: [] },
    ipc: ['rc_*'],
    fs: [],
    net: [],
  },
  // 回收站「一切皆插件」：2 个纯前端视图 L2 子插件（list 必备，其余可选）
  slots: [
    { id: 'recycle.list', type: 'panel', description: '回收站列表', capacity: 1 },
    { id: 'recycle.actions', type: 'panel', description: '回收站操作', capacity: 1 },
  ],
  i18nNamespace: 'recycle',
};