import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/customs/intelligence/features/dashboard/manifest.rs 逐字段对齐
 *  （L2 Feature，intelligence.dashboard；底层智能「一切皆插件」拆分；parent customs.intelligence；必备） */
export const manifest: Manifest = {
  id: 'intelligence.dashboard',
  name: '仪表盘',
  level: 'feature',
  parent: 'customs.intelligence',
  slot: 'intelligence.dashboard',
  version: '0.1.0',
  kernelApi: '1',
  permissions: { db: [], events: { subscribe: [], publish: [] }, ipc: [], fs: [], net: [] },
  slots: [],
  i18nNamespace: 'intelligence.dashboard',
};
