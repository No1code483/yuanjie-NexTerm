import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/xin/features/skill/manifest.rs 逐字段对齐
 *  （L2 Feature，xin.skill；小欣「一切皆插件」拆分） */
export const manifest: Manifest = {
  id: 'xin.skill',
  name: '技能',
  level: 'feature',
  parent: 'boards.xin',
  slot: 'xin.skill',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无自有表与自有 IPC（技能列表/执行来自 xo 命令）；纯前端面板。
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'xin.skill',
};
