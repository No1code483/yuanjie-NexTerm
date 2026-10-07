import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/xin/features/orchestration/manifest.rs 逐字段对齐（L2 Feature）。 */
export const manifest: Manifest = {
  id: 'xin.orchestration',
  name: '小欣编排',
  level: 'feature',
  parent: 'boards.xin',
  slot: 'xin.orchestration',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 4 张编排面表（仅归属登记，已由主应用历史迁移创建）。
    // 短码 `xo` 与 `xin_` 前缀不命中 → name_prefixed=0（裁定 2）。
    db: [
      'xin_conversations',
      'xin_checkpoints',
      'xin_compaction_config',
      'xin_compaction_records',
    ],
    events: { subscribe: [], publish: [] },
    // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
    ipc: ['xo_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'xin.orchestration',
};
