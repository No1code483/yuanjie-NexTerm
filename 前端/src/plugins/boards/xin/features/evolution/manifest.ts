import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/xin/features/evolution/manifest.rs 逐字段对齐
 *  （L2 Feature，xin.evolution；小欣「一切皆插件」拆分） */
export const manifest: Manifest = {
  id: 'xin.evolution',
  name: '人格进化',
  level: 'feature',
  parent: 'boards.xin',
  slot: 'xin.evolution',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表与自有 IPC（好感度/特质雷达来自 xw/xn 命令）；纯前端面板。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'xin.evolution',
};
