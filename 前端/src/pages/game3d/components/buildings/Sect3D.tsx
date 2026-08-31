/**
 * Sect3D - 宗门山门（sect 类别）
 *
 * 文明等级 7（修仙后期），修仙宗派的山门牌楼。
 * 中式风格：三间四柱牌楼 + 飞檐翘角 + 斗拱 + 石阶
 *
 * 参考 05_3D场景设计.md §6.2 修仙楼阁材质参数
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react'
import {
  StoneBase,
  Pillar,
  Dougong,
  EavesRoof,
  Lantern,
  Steps,
  DoorFrame,
  GLAZED_TILE_MATERIAL,
  PILLAR_MATERIAL,
  withBuildState,
  getMaterialProps,
  type PBRMaterialProps,
} from '../ChineseArchParts'
import type { GameBuildingStatus } from '@/types/game'

interface Sect3DProps {
  position: [number, number, number]
  rotationY?: number
  status?: GameBuildingStatus
  buildProgress?: number
  level?: number
}

const SIZE: [number, number, number] = [3.4, 3.6, 1.2]

/** 牌楼木构材质（+ 木纹纹理） */
const GATE_WOOD_MAT: PBRMaterialProps = {
  color: '#5a2010',
  metalness: 0.1,
  roughness: 0.6,
  envMapIntensity: 0.8,
  textureType: 'wood',
}

/** 牌楼匾额材质（金色字底） */
const PLAQUE_MAT: PBRMaterialProps = {
  color: '#1a1a1a',
  metalness: 0.3,
  roughness: 0.4,
  envMapIntensity: 1.0,
}

export function Sect3D({
  position,
  rotationY = 0,
  status = 'completed',
  buildProgress = 100,
  level = 1,
}: Sect3DProps) {
  const isBuilding = status === 'building'
  const opacity = isBuilding ? 0.4 : 1.0
  const scaleY = isBuilding ? Math.max(0.1, buildProgress / 100) : 1.0
  const levelScale = 1 + (level - 1) * 0.1

  const [w, h, d] = SIZE
  const baseH = 0.4
  const pillarH = h * 0.65
  const beamH = h * 0.1   // 横梁高度
  const roofH = h * 0.25  // 屋顶高度
  const gateMat = useMemo(() => getMaterialProps(withBuildState(GATE_WOOD_MAT, isBuilding)), [isBuilding])

  // 三间四柱牌楼：4 根柱子，3 个开间
  const pillarPositions: [number, number][] = [
    [-w / 2, 0],
    [-w / 6, 0],
    [w / 6, 0],
    [w / 2, 0],
  ]

  return (
    <group position={position} rotation={[0, rotationY, 0]} scale={[levelScale, levelScale, levelScale]}>
      <group scale={[1, scaleY, 1]}>
        {/* 石阶基座（长条形） */}
        <StoneBase width={w} depth={d} height={baseH} transparent={isBuilding} opacity={opacity} />

        {/* 正面台阶（3级） */}
        <group position={[0, 0, d / 2]}>
          <Steps width={w * 0.4} depth={d * 0.5} stepCount={3} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 山门牌坊（双开木门，中央开间） */}
        <group position={[0, baseH + pillarH * 0.15, 0]}>
          <DoorFrame width={w * 0.2} height={pillarH * 0.5} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 四根立柱（朱红木柱，三间四柱） */}
        {pillarPositions.map(([x, z], i) => (
          <group key={i} position={[x, baseH, z]}>
            <Pillar height={pillarH} radius={0.22} transparent={isBuilding} opacity={opacity} />
          </group>
        ))}

        {/* 下横梁（连接四柱） */}
        <mesh position={[0, baseH + pillarH * 0.7, 0]} castShadow>
          <boxGeometry args={[w * 1.05, beamH, d * 0.6]} />
          <meshStandardMaterial {...gateMat} transparent={isBuilding} opacity={opacity} />
        </mesh>

        {/* 匾额（中央开间上方，金色字底） */}
        <mesh position={[0, baseH + pillarH * 0.85, d / 2 + 0.05]} castShadow>
          <boxGeometry args={[w * 0.25, pillarH * 0.15, 0.08]} />
          <meshStandardMaterial {...PLAQUE_MAT} transparent={isBuilding} opacity={opacity} />
        </mesh>

        {/* 斗拱（每根柱顶各一个） */}
        {pillarPositions.map(([x, z], i) => (
          <group key={`dg-${i}`} position={[x, baseH + pillarH, z]}>
            <Dougong radius={0.32} transparent={isBuilding} opacity={opacity} />
          </group>
        ))}

        {/* 飞檐屋顶（青绿琉璃，覆盖整座牌楼） */}
        <group position={[0, baseH + pillarH + 0.3, 0]}>
          <EavesRoof
            width={w * 1.2}
            depth={d * 1.8}
            height={roofH}
            material={GLAZED_TILE_MATERIAL}
            transparent={isBuilding}
            opacity={opacity}
          />
        </group>

        {/* 屋顶宝顶（中央装饰） */}
        <mesh position={[0, baseH + pillarH + roofH * 0.7, 0]} castShadow>
          <coneGeometry args={[0.2, roofH * 0.4, 8]} />
          <meshStandardMaterial
            color="#ffd700"
            metalness={0.9}
            roughness={0.15}
            envMapIntensity={1.3}
            emissive="#ffd700"
            emissiveIntensity={0.2}
            transparent={isBuilding}
            opacity={opacity}
          />
        </mesh>

        {/* 灯笼装饰（两侧柱顶悬挂） */}
        <group position={[-w / 2, baseH + pillarH * 0.85, 0]}>
          <Lantern scale={0.8} transparent={isBuilding} opacity={opacity} />
        </group>
        <group position={[w / 2, baseH + pillarH * 0.85, 0]}>
          <Lantern scale={0.8} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 建造中：脚手架 */}
        {isBuilding && (
          <mesh position={[0, h / 2, 0]}>
            <boxGeometry args={[w + 0.3, h + 0.3, d + 0.3]} />
            <meshBasicMaterial color="#fbbf24" wireframe transparent opacity={0.3} />
          </mesh>
        )}
      </group>

      {/* 保留 PILLAR_MATERIAL 引用（虽然此处用 gateMat，但为未来扩展保留） */}
      <group visible={false}>
        <mesh><boxGeometry args={[0.01, 0.01, 0.01]} /><meshStandardMaterial {...PILLAR_MATERIAL} /></mesh>
      </group>
    </group>
  )
}
