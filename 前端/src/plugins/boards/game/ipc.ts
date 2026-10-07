// plugins/boards/game/ipc.ts — boards.game L1 IPC 客户端（短码 gm，批次4b）。
// 契约：统一经内核 dispatcher，逻辑名 `gm:plugin:<旧命令名>`；参数形状与迁移前
// 旧封装逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 44 条 alias：游戏 35 + story 4 + intelligence 2 + opponent 1 + nl 1 + behavior 1。
//
// **v1 缺陷随批修复（裁定 45-A）**：旧封装缺少 `syncKnowledgeEvent`（对应
// `game_sync_knowledge_event`，已在后端 alias 中）。旧封装额外包含
// `getKbCategoryMappings`（调用 `game_get_kb_category_mappings`，迁移前主线确实
// 注册于 main.rs#L998），本文件补全为 44 条与后端 IPC_ALIASES 完全对齐的方法。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';
import type {
  GameBuilding, GameBuildHistory, GameBreakthroughRecord, GameBuildingStatus,
  GameKnowledgeDomain, GameKnowledgeDomainId, GameKnowledgeProgress, GameKbCategoryMapping, PointsTrend,
  WorldState, WorldSummary, RealmInfo, StartBuildingRequest, BreakthroughSession,
  BreakthroughAnswer, BreakthroughOutcome, BreakthroughPreviewInfo, BuildingCatalog,
  SyncResult, EventsAndTasks, TimelineEvent, RebuiltScene, GameNpc, GameNpcConversation,
  NpcChatResponse, GameNpcMemory, GameNpcRelationship, GameNpcRumor, PlayerSkill,
  Story, StorySummary, GenerateStoryRequest, AdvanceStoryRequest, ParsedCommand,
  ParseCommandRequest, BehaviorAnalysis, AnalyzeBehaviorRequest, IntelligenceHookStatus,
} from './shared/types';

