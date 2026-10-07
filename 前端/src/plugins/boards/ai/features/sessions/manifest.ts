import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/ai/features/sessions/manifest.rs 逐字段对齐（L2 Feature）
 * batchC1：会话列表插件（AI会话必备子插件），拥有会话面 IPC + 4 张会话表。 */
export const manifest: Manifest = {
  id: 'ai.sessions',
  name: '会话列表',
  level: 'feature',
  parent: 'boards.ai',
  slot: 'ai.sessions',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 4 张会话表归标本插件（BUG-033 修订 2026-09-27：归属经本插件后端
    // migrations/0002_transfer_ownership.sql 自 boards.ai 转移而来）。
    db: ['conversations', 'conversation_participants', 'messages', 'prompt_templates'],
    events: { subscribe: [], publish: [] },
    // ACL/handler 键由内核按本插件短码 `ss` 构造。
    ipc: ['ss_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'ai.sessions',
};
