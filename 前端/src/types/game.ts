/**
 * 游戏 3D 重构 - TypeScript 类型定义
 *
 * 对齐后端 `models/game.rs` + `services/game_service.rs` +
 * `services/game_breakthrough_service.rs` + `db/repositories/game_repo.rs` +
 * `commands/game_commands.rs` 的全部数据结构。
 *
 * change-id: `game-3d-rebuild-refactor`
 * 对应 tasks.md 阶段5 Task 5.2
 *
 * 设计原则：
 * - 字段名与后端 Rust 结构体保持 snake_case 一致（Tauri invoke 直接透传）
 * - `Option<T>` → `T | null`（JSON 反序列化后是 null）
 * - 时间戳统一为毫秒级 Unix（number）
 * - 枚举使用字符串字面量联合类型 + 运行时常量数组（便于前端校验）
 */

// ============================================================================
// 一、枚举字面量联合类型（对齐 models/game.rs 的 9 个枚举）
// ============================================================================

/** 大境界（10 阶）。对应 `game_worlds.realm_major` 列。 */
export type GameRealmMajor =
  | 'mortal'               // 凡人
  | 'qi_refining'           // 练气
  | 'foundation_building'   // 筑基
  | 'golden_core'           // 金丹
  | 'nascent_soul'          // 元婴
  | 'spirit_transformation' // 化神
  | 'unity'                 // 合体
  | 'mahayana'              // 大乘
  | 'tribulation'           // 渡劫
  | 'immortal'              // 仙

/** 小境界阶位（3 级）。对应 `game_worlds.realm_minor` 列。 */
export type GameRealmMinor = 'early' | 'middle' | 'complete'

/** 道基品质（5 级）。对应 `game_worlds.dao_foundation` 列。 */
export type GameDaoFoundation = 'white' | 'blue' | 'red' | 'purple' | 'black'

/** 建筑大类（8 类，对应文明演进）。对应 `game_buildings.building_category` 列。 */
export type GameBuildingCategory =
  | 'house'
  | 'town'
  | 'city'
  | 'kingdom'
  | 'palace'
  | 'technology'
  | 'sect'
  | 'immortal'

/** 建造状态。对应 `game_buildings.status` 列。 */
export type GameBuildingStatus = 'planning' | 'building' | 'completed' | 'ruined'

/** 建造历史事件类型。对应 `game_build_history.event_type` 列。 */
export type GameBuildHistoryEventType =
  | 'build'
  | 'upgrade'
  | 'demolish'
  | 'move'
  | 'complete'

/** 境界突破结果。对应 `game_breakthrough_records.result` 列。 */
export type GameBreakthroughResult = 'success' | 'failed' | 'dropped'

/** 积分入账结果。对应 `game_points_log.result` 列。 */
export type GamePointsLogResult =
  | 'accepted'
  | 'rejected_daily_limit'
  | 'rejected_negative'

/** KB 分类→游戏领域映射来源。对应 `game_kb_category_mapping.mapping_source` 列。 */
export type GameMappingSource = 'manual' | 'ai_suggest' | 'ai_auto'

/** 12 个知识领域 ID（03 文档 §1）。 */
export type GameKnowledgeDomainId =
  | 'cs'
  | 'math'
  | 'physics'
  | 'literature'
  | 'history'
  | 'art'
  | 'engineering'
  | 'medicine'
  | 'philosophy'
  | 'economics'
  | 'language'
  | 'other'

/** 11 种积分来源类型（03 文档 §2）。 */
export type GamePointsSourceType =
  | 'kb_entry_create'
  | 'kb_category_create'
  | 'kb_entry_update'
  | 'todo_complete'
  | 'timer_short_complete'
  | 'timer_long_complete'
  | 'ai_chat_turn'
  | 'yuancode_use'
  | 'journal_record'
  | 'building_refund'
  | 'manual_adjust'

/** 突破题型（02 文档 §2.2）。D4.5 新增 `image_choice` 多模态图片选择题。 */
export type GameBreakthroughQuestionType =
  | 'choice'
  | 'short_answer'
  | 'application'
  | 'image_choice'

/** 弱点严重度。 */
export type GameWeaknessSeverity = 'low' | 'medium' | 'high'

// ============================================================================
// 二、运行时常量数组（用于前端表单校验，对齐后端 `domain::ALL` / `source_type::ALL`）
// ============================================================================

/** 全部 12 个知识领域 ID（顺序与种子数据一致）。 */
export const GAME_DOMAIN_IDS: readonly GameKnowledgeDomainId[] = [
  'cs', 'math', 'physics', 'literature', 'history', 'art',
  'engineering', 'medicine', 'philosophy', 'economics', 'language', 'other',
] as const

