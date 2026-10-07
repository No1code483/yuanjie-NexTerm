import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/home/features/journal/manifest.rs 逐字段对齐（L2 Feature） */
export const manifest: Manifest = {
  id: 'home.journal',
  name: '日志',
  level: 'feature',
  parent: 'boards.home',
  slot: 'home.journal',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // journals 为既有旧表，归属登记见后端 migrations/0001_baseline.sql。
    db: ['journals'],
    // 本批不发布/订阅领域事件（home 域事件契约属批次 1b-2c；本批仅 home.todo 接入，裁定 25-A）。
    events: { subscribe: [], publish: [] },
    // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
    ipc: ['jn_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'home.journal',
};
