/**
 * Immortal3D - 七层宝塔（immortal 类别）
 *
 * 文明等级 8（仙境界），最高等级建筑。
 * 中式修仙风格：七层宝塔 + 相轮宝顶 + 莲花座 + 祥云光晕
 *
 * 参考 05_3D场景设计.md §6.2 修仙圣地材质参数（最高规格）
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react'
import {
  StoneBase,
  EavesRoof,
  GoldenPinnacle,
  WindChime,
  GLAZED_TILE_MATERIAL,
  withBuildState,
  getMaterialProps,
  type PBRMaterialProps,
} from '../ChineseArchParts'
import type { GameBuildingStatus } from '@/types/game'

interface Immortal3DProps {
  position: [number, number, number]
  rotationY?: number
  status?: GameBuildingStatus
  buildProgress?: number
  level?: number
}

const SIZE: [number, number, number] = [3.0, 5.5, 3.0]

/** 仙界白玉材质（+ 石纹纹理） */
const JADE_TOWER_MAT: PBRMaterialProps = {
  color: '#f0ede0',
  metalness: 0.3,
  roughness: 0.25,
  envMapIntensity: 1.3,
  textureType: 'stone',
}

/** 琉璃瓦材质（+ 琉璃瓦纹理） */
const IMMORTAL_TILE_MAT: PBRMaterialProps = {
  color: '#2a5a8a',
  metalness: 0.5,
  roughness: 0.15,
  envMapIntensity: 1.4,
  textureType: 'glazedTile',
}

/** 宝塔层数（七层佛塔/仙塔传统） */
const TOWER_LEVELS = 7

export function Immortal3D({
  position,
  rotationY = 0,
  status = 'completed',
  buildProgress = 100,
  level = 1,
}: Immortal3DProps) {
  const isBuilding = status === 'building'
  const opacity = isBuilding ? 0.4 : 1.0
  const scaleY = isBuilding ? Math.max(0.1, buildProgress / 100) : 1.0
  const levelScale = 1 + (level - 1) * 0.1

  const [w, h, d] = SIZE
  const baseH = 0.5
  const towerH = h - baseH  // 塔身总高
  const levelH = towerH * 0.78 / TOWER_LEVELS  // 每层高度（留 22% 给宝顶）
  const pinnacleH = towerH * 0.22
  const jadeMat = useMemo(() => getMaterialProps(withBuildState(JADE_TOWER_MAT, isBuilding)), [isBuilding])
  const tileMat = useMemo(() => getMaterialProps(withBuildState(IMMORTAL_TILE_MAT, isBuilding)), [isBuilding])

  // 生成 7 层塔，每层缩小 8%
  const levels = Array.from({ length: TOWER_LEVELS }, (_, i) => {
    const scale = 1 - i * 0.08
    return {
      index: i,
      scale,
      y: baseH + i * levelH,
      width: w * scale,
      depth: d * scale,
    }
  })

  return (
    <group position={position} rotation={[0, rotationY, 0]} scale={[levelScale, levelScale, levelScale]}>
      <group scale={[1, scaleY, 1]}>
        {/* 莲花座基座（高须弥座） */}
        <StoneBase width={w} depth={d} height={baseH} transparent={isBuilding} opacity={opacity} />

        {/* 莲花瓣装饰（基座顶部，8 瓣） */}
        {Array.from({ length: 8 }, (_, i) => {
          const angle = (i / 8) * Math.PI * 2
          const r = w * 0.55
          return (
            <mesh
              key={`petal-${i}`}
              position={[Math.cos(angle) * r, baseH, Math.sin(angle) * r]}
              rotation={[0, -angle, Math.PI / 8]}
              castShadow
            >
              <boxGeometry args={[0.3, 0.15, 0.5]} />
              <meshStandardMaterial {...jadeMat} transparent={isBuilding} opacity={opacity} />
            </mesh>
          )
        })}

        {/* 七层宝塔主体 */}
        {levels.map((lv) => (
          <group key={lv.index} position={[0, lv.y, 0]}>
            {/* 塔身（每层一个八角形棱柱） */}
            <mesh position={[0, levelH * 0.35, 0]} castShadow receiveShadow>
              <cylinderGeometry args={[lv.width * 0.4, lv.width * 0.45, levelH * 0.7, 8]} />
              <meshStandardMaterial {...jadeMat} transparent={isBuilding} opacity={opacity} />
            </mesh>

            {/* 塔窗（每层 4 面，深色小方块） */}
            {[0, Math.PI / 2, Math.PI, Math.PI * 1.5].map((angle, j) => (
              <mesh
                key={j}
                position={[
                  Math.cos(angle) * lv.width * 0.42,
                  levelH * 0.4,
                  Math.sin(angle) * lv.width * 0.42,
                ]}
                rotation={[0, -angle, 0]}
              >
                <boxGeometry args={[0.2, 0.25, 0.03]} />
                <meshStandardMaterial color="#1a1a1a" roughness={1} metalness={0} />
              </mesh>
            ))}

            {/* 飞檐（每层屋顶，青蓝琉璃） */}
            <group position={[0, levelH * 0.7, 0]}>
              <EavesRoof
                width={lv.width * 1.15}
                depth={lv.depth * 1.15}
                height={levelH * 0.3}
                material={tileMat}
                transparent={isBuilding}
                opacity={opacity}
              />
            </group>

            {/* 风铃（每层檐角悬挂，4角各一） */}
            {[0, Math.PI / 2, Math.PI, Math.PI * 1.5].map((angle, j) => (
              <group
                key={`chime-${j}`}
                position={[
                  Math.cos(angle) * lv.width * 0.55,
                  levelH * 0.55,
                  Math.sin(angle) * lv.width * 0.55,
                ]}
              >
                <WindChime scale={0.5} transparent={isBuilding} opacity={opacity} />
              </group>
            ))}
          </group>
        ))}

        {/* 相轮宝顶（塔尖） */}
        <group position={[0, baseH + TOWER_LEVELS * levelH, 0]}>
          <GoldenPinnacle height={pinnacleH * 1.5} radius={0.35} transparent={isBuilding} opacity={opacity} />
        </group>

        {/* 祥云光晕（宝顶周围，仙气） */}
        {!isBuilding && (
          <mesh position={[0, baseH + TOWER_LEVELS * levelH + pinnacleH * 0.5, 0]}>
            <sphereGeometry args={[1.0, 16, 12]} />
            <meshStandardMaterial
              color="#e0f0ff"
              emissive="#87ceeb"
              emissiveIntensity={0.4}
              transparent
              opacity={0.12}
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

// 保留 GLAZED_TILE_MATERIAL 引用（修仙宝塔实际用 IMMORTAL_TILE_MAT，但保留默认材质供未来扩展）
void GLAZED_TILE_MATERIAL
