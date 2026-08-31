// @ts-nocheck
/**
 * @deprecated 旧版 2D 游戏页面 - 已归档
 *
 * 此文件为 NexTerm·元界 项目阶段5 Task 5.2 之前的旧版 Game.tsx。
 * 因 Task 5.1 重写后端 `game_commands.rs`（22 个新 IPC 命令）后，
 * Task 5.2 重写 `ipc.ts` 的 game 命名空间（删除全部旧命令），
 * 旧 Game.tsx 调用的 `game.createWorld` / `game.placeBuilding` /
 * `game.gameSave` / `game.gameCombatStart` 等方法已不存在，
 * 导致 TypeScript 编译失败。
 *
 * 处理方式：
 * - 旧 Game.tsx 归档到此处（`_archived/Game.legacy.tsx`）
 * - 新 `pages/Game.tsx` 替换为最小占位（"游戏模块重构中"）
 * - 阶段7 Task 7.2 将用完整的 `/game` 数据预览页替换占位
 *
 * 此文件加入 `@ts-nocheck` 跳过类型检查，仅供历史参考，不应被任何模块 import。
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useState, useEffect, useCallback, useRef } from 'react'
import { useSearchParams } from 'react-router-dom'
import { game, intelligence } from '@/lib/ipc'
import { useIntelligence } from '@/hooks/useIntelligence'
import styles from './Game.module.css'
import type {
  GameBuilding,
  GameTile,
  GameWorld,
  BuildingCatalogItem,
  BuildingLevelUpResult,
  RewardLog,
  KnowledgeStats,
  CultivationStageInfo,
  Achievement,
  RandomEvent,
} from './game/types'
import {
  BUILDING_PRODUCTION,
  BUILDING_SPECIAL_EFFECTS,
  ACHIEVEMENTS,
  BUILDING_EMOJI,
  TERRAIN_COLORS,
  TERRAIN_NAMES,
  STAGE_NAMES,
  BUILDING_NAMES,
  STAGE_COLORS,
  stageOrder,
  stageFromXp,
  calcProgress,
  isBuildableTerrain,
} from './game/types'

export default function Game() {
  const [searchParams] = useSearchParams()
  const activeTab = searchParams.get('tab') || 'catalog'
  const [loading, setLoading] = useState(true)
  const [world, setWorld] = useState<GameWorld | null>(null)
  const [worlds, setWorlds] = useState<GameWorld[]>([])
  const [catalog, setCatalog] = useState<BuildingCatalogItem[]>([])
  const [stages, setStages] = useState<CultivationStageInfo[]>([])
  const [kbStats, setKbStats] = useState<KnowledgeStats | null>(null)
  const [rewardLogs, setRewardLogs] = useState<RewardLog[]>([])
  const [placingType, setPlacingType] = useState<string | null>(null)
  const [selectedBuilding, setSelectedBuilding] = useState<{
    building: GameBuilding
    x: number
    y: number
  } | null>(null)
  const [hoverPos, setHoverPos] = useState<{ x: number; y: number } | null>(null)
  const [toast, setToast] = useState<{ msg: string; type: 'ok' | 'err' } | null>(null)
  const [showNewWorld, setShowNewWorld] = useState(false)
  const [newWorldName, setNewWorldName] = useState('')
  const [showWorldList, setShowWorldList] = useState(false)
  const [aiAdviceLoading, setAiAdviceLoading] = useState(false)
  const [aiAdvice, setAiAdvice] = useState('')
  const { aiOn, featureOn } = useIntelligence()
  const adviceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const [achievements, setAchievements] = useState<Achievement[]>(ACHIEVEMENTS.map(a => ({ ...a, unlocked: false })))
  const [unlockedBadge, setUnlockedBadge] = useState<string | null>(null)
  const [spiritStoneAnim, setSpiritStoneAnim] = useState(false)
  const [lastSpiritStones, setLastSpiritStones] = useState(0)

  const [productionCounts, setProductionCounts] = useState<Record<string, number>>({})
  const [nextProductionTick, setNextProductionTick] = useState<number>(30)
  const [showProductionPulse, setShowProductionPulse] = useState(false)
  const productionTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const [showEventModal, setShowEventModal] = useState(false)
  const [currentEvent, setCurrentEvent] = useState<RandomEvent | null>(null)
  const [eventCooldown, setEventCooldown] = useState(false)
  const eventTimerRef = useRef<ReturnType<typeof setInterval> | null>(null)

  // 存档管理
  const [saveSlots, setSaveSlots] = useState<any[]>([])
  const [saveLoading, setSaveLoading] = useState(false)
  const [saveSlotName, setSaveSlotName] = useState('')
  const [showSavePanel, setShowSavePanel] = useState(false)

  // 战斗系统
  const [combatState, setCombatState] = useState<any>(null)
  const [combatLoading, setCombatLoading] = useState(false)
  const [showCombatPanel, setShowCombatPanel] = useState(false)

  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [mapScale, setMapScale] = useState(0.7)
  const [mapOffset, setMapOffset] = useState({ x: 0, y: 0 })
  const [isDragging, setIsDragging] = useState(false)
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 })
  const [lastOffset, setLastOffset] = useState({ x: 0, y: 0 })
  const canvasSizeRef = useRef({ width: 0, height: 0 })
  
  // 缓存地图形状，避免每次渲染都重新生成
  const irregularShapeRef = useRef<boolean[][] | null>(null)
  const shapeSeedRef = useRef<number>(Math.random() * 10000)

  const PRODUCTION_INTERVAL = 30000
  const EVENT_INTERVAL = 90000
  const EVENT_CHANCE = 0.3

  const show = useCallback((msg: string, type: 'ok' | 'err') => {
    setToast({ msg, type })
    setTimeout(() => setToast(null), 3000)
  }, [])

  const loadInitialData = useCallback(async () => {
    setLoading(true)
    try {
      const [catalogRes, stagesRes, worldsRes] = await Promise.all([
        game.getBuildingCatalog(),
        game.getCultivationStages(),
        game.listWorlds(),
      ])

      if (catalogRes?.data) setCatalog(catalogRes.data)
      if (stagesRes?.data) setStages(stagesRes.data)
      if (worldsRes?.data && worldsRes.data.length > 0) {
        setWorlds(worldsRes.data)
        const firstWorld = worldsRes.data[0]
        setWorld(firstWorld)
        loadWorldData(firstWorld.id)
      }
    } catch (e) {
      show(String(e), 'err')
    } finally {
      setLoading(false)
    }
  }, [show])

  const loadWorldData = useCallback(
    async (worldId: string) => {
      try {
        const [kbRes, logsRes] = await Promise.all([
          game.getKbStats(),
          game.getRewardHistory(worldId),
        ])
        if (kbRes?.data) setKbStats(kbRes.data)
        if (logsRes?.data) setRewardLogs(logsRes.data)
      } catch (e) {
        show(String(e), 'err')
      }
    },
    [show]
  )

  const refreshWorld = useCallback(async () => {
    if (!world) return
    try {
      const res = await game.getWorld(world.id)
      if (res?.data) setWorld(res.data)
    } catch (e) {
      show(String(e), 'err')
    }
  }, [world, show])

  const fetchAiAdvice = useCallback(async () => {
    if (!world) return
    setAiAdviceLoading(true)
    setAiAdvice('')
    try {
      const res = await intelligence.gameRecommend(
        0,
        new Date().getHours(),
        [],
      )
      if (res?.data) {
        const parts: string[] = []
        if (res.data.addiction_risk) parts.push(`风险等级: ${res.data.addiction_risk}`)
        if (res.data.recommended_games?.length) parts.push(`推荐: ${res.data.recommended_games.join(', ')}`)
        if (res.data.anti_addiction_warnings?.length) parts.push(`提醒: ${res.data.anti_addiction_warnings.join('; ')}`)
        setAiAdvice(parts.join(' | ') || '暂无推荐')
      } else {
        setAiAdvice('智能推荐暂不可用')
      }
    } catch {
      setAiAdvice('智能推荐暂不可用')
    }
    setAiAdviceLoading(false)
  }, [world])

  // === Passive AI: auto-show strategy advice when world loads (debounced 3s) ===
  useEffect(() => {
    if (!aiOn || !featureOn('game_recommend') || !world) {
      setAiAdvice('')
      return
    }
    if (adviceTimerRef.current) clearTimeout(adviceTimerRef.current)
    setAiAdviceLoading(true)
    setAiAdvice('')
    adviceTimerRef.current = setTimeout(() => {
      fetchAiAdvice()
    }, 3000)
    return () => { if (adviceTimerRef.current) clearTimeout(adviceTimerRef.current) }
  }, [world?.id, world?.character?.spirit_stones, aiOn, featureOn, fetchAiAdvice])

  useEffect(() => {
    loadInitialData()
  }, [loadInitialData])

  useEffect(() => {
    if (!world) return
    let newUnlock: string | null = null
    setAchievements(prev =>
      prev.map(a => {
        if (a.unlocked) return a
        const ctx = { world: world!, catalog, kbStats, stageOrder }
        const achieved = a.condition(ctx)
        if (achieved && !a.unlocked) {
          newUnlock = newUnlock || a.id
          return { ...a, unlocked: true }
        }
        return a
      })
    )
    if (newUnlock) {
      setUnlockedBadge(newUnlock)
      setTimeout(() => setUnlockedBadge(null), 2500)
    }
  }, [world, kbStats, catalog])

  useEffect(() => {
    if (!world) return
    if (world.character.spirit_stones > lastSpiritStones) {
      setSpiritStoneAnim(true)
      setTimeout(() => setSpiritStoneAnim(false), 600)
    }
    setLastSpiritStones(world.character.spirit_stones)
  }, [world?.character.spirit_stones])

  useEffect(() => {
    if (!world) return

    productionTimerRef.current = setInterval(() => {
      setNextProductionTick(prev => {
        if (prev <= 1) {
          const counts: Record<string, number> = {}
          let totalSpiritStones = 0
          let totalXp = 0

          world.grid.forEach(row => {
            row.forEach(tile => {
              if (tile.building) {
                const bt = tile.building.building_type
                const lv = tile.building.level
                const base = BUILDING_PRODUCTION[bt]
                if (base) {
                  const mult = 1 + (lv - 1) * 0.3
                  const stones = Math.round(base.spirit_stones * mult)
                  const xp = Math.round(base.xp * mult)
                  counts[bt] = (counts[bt] || 0) + stones
                  totalSpiritStones += stones
                  totalXp += xp
                }
              }
            })
          })

          if (totalSpiritStones > 0 || totalXp > 0) {
            setProductionCounts(counts)
            setShowProductionPulse(true)
            setTimeout(() => setShowProductionPulse(false), 3000)

            setWorld(prev => {
              if (!prev) return prev
              return {
                ...prev,
                character: {
                  ...prev.character,
                  spirit_stones: prev.character.spirit_stones + totalSpiritStones,
                  total_xp: prev.character.total_xp + totalXp,
                  cultivation_stage: stageFromXp(prev.character.total_xp + totalXp),
                  cultivation_progress: calcProgress(prev.character.total_xp + totalXp),
                },
                updated_at: new Date().toISOString(),
              }
            })
          }
          return PRODUCTION_INTERVAL / 1000
        }
        return prev - 1
      })
    }, 1000)

    return () => {
      if (productionTimerRef.current) clearInterval(productionTimerRef.current)
    }
  }, [world?.id])

  useEffect(() => {
    if (!world) return

    const RANDOM_EVENTS: RandomEvent[] = [
      { type: 'spiritual_ore', title: '灵矿脉显现', message: '一道灵光冲天而起！地下矿脉被发现了，获得额外灵石奖励', icon: '⛏️', reward: { spirit_stones: 50 + Math.floor(Math.random() * 100) } },
      { type: 'heavenly_tribulation', title: '小天劫降临', message: '乌云密布，一道闪电劈向你的建筑！部分建筑受损', icon: '⚡', penalty: { spirit_stones: 30 + Math.floor(Math.random() * 50), building_damage: 1 } },
      { type: 'ancient_ruins', title: '上古遗迹', message: '探索中发现了一座上古修士的洞府，获得大量修为', icon: '🏛️', reward: { xp: 200 + Math.floor(Math.random() * 300) } },
      { type: 'demon_beast', title: '妖兽来袭', message: '一只妖兽闯入了你的领地，灵石被掠夺', icon: '👹', penalty: { spirit_stones: 40 + Math.floor(Math.random() * 80) } },
      { type: 'spirit_rain', title: '灵雨降临', message: '天降灵雨滋润大地，所有建筑产出翻倍一次', icon: '🌧️', reward: { spirit_stones: 30 + Math.floor(Math.random() * 60), xp: 50 + Math.floor(Math.random() * 100) } },
      { type: 'elder_guide', title: '高人指点', message: '一位路过的散修前辈指点你修行，修为大增', icon: '🧙', reward: { xp: 150 + Math.floor(Math.random() * 200) } },
    ]

    eventTimerRef.current = setInterval(() => {
      if (eventCooldown) return
      if (Math.random() < EVENT_CHANCE) {
        const event = RANDOM_EVENTS[Math.floor(Math.random() * RANDOM_EVENTS.length)]
        setCurrentEvent({ ...event, reward: event.reward ? { ...event.reward } : undefined, penalty: event.penalty ? { ...event.penalty } : undefined })
        setShowEventModal(true)
        setEventCooldown(true)
        setTimeout(() => setEventCooldown(false), 30000)
      }
    }, EVENT_INTERVAL)

    return () => {
      if (eventTimerRef.current) clearInterval(eventTimerRef.current)
    }
  }, [world?.id, eventCooldown])

  const handleAcceptEvent = async () => {
    if (!world || !currentEvent) return
    try {
      if (currentEvent.reward) {
        setWorld(prev => {
          if (!prev) return prev
          const newXp = prev.character.total_xp + (currentEvent.reward!.xp || 0)
          return {
            ...prev,
            character: {
              ...prev.character,
              spirit_stones: prev.character.spirit_stones + (currentEvent.reward!.spirit_stones || 0),
              total_xp: newXp,
              cultivation_stage: stageFromXp(newXp),
              cultivation_progress: calcProgress(newXp),
            },
            updated_at: new Date().toISOString(),
          }
        })
        show('事件奖励已发放！', 'ok')
      } else if (currentEvent.penalty) {
        const penaltyStones = currentEvent.penalty.spirit_stones || 0
        if (penaltyStones > 0) {
          setWorld(prev => {
            if (!prev) return prev
            return {
              ...prev,
              character: {
                ...prev.character,
                spirit_stones: Math.max(0, prev.character.spirit_stones - penaltyStones),
              },
              updated_at: new Date().toISOString(),
            }
          })
        }
        show('祸兮福所倚，继续修炼！', 'err')
      }
    } catch (e) {
      show(`事件处理失败: ${e}`, 'err')
    }
    setShowEventModal(false)
    setCurrentEvent(null)
  }

  const handleDismissEvent = () => {
    setShowEventModal(false)
    setCurrentEvent(null)
  }

  // 存档管理
  const loadSaveSlots = async () => {
    setSaveLoading(true)
    try {
      const res = await game.gameListSaves()
      if (res?.data) setSaveSlots(res.data)
    } catch (e) {
      show(String(e), 'err')
    } finally {
      setSaveLoading(false)
    }
  }

  const handleGameSave = async () => {
    if (!world) return
    setSaveLoading(true)
    try {
      const slotName = saveSlotName.trim() || `save_${Date.now()}`
      const saveData = JSON.stringify(world)
      const res = await game.gameSave(slotName, saveData)
      if (res?.data !== undefined) {
        show(`存档成功: ${slotName}`, 'ok')
        setSaveSlotName('')
        await loadSaveSlots()
      }
    } catch (e) {
      show(`存档失败: ${e}`, 'err')
    } finally {
      setSaveLoading(false)
    }
  }

  const handleGameLoad = async (slot: string) => {
    setSaveLoading(true)
    try {
      const res = await game.gameLoad(slot)
      if (res?.data) {
        const loadedWorld = JSON.parse(res.data)
        setWorld(loadedWorld)
        show(`已加载存档: ${slot}`, 'ok')
        setShowSavePanel(false)
      }
    } catch (e) {
      show(`加载失败: ${e}`, 'err')
    } finally {
      setSaveLoading(false)
    }
  }

  const handleGameDeleteSave = async (slot: string) => {
    setSaveLoading(true)
    try {
      await game.gameDeleteSave(slot)
      show(`已删除存档: ${slot}`, 'ok')
      await loadSaveSlots()
    } catch (e) {
      show(`删除失败: ${e}`, 'err')
    } finally {
      setSaveLoading(false)
    }
  }

  // 战斗系统
  const handleCombatStart = async (enemyType: string) => {
    setCombatLoading(true)
    try {
      const res = await game.gameCombatStart(enemyType, world?.character?.total_xp || 1)
      if (res?.data) {
        setCombatState(res.data)
        setShowCombatPanel(true)
      }
    } catch (e) {
      show(`战斗初始化失败: ${e}`, 'err')
    } finally {
      setCombatLoading(false)
    }
  }

  const handleCombatAction = async (action: string) => {
    if (!combatState) return
    setCombatLoading(true)
    try {
      const res = await game.gameCombatAction(combatState, action)
      if (res?.data) {
        setCombatState(res.data)
        if (res.data.game_over) {
          if (res.data.result === 'victory') {
            show('🎉 战斗胜利！', 'ok')
            if (res.data.reward) {
              setWorld((prev: any) => {
                if (!prev) return prev
                return {
                  ...prev,
                  character: {
                    ...prev.character,
                    spirit_stones: prev.character.spirit_stones + (res.data.reward.spirit_stones || 0),
                    total_xp: prev.character.total_xp + (res.data.reward.xp || 0),
                  },
                }
              })
            }
          } else {
            show('💀 战斗失败...', 'err')
          }
          setTimeout(() => {
            setShowCombatPanel(false)
            setCombatState(null)
          }, 2000)
        }
      }
    } catch (e) {
      show(`战斗操作失败: ${e}`, 'err')
    } finally {
      setCombatLoading(false)
    }
  }

  const getProductionDisplay = useCallback(() => {
    if (Object.keys(productionCounts).length === 0) return '待产出...'
    return Object.entries(productionCounts)
      .map(([type, amount]) => `${BUILDING_EMOJI[type] || ''} ${BUILDING_NAMES[type] || type} +${amount}`)
      .join(' | ')
  }, [productionCounts])

  const simpleNoise = useCallback((x: number, y: number, seed = 0): number => {
    const hash = Math.sin(x * 12.9898 + y * 78.233 + seed) * 43758.5453123
    return hash - Math.floor(hash)
  }, [])

  const interpolatedNoise = useCallback((x: number, y: number, seed: number): number => {
    const intX = Math.floor(x)
    const intY = Math.floor(y)
    const fracX = x - intX
    const fracY = y - intY

    const v00 = simpleNoise(intX, intY, seed)
    const v10 = simpleNoise(intX + 1, intY, seed)
    const v01 = simpleNoise(intX, intY + 1, seed)
    const v11 = simpleNoise(intX + 1, intY + 1, seed)

    const i1 = v00 * (1 - fracX) + v10 * fracX
    const i2 = v01 * (1 - fracX) + v11 * fracX

    return i1 * (1 - fracY) + i2 * fracY
  }, [simpleNoise])

  const perlinNoise = useCallback((x: number, y: number, seed: number, octaves: number = 3): number => {
    let result = 0
    let amplitude = 1
    let frequency = 1
    let maxValue = 0

    for (let i = 0; i < octaves; i++) {
      result += interpolatedNoise(x * frequency, y * frequency, seed + i * 100) * amplitude
      maxValue += amplitude
      amplitude *= 0.5
      frequency *= 2
    }

    return result / maxValue
  }, [interpolatedNoise])

  const adjustBrightness = useCallback((color: string, amount: number): string => {
    const hex = color.replace('#', '')
    const r = Math.max(0, Math.min(255, parseInt(hex.slice(0, 2), 16) + amount))
    const g = Math.max(0, Math.min(255, parseInt(hex.slice(2, 4), 16) + amount))
    const b = Math.max(0, Math.min(255, parseInt(hex.slice(4, 6), 16) + amount))
    return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`
  }, [])

  const roundRect = useCallback((ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) => {
    if (w < 2 * r) r = w / 2
    if (h < 2 * r) r = h / 2
    ctx.moveTo(x + r, y)
    ctx.arcTo(x + w, y, x + w, y + h, r)
    ctx.arcTo(x + w, y + h, x, y + h, r)
    ctx.arcTo(x, y + h, x, y, r)
    ctx.arcTo(x, y, x + w, y, r)
    ctx.closePath()
  }, [])

  const generateIrregularShape = useCallback((width: number, height: number): boolean[][] => {
    const shape: boolean[][] = []
    const centerX = width / 2
    const centerY = height / 2
    const maxRadius = Math.min(width, height) / 2 * 0.85
    const seed = shapeSeedRef.current

    for (let y = 0; y < height; y++) {
      shape[y] = []
      for (let x = 0; x < width; x++) {
        const dx = x - centerX
        const dy = y - centerY
        const distance = Math.sqrt(dx * dx + dy * dy)
        const angle = Math.atan2(dy, dx)
        
        // 使用 Perlin 噪声生成更自然的形状
        const noiseValue = perlinNoise(x * 0.15, y * 0.15, seed, 4)
        
        // 添加角度相关的变化
        const angleNoise = Math.sin(angle * 4 + seed * 0.01) * 0.1 +
                           Math.sin(angle * 7 + seed * 0.02) * 0.05
        
        // 计算半径，加入噪声
        const radius = maxRadius * (0.85 + noiseValue * 0.4 + angleNoise)
        
        // 检查是否在主形状内
        let inMainShape = distance <= radius
        
        // 添加小的卫星岛屿
        if (!inMainShape && distance > radius && distance < radius + 5) {
          const islandNoise = perlinNoise(x * 0.3, y * 0.3, seed + 500, 2)
          if (islandNoise > 0.7) {
            inMainShape = true
          }
        }
        
        shape[y][x] = inMainShape
      }
    }
    
    // 确保中心区域可建造（起点）
    shape[Math.floor(height / 2)][Math.floor(width / 2)] = true
    if (height > 2 && width > 2) {
      shape[Math.floor(height / 2) - 1][Math.floor(width / 2)] = true
      shape[Math.floor(height / 2)][Math.floor(width / 2) - 1] = true
      shape[Math.floor(height / 2)][Math.floor(width / 2) + 1] = true
      shape[Math.floor(height / 2) + 1][Math.floor(width / 2)] = true
    }
    
    return shape
  }, [perlinNoise])

  const drawMap = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas || !world) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const dpr = window.devicePixelRatio || 1
    const rect = canvas.getBoundingClientRect()
    canvas.width = rect.width * dpr
    canvas.height = rect.height * dpr
    ctx.scale(dpr, dpr)

    canvasSizeRef.current = { width: rect.width, height: rect.height }

    const cellSize = 32 * mapScale
    const mapWidth = world.width * cellSize
    const mapHeight = world.height * cellSize

    ctx.clearRect(0, 0, rect.width, rect.height)
    
    // 绘制背景光晕效果
    const bgGradient = ctx.createRadialGradient(
      rect.width / 2, rect.height / 2, 0,
      rect.width / 2, rect.height / 2, Math.max(rect.width, rect.height) / 2
    )
    bgGradient.addColorStop(0, 'rgba(0, 240, 255, 0.05)')
    bgGradient.addColorStop(0.5, 'rgba(0, 240, 255, 0.02)')
    bgGradient.addColorStop(1, 'rgba(0, 0, 0, 0)')
    ctx.fillStyle = bgGradient
    ctx.fillRect(0, 0, rect.width, rect.height)

    ctx.save()

    const offsetX = (rect.width - mapWidth) / 2 + mapOffset.x
    const offsetY = (rect.height - mapHeight) / 2 + mapOffset.y
    ctx.translate(offsetX, offsetY)

    // 检查是否需要重新生成形状
    if (!irregularShapeRef.current || 
        irregularShapeRef.current.length !== world.height || 
        (irregularShapeRef.current[0]?.length !== world.width)) {
      irregularShapeRef.current = generateIrregularShape(world.width, world.height)
    }
    
    const shape = irregularShapeRef.current
    const clipPath = new Path2D()

    for (let y = 0; y < world.height; y++) {
      for (let x = 0; x < world.width; x++) {
        if (shape[y][x]) {
          clipPath.rect(x * cellSize, y * cellSize, cellSize, cellSize)
        }
      }
    }

    ctx.clip(clipPath)

    // 绘制地形渐变背景
    const terrainGradient = ctx.createLinearGradient(0, 0, 0, mapHeight)
    terrainGradient.addColorStop(0, 'rgba(0, 50, 30, 0.3)')
    terrainGradient.addColorStop(0.5, 'rgba(0, 30, 50, 0.2)')
    terrainGradient.addColorStop(1, 'rgba(0, 50, 30, 0.3)')
    ctx.fillStyle = terrainGradient
    ctx.fillRect(0, 0, mapWidth, mapHeight)

    for (let y = 0; y < world.height; y++) {
      for (let x = 0; x < world.width; x++) {
        const tile = world.grid[y][x]
        const terrainColor = TERRAIN_COLORS[tile.terrain] ?? '#1a1a1a'
        
        // 为每个格子添加渐变效果
        const cellGradient = ctx.createLinearGradient(
          x * cellSize, y * cellSize,
          x * cellSize + cellSize, y * cellSize + cellSize
        )
        cellGradient.addColorStop(0, terrainColor)
        cellGradient.addColorStop(0.5, adjustBrightness(terrainColor, 10))
        cellGradient.addColorStop(1, adjustBrightness(terrainColor, -10))
        
        ctx.fillStyle = cellGradient
        ctx.fillRect(x * cellSize, y * cellSize, cellSize, cellSize)

        if (!shape[y][x]) {
          const voidGradient = ctx.createRadialGradient(
            x * cellSize + cellSize / 2, y * cellSize + cellSize / 2, 0,
            x * cellSize + cellSize / 2, y * cellSize + cellSize / 2, cellSize / 2
          )
          voidGradient.addColorStop(0, 'rgba(0, 0, 0, 0.7)')
          voidGradient.addColorStop(1, 'rgba(0, 0, 0, 0.95)')
          ctx.fillStyle = voidGradient
          ctx.fillRect(x * cellSize, y * cellSize, cellSize, cellSize)
        }

        // 绘制更精致的网格线
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.06)'
        ctx.lineWidth = 0.5
        ctx.strokeRect(x * cellSize + 0.5, y * cellSize + 0.5, cellSize - 1, cellSize - 1)

        if (tile.building) {
          // 建筑光晕效果
          const buildingGlow = ctx.createRadialGradient(
            x * cellSize + cellSize / 2, y * cellSize + cellSize / 2, 0,
            x * cellSize + cellSize / 2, y * cellSize + cellSize / 2, cellSize * 0.8
          )
          buildingGlow.addColorStop(0, 'rgba(255, 215, 0, 0.15)')
          buildingGlow.addColorStop(0.5, 'rgba(255, 215, 0, 0.08)')
          buildingGlow.addColorStop(1, 'rgba(255, 215, 0, 0)')
          ctx.fillStyle = buildingGlow
          ctx.beginPath()
          ctx.arc(x * cellSize + cellSize / 2, y * cellSize + cellSize / 2, cellSize * 0.8, 0, Math.PI * 2)
          ctx.fill()
          
          ctx.font = `${Math.max(14, cellSize * 0.5)}px serif`
          ctx.textAlign = 'center'
          ctx.textBaseline = 'middle'
          
          // 绘制建筑阴影
          ctx.shadowColor = 'rgba(0, 0, 0, 0.5)'
          ctx.shadowBlur = 4
          ctx.shadowOffsetX = 1
          ctx.shadowOffsetY = 1
          
          const emoji = BUILDING_EMOJI[tile.building.building_type] ?? '🏗️'
          ctx.fillText(
            emoji,
            x * cellSize + cellSize / 2,
            y * cellSize + cellSize / 2
          )
          
          ctx.shadowBlur = 0
          ctx.shadowOffsetX = 0
          ctx.shadowOffsetY = 0

          if (tile.building.level > 1) {
            // 等级标签背景
            const levelBg = ctx.createLinearGradient(
              x * cellSize + cellSize - 24, y * cellSize + cellSize - 20,
              x * cellSize + cellSize - 4, y * cellSize + cellSize - 4
            )
            levelBg.addColorStop(0, 'rgba(0, 0, 0, 0.9)')
            levelBg.addColorStop(1, 'rgba(0, 0, 0, 0.7)')
            
            ctx.fillStyle = levelBg
            ctx.beginPath()
            roundRect(ctx, x * cellSize + cellSize - 26, y * cellSize + cellSize - 22, 24, 20, 4)
            ctx.fill()
            
            ctx.strokeStyle = 'rgba(255, 215, 0, 0.5)'
            ctx.lineWidth = 1
            ctx.stroke()
            
            ctx.font = `bold ${Math.max(9, cellSize * 0.28)}px sans-serif`
            ctx.fillStyle = '#FFD700'
            ctx.shadowColor = 'rgba(255, 215, 0, 0.5)'
            ctx.shadowBlur = 2
            ctx.fillText(
              `${tile.building.level}`,
              x * cellSize + cellSize - 14,
              y * cellSize + cellSize - 9
            )
            ctx.shadowBlur = 0
          }
        } else if (!isBuildableTerrain(tile.terrain)) {
          ctx.font = `${Math.max(10, cellSize * 0.32)}px serif`
          ctx.textAlign = 'center'
          ctx.textBaseline = 'middle'
          ctx.globalAlpha = 0.8
          ctx.fillText('🌊', x * cellSize + cellSize / 2, y * cellSize + cellSize / 2)
          ctx.globalAlpha = 1
        }

        if (hoverPos?.x === x && hoverPos?.y === y) {
          // 悬停效果 - 脉冲高亮
          ctx.strokeStyle = placingType
            ? (isBuildableTerrain(tile.terrain) && !tile.building ? 'rgba(0, 255, 100, 0.9)' : 'rgba(255, 0, 110, 0.9)')
            : 'rgba(0, 240, 255, 0.7)'
          ctx.lineWidth = 2.5
          ctx.strokeRect(x * cellSize + 1.5, y * cellSize + 1.5, cellSize - 3, cellSize - 3)
          
          // 内部发光
          ctx.shadowColor = placingType
            ? (isBuildableTerrain(tile.terrain) && !tile.building ? 'rgba(0, 255, 100, 0.5)' : 'rgba(255, 0, 110, 0.5)')
            : 'rgba(0, 240, 255, 0.4)'
          ctx.shadowBlur = 8
          ctx.strokeRect(x * cellSize + 1.5, y * cellSize + 1.5, cellSize - 3, cellSize - 3)
          ctx.shadowBlur = 0
        }

        if (selectedBuilding?.x === x && selectedBuilding?.y === y) {
          // 选中建筑 - 双边框发光效果
          ctx.strokeStyle = '#00F0FF'
          ctx.lineWidth = 3
          ctx.strokeRect(x * cellSize + 1, y * cellSize + 1, cellSize - 2, cellSize - 2)
          
          ctx.shadowColor = '#00F0FF'
          ctx.shadowBlur = 12
          ctx.strokeRect(x * cellSize + 1, y * cellSize + 1, cellSize - 2, cellSize - 2)
          ctx.shadowBlur = 0
          
          // 内框
          ctx.strokeStyle = 'rgba(255, 255, 255, 0.3)'
          ctx.lineWidth = 1
          ctx.strokeRect(x * cellSize + 4, y * cellSize + 4, cellSize - 8, cellSize - 8)
        }
      }
    }

    // 绘制边界装饰
    ctx.strokeStyle = 'rgba(0, 240, 255, 0.4)'
    ctx.lineWidth = 2
    ctx.setLineDash([6, 4])
    ctx.lineDashOffset = 0
    ctx.stroke(clipPath)
    ctx.setLineDash([])

    ctx.restore()
  }, [world, mapScale, mapOffset, hoverPos, placingType, selectedBuilding, generateIrregularShape, adjustBrightness, roundRect])

  useEffect(() => {
    drawMap()
  }, [drawMap])

  useEffect(() => {
    const handleResize = () => drawMap()
    window.addEventListener('resize', handleResize)
    return () => window.removeEventListener('resize', handleResize)
  }, [drawMap])

  // 当世界变化时，重新生成形状和种子
  useEffect(() => {
    if (world) {
      shapeSeedRef.current = Math.random() * 10000
      irregularShapeRef.current = null
      drawMap()
    }
  }, [world?.width, world?.height, world?.id])

  const handleWheel = useCallback((e: React.WheelEvent<HTMLCanvasElement>) => {
    e.preventDefault()
    const zoomIntensity = 0.08
    const delta = e.deltaY < 0 ? 1 + zoomIntensity : 1 - zoomIntensity
    const newScale = Math.max(0.3, Math.min(4, mapScale * delta))

    const canvas = canvasRef.current
    if (!canvas) return

    const rect = canvas.getBoundingClientRect()
    const mouseX = e.clientX - rect.left
    const mouseY = e.clientY - rect.top

    setMapOffset(prev => ({
      x: mouseX - (mouseX - prev.x) * (newScale / mapScale),
      y: mouseY - (mouseY - prev.y) * (newScale / mapScale),
    }))
    setMapScale(newScale)
  }, [mapScale])

  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    setIsDragging(true)
    setDragStart({ x: e.clientX, y: e.clientY })
    setLastOffset({ ...mapOffset })
  }, [mapOffset])

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current
    if (!canvas) return

    const rect = canvas.getBoundingClientRect()
    const mouseX = e.clientX - rect.left
    const mouseY = e.clientY - rect.top

    if (isDragging) {
      const dx = e.clientX - dragStart.x
      const dy = e.clientY - dragStart.y
      setMapOffset({
        x: lastOffset.x + dx,
        y: lastOffset.y + dy,
      })
    } else {
      const cellSize = 32 * mapScale
      const mapWidth = world?.width ? world.width * cellSize : 0
      const mapHeight = world?.height ? world.height * cellSize : 0

      const offsetX = (rect.width - mapWidth) / 2 + mapOffset.x
      const offsetY = (rect.height - mapHeight) / 2 + mapOffset.y

      const gridX = Math.floor((mouseX - offsetX) / cellSize)
      const gridY = Math.floor((mouseY - offsetY) / cellSize)

      if (world && gridX >= 0 && gridX < world.width && gridY >= 0 && gridY < world.height) {
        setHoverPos({ x: gridX, y: gridY })
      } else {
        setHoverPos(null)
      }
    }
  }, [isDragging, dragStart, lastOffset, mapOffset, mapScale, world])

  const handleMouseUp = useCallback(() => {
    setIsDragging(false)
  }, [])

  const handleCanvasClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    if (isDragging) return

    const canvas = canvasRef.current
    if (!canvas || !world) return

    const rect = canvas.getBoundingClientRect()
    const mouseX = e.clientX - rect.left
    const mouseY = e.clientY - rect.top

    const cellSize = 32 * mapScale
    const mapWidth = world.width * cellSize
    const mapHeight = world.height * cellSize

    const offsetX = (rect.width - mapWidth) / 2 + mapOffset.x
    const offsetY = (rect.height - mapHeight) / 2 + mapOffset.y

    const gridX = Math.floor((mouseX - offsetX) / cellSize)
    const gridY = Math.floor((mouseY - offsetY) / cellSize)

    if (gridX < 0 || gridX >= world.width || gridY < 0 || gridY >= world.height) return

    if (placingType) {
      handlePlaceBuilding(gridX, gridY)
    } else {
      const tile = world.grid[gridY]?.[gridX]
      if (tile?.building) {
        setSelectedBuilding({ building: tile.building, x: gridX, y: gridY })
      } else {
        setSelectedBuilding(null)
      }
    }
  }, [world, mapScale, mapOffset, isDragging, placingType])

  const handleZoomIn = useCallback(() => {
    setMapScale(prev => Math.min(4, prev * 1.2))
  }, [])

  const handleZoomOut = useCallback(() => {
    setMapScale(prev => Math.max(0.3, prev / 1.2))
  }, [])

  const handleResetView = useCallback(() => {
    setMapScale(0.7)
    setMapOffset({ x: 0, y: 0 })
  }, [])

  const handleCreateWorld = async () => {
    if (!newWorldName.trim()) return
    try {
      const res = await game.createWorld(newWorldName.trim(), 25, 25)
      if (res?.data) {
        setWorld(res.data)
        setWorlds((prev) => [...prev, res.data!])
        setShowNewWorld(false)
        setNewWorldName('')
        loadWorldData(res.data.id)
        show('世界创建成功！欢迎来到元界', 'ok')
      }
    } catch (e) {
      show(String(e), 'err')
    }
  }

  const handleLoadWorld = async (w: GameWorld) => {
    try {
      const res = await game.getWorld(w.id)
      if (res?.data) {
        setWorld(res.data)
        setShowWorldList(false)
        loadWorldData(res.data.id)
      }
    } catch (e) {
      show(String(e), 'err')
    }
  }

  const handlePlaceBuilding = async (x: number, y: number) => {
    if (!world || !placingType) return

    const tile = world.grid[y]?.[x]
    if (!tile) return
    if (tile.building) {
      setSelectedBuilding({ building: tile.building, x, y })
      setPlacingType(null)
      return
    }
    if (!isBuildableTerrain(tile.terrain)) {
      show(`无法在${TERRAIN_NAMES[tile.terrain]}上建造`, 'err')
      setPlacingType(null)
      return
    }

    try {
      const res = await game.placeBuilding(world.id, placingType, x, y)
      if (res?.data) {
        await refreshWorld()
        await loadWorldData(world.id)
        setPlacingType(null)
        show(`成功建造：${res.data.name}`, 'ok')
      }
    } catch (e) {
      show(String(e), 'err')
      setPlacingType(null)
    }
  }

  const handleUpgradeBuilding = async () => {
    if (!selectedBuilding || !world) return
    try {
      const res = await game.upgradeBuilding(world.id, selectedBuilding.building.id)
      if (res?.data) {
        const result = res.data as BuildingLevelUpResult
        await refreshWorld()
        await loadWorldData(world.id)
        setSelectedBuilding(null)
        show(
          `${result.building.name} 升级到 Lv.${result.new_level}（${result.level_name}）消耗 ${result.spirit_stone_cost} 灵石`,
          'ok'
        )
      }
    } catch (e) {
      show(String(e), 'err')
    }
  }

  const handleRemoveBuilding = async () => {
    if (!selectedBuilding || !world) return
    try {
      const res = await game.removeBuilding(world.id, selectedBuilding.building.id)
      if (res?.data !== undefined) {
        const refund = res.data
        await refreshWorld()
        await loadWorldData(world.id)
        setSelectedBuilding(null)
        show(`已拆除，返还 ${refund} 灵石`, 'ok')
      }
    } catch (e) {
      show(String(e), 'err')
    }
  }

  const handleCollectRewards = async () => {
    if (!world) return
    try {
      const res = await game.collectKbRewards(world.id)
      if (res?.data) {
        await refreshWorld()
        await loadWorldData(world.id)
        const d = res.data
        show(
          `知识奖励已领取！+${d.xp_gain ?? 0} 修为，+${d.spirit_gain ?? 0} 灵石`,
          'ok'
        )
      }
    } catch (e) {
      show(String(e), 'err')
    }
  }

  const isBuildingUnlocked = (item: BuildingCatalogItem): boolean => {
    if (!kbStats || !world) return false
    if (kbStats.total_entries < item.required_entries) return false
    if (kbStats.total_categories < item.required_categories) return false
    if (stageOrder(world.character.cultivation_stage) < stageOrder(item.required_stage)) return false
    return true
  }

  const canAfford = (item: BuildingCatalogItem): boolean => {
    return world ? world.character.spirit_stones >= item.spirit_stone_cost : false
  }

  const buildableCount = world
    ? world.grid.reduce<GameTile[]>((acc, row) => acc.concat(row), []).filter((t) => t.building).length
    : 0

  const maxBuildings = world
    ? stages.find(
        (s) => s.stage === world.character.cultivation_stage
      )?.max_buildings ?? 3
    : 3

  const currentStage = world
    ? stages.find((s) => s.stage === world.character.cultivation_stage)
    : null

  const xpForNext = currentStage?.xp_required ?? 0
  const xpProgress = xpForNext > 0 ? (world?.character.total_xp ?? 0) / xpForNext : 1

  if (loading) {
    return (
      <div className={styles.page}>
        <div className={styles.loadingOverlay}>
          <span className={styles.loadingDot} />
          <span>正在进入元界...</span>
        </div>
      </div>
    )
  }

  if (!world) {
    return (
      <div className={styles.page}>
        <div className={styles.emptyState}>
          <div className={styles.emptyIcon}>🌏</div>
          <h3 className={styles.emptyTitle}>尚未创建世界</h3>
          <p className={styles.emptyDesc}>
            在元界中建造你的修仙城池，知识积累将化为灵石与修为
          </p>
          {showNewWorld ? (
            <div className={styles.createForm}>
              <input
                className={styles.input}
                value={newWorldName}
                onChange={(e) => setNewWorldName(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleCreateWorld()}
                placeholder="输入修士道号..."
                autoFocus
              />
              <button className={styles.btnPrimary} onClick={handleCreateWorld}>
                创世
              </button>
              <button
                className={styles.btnGhost}
                onClick={() => {
                  setShowNewWorld(false)
                  setNewWorldName('')
                }}
              >
                取消
              </button>
            </div>
          ) : (
            <button
              className={styles.btnPrimary}
              onClick={() => setShowNewWorld(true)}
            >
              + 创建新世界
            </button>
          )}
          {worlds.length > 0 && (
            <div className={styles.worldListSection}>
              <p className={styles.sectionLabel}>或选择已有世界</p>
              {worlds.map((w) => (
                <button
                  key={w.id}
                  className={styles.btnGhost}
                  onClick={() => handleLoadWorld(w)}
                >
                  {w.player_name}（{STAGE_NAMES[w.character.cultivation_stage] ?? '凡人'}）
                </button>
              ))}
            </div>
          )}
        </div>
        {showEventModal && currentEvent && (
        <div className={styles.eventOverlay}>
          <div className={`${styles.eventModal} ${currentEvent.reward ? styles.eventReward : styles.eventPenalty}`}>
            <div className={styles.eventIcon}>{currentEvent.icon}</div>
            <h3 className={styles.eventTitle}>{currentEvent.title}</h3>
            <p className={styles.eventMessage}>{currentEvent.message}</p>
            <div className={styles.eventEffects}>
              {currentEvent.reward && Object.entries(currentEvent.reward).map(([k, v]) => v !== undefined && (
                <span key={k} className={styles.eventRewardItem}>
                  {k === 'spirit_stones' ? '💎 +' : '⚡ +'}{v} {k === 'spirit_stones' ? '灵石' : '修为'}
                </span>
              ))}
              {currentEvent.penalty && Object.entries(currentEvent.penalty).map(([k, v]) => v !== undefined && (
                <span key={k} className={styles.eventPenaltyItem}>
                  {k === 'spirit_stones' ? '💎 -' : '🏚️ -'}{v} {k === 'spirit_stones' ? '灵石' : '建筑受损'}
                </span>
              ))}
            </div>
            <div className={styles.modalActions}>
              <button className={styles.btnPrimary} onClick={handleAcceptEvent}>
                {currentEvent.reward ? '接受' : '应对'}
              </button>
              <button className={styles.btnGhost} onClick={handleDismissEvent}>
                无视
              </button>
            </div>
          </div>
        </div>
      )}

      {toast && (
          <div
            className={`${styles.toast} ${
              toast.type === 'ok' ? styles.toastOk : styles.toastErr
            }`}
          >
            {toast.msg}
          </div>
        )}
      </div>
    )
  }

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <h2 className={styles.headerTitle}>
          🌏 元界 · 修仙建造世界
        </h2>
        <span className={styles.headerSub}>
          {world.player_name} · {STAGE_NAMES[world.character.cultivation_stage] ?? '凡人'}
        </span>
        <div className={styles.headerActions}>
          <button className={styles.btnGhost} onClick={() => setShowWorldList(!showWorldList)}>
            世界列表
          </button>
          <button className={styles.btnGhost} onClick={() => { setShowSavePanel(true); loadSaveSlots() }}>
            💾 存档
          </button>
          <button className={styles.btnGhost} onClick={() => handleCombatStart('goblin')} style={{ color: '#FF0000' }}>
            ⚔️ 战斗
          </button>
          <button className={styles.btnPrimary} onClick={() => setShowNewWorld(true)}>
            + 新世界
          </button>
          {aiOn && featureOn('game_recommend') && aiAdviceLoading && (
            <span className={styles.aiPulsingBadge}>� 分析中</span>
          )}
          {showWorldList && (
            <div className={styles.dropdown}>
              {worlds.map((w) => (
                <div
                  key={w.id}
                  className={`${styles.dropdownItem} ${w.id === world.id ? styles.dropdownItemActive : ''}`}
                  onClick={() => handleLoadWorld(w)}
                >
                  {w.player_name} · {STAGE_NAMES[w.character.cultivation_stage] ?? '凡人'}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className={styles.main}>
        {aiAdvice && (
          <div className={styles.aiAdvicePanel}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
              <span style={{ fontSize: 11, color: '#00F0FF', fontFamily: 'var(--nt-font-mono)' }}>🧙 策略建议</span>
              <button onClick={() => setAiAdvice('')} style={{ background:'none', border:'none', color:'#FF5050', cursor:'pointer', fontSize:11 }}>✕</button>
            </div>
            <div style={{ fontSize: 12, color: 'rgba(200,200,220,0.85)', lineHeight: 1.6 }}>{aiAdvice}</div>
          </div>
        )}
        {activeTab && (
          <div className={styles.contentPanel}>
            {activeTab === 'catalog' && (
              <>
                <div className={styles.sidebarTitle}>🏗️ 建筑目录</div>
                <div className={styles.buildLimit}>
                  已建造：{buildableCount}/{maxBuildings}
                  {buildableCount >= maxBuildings && (
                    <span className={styles.limitWarning}>（已达上限）</span>
                  )}
                </div>
                <div className={styles.catalogList}>
                  {catalog.map((item) => {
                    const unlocked = isBuildingUnlocked(item)
                    const affordable = canAfford(item)
                    const active = placingType === item.building_type
                    return (
                      <div
                        key={item.building_type}
                        className={`${styles.catalogItem} ${
                          !unlocked ? styles.catalogLocked : ''
                        } ${active ? styles.catalogActive : ''}`}
                        onClick={() => {
                          if (!unlocked) {
                            show(
                              `需要：${item.required_entries}条知识条目 + ${item.required_categories}个分类 + ${STAGE_NAMES[item.required_stage]}境界`,
                              'err'
                            )
                            return
                          }
                          if (!affordable) {
                            show(`灵石不足（需要${item.spirit_stone_cost}灵石）`, 'err')
                            return
                          }
                          setPlacingType(
                            placingType === item.building_type ? null : item.building_type
                          )
                          setSelectedBuilding(null)
                        }}
                      >
                        <div className={styles.catalogIcon}>
                          {BUILDING_EMOJI[item.building_type] ?? '🏗️'}
                        </div>
                        <div className={styles.catalogInfo}>
                          <div className={styles.catalogName}>
                            {item.name}
                            {!unlocked && ' 🔒'}
                            {unlocked && !affordable && ' ⚠️'}
                          </div>
                          <div className={styles.catalogDesc}>{item.description}</div>
                          <div className={styles.catalogMeta}>
                            <span>💎 {item.spirit_stone_cost}</span>
                            <span>📖 {item.required_entries}条目</span>
                            <span>📁 {item.required_categories}分类</span>
                            <span style={{ color: STAGE_COLORS[item.required_stage] }}>
                              {STAGE_NAMES[item.required_stage]}
                            </span>
                          </div>
                        </div>
                      </div>
                    )
                  })}
                </div>
                {placingType && (
                  <div className={styles.placementHint}>
                    选中：{BUILDING_NAMES[placingType] ?? placingType}
                    <br />
                    点击地图放置 · 再次点击取消
                  </div>
                )}
                <div className={`${styles.productionPanel} ${showProductionPulse ? styles.productionPulse : ''}`}>
                  <div className={styles.productionTitle}>
                    ⏳ 建筑产出 · 倒计时
                    <span className={styles.productionTimer}>{nextProductionTick}s</span>
                  </div>
                  <div className={styles.productionList}>
                    {getProductionDisplay()}
                  </div>
                </div>
              </>
            )}

            {activeTab === 'character' && (
              <>
                <div className={styles.sidebarTitle}>🧘 修士信息</div>
                <div className={styles.charPanel}>
                  <div className={styles.charRow}>
                    <span className={styles.charLabel}>道号</span>
                    <span className={styles.charValue}>{world.character.name}</span>
                  </div>
                  <div className={styles.charRow}>
                    <span className={styles.charLabel}>境界</span>
                    <span
                      className={styles.charValue}
                      style={{
                        color: STAGE_COLORS[world.character.cultivation_stage],
                      }}
                    >
                      {STAGE_NAMES[world.character.cultivation_stage] ?? '凡人'}
                    </span>
                  </div>
                  <div className={styles.charRow}>
                    <span className={styles.charLabel}>修为</span>
                    <span className={styles.charValue}>{world.character.total_xp} XP</span>
                  </div>
                  <div className={styles.progressBar}>
                    <div
                      className={styles.progressFill}
                      style={{
                        width: `${Math.min(xpProgress * 100, 100)}%`,
                        backgroundColor:
                          STAGE_COLORS[world.character.cultivation_stage],
                      }}
                    />
                  </div>
                  <div className={styles.progressLabel}>
                    {xpForNext > 0
                      ? `${world.character.total_xp} / ${xpForNext}`
                      : '已达最高境界'}
                  </div>
                  {stages.length > 0 && (
                    <div className={styles.stagePath}>
                      {stages.map((s) => (
                        <span
                          key={s.stage}
                          className={`${styles.stageDot} ${
                            stageOrder(s.stage) <= stageOrder(world.character.cultivation_stage)
                              ? styles.stageDotActive
                              : ''
                          }`}
                          style={
                            stageOrder(s.stage) <= stageOrder(world.character.cultivation_stage)
                              ? { backgroundColor: STAGE_COLORS[s.stage] }
                              : {}
                          }
                          title={s.name}
                        >
                          {s.name[0]}
                        </span>
                      ))}
                    </div>
                  )}
                  <div className={styles.divider} />
                  <div className={styles.charRow}>
                    <span className={styles.charLabel}>灵石</span>
                    <span
                      className={`${styles.charValue} ${spiritStoneAnim ? styles.spiritGlow : ''}`}
                      style={{ color: '#FFD700' }}
                    >
                      💎 {world.character.spirit_stones}
                    </span>
                  </div>
                </div>
              </>
            )}

            {activeTab === 'achievements' && (
              <>
                <div className={styles.sidebarTitle}>🏆 成就</div>
                <div className={styles.achievementPanel}>
                  {achievements.filter(a => a.unlocked).length === 0 ? (
                    <p className={styles.emptyHint}>暂无成就，继续探索吧~</p>
                  ) : (
                    achievements.map((ach) => {
                      const ctx = { world: world!, catalog, kbStats }
                      const progress = ach.progress?.(ctx)
                      const unlocked = ach.unlocked
                      return (
                        <div
                          key={ach.id}
                          className={`${styles.achievementItem} ${
                            unlocked ? styles.achievementUnlocked : styles.achievementLocked
                          }`}
                        >
                          <span className={styles.achievementIcon}>
                            {unlocked ? ach.icon : '🔒'}
                          </span>
                          <div className={styles.achievementInfo}>
                            <div className={styles.achievementName}>
                              {ach.name}
                              {unlocked && <span className={styles.achievementCheck}> ✓</span>}
                            </div>
                            <div className={styles.achievementDesc}>{ach.description}</div>
                            {progress && !unlocked && (
                              <div className={styles.achievementProgress}>
                                <div className={styles.achievementBar}>
                                  <div
                                    className={styles.achievementFill}
                                    style={{
                                      width: `${Math.min((progress.current / progress.target) * 100, 100)}%`,
                                    }}
                                  />
                                </div>
                                <span className={styles.achievementProgressText}>
                                  {progress.current}/{progress.target}
                                </span>
                              </div>
                            )}
                          </div>
                        </div>
                      )
                    })
                  )}
                </div>
              </>
            )}

            {activeTab === 'knowledge_rewards' && (
              <>
                <div className={styles.sidebarTitle}>📚 知识奖励</div>
                <div className={styles.kbPanel}>
                  {kbStats ? (
                    <>
                      <div className={styles.charRow}>
                        <span className={styles.charLabel}>知识条目</span>
                        <span className={styles.charValue}>{kbStats.total_entries}</span>
                      </div>
                      <div className={styles.charRow}>
                        <span className={styles.charLabel}>知识分类</span>
                        <span className={styles.charValue}>{kbStats.total_categories}</span>
                      </div>
                      <div className={styles.charRow}>
                        <span className={styles.charLabel}>待领取</span>
                        <span className={styles.charValue} style={{ color: '#FFD700' }}>
                          +{(kbStats.total_entries - world.character.last_kb_entry_count) * 3 +
                            (kbStats.total_categories - world.character.last_kb_category_count) * 10} 灵石
                        </span>
                      </div>
                      <button
                        className={styles.btnCollect}
                        onClick={handleCollectRewards}
                        disabled={
                          kbStats.total_entries <= world.character.last_kb_entry_count &&
                          kbStats.total_categories <= world.character.last_kb_category_count
                        }
                      >
                        领取知识奖励
                      </button>
                    </>
                  ) : (
                    <p className={styles.emptyHint}>加载中...</p>
                  )}
                </div>

                <div className={styles.sidebarTitle} style={{ marginTop: 8 }}>🏆 最近奖励</div>
                <div className={styles.logPanel}>
                  {rewardLogs.length === 0 ? (
                    <p className={styles.emptyHint}>暂无记录</p>
                  ) : (
                    rewardLogs.slice(0, 10).map((log) => (
                      <div key={log.id} className={styles.logItem}>
                        <span className={styles.logType}>
                          {log.reward_type === 'xp' ? '⚡' : '💎'}
                        </span>
                        <span className={styles.logAmount}>
                          +{log.amount}
                          {log.reward_type === 'xp' ? ' 修为' : ' 灵石'}
                        </span>
                        <span className={styles.logSource}>{log.source}</span>
                      </div>
                    ))
                  )}
                </div>
              </>
            )}
          </div>
        )}

        <div className={styles.mapContainer}>
          <div className={styles.mapCanvasWrapper}>
            <canvas
              ref={canvasRef}
              className={styles.mapCanvas}
              onWheel={handleWheel}
              onMouseDown={handleMouseDown}
              onMouseMove={handleMouseMove}
              onMouseUp={handleMouseUp}
              onMouseLeave={() => { setIsDragging(false); setHoverPos(null) }}
              onClick={handleCanvasClick}
            />
          </div>
          <div className={styles.mapControls}>
            <button className={styles.mapControlBtn} onClick={handleZoomIn} title="放大">+</button>
            <button className={styles.mapControlBtn} onClick={handleZoomOut} title="缩小">−</button>
            <button className={styles.mapControlBtn} onClick={handleResetView} title="重置视图">⊙</button>
          </div>
          <div className={styles.mapZoomLevel}>
            {(mapScale * 100).toFixed(0)}%
          </div>
          <div className={styles.mapLegend}>
            <div className={styles.mapLegendTitle}>地形图例</div>
            {Object.entries(TERRAIN_NAMES).map(([key, name]) => (
              <div key={key} className={styles.mapLegendItem}>
                <div 
                  className={styles.mapLegendColor} 
                  style={{ backgroundColor: TERRAIN_COLORS[key] }}
                />
                <span className={styles.mapLegendText}>{name}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      {selectedBuilding && (
        <div className={styles.buildingModal}>
          <div className={styles.buildingModalContent}>
            <h3 className={styles.modalTitle}>
              {BUILDING_EMOJI[selectedBuilding.building.building_type]}{' '}
              {selectedBuilding.building.name}
            </h3>
            <div className={styles.modalRow}>
              <span>等级</span>
              <span>Lv.{selectedBuilding.building.level}</span>
            </div>
            <div className={styles.modalRow}>
              <span>类型</span>
              <span>
                {BUILDING_NAMES[selectedBuilding.building.building_type] ??
                  selectedBuilding.building.building_type}
              </span>
            </div>
            <div className={styles.modalRow}>
              <span>位置</span>
              <span>
                ({selectedBuilding.x}, {selectedBuilding.y})
              </span>
            </div>
            {BUILDING_SPECIAL_EFFECTS[selectedBuilding.building.building_type] && (
              <div className={styles.specialEffect}>
                <span className={styles.specialEffectLabel}>
                  ⚡ {BUILDING_SPECIAL_EFFECTS[selectedBuilding.building.building_type].label}
                </span>
                <span className={styles.specialEffectDesc}>
                  {BUILDING_SPECIAL_EFFECTS[selectedBuilding.building.building_type].desc}
                </span>
              </div>
            )}
            <div className={styles.modalActions}>
              <button className={styles.btnPrimary} onClick={handleUpgradeBuilding}>
                ⬆ 升级
              </button>
              <button className={styles.btnDanger} onClick={handleRemoveBuilding}>
                ✕ 拆除
              </button>
              <button
                className={styles.btnGhost}
                onClick={() => setSelectedBuilding(null)}
              >
                关闭
              </button>
            </div>
          </div>
        </div>
      )}

      {showNewWorld && (
        <div className={styles.buildingModal}>
          <div className={styles.buildingModalContent}>
            <h3 className={styles.modalTitle}>创建新世界</h3>
            <input
              className={styles.input}
              value={newWorldName}
              onChange={(e) => setNewWorldName(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleCreateWorld()}
              placeholder="输入修士道号..."
              autoFocus
            />
            <div className={styles.modalActions}>
              <button className={styles.btnPrimary} onClick={handleCreateWorld}>
                创世
              </button>
              <button
                className={styles.btnGhost}
                onClick={() => {
                  setShowNewWorld(false)
                  setNewWorldName('')
                }}
              >
                取消
              </button>
            </div>
          </div>
        </div>
      )}

      {toast && (
        <div
          className={`${styles.toast} ${
            toast.type === 'ok' ? styles.toastOk : styles.toastErr
          }`}
        >
          {toast.msg}
        </div>
      )}

      {unlockedBadge && (
        <div className={styles.achievementBadge}>
          <div className={styles.achievementBadgeIcon}>
            {ACHIEVEMENTS.find(a => a.id === unlockedBadge)?.icon ?? '🏆'}
          </div>
          <div className={styles.achievementBadgeText}>
            <span className={styles.achievementBadgeLabel}>成就解锁！</span>
            <span className={styles.achievementBadgeName}>
              {ACHIEVEMENTS.find(a => a.id === unlockedBadge)?.name ?? ''}
            </span>
          </div>
        </div>
      )}

      {showSavePanel && (
        <div className={styles.buildingModal}>
          <div className={styles.buildingModalContent}>
            <h3 className={styles.modalTitle}>💾 游戏存档</h3>
            <div style={{ display: 'flex', gap: 8, marginBottom: 14 }}>
              <input
                className={styles.input}
                value={saveSlotName}
                onChange={(e) => setSaveSlotName(e.target.value)}
                placeholder="存档名称..."
                style={{ flex: 1 }}
                onKeyDown={(e) => e.key === 'Enter' && handleGameSave()}
              />
              <button
                className={styles.btnPrimary}
                onClick={handleGameSave}
                disabled={saveLoading}
              >
                {saveLoading ? '...' : '保存'}
              </button>
            </div>

            <div style={{ marginBottom: 8 }}>
              <button
                className={styles.btnGhost}
                onClick={loadSaveSlots}
                style={{ fontSize: 12 }}
              >
                🔄 刷新存档列表
              </button>
            </div>

            {saveLoading && <div style={{ color: '#888', fontSize: 12 }}>加载中...</div>}

            <div style={{ display: 'flex', flexDirection: 'column', gap: 6, maxHeight: 200, overflow: 'auto' }}>
              {saveSlots.length === 0 && !saveLoading && (
                <div style={{ color: '#666', fontSize: 12 }}>暂无存档</div>
              )}
              {saveSlots.map((s: any) => (
                <div key={s.slot || s.name} style={{
                  display: 'flex', alignItems: 'center', justifyContent: 'space-between',
                  padding: '8px 12px', background: '#0a0a10', border: '1px solid #00FF0020', borderRadius: 6,
                }}>
                  <div>
                    <span style={{ color: '#00FF00', fontSize: 13 }}>{s.slot || s.name}</span>
                    {s.created_at && (
                      <span style={{ color: '#666', fontSize: 10, marginLeft: 8 }}>
                        {new Date(s.created_at).toLocaleString()}
                      </span>
                    )}
                  </div>
                  <div style={{ display: 'flex', gap: 4 }}>
                    <button
                      className={styles.btnGhost}
                      onClick={() => handleGameLoad(s.slot || s.name)}
                      style={{ fontSize: 11, padding: '2px 8px', color: '#00FF00' }}
                    >加载</button>
                    <button
                      className={styles.btnGhost}
                      onClick={() => handleGameDeleteSave(s.slot || s.name)}
                      style={{ fontSize: 11, padding: '2px 8px', color: '#FF0000' }}
                    >删除</button>
                  </div>
                </div>
              ))}
            </div>

            <div className={styles.modalActions}>
              <button
                className={styles.btnGhost}
                onClick={() => setShowSavePanel(false)}
              >
                关闭
              </button>
            </div>
          </div>
        </div>
      )}

      {showCombatPanel && combatState && (
        <div className={styles.buildingModal}>
          <div className={styles.buildingModalContent}>
            <h3 className={styles.modalTitle}>⚔️ 战斗</h3>

            <div style={{ display: 'flex', gap: 20, marginBottom: 16 }}>
              <div style={{ flex: 1, textAlign: 'center' }}>
                <div style={{ fontSize: 28, marginBottom: 4 }}>🧙</div>
                <div style={{ color: '#00FF00', fontSize: 13, fontWeight: 600 }}>你</div>
                <div style={{ color: '#888', fontSize: 11 }}>
                  HP: <span style={{ color: '#00FF00' }}>{combatState.player?.hp}</span>/{combatState.player?.max_hp}
                </div>
                <div style={{ width: '100%', height: 6, background: '#222', borderRadius: 3, marginTop: 4 }}>
                  <div style={{
                    width: `${((combatState.player?.hp || 0) / (combatState.player?.max_hp || 1)) * 100}%`,
                    height: '100%', background: '#00FF00', borderRadius: 3, transition: 'width 0.3s',
                  }} />
                </div>
              </div>

              <div style={{ color: '#FF0000', fontSize: 24, alignSelf: 'center' }}>VS</div>

              <div style={{ flex: 1, textAlign: 'center' }}>
                <div style={{ fontSize: 28, marginBottom: 4 }}>
                  {combatState.enemy?.type === 'slime' ? '🟢' :
                   combatState.enemy?.type === 'goblin' ? '👹' :
                   combatState.enemy?.type === 'skeleton' ? '💀' :
                   combatState.enemy?.type === 'dragon' ? '🐉' : '👾'}
                </div>
                <div style={{ color: '#FF0000', fontSize: 13, fontWeight: 600 }}>
                  {combatState.enemy?.name || '敌人'}
                </div>
                <div style={{ color: '#888', fontSize: 11 }}>
                  HP: <span style={{ color: '#FF0000' }}>{combatState.enemy?.hp}</span>/{combatState.enemy?.max_hp}
                </div>
                <div style={{ width: '100%', height: 6, background: '#222', borderRadius: 3, marginTop: 4 }}>
                  <div style={{
                    width: `${((combatState.enemy?.hp || 0) / (combatState.enemy?.max_hp || 1)) * 100}%`,
                    height: '100%', background: '#FF0000', borderRadius: 3, transition: 'width 0.3s',
                  }} />
                </div>
              </div>
            </div>

            {combatState.last_action && (
              <div style={{
                padding: '8px 12px', background: '#0a0a10', border: '1px solid #00FF0020',
                borderRadius: 4, marginBottom: 14, fontSize: 12, color: '#00F0FF',
              }}>
                {combatState.last_action}
              </div>
            )}

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
              <button
                className={styles.btnPrimary}
                onClick={() => handleCombatAction('attack')}
                disabled={combatLoading || combatState.game_over}
                style={{ background: '#FF000030', borderColor: '#FF000060', color: '#FF0000' }}
              >
                ⚔️ 攻击
              </button>
              <button
                className={styles.btnPrimary}
                onClick={() => handleCombatAction('defend')}
                disabled={combatLoading || combatState.game_over}
                style={{ background: '#00F0FF20', borderColor: '#00F0FF60', color: '#00F0FF' }}
              >
                🛡️ 防御
              </button>
              <button
                className={styles.btnPrimary}
                onClick={() => handleCombatAction('skill')}
                disabled={combatLoading || combatState.game_over}
                style={{ background: '#B026FF20', borderColor: '#B026FF60', color: '#B026FF' }}
              >
                ✨ 技能
              </button>
              <button
                className={styles.btnPrimary}
                onClick={() => handleCombatAction('flee')}
                disabled={combatLoading || combatState.game_over}
                style={{ background: '#88888820', borderColor: '#88888860', color: '#888' }}
              >
                🏃 逃跑
              </button>
            </div>

            {combatState.game_over && (
              <div className={styles.modalActions}>
                <button
                  className={styles.btnGhost}
                  onClick={() => { setShowCombatPanel(false); setCombatState(null) }}
                >
                  关闭
                </button>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  )
}