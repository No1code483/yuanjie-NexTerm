/**
 * gameStore - 3D 场景 Zustand 状态管理（Task 6.4）
 *
 * 参考 05_3D场景设计.md §12 状态管理设计：
 *   - world: 世界数据（含地块网格 + 文明等级）
 *   - buildings: 建筑列表（3D 渲染数据源）
 *   - selectedBuilding: 当前选中建筑 ID
 *   - cameraPreset: 相机预设视角
 *   - viewMode: 画质模式（low/medium/high）
 *   - isLowPerf: 性能降级标记
 *
 * 设计原则（Zustand 而非 React Context）：
 *   1. useFrame 每秒 60 次通过 getState() 读取最新状态，不触发 React 渲染
 *   2. selector 订阅特定字段，仅订阅字段变化才重渲染
 *   3. 无 Provider，Canvas 内外皆可访问
 *
 * change-id: game-3d-rebuild-refactor
 */
import { create } from 'zustand'
import type {
  BreakthroughOutcome,
  BreakthroughPreviewInfo,
  BreakthroughSession,
  BuildingCatalog,
  GameBuilding,
  GameWorld,
  RebuiltScene,
  TimelineEvent,
} from '@/types/game'

// ===== 类型定义 =====

/** 地块类型（参考 05 文档 §8.2） */
export type TerrainType = 'plain' | 'water' | 'hill' | 'forest' | 'desert'

/** 单地块数据 */
export interface Tile {
  x: number
  z: number
  type: TerrainType
  height: number
}

/** 3D 场景世界数据（扩展 GameWorld，增加地块网格） */
export interface Game3DWorld {
  id: string
  playerName: string
  civilizationLevel: number
  mapWidth: number
  mapHeight: number
  tiles: Tile[]
}

/** 相机预设视角 */
export type CameraPreset = 'top' | 'iso' | 'side'

/** 画质模式 */
export type ViewMode = 'low' | 'medium' | 'high'

/** 放置模式状态 */
export type PlacementMode = 'idle' | 'placing' | 'moving'

/** 待放置的建筑目录项（从 BuildPanel 选中后传入） */
export interface PlacementItem {
  category: GameBuilding['building_category']
  subtype: string
  name: string
  baseCost: number
  /** 关联知识领域（v1 按 category 默认映射，后续可由用户选择） */
  knowledgeDomain: GameBuilding['knowledge_domain']
}

/** 时间轴回放播放速度（倍数，1=实时，2/4/8=快进）。 */
export type ReplaySpeed = 1 | 2 | 4 | 8

/**
 * 突破结果动画状态（T2，12_AI考验机制.md §4.5 / 02_境界系统设计.md §6）。
 * - `idle`：无动画（默认 / 答题中）
 * - `success`：成功，播放金光粒子动画（升阶金光）
 * - `failure`：失败未跌落，播放雷劫粒子动画（轻微）
 * - `dropped`：跌落（连续失败），播放雷劫粒子动画（重击）
 */
export type BreakthroughAnimationState = 'idle' | 'success' | 'failure' | 'dropped'

/** 建筑渲染数据（扩展 GameBuilding，增加 3D 渲染用字段） */
export interface Building3D {
  id: string
  category: GameBuilding['building_category']
  subtype: string
  name: string
  level: number
  position: [number, number, number]
  rotationY: number
  status: GameBuilding['status']
  buildProgress: number
  knowledgeDomain: GameBuilding['knowledge_domain']
}

/** 3D 场景状态接口 */
interface Game3DStore {
  // ===== 场景数据 =====
  world: Game3DWorld | null
  buildings: Building3D[]
  selectedBuildingId: string | null

  // ===== 相机 =====
  cameraPreset: CameraPreset

  // ===== 视图 =====
  viewMode: ViewMode
  isLowPerf: boolean

  // ===== UI =====
  introPlayed: boolean
  isReady: boolean

  // ===== 放置模式（Task 9.3 预建造 / Task 9.5 移动） =====
  placementMode: PlacementMode
  placementItem: PlacementItem | null
  /** 当前预览位置（世界坐标，y=0 地面） */
  previewPosition: [number, number, number] | null
  /** 移动模式下被移动的建筑 ID */
  movingBuildingId: string | null
  /** 操作进行中（IPC 调用期间，禁用交互） */
  actionInProgress: boolean

