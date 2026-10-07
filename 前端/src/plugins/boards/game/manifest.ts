import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/game/manifest.rs 逐字段对齐（L1 Board，游戏 3D 重构）。
 *  覆盖面：game 35 + game_story 4 + game_intelligence 2 + game_opponent 1
 *  + game_natural_language 1 + game_behavior 1 = 44 条 IPC。L2 插槽 `game.3d` /
 *  `game.2dpreview` 由后续批次挂载。
 */
export const manifest: Manifest = {
  id: 'boards.game',
  name: '游戏',
  level: 'board',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 17 张自有表（短码 `gm` 与 `game_` 前缀不命中 → name_prefixed=0）。
    db: [
      'game_worlds',
      'game_knowledge_domains',
      'game_buildings',
      'game_knowledge_progress',
      'game_breakthrough_records',
      'game_build_history',
      'game_kb_category_mapping',
      'game_points_log',
      'game_daily_limit_counter',
      'game_points_source_config',
      'game_npcs',
      'game_npc_conversations',
      'game_stories',
      'game_story_nodes',
      'game_npc_memories',
      'game_npc_relationships',
      'game_npc_rumors',
      'game_player_skill',
    ],
    // 本批不发布/订阅内核领域事件（S5 按实测登记）。
    events: { subscribe: [], publish: [] },
    // 44 条 IPC；ACL/handler 键由内核按短码 `gm` 构造（<短码>_<命令名>）。
    ipc: ['gm_*'],
    fs: [],
    net: [],
  },
  // 4b 后续批次挂载：game.3d / game.2dpreview
  // 游戏「一切皆插件」（2026-10-05）：再增 game.preview（必备）/ game.play3d（必备）/ game.mapping
  slots: [
    { id: 'game.3d', type: 'panel', description: '游戏 3D 场景', capacity: 1 },
    { id: 'game.2dpreview', type: 'panel', description: '游戏 2D 预览', capacity: 1 },
    { id: 'game.preview', type: 'panel', description: '游戏预览', capacity: 1 },
    { id: 'game.play3d', type: 'panel', description: '3D 游戏', capacity: 1 },
    { id: 'game.mapping', type: 'panel', description: '知识映射', capacity: 1 },
  ],
  i18nNamespace: 'game',
};