/** 全部 11 个积分来源类型。 */
export const GAME_SOURCE_TYPES: readonly GamePointsSourceType[] = [
  'kb_entry_create', 'kb_category_create', 'kb_entry_update',
  'todo_complete', 'timer_short_complete', 'timer_long_complete',
  'ai_chat_turn', 'yuancode_use', 'journal_record',
  'building_refund', 'manual_adjust',
] as const

/** 全部 10 个大境界（按升阶顺序）。 */
export const GAME_REALM_MAJORS: readonly GameRealmMajor[] = [
  'mortal', 'qi_refining', 'foundation_building', 'golden_core', 'nascent_soul',
  'spirit_transformation', 'unity', 'mahayana', 'tribulation', 'immortal',
] as const

/** 全部 5 个道基（按升序）。 */
export const GAME_DAO_FOUNDATIONS: readonly GameDaoFoundation[] = [
  'white', 'blue', 'red', 'purple', 'black',
] as const

/** 全部 8 个建筑大类（按文明等级递增）。 */
export const GAME_BUILDING_CATEGORIES: readonly GameBuildingCategory[] = [
  'house', 'town', 'city', 'kingdom', 'palace', 'technology', 'sect', 'immortal',
] as const

/** 道基系数表（02 文档 §2.2）：白 0.5 / 蓝 0.7 / 红 1.0 / 紫 1.3 / 黑 1.6。 */
export const GAME_DAO_MULTIPLIER: Record<GameDaoFoundation, number> = {
  white: 0.5,
  blue: 0.7,
  red: 1.0,
  purple: 1.3,
  black: 1.6,
}

/** 知识领域等级阶梯阈值（03 文档 §3，累计积分 → 等级 1~8）。 */
export const GAME_DOMAIN_LEVEL_THRESHOLDS: readonly number[] = [
  0, 100, 500, 2_000, 10_000, 50_000, 200_000, 1_000_000,
] as const

/** 各大境界 XP 边界（02 文档 §2.1，下界含/上界不含）。 */
export const GAME_REALM_XP_BOUNDS: ReadonlyArray<{
  realm: GameRealmMajor
  lower: number
  upper: number
  breakthroughQuestions: number
}> = [
  { realm: 'mortal', lower: 0, upper: 100, breakthroughQuestions: 0 },
  { realm: 'qi_refining', lower: 100, upper: 500, breakthroughQuestions: 3 },
  { realm: 'foundation_building', lower: 500, upper: 2_000, breakthroughQuestions: 4 },
  { realm: 'golden_core', lower: 2_000, upper: 8_000, breakthroughQuestions: 5 },
  { realm: 'nascent_soul', lower: 8_000, upper: 30_000, breakthroughQuestions: 6 },
  { realm: 'spirit_transformation', lower: 30_000, upper: 100_000, breakthroughQuestions: 8 },
  { realm: 'unity', lower: 100_000, upper: 300_000, breakthroughQuestions: 10 },
  { realm: 'mahayana', lower: 300_000, upper: 800_000, breakthroughQuestions: 12 },
  { realm: 'tribulation', lower: 800_000, upper: 2_000_000, breakthroughQuestions: 15 },
  { realm: 'immortal', lower: 2_000_000, upper: Number.MAX_SAFE_INTEGER, breakthroughQuestions: 0 },
] as const

// ============================================================================
// 三、DB 实体类型（对齐 models/game.rs 的 10 个结构体，FromRow 自动映射）
// ============================================================================

/** 对应 `game_worlds` 表。世界/文明主表。 */
export interface GameWorld {
  id: string
  player_name: string
  civilization_level: number
  realm_major: GameRealmMajor
  realm_minor: GameRealmMinor
  dao_foundation: GameDaoFoundation
  total_xp: number
  realm_xp: number
  map_width: number
  map_height: number
  created_at: number
  updated_at: number
}

/** 对应 `game_buildings` 表。建筑实例。 */
export interface GameBuilding {
  id: string
  world_id: string
  building_category: GameBuildingCategory
  building_subtype: string
  name: string
  level: number
  pos_x: number
  pos_y: number
  pos_z: number
  rotation_y: number
  status: GameBuildingStatus
  build_progress: number
  knowledge_domain: GameKnowledgeDomainId
  built_at: number | null
  completed_at: number | null
}

/** 对应 `game_knowledge_domains` 表。12 个知识领域静态字典。 */
export interface GameKnowledgeDomain {
  id: GameKnowledgeDomainId
  name: string
  name_en: string
  description: string
  building_category: GameBuildingCategory
  sort_order: number
}

