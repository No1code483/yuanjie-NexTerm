// ipc/game.ts — game 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）
import { ipc } from './core';
import type { GameBuilding, GameBuildHistory, GameBreakthroughRecord, GameBuildingStatus, GameKbCategoryMapping, GameKnowledgeDomain, GameKnowledgeDomainId, GameKnowledgeProgress, GamePointsSourceType, PointsTrend, WorldState, WorldSummary, RealmInfo, StartBuildingRequest, BreakthroughSession, BreakthroughAnswer, BreakthroughOutcome, BreakthroughPreviewInfo, BuildingCatalog, SyncResult, EventsAndTasks, TimelineEvent, RebuiltScene, GameNpc, GameNpcConversation, NpcChatResponse, GameNpcMemory, GameNpcRelationship, GameNpcRumor, PlayerSkill, Story, StorySummary, GenerateStoryRequest, AdvanceStoryRequest, ParsedCommand, ParseCommandRequest, BehaviorAnalysis, AnalyzeBehaviorRequest, IntelligenceHookStatus } from '@/types/game';


/**
 * 游戏 3D 重构 - IPC 命名空间
 *
 * 对齐后端 `commands/game_commands.rs` 的 22 个 Tauri Command（Task 5.1）。
 * 命令名与后端函数名一一对应，方法名为去掉 `game_` 前缀的 camelCase 形式。
 *
 * 分组：
 * 1. 世界与境界（6）：initWorld / getWorldState / getRealmInfo /
 *    getBreakthroughPreview / startBreakthrough / submitBreakthrough
 * 2. 建筑系统（7）：getBuildings / getBuildingCatalog / startBuilding /
 *    upgradeBuilding / removeBuilding / moveBuilding / getBuildHistory
 * 3. 知识联动（4）：getKnowledgeDomains / getKnowledgeProgress /
 *    mapKbCategory / syncKnowledgeEvent（内部便利命令）
 * 4. 突破历史 + 世界列表（3）：getBreakthroughHistory / listWorlds / deleteWorld
 * 5. 统计聚合便利（2）：getEventsAndTasks / getBuildTimeline
 *
 * 旧版 game 命名空间（createWorld / placeBuilding / gameSave / gameCombatStart
 * 等）对应的后端实现已废弃，本任务中已完全替换。
 */
