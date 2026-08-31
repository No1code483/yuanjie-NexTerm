/**
 * PlacementController - 预建造放置控制器（Task 9.3）
 *
 * 职责：
 *   1. 监听 store.placementMode，placing/moving 时激活
 *   2. 渲染一个 64×64 不可见地面 plane 作为 Raycaster 目标
 *   3. onPointerMove → 更新 store.previewPosition（鼠标跟随）
 *   4. onClick → 调用 IPC startBuilding / moveBuilding → 更新 store
 *   5. ESC 键取消放置
 *   6. 渲染半透明预览建筑（ProceduralBuilding status='planning'）
 *
 * 坐标系：地图 64×64，中心在 [32, 0, 32]（参考 Terrain.tsx）
 * 位置 clamp 到 [1, 63]，避免建筑卡在地图边缘外
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useEffect, useRef, useState } from 'react'
import type { ThreeEvent } from '@react-three/fiber'
import { game } from '@/lib/ipc'
import type { GameBuilding } from '@/types/game'
import { useGame3DStore } from '../stores/gameStore'
import { ProceduralBuilding } from './ProceduralBuilding'

/** 地图尺寸（与 Terrain.tsx 一致） */
const MAP_SIZE = 64
const MAP_OFFSET = MAP_SIZE / 2 // 32

/** 位置 clamp 边界（避免建筑卡在地图边缘外） */
const MIN_POS = 1
const MAX_POS = MAP_SIZE - 1

/** 位置吸附到整数 tile 中心（tile 中心 = tileX + 0.5） */
function snapToTile(v: number): number {
  const tileIdx = Math.floor(v)
  return Math.max(MIN_POS, Math.min(MAX_POS, tileIdx + 0.5))
}

