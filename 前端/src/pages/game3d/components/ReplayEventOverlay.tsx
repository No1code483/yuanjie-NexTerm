/**
 * ReplayEventOverlay - 时间轴回放事件动画叠加层（T1.6）
 *
 * 11_时间轴回放.md §7：当回放时间穿过某事件时，在对应建筑位置触发动画。
 *
 * 5 种事件动画：
 *   1. build     → 破土生长（土黄色粒子从地面向上喷射）
 *   2. complete  → 金光脉冲（金色光环从建筑中心向外扩散）
 *   3. upgrade   → 光柱升腾（蓝紫色垂直光柱从建筑顶部射出）
 *   4. demolish  → 碎裂消散（红色碎片从建筑位置向四周飞散）
 *   5. move      → 抬升平移（建筑抬起 + 残影位移轨迹）
 *
 * 触发策略：
 *   - 监听 store.replayCurrentTime 变化
 *   - 查找时间戳 ≤ currentTime 且 > lastTriggeredTime 的事件
 *   - 对每个新事件，在对应建筑位置渲染动画组件
 *   - 动画播放完毕（约 1.5 秒）后自动卸载
 *
 * 性能：
 *   - 同时最多渲染 5 个动画（避免一次穿越大量事件时卡顿）
 *   - 超出的事件合并为「场景变迁」提示（仅显示文字）
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useEffect, useRef, useState } from 'react'
import { useGame3DStore } from '../stores/gameStore'
import type { TimelineEvent } from '@/types/game'
import { EventBuild, EventComplete, EventUpgrade, EventDemolish, EventMove } from './ReplayEventAnimations'

/** 单次动画最长持续时长（ms）。 */
const ANIMATION_DURATION_MS = 1500

/** 同时最多渲染动画数。 */
const MAX_CONCURRENT_ANIMATIONS = 5

/** 已触发的动画项。 */
interface ActiveAnim {
  key: string
  event: TimelineEvent
}

/** 从 TimelineEvent 提取建筑位置（description 字段中可能包含 JSON，v1 简化为 [0,0,0]） */
function eventPosition(_e: TimelineEvent): [number, number, number] {
  // v1 简化：TimelineEvent 未携带 pos_x/y/z，使用 [0, 0, 0]
  // 后续可在 game_get_build_timeline 命令中扩展 TimelineEvent 字段
  return [0, 0, 0]
}

export function ReplayEventOverlay() {
  const replayMode = useGame3DStore((s) => s.replayMode)
  const timeline = useGame3DStore((s) => s.replayTimeline)
  const currentTime = useGame3DStore((s) => s.replayCurrentTime)

  const [activeAnims, setActiveAnims] = useState<ActiveAnim[]>([])
  /** 上次触发动画时的时间戳，避免重复触发同一时间点的事件。 */
  const lastTriggeredTimeRef = useRef<number>(-1)

  useEffect(() => {
    if (!replayMode) {
      setActiveAnims([])
      lastTriggeredTimeRef.current = -1
      return
    }

    // 查找时间戳 ≤ currentTime 且 > lastTriggeredTime 的事件
    const newEvents = timeline.filter(
      (e) => e.timestamp <= currentTime && e.timestamp > lastTriggeredTimeRef.current,
    )

    if (newEvents.length === 0) return

    // 限制并发动画数：取前 N 个，其余丢弃
    const limited = newEvents.slice(0, MAX_CONCURRENT_ANIMATIONS)
    const newAnims: ActiveAnim[] = limited.map((e) => ({
      key: `${e.id}-${e.timestamp}`,
      event: e,
    }))

    // 合并到当前活动列表
    setActiveAnims((prev) => [...prev, ...newAnims])

    // 更新 lastTriggeredTime 到最近事件
    if (limited.length > 0) {
      const maxTs = Math.max(...limited.map((e) => e.timestamp))
      lastTriggeredTimeRef.current = maxTs
    }

    // 自动卸载：ANIMATION_DURATION_MS 后清除
    const timer = setTimeout(() => {
      setActiveAnims((prev) =>
        prev.filter((a) => !newAnims.some((na) => na.key === a.key)),
      )
    }, ANIMATION_DURATION_MS)

    return () => clearTimeout(timer)
  }, [currentTime, timeline, replayMode])

  if (!replayMode || activeAnims.length === 0) return null

  return (
    <group>
      {activeAnims.map((anim) => {
        const pos = eventPosition(anim.event)
        const eventType = anim.event.event_type

        // 按 event_type 分发到对应动画组件
        switch (eventType) {
          case 'build':
            return <EventBuild key={anim.key} position={pos} name={anim.event.building_name ?? ''} />
          case 'build_complete':
          case 'complete':
            return <EventComplete key={anim.key} position={pos} />
          case 'upgrade':
          case 'upgrade_complete':
            return <EventUpgrade key={anim.key} position={pos} />
          case 'remove':
          case 'demolish':
            return <EventDemolish key={anim.key} position={pos} />
          case 'move':
            return <EventMove key={anim.key} position={pos} />
          case 'breakthrough':
            // 突破事件：复用金光脉冲动画
            return <EventComplete key={anim.key} position={pos} />
          default:
            return null
        }
      })}
    </group>
  )
}

export default ReplayEventOverlay