export const game = {
  // === 世界与境界（6）===

  /** 创建新世界（32×32 平原，凡人初期，白道基，12 领域进度初始化）。 */
  initWorld: (playerName?: string) => ipc.invoke<WorldState>('game_init_world', {
    player_name: playerName
  }),
  /** 获取世界完整状态（世界 + 全部建筑 + 12 领域进度）。 */
  getWorldState: (worldId: string) => ipc.invoke<WorldState | null>('game_get_world_state', {
    world_id: worldId
  }),
  /** 获取世界境界信息（境界信息卡数据源）。 */
  getRealmInfo: (worldId: string) => ipc.invoke<RealmInfo>('game_get_realm_info', {
    world_id: worldId
  }),
  /** 预览突破考验信息（不实际出题，仅返回境界/冷却等元数据）。 */
  getBreakthroughPreview: (worldId: string) => ipc.invoke<BreakthroughPreviewInfo>('game_get_breakthrough_preview', {
    world_id: worldId
  }),
  /** 开始突破考验（mock AI 出题，返回会话，需用户作答后调用 submitBreakthrough）。 */
  startBreakthrough: (worldId: string) => ipc.invoke<BreakthroughSession>('game_start_breakthrough', {
    world_id: worldId
  }),
  /** 提交突破考验（mock AI 批改 + 道基评定 + 结果处理）。 */
  submitBreakthrough: (session: BreakthroughSession, answers: BreakthroughAnswer[]) => ipc.invoke<BreakthroughOutcome>('game_submit_breakthrough', {
    session,
    answers
  }),
  // === 建筑系统（7）===

  /** 获取世界所有建筑（可按状态过滤）。 */
  getBuildings: (worldId: string, status?: GameBuildingStatus) => ipc.invoke<GameBuilding[]>('game_get_buildings', {
    world_id: worldId,
    status
  }),
  /** 获取建筑目录（8 大类 + 子类静态配置）。 */
  getBuildingCatalog: () => ipc.invoke<BuildingCatalog>('game_get_building_catalog'),
  /** 开始建造（预建造：扣积分 + 建建筑 + 记历史，事务保证原子性）。 */
  startBuilding: (request: StartBuildingRequest) => ipc.invoke<GameBuilding>('game_start_building', {
    request
  }),
  /** 升级建筑（扣积分 + 升级 + 记历史）。 */
  upgradeBuilding: (buildingId: string, upgradeCost: number) => ipc.invoke<GameBuilding>('game_upgrade_building', {
    building_id: buildingId,
    upgrade_cost: upgradeCost
  }),
  /** 拆除建筑（返还 50% 积分 + 删建筑 + 记历史），返回返还积分数。 */
  removeBuilding: (buildingId: string, refundRatio?: number) => ipc.invoke<number>('game_remove_building', {
    building_id: buildingId,
    refund_ratio: refundRatio
  }),
  /** 移动建筑（消耗 10% 建筑成本积分 + 更新坐标 + 记历史）。 */
  moveBuilding: (buildingId: string, newPosX: number, newPosY: number, newPosZ: number, moveCost: number, newRotationY?: number) => ipc.invoke<GameBuilding>('game_move_building', {
    building_id: buildingId,
    new_pos_x: newPosX,
    new_pos_y: newPosY,
    new_pos_z: newPosZ,
    new_rotation_y: newRotationY,
    move_cost: moveCost
  }),
  /** 获取建造历史（时间轴回放数据源）。 */
  getBuildHistory: (worldId: string, limit?: number) => ipc.invoke<GameBuildHistory[]>('game_get_build_history', {
    world_id: worldId,
    limit
  }),
  // === 知识联动（4）===

  /** 获取全部 12 知识领域。 */
  getKnowledgeDomains: () => ipc.invoke<GameKnowledgeDomain[]>('game_get_knowledge_domains'),
  /** 获取世界在某领域的进度（domainId 省略时返回全部 12 领域）。 */
  getKnowledgeProgress: (worldId: string, domainId?: GameKnowledgeDomainId) => ipc.invoke<GameKnowledgeProgress[]>('game_get_knowledge_progress', {
    world_id: worldId,
    domain_id: domainId
  }),
  /** 设置知识库分类到知识领域的映射（已存在则 UPSERT 覆盖）。 */
  mapKbCategory: (categoryId: number, domainId: GameKnowledgeDomainId) => ipc.invoke<void>('game_map_kb_category', {
    category_id: categoryId,
    domain_id: domainId
  }),
  /** 获取所有已存在的 KB 分类映射（设置页左栏列表用，前端与 kb_categories LEFT JOIN）。 */
  getKbCategoryMappings: () => ipc.invoke<GameKbCategoryMapping[]>('game_get_kb_category_mappings'),
  /**
   * 获取近 N 天每领域每日积分趋势（知识领域积分卡 sparkline 数据源）。
   *
   * `days` 默认 7，上限 30。返回 `PointsTrend`：日期标签 + 每领域每日增量 + 每日总积分。
   * 仅含 `result='accepted'` 的入账积分，日期对齐 Asia/Shanghai 时区。
   */
  getPointsTrend: (worldId: string, days?: number) => ipc.invoke<PointsTrend>('game_get_points_trend', {
    world_id: worldId,
    days
  }),
  /**
   * 内部便利命令：接收其它模块（知识库/待办/计时器/AI/YuanCode/日志）的积分事件。
   *
   * 注：此命令主要供后端内部调用（如 todo_complete / chat_send_message 完成后）。
   * `sourceData` 必须包含 `event_id` 字段作为幂等键。
   */
  syncKnowledgeEvent: (worldId: string, eventType: GamePointsSourceType, domainId: GameKnowledgeDomainId, points: number, sourceData: Record<string, unknown>) => ipc.invoke<SyncResult>('game_sync_knowledge_event', {
    world_id: worldId,
    event_type: eventType,
    domain_id: domainId,
    points,
    source_data: sourceData
  }),
  // === 突破历史 + 世界列表（3）===

  /** 获取突破历史（突破历程时间线数据源）。 */
  getBreakthroughHistory: (worldId: string, limit?: number) => ipc.invoke<GameBreakthroughRecord[]>('game_get_breakthrough_history', {
    world_id: worldId,
    limit
  }),
  /** 获取所有世界列表（含聚合摘要，LEFT JOIN 避免 N+1）。 */
  listWorlds: () => ipc.invoke<WorldSummary[]>('game_list_worlds'),
  /** 删除世界（级联删除建筑/进度/历史等）。 */
  deleteWorld: (worldId: string) => ipc.invoke<void>('game_delete_world', {
    world_id: worldId
  }),
  // === 统计聚合便利（2）===

  /** 获取首页所需的待办事件与提示信息（4 卡片之「事件与任务卡」数据源）。 */
  getEventsAndTasks: (worldId: string) => ipc.invoke<EventsAndTasks>('game_get_events_and_tasks', {
    world_id: worldId
  }),
  /** 获取建造事件时间线（按时间倒序返回，用于时间线视图回放，默认 7 天窗口）。 */
  getBuildTimeline: (worldId: string, fromTime?: number, toTime?: number) => ipc.invoke<TimelineEvent[]>('game_get_build_timeline', {
    world_id: worldId,
    from_time: fromTime,
    to_time: toTime
  }),
  // === 时间轴回放（T1，11_时间轴回放.md §3）===

  /**
   * 重建指定时间点的世界场景（用于时间轴回放）。
   *
   * 调用时机：用户拖动时间轴滑块松开时调用。
   * 返回：该时刻的世界状态 + 建筑列表 + 统计 + 元数据。
   */
  getWorldSnapshot: (worldId: string, timestamp: number) => ipc.invoke<RebuiltScene>('game_get_world_snapshot', {
    world_id: worldId,
    timestamp
  }),
  /**
   * 退出回放模式，清空后端 LRU 缓存。
   *
   * 调用时机：用户点击「返回游戏」按钮时调用。
   */
  exitReplay: (worldId: string) => ipc.invoke<void>('game_exit_replay', {
    world_id: worldId
  }),

  // ===== D4.2 智能 NPC 系统 =====
  /** 列出所有 NPC（首次调用自动 seed 内置 5 个 NPC） */
  npcList: () => ipc.invoke<GameNpc[]>('game_npc_list'),
  /** 获取单个 NPC 详情 */
  npcGet: (npcId: string) => ipc.invoke<GameNpc | null>('game_npc_get', {
    npc_id: npcId
  }),
  /** 加载与某 NPC 的对话历史 */
  npcHistory: (worldId: string, npcId: string, limit?: number) => ipc.invoke<GameNpcConversation[]>('game_npc_history', {
    world_id: worldId,
    npc_id: npcId,
    limit
  }),
  /** 与 NPC 对话（调用云端 API 多轮对话） */
  npcChat: (worldId: string, npcId: string, message: string, modelId?: number) => ipc.invoke<NpcChatResponse>('game_npc_chat', {
    world_id: worldId,
    npc_id: npcId,
    message,
    model_id: modelId
  }),
  /** 清空与某 NPC 的对话历史 */
  npcClearHistory: (worldId: string, npcId: string) => ipc.invoke<void>('game_npc_clear_history', {
    world_id: worldId,
    npc_id: npcId
  }),
  /** D4.6 加载 NPC 对玩家的长期记忆（按重要度倒序） */
  npcMemories: (worldId: string, npcId: string, limit?: number) => ipc.invoke<GameNpcMemory[]>('game_npc_memories', {
    world_id: worldId,
    npc_id: npcId,
    limit
  }),
  /** D4.6 获取 NPC 对玩家的当前关系状态（首次访问自动创建中立关系） */
  npcRelationship: (worldId: string, npcId: string) => ipc.invoke<GameNpcRelationship>('game_npc_relationship', {
    world_id: worldId,
    npc_id: npcId
  }),
  /** D4.7 加载传播给当前 NPC 的传闻（从其他 NPC 听说的关于玩家的事，按重要度倒序） */
  npcRumors: (worldId: string, npcId: string, limit?: number) => ipc.invoke<GameNpcRumor[]>('game_npc_rumors', {
    world_id: worldId,
    npc_id: npcId,
    limit
  }),
  /** D4.3 获取玩家能力评估（突破考验历史表现 + 难度乘数） */
  playerSkill: (worldId: string) => ipc.invoke<PlayerSkill>('game_player_skill', {
    world_id: worldId
  }),

  // ===== D4.4 动态剧情（云端 API 生成 + DB 持久化） =====

  /** 生成动态剧情（调用云端 API，AI 失败降级到预设模板） */
  storyGenerate: (request: GenerateStoryRequest) => ipc.invoke<Story>('game_story_generate', {
    world_id: request.world_id,
    player_name: request.player_name,
    realm: request.realm,
    model_id: request.model_id,
    theme: request.theme ?? null
  }),
  /** 推进剧情到下一节点（根据玩家选择） */
  storyAdvance: (request: AdvanceStoryRequest) => ipc.invoke<Story>('game_story_advance', {
    story_id: request.story_id,
    choice_id: request.choice_id,
    model_id: request.model_id
  }),
  /** 获取当前剧情状态（从 DB 读取完整剧情 + 所有节点） */
  storyGet: (storyId: string) => ipc.invoke<Story | null>('game_story_get', {
    story_id: storyId
  }),
  /** 列出玩家历史剧情摘要（按最后推进时间倒序，跨会话恢复） */
  storyList: (limit?: number) => ipc.invoke<StorySummary[]>('game_story_list', {
    limit: limit ?? 50
  }),

  // ===== D4.4 自然语言交互（云端 API 解析玩家命令 + 规则降级） =====

  /** 解析玩家自然语言命令为结构化动作（调用云端 API，AI 失败降级到规则解析） */
  nlParse: (request: ParseCommandRequest) => ipc.invoke<ParsedCommand>('game_nl_parse', {
    world_id: request.world_id,
    player_name: request.player_name,
    realm: request.realm,
    message: request.message,
    model_id: request.model_id
  }),

  // ===== D4.6 数据分析 AI 洞察（云端 API 分析玩家行为 + 规则降级） =====

  /** 分析玩家行为，输出 AI 洞察（调用云端 API，AI 失败降级到规则分析） */
  analyzeBehavior: (request: AnalyzeBehaviorRequest) => ipc.invoke<BehaviorAnalysis>('game_analyze_behavior', {
    world_id: request.world_id,
    player_name: request.player_name,
    realm: request.realm,
    model_id: request.model_id
  }),

  // ===== D4.6 游戏数据底层智能监测接入（非侵入式、可关闭） =====

  /** 非侵入式记录游戏事件供底层智能监测（底层智能关闭时 no-op） */
  intelligenceRecordEvent: (eventType: string, detail?: string) => ipc.invoke<void>('game_intelligence_record_event', {
    event_type: eventType,
    detail: detail ?? null
  }),
  /** 查询底层智能监测钩子状态（启用与否 + 近期游戏事件数） */
  intelligenceStatus: () => ipc.invoke<IntelligenceHookStatus>('game_intelligence_status', {})
};
