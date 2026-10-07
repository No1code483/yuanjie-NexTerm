import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/ai/features/chat/manifest.rs 逐字段对齐
 *  （L2 Feature，ai.chat；AI会话「一切皆插件」拆分；parent boards.ai；必备） */
export const manifest: Manifest = {
  id: 'ai.chat',
  name: '对话',
  level: 'feature',
  parent: 'boards.ai',
  slot: 'ai.chat',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表/命令（复用 L1 boards.ai 的 ai_* 与 L2 ai.sessions 的 ss_* 命名空间）。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'ai.chat',
};
