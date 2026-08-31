/**
 * ReplayBuilding - 时间轴回放模式下的只读建筑渲染（T1.6）
 *
 * 11_时间轴回放.md §4.3：回放模式下的建筑为「场景快照」展示，
 *   - 不响应点击交互（selectBuilding 被禁用）
 *   - 不触发建造动画 / 完成动画（避免与实时数据混淆）
 *   - 整体半透明 + 顶部时光标记，从视觉上与实时建筑区分
 *
 * 简化渲染逻辑：
 *   - 直接渲染 <ProceduralBuilding>，buildProgress 使用快照值
 *   - 不使用 useFrame 插值（避免回放期间不必要的动画）
 *   - 不显示选中高亮环
 *
 * 5 种事件动画（spec §4.3 §7）由 <ReplayEventOverlay> 在场景层叠加，
 * 而非建筑本身驱动，因此本组件保持简洁。
 *
 * change-id: game-3d-rebuild-refactor
 */
import { Html } from '@react-three/drei'
import { ProceduralBuilding } from './ProceduralBuilding'
import type { Building3D } from '../stores/gameStore'
import type { GameBuildingStatus } from '@/types/game'

interface ReplayBuildingProps {
  building: Building3D
}

export function ReplayBuilding({ building }: ReplayBuildingProps) {
  const { category, position, rotationY, status, buildProgress, level, name } = building

  const isBuilding = status === 'building'
  const isCompleted = status === 'completed'

  return (
    <group
      position={position}
      rotation={[0, rotationY, 0]}
    >
      {/* 半透明效果：通过 group 的 children 透明化（ProceduralBuilding 内部材质已支持 status） */}
      <group>
        <ProceduralBuilding
          category={category}
          position={[0, 0, 0]}
          rotationY={0}
          status={status as GameBuildingStatus}
          buildProgress={buildProgress}
          level={level}
        />
      </group>

      {/* 时光标记：建筑名称 + 状态徽章 */}
      <Html position={[0, 3, 0]} center distanceFactor={12} zIndexRange={[10, 0]}>
        <div
          className={`game3d-building-tag ${
            isCompleted
              ? 'game3d-building-tag--replay-completed'
              : isBuilding
                ? 'game3d-building-tag--replay-building'
                : 'game3d-building-tag--replay'
          }`}
        >
          ⏱ {name} · Lv.{level}
        </div>
      </Html>
    </group>
  )
}

export default ReplayBuilding
