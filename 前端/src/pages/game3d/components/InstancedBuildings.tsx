/**
 * InstancedBuildings - 高密度场景实例化渲染（T4.1，05_3D场景设计.md §10.2）
 *
 * 触发条件：Buildings.tsx 检测 store.buildings.length ≥ 100 时启用。
 *
 * 渲染策略：
 *   - 仅对 `completed` 状态建筑实例化（building/planning 数量少，仍走 <Building>）
 *   - 按 category 分组，每组用两个 <Instances>（drei）：
 *       1. box 主体（共享 geometry + material）
 *       2. cone 屋顶（共享 geometry + material）
 *   - 不同 category 用不同颜色 + 不同尺寸，保持视觉识别
 *
 * 交互：
 *   - <Instance> 的 onClick 事件透传 building.id → selectBuilding
 *   - 选中建筑通过 scale + color 高亮
 *
 * 性能收益（100 建筑均匀分布到 8 个 category）：
 *   - 原渲染：100 buildings × 8 mesh/building ≈ 800 drawcall
 *   - 实例化：8 category × 2 Instances = 16 drawcall
 *   - 收益：~50× drawcall 减少
 *
 * 注意：drei <Instances> 容器内只能包含 <Instance> 子节点，故 cone 屋顶必须
 *      独立成另一个 <Instances>，不能嵌入主体 Instances 内部。
 *
 * change-id: game-3d-rebuild-refactor
 */
import { Instances, Instance } from '@react-three/drei'
import { useMemo } from 'react'
import type { ThreeEvent } from '@react-three/fiber'
import type { GameBuildingCategory } from '@/types/game'
import type { Building3D } from '../stores/gameStore'
import { useGame3DStore } from '../stores/gameStore'
import { getTexturePair, type TextureType } from '../utils/TextureGenerator'

/** Category 视觉参数：颜色 + 包围盒尺寸 [w, h, d] + 顶 cone 高度 + 纹理类型 */
const CATEGORY_VISUALS: Record<
  GameBuildingCategory,
  { color: string; size: [number, number, number]; coneHeight: number; textureType: TextureType }
> = {
  house: { color: '#8b6f47', size: [2.0, 1.6, 2.0], coneHeight: 0.8, textureType: 'mud' },
  town: { color: '#a67c52', size: [3.0, 2.5, 3.0], coneHeight: 1.0, textureType: 'wood' },
  city: { color: '#6b7d8c', size: [4.0, 3.5, 4.0], coneHeight: 1.2, textureType: 'stone' },
  kingdom: { color: '#8b5a3c', size: [5.0, 4.5, 5.0], coneHeight: 1.5, textureType: 'brick' },
  palace: { color: '#c084fc', size: [4.5, 4.0, 4.5], coneHeight: 1.3, textureType: 'glazedTile' },
  technology: { color: '#4a9eff', size: [2.5, 2.2, 2.5], coneHeight: 1.0, textureType: 'brick' },
  sect: { color: '#9aa5b8', size: [3.5, 3.0, 3.5], coneHeight: 1.2, textureType: 'wood' },
  immortal: { color: '#ffd700', size: [4.0, 4.5, 4.0], coneHeight: 2.0, textureType: 'glazedTile' },
}

/** 选中高亮色 */
const SELECTED_COLOR = '#fbbf24'

/** 单 category 实例化组（含纹理材质） */
function CategoryInstances({
  category,
  items,
  selectedBuildingId,
  onInstanceClick,
}: {
  category: GameBuildingCategory
  items: Building3D[]
  selectedBuildingId: string | null
  onInstanceClick: (id: string, e: ThreeEvent<MouseEvent>) => void
}) {
  const visual = CATEGORY_VISUALS[category]
  const [w, h, d] = visual.size
  const coneRadius = Math.max(w, d) * 0.7

  // 为当前 category 获取纹理
  const textures = useMemo(() => getTexturePair(visual.textureType), [visual.textureType])

  return (
    <group>
      {/* === 主体 box 实例化（带纹理） === */}
      <Instances limit={items.length} range={items.length} castShadow receiveShadow>
        <boxGeometry args={[w, h, d]} />
        <meshStandardMaterial
          color={visual.color}
          roughness={0.7}
          metalness={0.1}
          map={textures.map}
          roughnessMap={textures.roughnessMap}
        />
        {items.map((b) => {
          const isSelected = b.id === selectedBuildingId
          return (
            <Instance
              key={b.id}
              position={[b.position[0], h / 2, b.position[2]]}
              rotation={[0, b.rotationY, 0]}
              scale={isSelected ? 1.1 : 1}
              color={isSelected ? SELECTED_COLOR : visual.color}
              onClick={(e: ThreeEvent<MouseEvent>) => onInstanceClick(b.id, e)}
              onPointerOver={(e: ThreeEvent<PointerEvent>) => {
                e.stopPropagation()
                document.body.style.cursor = 'pointer'
              }}
              onPointerOut={() => {
                document.body.style.cursor = 'default'
              }}
            />
          )
        })}
      </Instances>

      {/* === 屋顶 cone 实例化（独立 Instances，与主体共享 transform） === */}
      <Instances limit={items.length} range={items.length} castShadow>
        <coneGeometry args={[coneRadius, visual.coneHeight, 4]} />
        <meshStandardMaterial
          color={visual.color}
          roughness={0.7}
          map={textures.map}
          roughnessMap={textures.roughnessMap}
        />
        {items.map((b) => (
          <Instance
            key={`cone-${b.id}`}
            position={[
              b.position[0],
              h + visual.coneHeight / 2,
              b.position[2],
            ]}
            rotation={[0, b.rotationY + Math.PI / 4, 0]}
          />
        ))}
      </Instances>
    </group>
  )
}

interface InstancedBuildingsProps {
  /** 仅 completed 状态建筑（调用方已过滤） */
  buildings: Building3D[]
}

/**
 * 按 category 分组渲染实例化建筑。
 * 每 category 用两个 <Instances>（box + cone），整体 drawcall 固定 16。
 */
export function InstancedBuildings({ buildings }: InstancedBuildingsProps) {
  const selectedBuildingId = useGame3DStore((s) => s.selectedBuildingId)
  const selectBuilding = useGame3DStore((s) => s.selectBuilding)
  const placementMode = useGame3DStore((s) => s.placementMode)
  const actionInProgress = useGame3DStore((s) => s.actionInProgress)

  // 按 category 分组
  const groups = new Map<GameBuildingCategory, Building3D[]>()
  for (const b of buildings) {
    if (!groups.has(b.category)) groups.set(b.category, [])
    groups.get(b.category)!.push(b)
  }

  /** 点击单个实例：选中（再次点击取消） */
  const handleClick = (buildingId: string, e: ThreeEvent<MouseEvent>) => {
    e.stopPropagation()
    if (placementMode !== 'idle' || actionInProgress) return
    selectBuilding(selectedBuildingId === buildingId ? null : buildingId)
  }

  return (
    <group>
      {Array.from(groups.entries()).map(([category, items]) => (
        <CategoryInstances
          key={category}
          category={category}
          items={items}
          selectedBuildingId={selectedBuildingId}
          onInstanceClick={handleClick}
        />
      ))}
    </group>
  )
}
