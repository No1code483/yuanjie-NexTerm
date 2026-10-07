import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/ai/features/prompts/manifest.rs 逐字段对齐
 *  （L2 Feature，ai.prompts；AI会话「一切皆插件」拆分；parent boards.ai） */
export const manifest: Manifest = {
  id: 'ai.prompts',
  name: '提示词模板',
  level: 'feature',
  parent: 'boards.ai',
  slot: 'ai.prompts',
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
  i18nNamespace: 'ai.prompts',
};
