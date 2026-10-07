import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/game/features/preview/manifest.rs 逐字段对齐
 *  （L2 Feature，game.preview；游戏「一切皆插件」拆分；parent boards.game；必备） */
export const manifest: Manifest = {
  id: 'game.preview',
  name: '游戏预览',
  level: 'feature',
  parent: 'boards.game',
  slot: 'game.preview',
  version: '0.1.0',
  kernelApi: '1',
  permissions: { db: [], events: { subscribe: [], publish: [] }, ipc: [], fs: [], net: [] },
  slots: [],
  i18nNamespace: 'game.preview',
};