  // ===== 建筑目录缓存（Task 9.5 升级/移动 cost 计算用） =====
  catalogCache: BuildingCatalog | null

  // ===== 时间轴回放（T1，11_时间轴回放.md §4）=====
  /** 是否处于回放模式（true 时场景由 replayCurrentScene 驱动而非实时数据） */
  replayMode: boolean
  /** 已加载的时间轴事件列表（按时间正序）。 */
  replayTimeline: TimelineEvent[]
  /** 当前回放时间戳（毫秒级 Unix）。 */
  replayCurrentTime: number
  /** 时间轴起点时间戳（replayTimeline[0].timestamp）。 */
  replayMinTime: number
  /** 时间轴终点时间戳（最近事件 timestamp）。 */
  replayMaxTime: number
  /** 是否正在播放（true 时按 replaySpeed 自动推进时间）。 */
  replayPlaying: boolean
  /** 播放速度（1/2/4/8 倍）。 */
  replaySpeed: ReplaySpeed
  /** 最近一次重建得到的场景（驱动 3D 渲染 + UI 面板显示）。 */
  replayCurrentScene: RebuiltScene | null
  /** 是否正在加载场景重建（拖动滑块时短暂为 true）。 */
  replayLoading: boolean

  // ===== 境界突破考验（T2，12_AI考验机制.md / 02_境界系统设计.md）=====
  /** 突破预览信息（境界/道基/是否可突破/冷却）。来自 `game.getBreakthroughPreview`。 */
  breakthroughPreview: BreakthroughPreviewInfo | null
  /** 当前突破会话（出题后由 `game.startBreakthrough` 返回，含固定题目列表）。 */
  breakthroughSession: BreakthroughSession | null
  /**
   * 用户答案映射（question_id → 答案文本）。
   * 选用 Record 而非 Map，便于 Zustand 不可变更新与 React selector 监听。
   */
  breakthroughAnswers: Record<string, string>
  /** 提交中（true 时禁用答题 UI 并显示加载指示）。 */
  breakthroughSubmitting: boolean
  /** 提交结果（由 `game.submitBreakthrough` 返回）。 */
  breakthroughOutcome: BreakthroughOutcome | null
  /** 当前播放的结果动画状态（驱动金光/雷劫粒子组件渲染）。 */
  breakthroughAnimation: BreakthroughAnimationState
  /** 预览/会话加载中（首次进入或刷新预览时短暂为 true）。 */
  breakthroughLoading: boolean

  // ===== 动作 =====
  setWorld: (world: Game3DWorld | null) => void
  setBuildings: (buildings: Building3D[]) => void
  addBuilding: (building: Building3D) => void
  removeBuilding: (id: string) => void
  updateBuilding: (id: string, patch: Partial<Building3D>) => void
  selectBuilding: (id: string | null) => void
  setCameraPreset: (preset: CameraPreset) => void
  setViewMode: (mode: ViewMode) => void
  setLowPerf: (low: boolean) => void
  setIntroPlayed: (played: boolean) => void
  setReady: (ready: boolean) => void
  /** 从后端 GameBuilding[] 转换并加载到 3D 场景 */
  loadFromBackendBuildings: (buildings: GameBuilding[]) => void
  /** 从后端 GameWorld 转换并加载到 3D 场景 */
  loadFromBackendWorld: (world: GameWorld) => void

  // ===== 放置模式动作 =====
  /** 进入放置模式（建造新建筑） */
  startPlacement: (item: PlacementItem) => void
  /** 进入移动模式（移动已有建筑） */
  startMoveMode: (buildingId: string) => void
  /** 退出放置/移动模式 */
  cancelPlacement: () => void
  /** 更新预览位置（鼠标移动时由 PlacementController 调用） */
  setPreviewPosition: (pos: [number, number, number] | null) => void
  /** 标记操作进行中 */
  setActionInProgress: (inProgress: boolean) => void

  // ===== 目录缓存动作 =====
  /** 缓存建筑目录（BuildPanel 加载后调用，供 BuildingDetail 计算 cost） */
  setCatalogCache: (catalog: BuildingCatalog | null) => void
  /** 按 category + subtype 查 base_cost（level=1 基础积分） */
  getBaseCost: (category: string, subtype: string) => number | null

