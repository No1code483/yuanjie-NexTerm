import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/xin/features/wellness/manifest.rs 逐字段对齐（L2 Feature） */
export const manifest: Manifest = {
  id: 'xin.wellness',
  name: '小欣健康助手',
  level: 'feature',
  parent: 'boards.xin',
  slot: 'xin.wellness',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 2 张零写入预留表（短码 `xw` 与 `xin_` 前缀不命中 → name_prefixed=0）；
    // 归属登记见后端 features/wellness/migrations/0001_baseline.sql。
    db: ['xin_reminders', 'xin_habits'],
    events: { subscribe: [], publish: [] },
    // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
    ipc: ['xw_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'xin.wellness',
};