import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/xin/features/chat/manifest.rs 逐字段对齐
 *  （L2 Feature，xin.chat；小欣「一切皆插件」拆分；必备子插件） */
export const manifest: Manifest = {
  id: 'xin.chat',
  name: '对话',
  level: 'feature',
  parent: 'boards.xin',
  slot: 'xin.chat',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表（共用 L1 的 xin_* 表）、无自有 IPC（xn/xo 全集归属 L1/L2）；纯前端面板。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'xin.chat',
};