  // ===== 时间轴回放动作（T1）=====
  /** 进入回放模式（不发起 IPC，仅切换状态；时间轴数据由 UI 加载后调用 setReplayTimeline） */
  enterReplay: () => void
  /** 退出回放模式（清空回放状态；UI 层负责调用 ipc.exitReplay 清空后端缓存） */
  exitReplay: () => void
  /** 设置时间轴事件数据（已按时间正序排列）；自动计算 minTime/maxTime 并定位到最新时间 */
  setReplayTimeline: (events: TimelineEvent[]) => void
  /** 设置当前回放时间戳（拖动滑块时调用） */
  setReplayCurrentTime: (ts: number) => void
  /** 设置播放状态（true=播放，false=暂停） */
  setReplayPlaying: (playing: boolean) => void
  /** 设置播放速度（1/2/4/8 倍） */
  setReplaySpeed: (speed: ReplaySpeed) => void
  /** 设置当前重建场景（ipc.getWorldSnapshot 返回后调用） */
  setReplayCurrentScene: (scene: RebuiltScene | null) => void
  /** 设置加载状态（拖动滑块时短暂为 true） */
  setReplayLoading: (loading: boolean) => void
  /** 单步前进（跳到下一个事件的时间戳） */
  stepForward: () => void
  /** 单步后退（跳到上一个事件的时间戳） */
  stepBackward: () => void

  // ===== 境界突破考验动作（T2）=====
  /** 设置突破预览信息（UI 层调用 `game.getBreakthroughPreview` 后写入） */
  setBreakthroughPreview: (preview: BreakthroughPreviewInfo | null) => void
  /**
   * 设置突破会话（UI 层调用 `game.startBreakthrough` 后写入）。
   * 会同时重置答案表 + 提交状态 + 结果状态 + 动画状态。
   */
  setBreakthroughSession: (session: BreakthroughSession | null) => void
  /** 设置单题答案（questionId → answer 文本） */
  setBreakthroughAnswer: (questionId: string, answer: string) => void
  /** 设置提交中状态 */
  setBreakthroughSubmitting: (submitting: boolean) => void
  /** 设置突破结果（提交后由 `game.submitBreakthrough` 返回） */
  setBreakthroughOutcome: (outcome: BreakthroughOutcome | null) => void
  /** 设置结果动画状态（驱动粒子组件渲染） */
  setBreakthroughAnimation: (animation: BreakthroughAnimationState) => void
  /** 设置加载状态 */
  setBreakthroughLoading: (loading: boolean) => void
  /**
   * 清空全部突破状态（用户主动退出 / 动画播放完毕重置）。
   * 保留 preview（境界信息卡仍需展示）。
   */
  clearBreakthroughState: () => void

  /**
   * 重置整个 3D 场景状态（卸载 Game3D 组件时调用，避免内存泄漏）。
   *
   * 依据 `01_架构设计.md` §2.3.2 资源释放策略：
   *   - 清空 world / buildings / catalog 缓存（避免大对象驻留）
   *   - 退出放置/移动模式
   *   - 取消建筑选中
   *   - 清空回放状态
   *   - 清空突破状态（保留 preview）
   *   - 重置相机/UI 状态
   *
   * 注意：GLB / Texture / Geometry 等 Three.js 资源由 R3F 在 Canvas 卸载时自动 dispose，
   *      本方法仅清理 React 层状态，避免 React 状态在下次进入时残留旧数据。
   */
  reset: () => void
}

// ===== Store 实现 =====