/** 对应 `game_knowledge_progress` 表。世界在各领域的积分进度。 */
export interface GameKnowledgeProgress {
  id: string
  world_id: string
  domain_id: GameKnowledgeDomainId
  points: number
  /** 领域等级 1~8。 */
  level: number
  total_earned: number
  total_consumed: number
  created_at: number
  updated_at: number
}

/** 对应 `game_breakthrough_records` 表。境界突破流水。 */
export interface GameBreakthroughRecord {
  id: string
  world_id: string
  /** 突破前境界，格式 `{realm_major}:{realm_minor}`。 */
  from_realm: string
  /** 突破后境界，失败时为 `null`。 */
  to_realm: string | null
  /** AI 考验得分 0~100；未答题时为 `null`。 */
  score: number | null
  /** 评定道基；失败时为 `null`。 */
  dao_foundation_awarded: GameDaoFoundation | null
  result: GameBreakthroughResult
  /** 考题快照（JSON 字符串）。 */
  questions_json: string | null
  /** 用户答案快照（JSON 字符串）。 */
  answers_json: string | null
  /** AI 批改评语。 */
  ai_review: string | null
  /** 暴露的弱点列表（JSON 字符串）。 */
  weakness_json: string | null
  created_at: number
}

/** 对应 `game_build_history` 表。建造/升级/拆除/移动/完成事件流水。 */
export interface GameBuildHistory {
  id: string
  world_id: string
  event_type: GameBuildHistoryEventType
  /** 关联建筑 ID（建筑删除后仍保留历史）。 */
  building_id: string
  /** 建筑名称快照。 */
  building_name: string
  /** 事件发生时建筑 3D 坐标快照。 */
  pos_x: number | null
  pos_y: number | null
  pos_z: number | null
  /** 事件发生时建筑等级快照。 */
  level: number | null
  /** 事件发生时建造进度快照。 */
  progress: number | null
  /** 该时间点世界状态快照（JSON 字符串，用于时间轴回放）。 */
  snapshot_json: string | null
  created_at: number
}

/** 对应 `game_kb_category_mapping` 表。KB 分类 → 游戏领域映射（一对一）。 */
export interface GameKbCategoryMapping {
  /** KB 分类 ID，主键。 */
  category_id: number
  domain_id: GameKnowledgeDomainId
  mapped_at: number
  mapped_by: GameMappingSource
  /** AI 映配置信度 0.0~1.0；`manual` 时为 `null`。 */
  confidence: number | null
}

/** 对应 `game_points_log` 表。积分变更日志（幂等 event_id）。 */
export interface GamePointsLog {
  id: string
  world_id: string
  source_type: GamePointsSourceType
  domain_id: GameKnowledgeDomainId
  /** 积分变动值，正数入账，负数扣除。 */
  points_delta: number
  /** 变动后该领域可用积分快照。 */
  points_after: number
  result: GamePointsLogResult
  /** 业务事件唯一 ID，UNIQUE 约束保证幂等。 */
  event_id: string
  /** 附加元数据（JSON 字符串）。 */
  metadata_json: string | null
  created_at: number
}

/** 对应 `game_daily_limit_counter` 表。每日上限计数器。 */
export interface GameDailyLimitCounter {
  id: string
  world_id: string
  /** 计数日期，格式 `'YYYY-MM-DD'`（Asia/Shanghai 时区）。 */
  counter_date: string
  domain_id: GameKnowledgeDomainId
  source_type: GamePointsSourceType
  /** 当日已入账次数。 */
  current_count: number
  updated_at: number
}

/** 对应 `game_points_source_config` 表。11 种积分来源配置。 */
export interface GamePointsSourceConfig {
  source_type: GamePointsSourceType
  /** 默认入账领域 id；`null` 表示由调用方指定。 */
  default_domain_id: GameKnowledgeDomainId | null
  /** 单次事件积分数。 */
  points_per_event: number
  /** 每日上限次数；`null` 表示无上限。 */
  daily_limit: number | null
  description: string | null
  enabled: boolean
  updated_at: number
}

// ============================================================================
// 四、Service 层 DTO（对齐 game_service.rs / game_breakthrough_service.rs）
// ============================================================================

/** 境界信息聚合 DTO（前端境界信息卡展示用）。 */
export interface RealmInfo {
  world_id: string
  player_name: string
  realm_major: GameRealmMajor
  realm_minor: GameRealmMinor
  dao_foundation: GameDaoFoundation
  /** 道基系数（0.5 / 0.7 / 1.0 / 1.3 / 1.6）。 */
  dao_multiplier: number
  /** 累计总修为（永不减）。 */
  total_xp: number
  /** 当前大境界内修为。 */
  realm_xp: number
  /** 当前大境界 XP 下限（含）。 */
  realm_xp_lower: number
  /** 当前大境界 XP 上限（不含）。 */
  realm_xp_upper: number
  /** 当前大境界序号（1-10）。 */
  realm_ordinal: number
  /** 下一阶大境界；仙境界为 `null`。 */
  next_realm_major: GameRealmMajor | null
  /** 突破考验题目数量（凡人/仙 = 0，其他 3-15）。 */
  breakthrough_question_count: number
  civilization_level: number
}

