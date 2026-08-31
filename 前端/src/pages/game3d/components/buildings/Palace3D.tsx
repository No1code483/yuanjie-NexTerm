/**
 * Palace3D - 仙家宫殿（palace 类别）
 *
 * 文明等级 5（修仙初期），修仙门派主殿。
 * 中式修仙风格：金色宝顶 + 飞檐翘角 + 玉石基座 + 灵气
 *
 * 参考 05_3D场景设计.md §6.2 修仙楼阁材质参数
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react'
import {
  StoneBase,
  FourPillars,
  Dougong,
  EavesRoof,
  GoldenPinnacle,
  Lantern,
  RoofRidge,
  Steps,
  GLAZED_TILE_MATERIAL,
  withBuildState,
  getMaterialProps,
  type PBRMaterialProps,
} from '../ChineseArchParts'
import type { GameBuildingStatus } from '@/types/game'

interface Palace3DProps {
  position: [number, number, number]
  rotationY?: number
  status?: GameBuildingStatus
  buildProgress?: number
  level?: number
}

const SIZE: [number, number, number] = [4.0, 3.8, 4.0]

/** 玉石墙材质（+ 石纹纹理） */
const JADE_WALL_MAT: PBRMaterialProps = {
  color: '#e8e4d0',
  metalness: 0.2,
  roughness: 0.35,
  envMapIntensity: 1.1,
  textureType: 'stone',
}

/** 琉璃瓦材质（修仙用青蓝琉璃，+ 琉璃瓦纹理） */
const IMMORTAL_TILE_MAT: PBRMaterialProps = {
  color: '#2d5a8a',
  metalness: 0.4,
  roughness: 0.25,
  envMapIntensity: 1.2,
  textureType: 'glazedTile',
}

export function Palace3D({
  position,
  rotationY = 0,
  status = 'completed',
  buildProgress = 100,
  level = 1,
}: Palace3DProps) {
  const isBuilding = status === 'building'
  const opacity = isBuilding ? 0.4 : 1.0
  const scaleY = isBuilding ? Math.max(0.1, buildProgress / 100) : 1.0
  const levelScale = 1 + (level - 1) * 0.1

  const [w, h, d] = SIZE
  const baseH = 0.6
  const wallH = h * 0.5
  const roofH = h * 0.4
  const pinnacleH = h * 0.1
  const wallMat = useMemo(() => getMaterialProps(withBuildState(JADE_WALL_MAT, isBuilding)), [isBuilding])

  return (
    <group position={position} rotation={[0, rotationY, 0]} scale={[levelScale, levelScale, levelScale]}>
      <group scale={[1, scaleY, 1]}>
        {/* 玉石基座（高须弥座） */}
        <StoneBase width={w} depth={d} height={baseH} transparent={isBuilding} opacity={opacity} />

        {/* 正面台阶（3级） */}
        <group position={[0, 0, d / 2]}>
          <Steps width={w * 0.5} depth={d * 0.3} stepCount={3} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 主体玉石墙 */}
        <mesh position={[0, baseH + wallH / 2, 0]} castShadow receiveShadow>
          <boxGeometry args={[w, wallH, d]} />
          <meshStandardMaterial {...wallMat} transparent={isBuilding} opacity={opacity} />
        </mesh>

        {/* 四立柱（玉石柱） */}
        <group position={[0, baseH, 0]}>
          <FourPillars width={w} depth={d} height={wallH} inset={0.35} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 斗拱（四角） */}
        {[
          [w / 2 - 0.35, d / 2 - 0.35],
          [-w / 2 + 0.35, d / 2 - 0.35],
          [w / 2 - 0.35, -d / 2 + 0.35],
          [-w / 2 + 0.35, -d / 2 + 0.35],
        ].map(([x, z], i) => (
          <group key={i} position={[x, baseH + wallH, z]}>
            <Dougong radius={0.45} transparent={isBuilding} opacity={opacity} />
          </group>
        ))}

        {/* 飞檐屋顶（青蓝琉璃） */}
        <group position={[0, baseH + wallH, 0]}>
          <EavesRoof
            width={w * 1.15}
            depth={d * 1.15}
            height={roofH}
            material={IMMORTAL_TILE_MAT}
            transparent={isBuilding}
            opacity={opacity}
          />
        </group>

        {/* 屋脊吻兽（屋顶两端装饰） */}
        <group position={[0, baseH + wallH + roofH * 0.55, 0]}>
          <RoofRidge width={w * 1.1} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 金色宝顶（屋顶中央） */}
        <group position={[0, baseH + wallH + roofH * 0.5, 0]}>
          <GoldenPinnacle height={pinnacleH * 2} radius={0.4} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 灵气光晕（宝顶周围，半透明球） */}
        {!isBuilding && (
          <mesh position={[0, baseH + wallH + roofH * 0.8, 0]}>
            <sphereGeometry args={[0.8, 16, 12]} />
            <meshStandardMaterial
              color="#ffe4b5"
              emissive="#ffd700"
              emissiveIntensity={0.3}
              transparent
              opacity={0.15}
              depthWrite={false}
            />
          </mesh>
        )}

        {/* 灯笼装饰（四角悬挂） */}
        {[
          [w * 0.45, d * 0.45],
          [-w * 0.45, d * 0.45],
          [w * 0.45, -d * 0.45],
          [-w * 0.45, -d * 0.45],
        ].map(([x, z], i) => (
          <group key={`lantern-${i}`} position={[x, baseH + wallH * 0.7, z]}>
            <Lantern scale={0.7} transparent={isBuilding} opacity={opacity} />
          </group>
        ))}

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

// 保留 GLAZED_TILE_MATERIAL 引用避免未使用告警（修仙建筑实际用 IMMORTAL_TILE_MAT）
void GLAZED_TILE_MATERIAL