export const useGame3DStore = create<Game3DStore>((set, get) => ({
  // 初始状态
  world: null,
  buildings: [],
  selectedBuildingId: null,
  cameraPreset: 'iso',
  viewMode: 'high',
  isLowPerf: false,
  introPlayed: false,
  isReady: false,

  // 放置模式初始状态
  placementMode: 'idle',
  placementItem: null,
  previewPosition: null,
  movingBuildingId: null,
  actionInProgress: false,

  // 目录缓存
  catalogCache: null,

  // ===== 时间轴回放初始状态（T1）=====
  replayMode: false,
  replayTimeline: [],
  replayCurrentTime: 0,
  replayMinTime: 0,
  replayMaxTime: 0,
  replayPlaying: false,
  replaySpeed: 1,
  replayCurrentScene: null,
  replayLoading: false,

  // ===== 境界突破考验初始状态（T2）=====
  breakthroughPreview: null,
  breakthroughSession: null,
  breakthroughAnswers: {},
  breakthroughSubmitting: false,
  breakthroughOutcome: null,
  breakthroughAnimation: 'idle',
  breakthroughLoading: false,

  // ===== 场景数据 =====
  setWorld: (world) => set({ world }),

  setBuildings: (buildings) => set({ buildings }),

  addBuilding: (building) =>
    set((s) => ({ buildings: [...s.buildings, building] })),

  removeBuilding: (id) =>
    set((s) => ({
      buildings: s.buildings.filter((b) => b.id !== id),
      selectedBuildingId: s.selectedBuildingId === id ? null : s.selectedBuildingId,
    })),

  updateBuilding: (id, patch) =>
    set((s) => ({
      buildings: s.buildings.map((b) => (b.id === id ? { ...b, ...patch } : b)),
    })),

  selectBuilding: (id) => set({ selectedBuildingId: id }),

  // ===== 相机 =====
  setCameraPreset: (preset) => set({ cameraPreset: preset }),

  // ===== 视图 =====
  setViewMode: (mode) => set({ viewMode: mode }),
  setLowPerf: (low) => set({ isLowPerf: low }),

  // ===== UI =====
  setIntroPlayed: (played) => set({ introPlayed: played }),
  setReady: (ready) => set({ isReady: ready }),

  // ===== 数据转换 =====
  loadFromBackendBuildings: (backendBuildings) => {
    const buildings3D: Building3D[] = backendBuildings.map((b) => ({
      id: b.id,
      category: b.building_category,
      subtype: b.building_subtype,
      name: b.name,
      level: b.level,
      position: [b.pos_x, b.pos_y, b.pos_z],
      rotationY: b.rotation_y,
      status: b.status,
      buildProgress: b.build_progress,
      knowledgeDomain: b.knowledge_domain,
    }))
    set({ buildings: buildings3D })
  },

  loadFromBackendWorld: (backendWorld) => {
    // 生成初始地块网格（全平原）
    const tiles: Tile[] = []
    for (let x = 0; x < backendWorld.map_width; x++) {
      for (let z = 0; z < backendWorld.map_height; z++) {
        tiles.push({ x, z, type: 'plain', height: 0 })
      }
    }
    const world3D: Game3DWorld = {
      id: backendWorld.id,
      playerName: backendWorld.player_name,
      civilizationLevel: backendWorld.civilization_level,
      mapWidth: backendWorld.map_width,
      mapHeight: backendWorld.map_height,
      tiles,
    }
    set({ world: world3D })
  },

  // ===== 放置模式动作 =====
  startPlacement: (item) =>
    set({
      placementMode: 'placing',
      placementItem: item,
      previewPosition: null,
      selectedBuildingId: null,
      movingBuildingId: null,
    }),

  startMoveMode: (buildingId) =>
    set({
      placementMode: 'moving',
      movingBuildingId: buildingId,
      placementItem: null,
      previewPosition: null,
      selectedBuildingId: null,
    }),

  cancelPlacement: () =>
    set({
      placementMode: 'idle',
      placementItem: null,
      previewPosition: null,
      movingBuildingId: null,
    }),

  setPreviewPosition: (pos) => set({ previewPosition: pos }),

  setActionInProgress: (inProgress) => set({ actionInProgress: inProgress }),

  // ===== 目录缓存动作 =====
  setCatalogCache: (catalog) => set({ catalogCache: catalog }),

  getBaseCost: (category, subtype) => {
    const cache = get().catalogCache
    if (!cache) return null
    for (const cat of cache.categories) {
      if (cat.category !== category) continue
      for (const sub of cat.subtypes) {
        if (sub.subtype === subtype) return sub.base_cost
      }
    }
    return null
  },

  // ===== 时间轴回放动作（T1）=====
  enterReplay: () =>
    set({
      replayMode: true,
      replayPlaying: false,
      replaySpeed: 1,
      replayCurrentScene: null,
      replayLoading: false,
    }),

  exitReplay: () =>
    set({
      replayMode: false,
      replayTimeline: [],
      replayCurrentTime: 0,
      replayMinTime: 0,
      replayMaxTime: 0,
      replayPlaying: false,
      replaySpeed: 1,
      replayCurrentScene: null,
      replayLoading: false,
    }),

  setReplayTimeline: (events) => {
    // 事件列表应按时间正序传入；若为倒序则就地排序
    const sorted = [...events].sort((a, b) => a.timestamp - b.timestamp)
    const minTime = sorted.length > 0 ? sorted[0].timestamp : 0
    const maxTime = sorted.length > 0 ? sorted[sorted.length - 1].timestamp : 0
    set({
      replayTimeline: sorted,
      replayMinTime: minTime,
      replayMaxTime: maxTime,
      // 默认定位到最新时间
      replayCurrentTime: maxTime,
    })
  },

  setReplayCurrentTime: (ts) => set({ replayCurrentTime: ts }),

  setReplayPlaying: (playing) => set({ replayPlaying: playing }),

  setReplaySpeed: (speed) => set({ replaySpeed: speed }),

  setReplayCurrentScene: (scene) => set({ replayCurrentScene: scene }),

  setReplayLoading: (loading) => set({ replayLoading: loading }),

  stepForward: () => {
    const { replayTimeline, replayCurrentTime } = get()
    // 找到下一个时间戳严格大于当前时间的事件
    const next = replayTimeline.find((e) => e.timestamp > replayCurrentTime)
    if (next) {
      set({ replayCurrentTime: next.timestamp })
    }
  },

  stepBackward: () => {
    const { replayTimeline, replayCurrentTime } = get()
    // 找到上一个时间戳严格小于当前时间的事件（取最大的那个）
    let prev: TimelineEvent | null = null
    for (const e of replayTimeline) {
      if (e.timestamp < replayCurrentTime) {
        if (prev === null || e.timestamp > prev.timestamp) prev = e
      } else {
        break
      }
    }
    if (prev) {
      set({ replayCurrentTime: prev.timestamp })
    }
  },

  // ===== 境界突破考验动作（T2）=====
  setBreakthroughPreview: (preview) => set({ breakthroughPreview: preview }),

  setBreakthroughSession: (session) =>
    set({
      breakthroughSession: session,
      // 进入新会话时清空答案表 + 提交/结果/动画状态
      breakthroughAnswers: {},
      breakthroughSubmitting: false,
      breakthroughOutcome: null,
      breakthroughAnimation: 'idle',
    }),

  setBreakthroughAnswer: (questionId, answer) =>
    set((s) => ({
      breakthroughAnswers: { ...s.breakthroughAnswers, [questionId]: answer },
    })),

  setBreakthroughSubmitting: (submitting) =>
    set({ breakthroughSubmitting: submitting }),

  setBreakthroughOutcome: (outcome) => {
    // 同步推导动画状态：成功 → 金光；失败 → 轻雷劫；跌落 → 重雷劫
    const animation: BreakthroughAnimationState = outcome
      ? outcome.result === 'success'
        ? 'success'
        : outcome.result === 'dropped'
          ? 'dropped'
          : 'failure'
      : 'idle'
    set({ breakthroughOutcome: outcome, breakthroughAnimation: animation })
  },

  setBreakthroughAnimation: (animation) => set({ breakthroughAnimation: animation }),

  setBreakthroughLoading: (loading) => set({ breakthroughLoading: loading }),

  clearBreakthroughState: () =>
    set({
      // 保留 breakthroughPreview（境界信息卡仍需展示）
      breakthroughSession: null,
      breakthroughAnswers: {},
      breakthroughSubmitting: false,
      breakthroughOutcome: null,
      breakthroughAnimation: 'idle',
      breakthroughLoading: false,
    }),

  reset: () =>
    set({
      // 场景数据
      world: null,
      buildings: [],
      selectedBuildingId: null,
      // 相机与视图（保留 viewMode/isLowPerf 用户偏好）
      cameraPreset: 'iso',
      introPlayed: false,
      isReady: false,
      // 放置模式
      placementMode: 'idle',
      placementItem: null,
      previewPosition: null,
      movingBuildingId: null,
      actionInProgress: false,
      // 目录缓存
      catalogCache: null,
      // 时间轴回放
      replayMode: false,
      replayTimeline: [],
      replayCurrentTime: 0,
      replayMinTime: 0,
      replayMaxTime: 0,
      replayPlaying: false,
      replaySpeed: 1,
      replayCurrentScene: null,
      replayLoading: false,
      // 突破考验（保留 preview）
      breakthroughSession: null,
      breakthroughAnswers: {},
      breakthroughSubmitting: false,
      breakthroughOutcome: null,
      breakthroughAnimation: 'idle',
      breakthroughLoading: false,
    }),
}))
