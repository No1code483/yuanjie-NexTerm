import type { Manifest } from '@/kernel/types';

/** 与后端 plugins/customs/intelligence/manifest.rs 逐字段对齐（L1 Custom，底层智能）。
 *  4 张自有表：activity_logs / suggestions / behavior_patterns / intelligence_settings
 *  (由 migrations v61-v64 创建，db_归属判定表 §4 登记 ownership)。
 */
export const manifest: Manifest = {
  id: 'customs.intelligence',
  name: '底层智能',
  level: 'custom',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    db: ['activity_logs', 'suggestions', 'behavior_patterns', 'intelligence_settings'],
    events: {
      subscribe: ['home:*', 'kb:*', 'ai:*', 'game:*'],
      publish: ['intelligence:*'],
    },
    ipc: ['sp_*'],
    fs: [],
    net: [],
  },
  // 底层智能「一切皆插件」：5 个 L2 子插件插槽（dashboard/settings 必备；其余可选）
  slots: [
    { id: 'intelligence.dashboard', type: 'panel', description: '仪表盘', capacity: 1 },
    { id: 'intelligence.suggestions', type: 'panel', description: '智能建议', capacity: 1 },
    { id: 'intelligence.behavior', type: 'panel', description: '行为分析', capacity: 1 },
    { id: 'intelligence.activity', type: 'panel', description: '活动日志', capacity: 1 },
    { id: 'intelligence.settings', type: 'panel', description: '设置', capacity: 1 },
  ],
  i18nNamespace: 'intelligence',
};