/** 建造请求参数（game_start_building 输入）。 */
export interface StartBuildingRequest {
  world_id: string
  building_category: GameBuildingCategory
  building_subtype: string
  name: string
  pos_x: number
  pos_y: number
  pos_z: number
  rotation_y: number
  /** 关联的知识领域 id（决定扣哪个领域的积分）。 */
  knowledge_domain: GameKnowledgeDomainId
  /** 建造所需积分（由建筑子类目录决定，调用方传入）。 */
  base_cost: number
}

/** 突破考验单题。D4.5 新增 `media_*` 多模态字段（image_choice 题型必填）。 */
export interface BreakthroughQuestion {
  question_id: string
  question_type: GameBreakthroughQuestionType
  domain_id: GameKnowledgeDomainId
  knowledge_point: string
  content: string
  /** 选择题 4 选项；其他题型为 `null`。 */
  options: string[] | null
  /** 标准答案。 */
  standard_answer: string
  /** 难度系数（0.5-1.5）。 */
  difficulty: number
  /** D4.5 多模态：媒体 URL（图片/音频）。`null` 表示纯文本题。 */
  media_url?: string | null
  /** D4.5 多模态：媒体类型 `"image"` / `"audio"` / `null`。 */
  media_type?: string | null
  /** D4.5 多模态：媒体内容描述（图片画面描述 / 音频内容说明）。 */
  media_description?: string | null
}

/** 用户答案。 */
export interface BreakthroughAnswer {
  question_id: string
  user_answer: string
}

/** 弱点项（02 文档 §6.5.2）。 */
export interface WeaknessItem {
  domain_id: GameKnowledgeDomainId
  knowledge_point: string
  error_count: number
  severity: GameWeaknessSeverity
  last_exposed_at: number
  resolved: boolean
  resolved_at: number | null
}

/** 单题评分（4 维度加权）。 */
export interface QuestionScore {
  question_id: string
  score: number
  correctness: number
  completeness: number
  expression: number
  depth: number
  comment: string
  weak_points: string[]
}

/** 批改结果。 */
export interface GradingResult {
  total_score: number
  daoji_level: GameDaoFoundation
  question_scores: QuestionScore[]
  overall_comment: string
  weakness_analysis: WeaknessItem[]
}

/**
 * 突破考验会话（game_start_breakthrough 返回，game_submit_breakthrough 输入）。
 *
 * 不入库，由前端 zustand store 暂存。题目已固定，用户作答后用同一 session 提交。
 */
export interface BreakthroughSession {
  session_id: string
  world_id: string
  from_realm_major: GameRealmMajor
  from_realm_minor: GameRealmMinor
  target_realm_major: GameRealmMajor
  question_count: number
  difficulty_coefficient: number
  questions: BreakthroughQuestion[]
  created_at: number
}

/** 突破最终结果（game_submit_breakthrough 返回）。 */
export interface BreakthroughOutcome {
  result: GameBreakthroughResult
  score: number
  daoji_awarded: GameDaoFoundation
  new_realm_major: GameRealmMajor
  new_realm_minor: GameRealmMinor
  new_dao_foundation: GameDaoFoundation
  weakness_updated: boolean
  /** 失败/跌落时为 24h 后的时间戳；成功时为 `null`。 */
  cooldown_until: number | null
  ai_review: string
  /**
   * D4.6 深化#11：突破失败后的个性化复习建议（仅 result=failed/dropped 时可能有值）。
   * AI 调用失败或无弱点时为 `null`。前端在结果页展示「天道建议」卡片。
   */
  review_suggestions?: ReviewSuggestion[] | null
}

/**
 * D4.6 深化#11：个性化复习建议项。
 * 对应后端 `game_breakthrough_deepening::ReviewSuggestion`。
 */
export interface ReviewSuggestion {
  domain_id: string
  knowledge_point: string
  reason: string
  suggested_action: string
  /** 关联的 KB 条目 ID（可选）。 */
  related_entry_id: string | null
}

// ============================================================================
// 五、Repository 聚合 DTO（对齐 game_repo.rs）
// ============================================================================

/** 世界完整状态（世界主表 + 全部建筑 + 12 领域进度）。 */
export interface WorldState {
  world: GameWorld
  buildings: GameBuilding[]
  progress: GameKnowledgeProgress[]
}

