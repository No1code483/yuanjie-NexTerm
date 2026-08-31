/**
 * City3D - 城楼（city 类别）
 *
 * 文明等级 3（凡人顶峰），带城墙的城门楼。
 * 中式风格：城墙带垛口 + 歇山顶 + 城门洞
 *
 * 参考 05_3D场景设计.md §6.2 石屋/砖石楼房材质参数
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react'
import {
  StoneBase,
  CityWall,
  FourPillars,
  EavesRoof,
  DoorFrame,
  WindowLattice,
  GLAZED_TILE_MATERIAL,
  withBuildState,
  getMaterialProps,
  type PBRMaterialProps,
} from '../ChineseArchParts'
import type { GameBuildingStatus } from '@/types/game'

interface City3DProps {
  position: [number, number, number]
  rotationY?: number
  status?: GameBuildingStatus
  buildProgress?: number
  level?: number
}

const SIZE: [number, number, number] = [2.6, 3.2, 2.6]

/** 砖石材质（+ 砖墙纹理） */
const BRICK_MAT: PBRMaterialProps = {
  color: '#7a6b5a',
  metalness: 0.05,
  roughness: 0.75,
  envMapIntensity: 0.5,
  textureType: 'brick',
}

export function City3D({
  position,
  rotationY = 0,
  status = 'completed',
  buildProgress = 100,
  level = 1,
}: City3DProps) {
  const isBuilding = status === 'building'
  const opacity = isBuilding ? 0.4 : 1.0
  const scaleY = isBuilding ? Math.max(0.1, buildProgress / 100) : 1.0
  const levelScale = 1 + (level - 1) * 0.1

  const [w, h, d] = SIZE
  const wallH = h * 0.55
  const towerH = h * 0.45
  const brickMat = useMemo(() => getMaterialProps(withBuildState(BRICK_MAT, isBuilding)), [isBuilding])

  return (
    <group position={position} rotation={[0, rotationY, 0]} scale={[levelScale, levelScale, levelScale]}>
      <group scale={[1, scaleY, 1]}>
        {/* 城墙基础（大方形基座） */}
        <StoneBase width={w} depth={d} height={0.3} transparent={isBuilding} opacity={opacity} />

        {/* 城墙四面（带垛口） */}
        <group position={[0, 0.3, 0]}>
          {/* 前墙（带城门洞，用稍矮高度 + 留缺口） */}
          <CityWall width={w} height={wallH * 0.7} depth={0.3} battlementCount={6} transparent={isBuilding} opacity={opacity} />
          {/* 后墙 */}
          <group position={[0, 0, -d]} rotation={[0, Math.PI, 0]}>
            <CityWall width={w} height={wallH * 0.7} depth={0.3} battlementCount={6} transparent={isBuilding} opacity={opacity} />
          </group>
          {/* 左墙 */}
          <group position={[-w / 2, 0, -d / 2]} rotation={[0, Math.PI / 2, 0]}>
            <CityWall width={d} height={wallH * 0.7} depth={0.3} battlementCount={6} transparent={isBuilding} opacity={opacity} />
          </group>
          {/* 右墙 */}
          <group position={[w / 2, 0, -d / 2]} rotation={[0, -Math.PI / 2, 0]}>
            <CityWall width={d} height={wallH * 0.7} depth={0.3} battlementCount={6} transparent={isBuilding} opacity={opacity} />
          </group>
        </group>

        {/* 城门洞（双开门+门环） */}
        <group position={[0, 0.3 + wallH * 0.1, w / 2 - 0.02]}>
          <DoorFrame width={w * 0.35} height={wallH * 0.7} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 城楼主体（城墙上的楼阁） */}
        <group position={[0, 0.3 + wallH * 0.7, 0]}>
          {/* 楼阁墙体 */}
          <mesh position={[0, towerH * 0.3, 0]} castShadow receiveShadow>
            <boxGeometry args={[w * 0.85, towerH * 0.6, d * 0.85]} />
            <meshStandardMaterial {...brickMat} transparent={isBuilding} opacity={opacity} />
          </mesh>

          {/* 城楼窗棂（正面两侧） */}
          <group position={[w * 0.25, 0.3 + wallH * 0.7 + towerH * 0.3, d * 0.85 / 2 + 0.01]}>
            <WindowLattice width={0.3} height={0.25} transparent={isBuilding} opacity={opacity} />
          </group>
          <group position={[-w * 0.25, 0.3 + wallH * 0.7 + towerH * 0.3, d * 0.85 / 2 + 0.01]}>
            <WindowLattice width={0.3} height={0.25} transparent={isBuilding} opacity={opacity} />
          </group>

          {/* 楼阁四立柱 */}
          <group position={[0, 0, 0]}>
            <FourPillars width={w * 0.85} depth={d * 0.85} height={towerH * 0.6} transparent={isBuilding} opacity={opacity} />
          </group>

          {/* 歇山顶飞檐屋顶 */}
          <group position={[0, towerH * 0.6, 0]}>
            <EavesRoof
              width={w * 0.95}
              depth={d * 0.95}
              height={towerH * 0.5}
              material={GLAZED_TILE_MATERIAL}
              transparent={isBuilding}
              opacity={opacity}
            />
          </group>
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
