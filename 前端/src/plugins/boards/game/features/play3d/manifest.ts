import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/game/features/play3d/manifest.rs 逐字段对齐
 *  （L2 Feature，game.play3d；游戏「一切皆插件」拆分；parent boards.game；必备） */
export const manifest: Manifest = {
  id: 'game.play3d',
  name: '3D 游戏',
  level: 'feature',
  parent: 'boards.game',
  slot: 'game.play3d',
  version: '0.1.0',
  kernelApi: '1',
  permissions: { db: [], events: { subscribe: [], publish: [] }, ipc: [], fs: [], net: [] },
  slots: [],
  i18nNamespace: 'game.play3d',
};