/** 世界列表摘要（含聚合统计，避免 N+1）。 */
export interface WorldSummary {
  id: string
  player_name: string
  civilization_level: number
  realm_major: GameRealmMajor
  realm_minor: GameRealmMinor
  dao_foundation: GameDaoFoundation
  total_xp: number
  realm_xp: number
  /** 建筑总数（含所有状态）。 */
  building_count: number
  /** 已建成建筑数（status='completed'）。 */
  completed_building_count: number
  /** 12 领域累计积分总和。 */
  total_points: number
  updated_at: number
  created_at: number
}

// ============================================================================
// 六、IPC 专用 DTO（对齐 commands/game_commands.rs 的 11 个 DTO）
// ============================================================================

/** 突破预览信息（前端境界突破按钮状态判断）。 */
export interface BreakthroughPreviewInfo {
  realm_major: GameRealmMajor
  realm_minor: GameRealmMinor
  dao_foundation: GameDaoFoundation
  /** 是否可突破（境界/修为/道基/冷却全部通过）。 */
  can_breakthrough: boolean
  /** 冷却截止时间戳（ms）；无冷却时为 `null`。 */
  cooldown_until: number | null
}

/** 建筑子类信息（简化版，完整属性由前端资源层维护）。 */
export interface BuildingSubtypeInfo {
  subtype: string
  name: string
  /** level=1 时所需领域积分（由后端 `building_required_points` 计算）。 */
  base_cost: number
}

/** 建筑大类信息。 */
export interface BuildingCategoryInfo {
  category: GameBuildingCategory
  name: string
  civilization_level_required: number
  subtypes: BuildingSubtypeInfo[]
}

/** 建筑目录（8 大类 + 子类）。 */
export interface BuildingCatalog {
  categories: BuildingCategoryInfo[]
}

/** 建筑进度增量（game_sync_knowledge_event 返回）。 */
export interface BuildProgressDelta {
  building_id: string
  progress_before: number
  progress_after: number
  delta: number
  completed: boolean
}

/** 积分事件同步结果（game_sync_knowledge_event 返回）。 */
export interface SyncResult {
  /** 实际入账积分（被拒绝则为 0）。 */
  points_added: number
  /** 被推进进度的建筑列表（当前实现返回空数组）。 */
  build_progress_updated: BuildProgressDelta[]
  /** 修为增量 = 积分 × 道基系数。 */
  realm_xp_added: number
}

/** 即将完工建筑（game_get_events_and_tasks 返回）。 */
export interface UpcomingBuilding {
  building_id: string
  name: string
  building_subtype: string
  progress: number
  remaining_points: number
  estimated_complete_at: number | null
}

/** 突破提示（game_get_events_and_tasks 返回）。 */
export interface BreakthroughHint {
  can_breakthrough: boolean
  /** 不可突破时给出原因。 */
  reason: string
  /** 冷却剩余毫秒数；无冷却时为 0。 */
  cooldown_remaining: number
}

/** 近期事件（game_get_events_and_tasks 返回）。 */
export interface RecentEvent {
  event_id: string
  event_type: string
  title: string
  description: string
  timestamp: number
}

/** 事件与任务卡数据（game_get_events_and_tasks 返回）。 */
export interface EventsAndTasks {
  upcoming_buildings: UpcomingBuilding[]
  breakthrough_hint: BreakthroughHint
  recent_events: RecentEvent[]
}

/**
 * 近 N 天积分趋势（game_get_points_trend 返回）。
 *
 * 用于知识领域积分卡的 sparkline 趋势图与日均/周累统计。
 * - `date_labels`：日期标签数组（`'YYYY-MM-DD'`，Asia/Shanghai 时区，长度 = `days`）
 * - `trend`：每领域每日入账积分增量（长度 = `days`，缺日补 0）
 * - `daily_totals`：全领域每日总积分（长度 = `days`）
 */
export interface PointsTrend {
  days: number
  date_labels: string[]
  /** `Record<domain_id, number[]>`，每领域每日积分增量。 */
  trend: Record<string, number[]>
  daily_totals: number[]
}

/** 时间线事件（game_get_build_timeline 返回）。 */
export interface TimelineEvent {
  id: string
  /**
   * 事件类型：
   * `build_start` / `build_complete` / `upgrade_start` / `upgrade_complete` /
   * `remove` / `move` / `breakthrough`
   */
  event_type: string
  building_id: string | null
  building_name: string | null
  level: number | null
  /** 各领域积分变化映射；无变化时为 `null`。 */
  points_delta: Record<string, number> | null
  description: string
  timestamp: number
}

