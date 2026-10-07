import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/knowledge/features/templates/manifest.rs 逐字段对齐
 *  （L2 Feature，knowledge.templates，批C3 深度拆分；可选子插件） */
export const manifest: Manifest = {
  id: 'knowledge.templates',
  name: '插入模版',
  level: 'feature',
  parent: 'boards.knowledge',
  slot: 'knowledge.templates',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // kb_templates 自 L1 迁入（归属迁移见后端 features/templates/migrations/0001_baseline.sql）。
    db: ['kb_templates'],
    events: { subscribe: [], publish: [] },
    // 4 条模版命令（kb_get/create/update/delete_template，短码 kt）。
    ipc: ['kt_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'knowledge.templates',
};
