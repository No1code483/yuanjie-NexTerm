/**
 * House3D - 凡人茅屋（house 类别）
 *
 * 文明等级 1（凡人阶段），最基础的建筑。
 * 中式风格：双坡茅草顶 + 木骨泥墙 + 小窗
 *
 * 参考 05_3D场景设计.md §6.2 原始茅屋/木屋材质参数
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react'
import {
  StoneBase,
  WindowLattice,
  DoorFrame,
  withBuildState,
  getMaterialProps,
  type PBRMaterialProps,
} from '../ChineseArchParts'
import type { GameBuildingStatus } from '@/types/game'

interface House3DProps {
  position: [number, number, number]
  rotationY?: number
  status?: GameBuildingStatus
  buildProgress?: number
  level?: number
}

/** 凡人茅屋尺寸（宽×高×深） */
const SIZE: [number, number, number] = [1.8, 1.4, 1.8]

/** 墙体材质（土黄泥墙 + 夯土纹理） */
const WALL_MAT: PBRMaterialProps = {
  color: '#8b6f47',
  metalness: 0.0,
  roughness: 0.85,
  envMapIntensity: 0.4,
  textureType: 'mud',
}

/** 茅草屋顶材质（褐黄色 + 茅草纹理） */
const THATCH_MAT: PBRMaterialProps = {
  color: '#a08040',
  metalness: 0.0,
  roughness: 0.95,
  envMapIntensity: 0.3,
  textureType: 'thatch',
}

export function House3D({
  position,
  rotationY = 0,
  status = 'completed',
  buildProgress = 100,
  level = 1,
}: House3DProps) {
  const isBuilding = status === 'building'
  const opacity = isBuilding ? 0.4 : 1.0
  const scaleY = isBuilding ? Math.max(0.1, buildProgress / 100) : 1.0
  const levelScale = 1 + (level - 1) * 0.1

  const [w, h, d] = SIZE
  const wallMat = useMemo(() => getMaterialProps(withBuildState(WALL_MAT, isBuilding)), [isBuilding])
  const thatchMat = useMemo(() => getMaterialProps(withBuildState(THATCH_MAT, isBuilding)), [isBuilding])

  return (
    <group position={position} rotation={[0, rotationY, 0]} scale={[levelScale, levelScale, levelScale]}>
      <group scale={[1, scaleY, 1]}>
        {/* 石制基座（低矮） */}
        <StoneBase width={w} depth={d} height={0.2} transparent={isBuilding} opacity={opacity} />

        {/* 主体墙身：木骨泥墙 */}
        <mesh position={[0, 0.2 + (h * 0.6) / 2, 0]} castShadow receiveShadow>
          <boxGeometry args={[w, h * 0.6, d]} />
          <meshStandardMaterial {...wallMat} transparent={isBuilding} opacity={opacity} />
        </mesh>

        {/* 双坡屋顶：用三角形棱柱模拟茅草顶 */}
        <group position={[0, 0.2 + h * 0.6, 0]}>
          {/* 主屋顶（金字塔形，4 面） */}
          <mesh position={[0, h * 0.2, 0]} castShadow>
            <coneGeometry args={[w * 0.75, h * 0.4, 4]} />
            <meshStandardMaterial {...thatchMat} transparent={isBuilding} opacity={opacity} />
          </mesh>
          {/* 屋脊延长（前后坡延伸感） */}
          <mesh position={[0, h * 0.05, d * 0.5]} rotation={[Math.PI / 2, 0, 0]} castShadow>
            <boxGeometry args={[w * 1.1, 0.05, d * 0.2]} />
            <meshStandardMaterial {...thatchMat} transparent={isBuilding} opacity={opacity} />
          </mesh>
        </group>

        {/* 十字格窗棂（正面） */}
        <group position={[0, 0.2 + h * 0.35, d / 2 + 0.01]}>
          <WindowLattice width={0.35} height={0.3} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 双开门（正面） */}
        <group position={[0, 0.2 + h * 0.15, d / 2 + 0.02]}>
          <DoorFrame width={0.45} height={0.55} transparent={isBuilding} opacity={opacity} />
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