// ============================================================================
// 七、时间轴回放类型（T1，对齐 11_时间轴回放.md §6.1 / models/game.rs §四-补）
// ============================================================================

/** 快照触发原因。对应后端 `SnapshotTriggerReason` 枚举。 */
export type SnapshotTriggerReason =
  | 'periodic_10'      // 周期性快照（每 10 个事件一次）
  | 'complete'         // 建造完成事件触发
  | 'upgrade'          // 升级事件触发
  | 'remove'           // 拆除事件触发
  | 'world_init'       // 世界初始化首条事件触发
  | 'civ_level_change' // 文明等级变化触发

/** 快照元数据。对应后端 `SnapshotMeta` 结构体。 */
export interface SnapshotMeta {
  /** 触发快照的事件 ID。 */
  event_id: string
  /** 该事件在当前世界事件流中的序号（从 1 开始）。 */
  event_seq: number
  world_id: string
  /** 快照生成时间戳（毫秒级 Unix）。 */
  captured_at: number
  /** 快照触发原因（字符串化枚举）。 */
  trigger_reason: SnapshotTriggerReason
}

/** 世界状态快照（不含建筑列表）。对应后端 `WorldStateSnapshot`。 */
export interface WorldStateSnapshot {
  civilization_level: number
  realm_major: GameRealmMajor
  realm_minor: GameRealmMinor
  dao_foundation: GameDaoFoundation
  total_xp: number
  realm_xp: number
}

/** 单建筑快照（用于 `WorldSnapshot.buildings` 列表项）。对应后端 `BuildingSnapshot`。 */
export interface BuildingSnapshot {
  id: string
  /** 建筑子类型 ID（如 thatch_cottage / wooden_house）。 */
  building_subtype: string
  name: string
  level: number
  pos_x: number
  pos_y: number
  pos_z: number
  rotation_y: number
  /** 建造状态：planning / building / completed / ruined。 */
  status: GameBuildingStatus
  build_progress: number
  knowledge_domain: GameKnowledgeDomainId
}

/** 快照统计信息。对应后端 `SnapshotStats`。 */
export interface SnapshotStats {
  building_count: number
  total_levels: number
}

/**
 * 完整世界状态快照（存储到 `game_build_history.snapshot_json`，zstd 压缩）。
 * 对应后端 `WorldSnapshot`。
 */
export interface WorldSnapshot {
  schema_version: number
  snapshot_meta: SnapshotMeta
  world_state: WorldStateSnapshot
  buildings: BuildingSnapshot[]
  stats: SnapshotStats
}

/**
 * 场景重建结果（`game_get_world_snapshot` 命令返回）。
 * 对应后端 `RebuiltScene`。
 */
export interface RebuiltScene {
  world_state: WorldStateSnapshot
  buildings: BuildingSnapshot[]
  stats: SnapshotStats
  /** 重建目标时间戳（毫秒级 Unix）。 */
  target_timestamp: number
  /** 基准快照的事件 ID；`null` 表示从空世界重建。 */
  source_snapshot_event_id: string | null
  /** 重放的增量事件数。 */
  replayed_event_count: number
}

/**
 * 建造历史分页查询结果（用于时间轴回放初始化加载）。
 * 后端目前未提供该 DTO（直接返回 `Vec<GameBuildHistory>`），此处为前端封装。
 */
export interface BuildHistoryPage {
  /** 正序排列的建造历史事件列表。 */
  events: GameBuildHistory[]
  /** 总事件数（用于显示「共 N 条」）。 */
  total: number
  /** 当前世界是否存在任何快照点（用于 UI 提示）。 */
  has_snapshot: boolean
}

// ============================================================================
// D4.2 智能 NPC 系统类型
// ============================================================================

/** NPC 定义（与后端 GameNpc 对应） */
export interface GameNpc {
  id: string;
  name: string;
  role: 'guide' | 'elder' | 'merchant' | 'scholar' | 'artisan' | 'rival';
  realm_level: string;
  personality: string;
  knowledge_domains: string;  // JSON 数组字符串
  greeting: string;
  system_prompt: string;
  avatar_emoji: string;
  location: string | null;
  unlock_realm: string;
  created_at: number;
  updated_at: number;
}

/** NPC 对话历史记录 */
export interface GameNpcConversation {
  id: string;
  world_id: string;
  npc_id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  turn_index: number;
  created_at: number;
}