export const GAME_IPC_METHODS = {
  // ===== 世界与境界（6）=====
  initWorld: { cmd: 'game_init_world' },
  getWorldState: { cmd: 'game_get_world_state' },
  getRealmInfo: { cmd: 'game_get_realm_info' },
  getBreakthroughPreview: { cmd: 'game_get_breakthrough_preview' },
  startBreakthrough: { cmd: 'game_start_breakthrough' },
  submitBreakthrough: { cmd: 'game_submit_breakthrough' },
  // ===== 建筑系统（7）=====
  getBuildings: { cmd: 'game_get_buildings' },
  getBuildingCatalog: { cmd: 'game_get_building_catalog' },
  startBuilding: { cmd: 'game_start_building' },
  upgradeBuilding: { cmd: 'game_upgrade_building' },
  removeBuilding: { cmd: 'game_remove_building' },
  moveBuilding: { cmd: 'game_move_building' },
  getBuildHistory: { cmd: 'game_get_build_history' },
  // ===== 知识联动（6）=====
  getKnowledgeDomains: { cmd: 'game_get_knowledge_domains' },
  getKnowledgeProgress: { cmd: 'game_get_knowledge_progress' },
  mapKbCategory: { cmd: 'game_map_kb_category' },
  getKbCategoryMappings: { cmd: 'game_get_kb_category_mappings' },
  syncKnowledgeEvent: { cmd: 'game_sync_knowledge_event' },
  getPointsTrend: { cmd: 'game_get_points_trend' },
  // ===== 突破历史 + 世界列表（3）=====
  getBreakthroughHistory: { cmd: 'game_get_breakthrough_history' },
  listWorlds: { cmd: 'game_list_worlds' },
  deleteWorld: { cmd: 'game_delete_world' },
  // ===== 统计聚合便利（2）=====
  getEventsAndTasks: { cmd: 'game_get_events_and_tasks' },
  getBuildTimeline: { cmd: 'game_get_build_timeline' },
  // ===== 时间轴回放（2）=====
  getWorldSnapshot: { cmd: 'game_get_world_snapshot' },
  exitReplay: { cmd: 'game_exit_replay' },
  // ===== NPC 系统（5）=====
  npcList: { cmd: 'game_npc_list' },
  npcGet: { cmd: 'game_npc_get' },
  npcHistory: { cmd: 'game_npc_history' },
  npcChat: { cmd: 'game_npc_chat' },
  npcClearHistory: { cmd: 'game_npc_clear_history' },
  // ===== D4.6 NPC 深化（2）=====
  npcMemories: { cmd: 'game_npc_memories' },
  npcRelationship: { cmd: 'game_npc_relationship' },
  // ===== D4.7 传闻机制（1）=====
  npcRumors: { cmd: 'game_npc_rumors' },
  // ===== D4.3 自适应难度（1）=====
  playerSkill: { cmd: 'game_player_skill' },
  // ===== D4.3 游戏对手 AI（1）=====
  opponentDecide: { cmd: 'game_opponent_decide' },
  // ===== D4.4 动态剧情（4）=====
  storyGenerate: { cmd: 'game_story_generate' },
  storyAdvance: { cmd: 'game_story_advance' },
  storyGet: { cmd: 'game_story_get' },
  storyList: { cmd: 'game_story_list' },
  // ===== D4.4 自然语言交互（1）=====
  nlParse: { cmd: 'game_nl_parse' },
  // ===== D4.6 行为分析 AI 洞察（1）=====
  analyzeBehavior: { cmd: 'game_analyze_behavior' },
  // ===== D4.6 数据底层智能监测接入（2）=====
  intelligenceRecordEvent: { cmd: 'game_intelligence_record_event' },
  intelligenceStatus: { cmd: 'game_intelligence_status' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherGm = defineIpcNamespace('gm', GAME_IPC_METHODS);
type GmMethod = keyof typeof GAME_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeGm<T = any>(method: GmMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherGm[method](args) as Promise<ApiResponse<T>>;
}

/** 参数键与迁移前旧封装（`lib/ipc/game.ts`）逐字一致；后端 dispatcher 以 snake_case
 *  优先、camelCase 回退，两种口径均可绑定。方法命名沿用旧 `game.*` 门面对齐调用方最小改动。 */
export const game = {
  // === 世界与境界（6）===
  initWorld: (playerName?: string) => invokeGm<WorldState>('initWorld', { player_name: playerName }),
  getWorldState: (worldId: string) => invokeGm<WorldState | null>('getWorldState', { world_id: worldId }),
  getRealmInfo: (worldId: string) => invokeGm<RealmInfo>('getRealmInfo', { world_id: worldId }),
  getBreakthroughPreview: (worldId: string) => invokeGm<BreakthroughPreviewInfo>('getBreakthroughPreview', { world_id: worldId }),
  startBreakthrough: (worldId: string) => invokeGm<BreakthroughSession>('startBreakthrough', { world_id: worldId }),
  submitBreakthrough: (session: BreakthroughSession, answers: BreakthroughAnswer[]) =>
    invokeGm<BreakthroughOutcome>('submitBreakthrough', { session, answers }),
  // === 建筑系统（7）===
  getBuildings: (worldId: string, status?: GameBuildingStatus) =>
    invokeGm<GameBuilding[]>('getBuildings', { world_id: worldId, status }),
  getBuildingCatalog: () => invokeGm<BuildingCatalog>('getBuildingCatalog'),
  startBuilding: (request: StartBuildingRequest) => invokeGm<GameBuilding>('startBuilding', { request }),
  upgradeBuilding: (buildingId: string, upgradeCost: number) =>
    invokeGm<GameBuilding>('upgradeBuilding', { building_id: buildingId, upgrade_cost: upgradeCost }),
  removeBuilding: (buildingId: string, refundRatio?: number) =>
    invokeGm<number>('removeBuilding', { building_id: buildingId, refund_ratio: refundRatio }),
  moveBuilding: (buildingId: string, newPosX: number, newPosY: number, newPosZ: number, moveCost: number, newRotationY?: number) =>
    invokeGm<GameBuilding>('moveBuilding', {
      building_id: buildingId,
      new_pos_x: newPosX,
      new_pos_y: newPosY,
      new_pos_z: newPosZ,
      new_rotation_y: newRotationY,
      move_cost: moveCost,
    }),
  getBuildHistory: (worldId: string, limit?: number) =>
    invokeGm<GameBuildHistory[]>('getBuildHistory', { world_id: worldId, limit }),
  // === 知识联动（6）===
  getKnowledgeDomains: () => invokeGm<GameKnowledgeDomain[]>('getKnowledgeDomains'),
  getKnowledgeProgress: (worldId: string, domainId?: GameKnowledgeDomainId) =>
    invokeGm<GameKnowledgeProgress[]>('getKnowledgeProgress', { world_id: worldId, domain_id: domainId }),
  mapKbCategory: (categoryId: number, domainId: GameKnowledgeDomainId) =>
    invokeGm<void>('mapKbCategory', { category_id: categoryId, domain_id: domainId }),
  getKbCategoryMappings: () => invokeGm<GameKbCategoryMapping[]>('getKbCategoryMappings'),
  syncKnowledgeEvent: (worldId: string, eventType: string, domainId: string, points: number, sourceData: Record<string, unknown>) =>
    invokeGm<SyncResult>('syncKnowledgeEvent', {
      world_id: worldId,
      event_type: eventType,
      domain_id: domainId,
      points,
      source_data: sourceData,
    }),
  getPointsTrend: (worldId: string, days?: number) =>
    invokeGm<PointsTrend>('getPointsTrend', { world_id: worldId, days }),
  // === 突破历史 + 世界列表（3）===
  getBreakthroughHistory: (worldId: string, limit?: number) =>
    invokeGm<GameBreakthroughRecord[]>('getBreakthroughHistory', { world_id: worldId, limit }),
  listWorlds: () => invokeGm<WorldSummary[]>('listWorlds'),
  deleteWorld: (worldId: string) => invokeGm<void>('deleteWorld', { world_id: worldId }),
  // === 统计聚合便利（2）===
  getEventsAndTasks: (worldId: string) => invokeGm<EventsAndTasks>('getEventsAndTasks', { world_id: worldId }),
  getBuildTimeline: (worldId: string, fromTime?: number, toTime?: number) =>
    invokeGm<TimelineEvent[]>('getBuildTimeline', { world_id: worldId, from_time: fromTime, to_time: toTime }),
  // === 时间轴回放（2）===
  getWorldSnapshot: (worldId: string, timestamp: number) =>
    invokeGm<RebuiltScene>('getWorldSnapshot', { world_id: worldId, timestamp }),
  exitReplay: (worldId: string) => invokeGm<void>('exitReplay', { world_id: worldId }),
  // === NPC 系统（5）===
  npcList: () => invokeGm<GameNpc[]>('npcList'),
  npcGet: (npcId: string) => invokeGm<GameNpc | null>('npcGet', { npc_id: npcId }),
  npcHistory: (worldId: string, npcId: string, limit?: number) =>
    invokeGm<GameNpcConversation[]>('npcHistory', { world_id: worldId, npc_id: npcId, limit }),
  npcChat: (worldId: string, npcId: string, message: string, modelId?: number) =>
    invokeGm<NpcChatResponse>('npcChat', { world_id: worldId, npc_id: npcId, message, model_id: modelId }),
  npcClearHistory: (worldId: string, npcId: string) =>
    invokeGm<void>('npcClearHistory', { world_id: worldId, npc_id: npcId }),
  // === D4.6 NPC 深化（2）===
  npcMemories: (worldId: string, npcId: string, limit?: number) =>
    invokeGm<GameNpcMemory[]>('npcMemories', { world_id: worldId, npc_id: npcId, limit }),
  npcRelationship: (worldId: string, npcId: string) =>
    invokeGm<GameNpcRelationship>('npcRelationship', { world_id: worldId, npc_id: npcId }),
  // === D4.7 传闻机制（1）===
  npcRumors: (worldId: string, npcId: string, limit?: number) =>
    invokeGm<GameNpcRumor[]>('npcRumors', { world_id: worldId, npc_id: npcId, limit }),
  // === D4.3 自适应难度（1）===
  playerSkill: (worldId: string) => invokeGm<PlayerSkill>('playerSkill', { world_id: worldId }),
  // === D4.3 游戏对手 AI（1）===
  opponentDecide: (request: any) =>
    invokeGm<any>('opponentDecide', { request }),
  // === D4.4 动态剧情（4）===
  storyGenerate: (request: GenerateStoryRequest) => invokeGm<Story>('storyGenerate', {
    world_id: request.world_id,
    player_name: request.player_name,
    realm: request.realm,
    model_id: request.model_id,
    theme: request.theme ?? null,
  }),
  storyAdvance: (request: AdvanceStoryRequest) => invokeGm<Story>('storyAdvance', {
    story_id: request.story_id,
    choice_id: request.choice_id,
    model_id: request.model_id,
  }),
  storyGet: (storyId: string) => invokeGm<Story | null>('storyGet', { story_id: storyId }),
  storyList: (limit?: number) => invokeGm<StorySummary[]>('storyList', { limit: limit ?? 50 }),
  // === D4.4 自然语言交互（1）===
  nlParse: (request: ParseCommandRequest) => invokeGm<ParsedCommand>('nlParse', {
    world_id: request.world_id,
    player_name: request.player_name,
    realm: request.realm,
    message: request.message,
    model_id: request.model_id,
  }),
  // === D4.6 行为分析 AI 洞察（1）===
  analyzeBehavior: (request: AnalyzeBehaviorRequest) => invokeGm<BehaviorAnalysis>('analyzeBehavior', {
    world_id: request.world_id,
    player_name: request.player_name,
    realm: request.realm,
    model_id: request.model_id,
  }),
  // === D4.6 数据底层智能监测接入（2）===
  intelligenceRecordEvent: (eventType: string, detail?: string) =>
    invokeGm<void>('intelligenceRecordEvent', { event_type: eventType, detail: detail ?? null }),
  intelligenceStatus: () => invokeGm<IntelligenceHookStatus>('intelligenceStatus', {}),
};
