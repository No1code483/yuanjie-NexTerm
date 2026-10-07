import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/home/features/focus/manifest.rs 逐字段对齐
 *  （L2 Feature，home.focus，批C4；可选子插件） */
export const manifest: Manifest = {
  id: 'home.focus',
  name: '专注',
  level: 'feature',
  parent: 'boards.home',
  slot: 'home.focus',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 无业务表（专注历史存 localStorage，key: nexterm_focus_history）。
    db: [],
    events: { subscribe: [], publish: [] },
    // 无 IPC（纯前端计时组件，Web Audio 白噪音本地生成）。
    ipc: [],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'home.focus',
};
