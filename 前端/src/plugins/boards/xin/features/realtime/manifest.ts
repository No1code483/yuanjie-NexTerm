import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/xin/features/realtime/manifest.rs 逐字段对齐（L2 Feature） */
export const manifest: Manifest = {
  id: 'xin.realtime',
  name: '小欣实时语音',
  level: 'feature',
  parent: 'boards.xin',
  slot: 'xin.realtime',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 不持有业务表（内存态会话，不落库）。
    db: [],
    events: { subscribe: [], publish: [] },
    // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
    ipc: ['xr_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'xin.realtime',
};