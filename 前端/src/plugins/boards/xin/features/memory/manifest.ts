import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/xin/features/memory/manifest.rs 逐字段对齐
 *  （L2 Feature，xin.memory；小欣「一切皆插件」拆分） */
export const manifest: Manifest = {
  id: 'xin.memory',
  name: '记忆',
  level: 'feature',
  parent: 'boards.xin',
  slot: 'xin.memory',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表（共用 L1 的 xin_memories 等）与自有 IPC；纯前端面板。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'xin.memory',
};
