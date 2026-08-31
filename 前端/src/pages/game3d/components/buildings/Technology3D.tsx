/**
 * Technology3D - 炼丹坊（technology 类别）
 *
 * 文明等级 6（修仙中期），炼丹/炼器工坊。
 * 中式修仙风格：八卦炉 + 烟囱 + 小飞檐 + 火焰光晕
 *
 * 参考 05_3D场景设计.md §6.2 科研实验室材质参数（修仙版改造）
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react'
import {
  StoneBase,
  EavesRoof,
  DoorFrame,
  GREY_TILE_MAT,
  withBuildState,
  getMaterialProps,
  type PBRMaterialProps,
} from '../ChineseArchParts'
import type { GameBuildingStatus } from '@/types/game'

interface Technology3DProps {
  position: [number, number, number]
  rotationY?: number
  status?: GameBuildingStatus
  buildProgress?: number
  level?: number
}

const SIZE: [number, number, number] = [2.8, 3.2, 2.8]

/** 砖石墙体（+ 砖墙纹理） */
const FURNACE_MAT: PBRMaterialProps = {
  color: '#5a4030',
  metalness: 0.2,
  roughness: 0.7,
  envMapIntensity: 0.6,
  textureType: 'brick',
}

/** 铜质炉鼎材质 */
const BRONZE_MAT: PBRMaterialProps = {
  color: '#8c6d3f',
  metalness: 0.7,
  roughness: 0.35,
  envMapIntensity: 1.0,
}

export function Technology3D({
  position,
  rotationY = 0,
  status = 'completed',
  buildProgress = 100,
  level = 1,
}: Technology3DProps) {
  const isBuilding = status === 'building'
  const opacity = isBuilding ? 0.4 : 1.0
  const scaleY = isBuilding ? Math.max(0.1, buildProgress / 100) : 1.0
  const levelScale = 1 + (level - 1) * 0.1

  const [w, h, d] = SIZE
  const baseH = 0.3
  const wallH = h * 0.5
  const roofH = h * 0.3
  const furnaceH = h * 0.2  // 烟囱+炉顶
  const furnaceMat = useMemo(() => getMaterialProps(withBuildState(FURNACE_MAT, isBuilding)), [isBuilding])
  const bronzeMat = useMemo(() => withBuildState(BRONZE_MAT, isBuilding), [isBuilding])

  return (
    <group position={position} rotation={[0, rotationY, 0]} scale={[levelScale, levelScale, levelScale]}>
      <group scale={[1, scaleY, 1]}>
        {/* 石制基座 */}
        <StoneBase width={w} depth={d} height={baseH} transparent={isBuilding} opacity={opacity} />

        {/* 主体：耐火砖墙 */}
        <mesh position={[0, baseH + wallH / 2, 0]} castShadow receiveShadow>
          <boxGeometry args={[w, wallH, d]} />
          <meshStandardMaterial {...furnaceMat} transparent={isBuilding} opacity={opacity} />
        </mesh>

        {/* 正面大门（炼丹坊牌匾门） */}
        <group position={[0, baseH + wallH * 0.15, d / 2 + 0.01]}>
          <DoorFrame width={w * 0.35} height={wallH * 0.7} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 小飞檐屋顶（灰瓦） */}
        <group position={[0, baseH + wallH, 0]}>
          <EavesRoof
            width={w * 1.1}
            depth={d * 1.1}
            height={roofH}
            material={GREY_TILE_MAT}
            transparent={isBuilding}
            opacity={opacity}
          />
        </group>

        {/* 中央八卦炉（屋顶上的炼丹炉） */}
        <group position={[0, baseH + wallH + roofH * 0.4, 0]}>
          {/* 炉身（圆柱） */}
          <mesh position={[0, furnaceH * 0.3, 0]} castShadow>
            <cylinderGeometry args={[0.4, 0.5, furnaceH * 0.6, 16]} />
            <meshStandardMaterial {...bronzeMat} transparent={isBuilding} opacity={opacity} />
          </mesh>
          {/* 炉口（小球，象征丹炉口） */}
          <mesh position={[0, furnaceH * 0.65, 0]} castShadow>
            <sphereGeometry args={[0.35, 16, 12]} />
            <meshStandardMaterial {...bronzeMat} transparent={isBuilding} opacity={opacity} />
          </mesh>
          {/* 烟囱/丹气（细长圆柱） */}
          <mesh position={[0, furnaceH * 1.2, 0]} castShadow>
            <cylinderGeometry args={[0.08, 0.12, furnaceH * 0.8, 8]} />
            <meshStandardMaterial {...bronzeMat} transparent={isBuilding} opacity={opacity} />
          </mesh>
        </group>

        {/* 火焰光晕（丹炉口发光，仅建造完成时显示） */}
        {!isBuilding && (
          <mesh position={[0, baseH + wallH + roofH * 0.4 + furnaceH * 0.65, 0]}>
            <sphereGeometry args={[0.25, 12, 8]} />
            <meshStandardMaterial
              color="#ff6b35"
              emissive="#ff4500"
              emissiveIntensity={0.8}
              transparent
              opacity={0.6}
              depthWrite={false}
            />
          </mesh>
        )}

        {/* 建造中：脚手架 */}
        {isBuilding && (
          <mesh position={[0, h / 2, 0]}>
            <boxGeometry args={[w + 0.3, h + 0.3, d + 0.3]} />
            <meshBasicMaterial color="#fbbf24" wireframe transparent opacity={0.3} />
          </mesh>
        )}
      </group>
    </group>
  )
}