/** NPC 对话响应（D4.6 扩展：携带关系值 + 记忆条数，供前端展示关系条 + 记忆提示） */
export interface NpcChatResponse {
  npc_id: string;
  reply: string;
  turn_index: number;
  /** D4.6 NPC 当前对玩家的关系值（-100~100，0 为中立） */
  relationship_value: number;
  /** D4.6 关系等级标签（hostile/cold/neutral/warm/close/sworn） */
  relationship_label: 'hostile' | 'cold' | 'neutral' | 'warm' | 'close' | 'sworn';
  /** D4.6 NPC 已记住玩家的记忆条数（前端"已记住 N 件事"提示） */
  memory_count: number;
  /** D4.6 本次互动关系变化增量（-5~+5，AI 评估） */
  relationship_delta: number;
  /** D4.6 本次互动新增的记忆条数（规则匹配 + AI 批量） */
  memory_added: number;
}

// ----------------------------------------------------------------------------
// D4.6 智能 NPC 深化：长期记忆 + 关系网类型
// ----------------------------------------------------------------------------

/** NPC 对玩家的长期记忆类型 */
export type GameNpcMemoryType = 'fact' | 'preference' | 'commitment' | 'event';

/** NPC 对玩家的长期记忆条目（game_npc_memories 表） */
export interface GameNpcMemory {
  id: string;
  world_id: string;
  npc_id: string;
  memory_type: GameNpcMemoryType;
  /** 自然语言描述，如"玩家叫张三" */
  content: string;
  /** 重要度 0.0-1.0（影响召回优先级） */
  importance: number;
  /** 来源：rule（规则匹配）/ ai（AI 提取） */
  source: 'rule' | 'ai';
  recall_count: number;
  last_recalled_at: number | null;
  created_at: number;
  updated_at: number;
}

/** NPC 关系等级标签（由 relationship_value 派生） */
export type GameNpcRelationshipLabel =
  | 'hostile'   // ≤ -50 仇恨
  | 'cold'      // -49 ~ -10 冷淡
  | 'neutral'   // -9 ~ +9 中立
  | 'warm'      // +10 ~ +39 友好
  | 'close'     // +40 ~ +79 亲密
  | 'sworn';    // ≥ +80 挚友

/** NPC↔玩家关系记录（game_npc_relationships 表） */
export interface GameNpcRelationship {
  id: string;
  world_id: string;
  npc_id: string;
  /** -100（仇恨）~ +100（挚友），0 为中立 */
  relationship_value: number;
  interaction_count: number;
  first_interaction_at: number;
  last_interaction_at: number;
  /** JSON 数组字符串，每项 {turn, delta, reason, ts}，最多 50 条 */
  relationship_history: string;
  relationship_label: GameNpcRelationshipLabel;
  created_at: number;
  updated_at: number;
}

/** 关系历史条目（relationship_history JSON 数组项，前端解析后使用） */
export interface RelationshipHistoryEntry {
  turn: number;
  delta: number;
  /** positive / negative / no_change */
  reason: 'positive' | 'negative' | 'no_change';
  ts: number;
}

// ----------------------------------------------------------------------------
// D4.7 跨 NPC 关系联动：传闻机制类型
// ----------------------------------------------------------------------------

/**
 * 跨 NPC 传闻（game_npc_rumors 表）。
 *
 * 当 NPC A 与玩家互动产生重要记忆（importance >= 0.5）时，该记忆会传播为
 * "传闻"到同世界其他 NPC，让其他 NPC 在对话中能"听说"关于玩家的事。
 *
 * 与 GameNpcMemory 区别：
 * - GameNpcMemory 存"亲历记忆"（NPC 自己与玩家互动产生的，source='rule'/'ai'）
 * - GameNpcRumor  存"传闻记忆"（从其他 NPC 传播来的，非亲历）
 */
export interface GameNpcRumor {
  id: string;
  world_id: string;
  /** 传闻来源 NPC（产生该记忆的亲历者） */
  source_npc_id: string;
  /** 传闻目标 NPC（听到该传闻的 NPC，即当前查询的 NPC） */
  target_npc_id: string;
  /** fact / preference / commitment / event（继承自源记忆） */
  memory_type: GameNpcMemoryType;
  /** 传闻内容（自然语言） */
  content: string;
  /** 重要度 0.0-1.0（继承自源记忆，影响召回优先级） */
  importance: number;
  /** 源记忆 ID（用于去重：同一记忆不重复传播给同一 NPC） */
  origin_memory_id: string;
  created_at: number;
}

// ----------------------------------------------------------------------------
// D4.3 自适应难度：玩家能力评估类型
// ----------------------------------------------------------------------------

/** 玩家技能评级标签（由 skill_score 派生） */
export type PlayerSkillLabel = 'novice' | 'beginner' | 'proficient' | 'expert' | 'master';

