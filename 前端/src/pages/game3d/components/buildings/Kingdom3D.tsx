/**
 * Kingdom3D - 凡人王国宫殿（kingdom 类别）
 *
 * 文明等级 4（凡人极致），最豪华的凡人建筑。
 * 中式风格：重檐庑殿顶 + 红墙 + 斗拱 + 高台基
 *
 * 参考 05_3D场景设计.md §6.2 砖石楼房材质参数
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react'
import {
  StoneBase,
  FourPillars,
  Dougong,
  DoubleEavesRoof,
  RoofRidge,
  Steps,
  DoorFrame,
  GLAZED_TILE_MATERIAL,
  withBuildState,
  getMaterialProps,
  type PBRMaterialProps,
} from '../ChineseArchParts'
import type { GameBuildingStatus } from '@/types/game'

interface Kingdom3DProps {
  position: [number, number, number]
  rotationY?: number
  status?: GameBuildingStatus
  buildProgress?: number
  level?: number
}

const SIZE: [number, number, number] = [3.4, 4.2, 3.4]

/** 红墙材质（+ 砖墙纹理） */
const RED_WALL_MAT: PBRMaterialProps = {
  color: '#9b3020',
  metalness: 0.05,
  roughness: 0.7,
  envMapIntensity: 0.6,
  textureType: 'brick',
}

export function Kingdom3D({
  position,
  rotationY = 0,
  status = 'completed',
  buildProgress = 100,
  level = 1,
}: Kingdom3DProps) {
  const isBuilding = status === 'building'
  const opacity = isBuilding ? 0.4 : 1.0
  const scaleY = isBuilding ? Math.max(0.1, buildProgress / 100) : 1.0
  const levelScale = 1 + (level - 1) * 0.1

  const [w, h, d] = SIZE
  const baseH = 0.5          // 高台基
  const wallH = h * 0.4      // 墙身
  const roofH = h * 0.6      // 重檐屋顶总高
  const wallMat = useMemo(() => getMaterialProps(withBuildState(RED_WALL_MAT, isBuilding)), [isBuilding])

  return (
    <group position={position} rotation={[0, rotationY, 0]} scale={[levelScale, levelScale, levelScale]}>
      <group scale={[1, scaleY, 1]}>
        {/* 高台基（须弥座，三层） */}
        <StoneBase width={w} depth={d} height={baseH} transparent={isBuilding} opacity={opacity} />

        {/* 主体红墙 */}
        <mesh position={[0, baseH + wallH / 2, 0]} castShadow receiveShadow>
          <boxGeometry args={[w, wallH, d]} />
          <meshStandardMaterial {...wallMat} transparent={isBuilding} opacity={opacity} />
        </mesh>

        {/* 四立柱（朱红柱，更显皇家气派） */}
        <group position={[0, baseH, 0]}>
          <FourPillars width={w} depth={d} height={wallH} inset={0.3} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 斗拱（柱顶，四角各一个） */}
        {[
          [w / 2 - 0.3, d / 2 - 0.3],
          [-w / 2 + 0.3, d / 2 - 0.3],
          [w / 2 - 0.3, -d / 2 + 0.3],
          [-w / 2 + 0.3, -d / 2 + 0.3],
        ].map(([x, z], i) => (
          <group key={i} position={[x, baseH + wallH, z]}>
            <Dougong radius={0.4} transparent={isBuilding} opacity={opacity} />
          </group>
        ))}

        {/* 重檐庑殿顶（双层屋顶） */}
        <group position={[0, baseH + wallH, 0]}>
          <DoubleEavesRoof
            width={w * 1.1}
            depth={d * 1.1}
            lowerHeight={roofH * 0.55}
            upperHeight={roofH * 0.45}
            material={GLAZED_TILE_MATERIAL}
            transparent={isBuilding}
            opacity={opacity}
          />
        </group>

        {/* 正面台阶（5级，高台基） */}
        <group position={[0, 0, d / 2]}>
          <Steps width={w * 0.5} depth={d * 0.35} stepCount={5} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 正门（双开门+门环） */}
        <group position={[0, baseH + wallH * 0.15, d / 2 + 0.02]}>
          <DoorFrame width={w * 0.35} height={wallH * 0.6} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 屋脊吻兽（重檐屋顶两端装饰） */}
        <group position={[0, baseH + wallH + roofH * 0.5, 0]}>
          <RoofRidge width={w * 1.05} transparent={isBuilding} opacity={opacity} />
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
