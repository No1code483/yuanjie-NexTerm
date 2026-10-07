import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/knowledge/features/media/manifest.rs 逐字段对齐
 *  （L2 Feature，knowledge.media；本轮知识库「一切皆插件」拆分；可选子插件） */
export const manifest: Manifest = {
  id: 'knowledge.media',
  name: '媒体查看器',
  level: 'feature',
  parent: 'boards.knowledge',
  slot: 'knowledge.media',
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
  i18nNamespace: 'knowledge.media',
};