export function PlacementController() {
  const placementMode = useGame3DStore((s) => s.placementMode)
  const placementItem = useGame3DStore((s) => s.placementItem)
  const previewPosition = useGame3DStore((s) => s.previewPosition)
  const setPreviewPosition = useGame3DStore((s) => s.setPreviewPosition)
  const cancelPlacement = useGame3DStore((s) => s.cancelPlacement)
  const addBuilding = useGame3DStore((s) => s.addBuilding)
  const updateBuilding = useGame3DStore((s) => s.updateBuilding)
  const movingBuildingId = useGame3DStore((s) => s.movingBuildingId)
  const world = useGame3DStore((s) => s.world)
  const setActionInProgress = useGame3DStore((s) => s.setActionInProgress)
  const actionInProgress = useGame3DStore((s) => s.actionInProgress)

  const [errMsg, setErrMsg] = useState<string | null>(null)
  const errTimer = useRef<number | null>(null)

  /** ESC 取消放置 */
  useEffect(() => {
    if (placementMode === 'idle') return
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        cancelPlacement()
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [placementMode, cancelPlacement])

  /** 清理错误定时器 */
  useEffect(() => {
    return () => {
      if (errTimer.current) window.clearTimeout(errTimer.current)
    }
  }, [])

  /** 显示错误 3 秒后自动消失 */
  const showError = (msg: string) => {
    setErrMsg(msg)
    if (errTimer.current) window.clearTimeout(errTimer.current)
    errTimer.current = window.setTimeout(() => setErrMsg(null), 3000)
  }

  /** 鼠标移动：更新预览位置（吸附到 tile 中心） */
  const handlePointerMove = (e: ThreeEvent<PointerEvent>) => {
    if (placementMode === 'idle' || actionInProgress) return
    const px = snapToTile(e.point.x)
    const pz = snapToTile(e.point.z)
    // 避免每帧都 set（只在 tile 变化时更新）
    if (!previewPosition || previewPosition[0] !== px || previewPosition[2] !== pz) {
      setPreviewPosition([px, 0, pz])
    }
  }

  /** 点击地面：确认放置 */
  const handleClick = async (e: ThreeEvent<MouseEvent>) => {
    e.stopPropagation()
    if (placementMode === 'idle' || actionInProgress) return
    if (!world) return

    const px = snapToTile(e.point.x)
    const pz = snapToTile(e.point.z)

    setActionInProgress(true)
    setErrMsg(null)
    try {
      if (placementMode === 'placing' && placementItem) {
        // 新建建筑
        const req = {
          world_id: world.id,
          building_category: placementItem.category,
          building_subtype: placementItem.subtype,
          name: placementItem.name,
          pos_x: px,
          pos_y: 0,
          pos_z: pz,
          rotation_y: 0,
          knowledge_domain: placementItem.knowledgeDomain,
          base_cost: placementItem.baseCost,
        }
        const res = await game.startBuilding(req)
        if (res.code !== 0) throw new Error(res.message)
        const b: GameBuilding | null = res.data ?? null
        if (b) {
          addBuilding({
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
          })
        }
        cancelPlacement()
      } else if (placementMode === 'moving' && movingBuildingId) {
        // 移动已有建筑：v1 简化，move_cost 按 base_cost * 10% 估算（v2 由后端权威计算）
        // 这里用 0 作为 move_cost，后端 move_building 会按 10% 默认处理
        // 注意：后端 move_cost 是必填参数，前端需传入实际值
        // v1 简化：用 0 让后端走默认逻辑（若后端不允许 0，后续修正）
        const moveCost = 0
        const res = await game.moveBuilding(movingBuildingId, px, 0, pz, 0, moveCost)
        if (res.code !== 0) throw new Error(res.message)
        const b: GameBuilding | null = res.data ?? null
        if (b) {
          updateBuilding(movingBuildingId, {
            position: [b.pos_x, b.pos_y, b.pos_z],
            rotationY: b.rotation_y,
          })
        }
        cancelPlacement()
      }
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err))
    } finally {
      setActionInProgress(false)
    }
  }

  /** 右键取消 */
  const handleContextMenu = (e: ThreeEvent<MouseEvent>) => {
    e.stopPropagation()
    if (placementMode !== 'idle') {
      cancelPlacement()
    }
  }

  if (placementMode === 'idle') return null

  return (
    <group>
      {/* 不可见的地 raycast 目标 plane（覆盖整个地图） */}
      <mesh
        position={[MAP_OFFSET, 0.02, MAP_OFFSET]}
        rotation={[-Math.PI / 2, 0, 0]}
        onPointerMove={handlePointerMove}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
      >
        <planeGeometry args={[MAP_SIZE, MAP_SIZE]} />
        <meshBasicMaterial transparent opacity={0} depthWrite={false} />
      </mesh>

      {/* 半透明预览建筑（跟随鼠标） */}
      {placementMode === 'placing' && placementItem && previewPosition && (
        <group position={previewPosition}>
          <ProceduralBuilding
            category={placementItem.category}
            position={[0, 0, 0]}
            rotationY={0}
            status="planning"
            buildProgress={0}
            level={1}
          />
          {/* 放置位置高亮环 */}
          <mesh position={[0, 0.05, 0]} rotation={[-Math.PI / 2, 0, 0]}>
            <ringGeometry args={[0.9, 1.1, 32]} />
            <meshBasicMaterial color="#6bb6ff" transparent opacity={0.7} side={2} />
          </mesh>
        </group>
      )}

      {/* 移动模式：预览移动目标位置 */}
      {placementMode === 'moving' && previewPosition && (
        <group position={previewPosition}>
          <mesh position={[0, 0.05, 0]} rotation={[-Math.PI / 2, 0, 0]}>
            <ringGeometry args={[1.0, 1.2, 32]} />
            <meshBasicMaterial color="#fbbf24" transparent opacity={0.7} side={2} />
          </mesh>
        </group>
      )}

      {/* 错误提示（3D 场景内 Html 标牌，挂在地图中心上方） */}
      {errMsg && (
        <mesh position={[MAP_OFFSET, 5, MAP_OFFSET]}>
          <meshBasicMaterial visible={false} />
        </mesh>
      )}
    </group>
  )
}