/** 玩家能力评估记录（game_player_skill 表，单玩家世界一行） */
export interface PlayerSkill {
  world_id: string;
  /** 综合能力评分 0.0-100.0 */
  skill_score: number;
  attempt_count: number;
  success_count: number;
  fail_count: number;
  total_score: number;
  avg_score: number;
  last_score: number;
  /** success / failed / dropped / ''（首次） */
  last_result: string;
  /** 连胜（正数）/ 连败（负数）/ 0 */
  streak: number;
  /** 难度乘数 0.6-1.5 */
  difficulty_multiplier: number;
  created_at: number;
  updated_at: number;
}

// ============================================================================
// D4.4 动态剧情（game_story_service）
// 字段对齐后端 game_story_service.rs（snake_case 透传）
// ============================================================================

/** 剧情选择项 */
export interface StoryChoice {
  id: string;
  text: string;
  hint: string | null;
}

/** 剧情节点（一个分支点） */
export interface StoryNode {
  id: string;
  title: string;
  description: string;
  narration: string;
  choices: StoryChoice[];
  is_ending: boolean;
}

/** 完整剧情（多节点分支树） */
export interface Story {
  id: string;
  world_id: string;
  theme: string;
  current_node_id: string;
  nodes: StoryNode[];
  /** 是否使用 AI 生成（false 表示降级到预设模板） */
  used_ai: boolean;
}

/** 剧情列表摘要（D4.4b 历史剧情时间线） */
export interface StorySummary {
  id: string;
  world_id: string;
  theme: string;
  current_node_id: string;
  used_ai: boolean;
  is_finished: boolean;
  node_count: number;
  created_at: string;
  updated_at: string;
}

/** 生成剧情请求 */
export interface GenerateStoryRequest {
  world_id: string;
  player_name: string;
  realm: string;
  model_id: number;
  /** user_id 由后端 require_auth 注入，前端不传 */
  user_id?: number;
  theme?: string | null;
}

/** 推进剧情请求 */
export interface AdvanceStoryRequest {
  story_id: string;
  choice_id: string;
  model_id: number;
  user_id?: number;
}

// ============================================================================
// D4.4 自然语言交互类型
// ============================================================================

/** 解析出的动作类型（前端据此分发到既有 game 命令执行） */
export type NlAction =
  | 'build'
  | 'upgrade'
  | 'remove'
  | 'breakthrough'
  | 'query_status'
  | 'chat_npc'
  | 'unknown';

/** 解析后的玩家命令（与后端 ParsedCommand 对应） */
export interface ParsedCommand {
  /** 识别出的动作类型 */
  action: NlAction;
  /** 动作目标（建筑名 / NPC 名 / 查询对象） */
  target: string;
  /** 附加参数（JSON 对象，如建筑子类型） */
  params: Record<string, unknown>;
  /** 置信度 0.0-1.0 */
  confidence: number;
  /** 给玩家的自然语言解释 */
  explanation: string;
  /** 是否走了 AI（false 表示降级到规则解析） */
  used_ai: boolean;
}

/** 自然语言解析请求 */
export interface ParseCommandRequest {
  world_id: string;
  player_name: string;
  realm: string;
  message: string;
  model_id: number;
  /** user_id 由后端 require_auth 注入，前端不传 */
  user_id?: number;
}

// ============================================================================
// D4.6 数据分析 AI 洞察类型
// ============================================================================

/** 洞察类型 */
export type InsightType =
  | 'learning_style'
  | 'weak_area'
  | 'breakthrough_strategy'
  | 'pace_advice'
  | 'overall';

/** 优先级 */
export type InsightSeverity = 'info' | 'warning' | 'critical';

/** 单条洞察（与后端 GameInsight 对应） */
export interface GameInsight {
  insight_type: InsightType;
  severity: InsightSeverity;
  title: string;
  content: string;
}

/** 行为快照摘要（脱敏给前端展示关键指标） */
export interface BehaviorSnapshotSummary {
  build_total: number;
  breakthrough_total: number;
  breakthrough_success: number;
  npc_chat_count: number;
  skill_score: number;
  streak: number;
}

/** 行为分析结果（与后端 BehaviorAnalysis 对应） */
export interface BehaviorAnalysis {
  insights: GameInsight[];
  snapshot: BehaviorSnapshotSummary;
  /** 是否走了 AI */
  used_ai: boolean;
}

/** 行为分析请求 */
export interface AnalyzeBehaviorRequest {
  world_id: string;
  player_name: string;
  realm: string;
  model_id: number;
  user_id?: number;
}

// ============================================================================
// D4.6 游戏数据底层智能监测接入类型
// ============================================================================

/** 底层智能监测钩子状态（与后端 IntelligenceHookStatus 对应） */
export interface IntelligenceHookStatus {
  /** 底层智能全局开关是否启用 */
  enabled: boolean;
  /** 近 7 天游戏事件数（module='game'） */
  recent_event_count: number;
}

