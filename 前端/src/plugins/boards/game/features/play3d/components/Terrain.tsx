/**
 * Terrain - 地形系统 v2 升级：tile 差异化渲染 + 微起伏
 *
 * 参考 05_3D场景设计.md 第8节 地形系统：
 *   - 初始 32x32 网格，每 tile 2x2 单位，总尺寸 64x64
 *   - 5 种地形材质：plain/water/hill/forest/desert
 *   - 微起伏：plain +-0.05、hill +0.3~0.8、water -0.2、desert +-0.15
 *
 * v2 升级：
 *   - 按 world.tiles 数据渲染独立 tile mesh（替代单 plane）
 *   - 每种地形类型独立颜色 + 微起伏高度
 *   - 保留 Grid 网格线可视化
 *   - 无 world 数据时回退到单 plane
 *
 * 性能优化：
 *   - tile 网格 (32x32=1024) 个 mesh 在没数据时回退为 1 个 plane
 *   - 有数据时使用 useMemo 分组减少重复计算
 *
 * change-id: game-3d-rebuild-refactor-modeling-opt
 */
import { useMemo } from 'react'
import { Grid } from '@react-three/drei'
import { useGame3DStore, type TerrainType } from '../stores/gameStore'

/** 5 种地形材质参数 */
const TERRAIN_MATERIALS: Record<
  TerrainType,
  { color: string; roughness: number; metalness: number }
> = {
  plain: { color: '#4a6b3a', roughness: 0.9, metalness: 0.0 },
  water: { color: '#2d5a87', roughness: 0.2, metalness: 0.1 },
  hill: { color: '#6b5a3a', roughness: 0.95, metalness: 0.0 },
  forest: { color: '#2d4a1f', roughness: 1.0, metalness: 0.0 },
  desert: { color: '#c4a35a', roughness: 1.0, metalness: 0.0 },
}

/** 各地形微起伏高度范围 */
const TERRAIN_HEIGHT_OFFSET: Record<TerrainType, number> = {
  plain: 0.02,
  water: -0.15,
  hill: 0.4,
  forest: 0.05,
  desert: 0.03,
}

/** 地图尺寸常量（32x32 tile，每 tile 2 单位，总 64x64） */
const MAP_TILES = 32
const TILE_SIZE = 2
const MAP_SIZE = MAP_TILES * TILE_SIZE
const MAP_OFFSET = MAP_SIZE / 2

/**
 * 地形组件
 * - v2：按 world.tiles 渲染独立 tile mesh（差异化 + 微起伏）
 * - 无 world 数据时：回退单 plane + Grid
 */
export function Terrain() {
  const world = useGame3DStore((s) => s.world)

  if (!world || !world.tiles || world.tiles.length === 0) {
    return (
      <group>
        <mesh
          receiveShadow
          rotation={[-Math.PI / 2, 0, 0]}
          position={[MAP_OFFSET, 0, MAP_OFFSET]}
        >
          <planeGeometry args={[MAP_SIZE, MAP_SIZE]} />
          <meshStandardMaterial {...TERRAIN_MATERIALS.plain} />
        </mesh>

        <Grid
          position={[MAP_OFFSET, 0.01, MAP_OFFSET]}
          args={[MAP_SIZE, MAP_SIZE]}
          cellSize={TILE_SIZE}
          cellColor="#2d4a1f"
          sectionSize={TILE_SIZE * 4}
          sectionColor="#4a6b3a"
          fadeDistance={80}
          fadeStrength={1}
          infiniteGrid={false}
          followCamera={false}
        />
      </group>
    )
  }

  const tileGroups = useMemo(() => {
    const groups: Record<TerrainType, { x: number; z: number; height: number }[]> = {
      plain: [],
      water: [],
      hill: [],
      forest: [],
      desert: [],
    }
    for (const tile of world.tiles) {
      groups[tile.type].push({ x: tile.x, z: tile.z, height: tile.height })
    }
    return groups
  }, [world.tiles])

  return (
    <group>
      {(Object.entries(tileGroups) as [TerrainType, { x: number; z: number; height: number }[]][]).map(
        ([type, tiles]) => {
          if (tiles.length === 0) return null
          const mat = TERRAIN_MATERIALS[type]
          const baseOffset = TERRAIN_HEIGHT_OFFSET[type]

          return tiles.map((tile) => {
            const seed = (tile.x * 7 + tile.z * 13) % 100
            const microRelief = (seed / 100 - 0.5) * baseOffset * 0.5
            const yOffset = tile.height > 0 ? tile.height : baseOffset + microRelief

            return (
              <mesh
                key={`tile-${tile.x}-${tile.z}`}
                position={[
                  tile.x * TILE_SIZE + TILE_SIZE / 2,
                  yOffset,
                  tile.z * TILE_SIZE + TILE_SIZE / 2,
                ]}
                rotation={[-Math.PI / 2, 0, 0]}
                receiveShadow
              >
                <planeGeometry args={[TILE_SIZE, TILE_SIZE]} />
                <meshStandardMaterial {...mat} />
              </mesh>
            )
          })
        },
      )}

      <Grid
        position={[MAP_OFFSET, 0.02, MAP_OFFSET]}
        args={[MAP_SIZE, MAP_SIZE]}
        cellSize={TILE_SIZE}
        cellColor="#2d4a1f"
        sectionSize={TILE_SIZE * 4}
        sectionColor="#4a6b3a"
        fadeDistance={80}
        fadeStrength={1}
        infiniteGrid={false}
        followCamera={false}
      />
    </group>
  )
}