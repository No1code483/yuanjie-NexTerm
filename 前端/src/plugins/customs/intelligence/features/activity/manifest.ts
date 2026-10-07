import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/intelligence/features/activity/manifest.rs 逐字段对齐
 *  （L2 Feature，intelligence.activity；底层智能「一切皆插件」拆分；parent customs.intelligence） */
export const manifest: Manifest = {
  id: 'intelligence.activity',
  name: '活动日志',
  level: 'feature',
  parent: 'customs.intelligence',
  slot: 'intelligence.activity',
  version: '0.1.0',
  kernelApi: '1',
  permissions: { db: [], events: { subscribe: [], publish: [] }, ipc: [], fs: [], net: [] },
  slots: [],
  i18nNamespace: 'intelligence.activity',
};
