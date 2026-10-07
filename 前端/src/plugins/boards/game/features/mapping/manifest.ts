import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/game/features/mapping/manifest.rs 逐字段对齐
 *  （L2 Feature，game.mapping；游戏「一切皆插件」拆分；parent boards.game） */
export const manifest: Manifest = {
  id: 'game.mapping',
  name: '知识映射',
  level: 'feature',
  parent: 'boards.game',
  slot: 'game.mapping',
  version: '0.1.0',
  kernelApi: '1',
  permissions: { db: [], events: { subscribe: [], publish: [] }, ipc: [], fs: [], net: [] },
  slots: [],
  i18nNamespace: 'game.mapping',
};
