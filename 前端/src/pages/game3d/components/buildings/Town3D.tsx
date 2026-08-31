/**
 * Town3D - 集镇铺面（town 类别）
 *
 * 文明等级 2（凡人后期），集镇店铺建筑。
 * 中式风格：悬山顶 + 木板门面 + 灯笼装饰
 *
 * 参考 05_3D场景设计.md §6.2 木屋/木板房材质参数
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react'
import {
  StoneBase,
  FourPillars,
  EavesRoof,
  Lantern,
  WindowLattice,
  DoorFrame,
  withBuildState,
  getMaterialProps,
  type PBRMaterialProps,
} from '../ChineseArchParts'
import type { GameBuildingStatus } from '@/types/game'

interface Town3DProps {
  position: [number, number, number]
  rotationY?: number
  status?: GameBuildingStatus
  buildProgress?: number
  level?: number
}

const SIZE: [number, number, number] = [2.2, 2.0, 2.2]

/** 木板墙材质（+ 木纹纹理） */
const PLANK_MAT: PBRMaterialProps = {
  color: '#a67c52',
  metalness: 0.0,
  roughness: 0.8,
  envMapIntensity: 0.5,
  textureType: 'wood',
}

/** 灰瓦屋顶材质（+ 灰瓦纹理） */
const GREY_TILE_MAT: PBRMaterialProps = {
  color: '#4a4a4a',
  metalness: 0.1,
  roughness: 0.75,
  envMapIntensity: 0.5,
  textureType: 'greyTile',
}

export function Town3D({
  position,
  rotationY = 0,
  status = 'completed',
  buildProgress = 100,
  level = 1,
}: Town3DProps) {
  const isBuilding = status === 'building'
  const opacity = isBuilding ? 0.4 : 1.0
  const scaleY = isBuilding ? Math.max(0.1, buildProgress / 100) : 1.0
  const levelScale = 1 + (level - 1) * 0.1

  const [w, h, d] = SIZE
  const plankMat = useMemo(() => getMaterialProps(withBuildState(PLANK_MAT, isBuilding)), [isBuilding])

  return (
    <group position={position} rotation={[0, rotationY, 0]} scale={[levelScale, levelScale, levelScale]}>
      <group scale={[1, scaleY, 1]}>
        {/* 石制基座 */}
        <StoneBase width={w} depth={d} height={0.25} transparent={isBuilding} opacity={opacity} />

        {/* 主体：木板墙 */}
        <mesh position={[0, 0.25 + (h * 0.55) / 2, 0]} castShadow receiveShadow>
          <boxGeometry args={[w, h * 0.55, d]} />
          <meshStandardMaterial {...plankMat} transparent={isBuilding} opacity={opacity} />
        </mesh>

        {/* 四立柱（正面+背面各显两根） */}
        <group position={[0, 0.25, 0]}>
          <FourPillars width={w} depth={d} height={h * 0.55} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 木板门（正面，双开门+门环） */}
        <group position={[0, 0.25 + h * 0.15, d / 2 + 0.01]}>
          <DoorFrame width={w * 0.4} height={h * 0.4} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 窗棂（两侧各一扇） */}
        <group position={[w * 0.3, 0.25 + h * 0.32, d / 2 + 0.01]}>
          <WindowLattice width={0.3} height={0.25} transparent={isBuilding} opacity={opacity} />
        </group>
        <group position={[-w * 0.3, 0.25 + h * 0.32, d / 2 + 0.01]}>
          <WindowLattice width={0.3} height={0.25} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 悬山顶飞檐屋顶 */}
        <group position={[0, 0.25 + h * 0.55, 0]}>
          <EavesRoof
            width={w * 1.15}
            depth={d * 1.15}
            height={h * 0.4}
            material={GREY_TILE_MAT}
            transparent={isBuilding}
            opacity={opacity}
          />
        </group>

        {/* 灯笼装饰（两侧悬挂） */}
        <group position={[w * 0.5, 0.25 + h * 0.5, d * 0.4]}>
          <Lantern scale={0.6} transparent={isBuilding} opacity={opacity} />
        </group>
        <group position={[-w * 0.5, 0.25 + h * 0.5, d * 0.4]}>
          <Lantern scale={0.6} transparent={isBuilding} opacity={opacity} />
        </group>

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
