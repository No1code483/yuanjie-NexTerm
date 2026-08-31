/**
 * SimpleBuilding - 远距离 LOD 简化建筑（T4.2，05_3D场景设计.md §10.3）
 *
 * 用于 `<Detailed distances={[0, 30]}>` 的 LOD1 级别。
 * 当相机距离建筑 > 30 单位时，由 Building.tsx 切换到此简化版。
 *
 * 设计：每 category 一个 box + cone 屋顶，颜色与 InstancedBuildings 对齐，
 * 保持视觉一致性（远观时与高密度场景的实例化建筑风格统一）。
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react'
import type { GameBuildingCategory, GameBuildingStatus } from '@/types/game'
import { getTexturePair, type TextureType } from '../utils/TextureGenerator'

/** 与 InstancedBuildings.CATEGORY_VISUALS 对齐 */
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

interface SimpleBuildingProps {
  category: GameBuildingCategory
  status?: GameBuildingStatus
  /** 建造进度 0~100（仅 status='building' 时影响透明度） */
  buildProgress?: number
}

/**
 * 简化建筑：box + cone 屋顶。
 * 不含光照计算外的装饰，几何体顶点数 ~24（vs ProceduralBuilding 数百）。
 */
export function SimpleBuilding({
  category,
  status = 'completed',
  buildProgress = 100,
}: SimpleBuildingProps) {
  const visual = CATEGORY_VISUALS[category]
  const [w, h, d] = visual.size
  // 建造中：随进度升高 box 高度（模拟生长）
  const progressScale = status === 'building' ? Math.max(0.1, buildProgress / 100) : 1
  const actualH = h * progressScale

  // 获取纹理
  const textures = useMemo(() => getTexturePair(visual.textureType), [visual.textureType])

  return (
    <group>
      {/* 主体 box */}
      <mesh position={[0, actualH / 2, 0]} castShadow receiveShadow>
        <boxGeometry args={[w, actualH, d]} />
        <meshStandardMaterial
          color={visual.color}
          roughness={0.7}
          metalness={0.1}
          map={textures.map}
          roughnessMap={textures.roughnessMap}
        />
      </mesh>
      {/* 屋顶 cone（仅进度 > 50% 时显示） */}
      {progressScale > 0.5 && (
        <mesh
          position={[0, actualH + visual.coneHeight / 2, 0]}
          rotation={[0, Math.PI / 4, 0]}
          castShadow
        >
          <coneGeometry args={[Math.max(w, d) * 0.7, visual.coneHeight, 4]} />
          <meshStandardMaterial
            color={visual.color}
            roughness={0.7}
            map={textures.map}
            roughnessMap={textures.roughnessMap}
          />
        </mesh>
      )}
    </group>
  )
}
