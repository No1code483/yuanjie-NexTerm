import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/xin/features/mood/manifest.rs 逐字段对齐
 *  （L2 Feature，xin.mood；小欣「一切皆插件」拆分） */
export const manifest: Manifest = {
  id: 'xin.mood',
  name: '心情',
  level: 'feature',
  parent: 'boards.xin',
  slot: 'xin.mood',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表（共用 L1 的 xin_moods）与自有 IPC；纯前端面板。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'xin.mood',
};
