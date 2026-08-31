/**
 * ChineseArchParts - 中式建筑共享零件库
 *
 * 为 8 大类程序化建筑提供中式修仙风格的可复用零件：
 *   - <StoneBase/>       石制基座（须弥座/台基）
 *   - <Pillar/>          红色立柱（木柱）
 *   - <Dougong/>         斗拱（柱顶与屋顶之间的层叠木结构）
 *   - <EavesRoof/>       飞檐屋顶（歇山/悬山/庑殿 顶）
 *   - <GlazedRoof/>      琉璃瓦屋顶（带屋脊）
 *   - <GoldenPinnacle/>  金色宝顶（修仙建筑顶部装饰）
 *   - <Lantern/>         灯笼（装饰）
 *   - <CityWall/>        城墙带垛口
 *
 * 设计参考：
 *   - 05_3D场景设计.md §6.2 PBR 材质参数标准
 *   - 中式古建特征：飞檐翘角、斗拱、琉璃瓦、宝顶、须弥座
 *
 * 几何手法说明：
 *   - 飞檐：用倒置截头方锥（4 面 coneGeometry）+ 上翘旋转模拟翘角
 *   - 斗拱：层叠的小方块（boxGeometry）模拟枋木 + 斗
 *   - 宝顶：金色圆锥 + 宝珠（sphere）
 *   - 须弥座：层叠 box（上枋+束腰+下枋）
 *
 * v2 纹理升级：材质预设集成 Canvas 程序化纹理（map + roughnessMap），
 *   通过 getTexturePair() 获取缓存的纹理对。
 *
 * change-id: game-3d-rebuild-refactor-modeling-opt
 */
import { useMemo } from 'react'
import * as THREE from 'three'
import { getTexturePair, type TextureType } from '../utils/TextureGenerator'

// ============================================================================
// 一、材质预设（参考 05_3D场景设计.md §6.2）
// ============================================================================

export interface PBRMaterialProps {
  color: string
  metalness: number
  roughness: number
  envMapIntensity: number
  transparent?: boolean
  opacity?: number
  emissive?: string
  emissiveIntensity?: number
  /** Canvas 程序化纹理类型（设置后自动附加 map + roughnessMap） */
  textureType?: TextureType
}

/** 红色立柱材质（修仙建筑常用朱红柱） */
export const PILLAR_MATERIAL: PBRMaterialProps = {
  color: '#8b2c1a',     // 朱红
  metalness: 0.05,
  roughness: 0.7,
  envMapIntensity: 0.5,
  textureType: 'wood',
}

/** 灰色石材材质（基座/台阶/城墙） */
export const STONE_MATERIAL: PBRMaterialProps = {
  color: '#6b6b6b',
  metalness: 0.0,
  roughness: 0.9,
  envMapIntensity: 0.4,
  textureType: 'stone',
}

/** 琉璃瓦材质（青绿色，皇家/宫殿用） */
export const GLAZED_TILE_MATERIAL: PBRMaterialProps = {
  color: '#3a6b5e',     // 青绿琉璃
  metalness: 0.3,
  roughness: 0.35,
  envMapIntensity: 0.9,
  textureType: 'glazedTile',
}

/** 金色宝顶材质（高反射，无纹理——金属自发光） */
export const GOLDEN_MATERIAL: PBRMaterialProps = {
  color: '#ffd700',
  metalness: 0.9,
  roughness: 0.15,
  envMapIntensity: 1.4,
  emissive: '#5a3d00',
  emissiveIntensity: 0.15,
}

/** 木材材质（褐色，普通建筑用） */
export const WOOD_MATERIAL: PBRMaterialProps = {
  color: '#7a5230',
  metalness: 0.0,
  roughness: 0.8,
  envMapIntensity: 0.4,
  textureType: 'wood',
}

/** 灰瓦材质（普通建筑用灰瓦，非琉璃） */
export const GREY_TILE_MAT: PBRMaterialProps = {
  color: '#4a4a4a',
  metalness: 0.1,
  roughness: 0.75,
  envMapIntensity: 0.5,
  textureType: 'greyTile',
}

// ============================================================================
// 二、基础零件组件
// ============================================================================

interface PartProps {
  /** 透明度覆盖（建造中状态用） */
  transparent?: boolean
  opacity?: number
}

// --- 石制基座（台基/须弥座简化版） ---
interface StoneBaseProps extends PartProps {
  width: number
  depth: number
  height?: number
}

/**
 * 石制基座：层叠结构（下枋+束腰+上枋），模拟须弥座
 * 高度默认为宽度的 0.15 倍
 */
export function StoneBase({ width, depth, height, transparent, opacity = 1 }: StoneBaseProps) {
  const h = height ?? Math.max(0.3, Math.min(width, depth) * 0.15)
  const lowerH = h * 0.3
  const waistH = h * 0.4
  const upperH = h * 0.3
  const lowerW = width + 0.2
  const lowerD = depth + 0.2
  const upperW = width + 0.1
  const upperD = depth + 0.1

  const mat = useMemo<Record<string, unknown>>(
    () => getMaterialProps({ ...STONE_MATERIAL, transparent, opacity }),
    [transparent, opacity],
  )

  return (
    <group>
      {/* 下枋 */}
      <mesh position={[0, lowerH / 2, 0]} receiveShadow castShadow>
        <boxGeometry args={[lowerW, lowerH, lowerD]} />
        <meshStandardMaterial {...mat} />
      </mesh>
      {/* 束腰 */}
      <mesh position={[0, lowerH + waistH / 2, 0]} castShadow>
        <boxGeometry args={[width, waistH, depth]} />
        <meshStandardMaterial {...mat} />
      </mesh>
      {/* 上枋 */}
      <mesh position={[0, lowerH + waistH + upperH / 2, 0]} receiveShadow castShadow>
        <boxGeometry args={[upperW, upperH, upperD]} />
        <meshStandardMaterial {...mat} />
      </mesh>
    </group>
  )
}

// --- 红色立柱 ---
interface PillarProps extends PartProps {
  height: number
  radius?: number
}

/**
 * 圆形立柱，默认朱红色，模拟木柱
 */
export function Pillar({ height, radius = 0.18, transparent, opacity = 1 }: PillarProps) {
  const mat = useMemo<Record<string, unknown>>(
    () => getMaterialProps({ ...PILLAR_MATERIAL, transparent, opacity }),
    [transparent, opacity],
  )
  return (
    <mesh position={[0, height / 2, 0]} castShadow>
      <cylinderGeometry args={[radius, radius * 1.1, height, 12]} />
      <meshStandardMaterial {...mat} />
    </mesh>
  )
}

// --- 四立柱组（建筑四角） ---
interface FourPillarsProps extends PartProps {
  width: number
  depth: number
  height: number
  inset?: number
}

/**
 * 四角立柱组，inset 为柱子相对边缘的内缩距离
 */
export function FourPillars({ width, depth, height, inset = 0.25, ...rest }: FourPillarsProps) {
  const x = width / 2 - inset
  const z = depth / 2 - inset
  return (
    <group>
      <group position={[x, 0, z]}><Pillar height={height} {...rest} /></group>
      <group position={[-x, 0, z]}><Pillar height={height} {...rest} /></group>
      <group position={[x, 0, -z]}><Pillar height={height} {...rest} /></group>
      <group position={[-x, 0, -z]}><Pillar height={height} {...rest} /></group>
    </group>
  )
}

// --- 斗拱（柱顶装饰） ---
interface DougongProps extends PartProps {
  radius?: number
}

/**
 * 斗拱：柱顶的层叠木结构
 * 用两层小方块模拟"斗"和"拱"，是中式建筑核心特征
 */
export function Dougong({ radius = 0.35, transparent, opacity = 1 }: DougongProps) {
  const mat = useMemo<Record<string, unknown>>(
    () => getMaterialProps({ ...WOOD_MATERIAL, transparent, opacity }),
    [transparent, opacity],
  )
  return (
    <group>
      {/* 下层斗（方斗） */}
      <mesh position={[0, 0.1, 0]} castShadow>
        <boxGeometry args={[radius * 0.8, 0.2, radius * 0.8]} />
        <meshStandardMaterial {...mat} />
      </mesh>
      {/* 中层拱（横木） */}
      <mesh position={[0, 0.28, 0]} castShadow>
        <boxGeometry args={[radius * 1.8, 0.16, radius * 0.4]} />
        <meshStandardMaterial {...mat} />
      </mesh>
      {/* 上层斗（小方斗） */}
      <mesh position={[0, 0.42, 0]} castShadow>
        <boxGeometry args={[radius * 0.6, 0.16, radius * 0.6]} />
        <meshStandardMaterial {...mat} />
      </mesh>
    </group>
  )
}

// --- 飞檐屋顶 ---
interface EavesRoofProps extends PartProps {
  width: number
  depth: number
  height: number
  material?: PBRMaterialProps | Record<string, unknown>
}

/**
 * 飞檐屋顶：四坡顶 + 翘角
 *
 * 用 4 个倾斜的"翼板"模拟歇山顶/庑殿顶：
 *   - 主体：扁平四棱锥（金字塔形），4 面
 *   - 翘角：4 个角向上翘起的小三角板
 *
 * 这是中式建筑最具辨识度的特征
 */
export function EavesRoof({
  width,
  depth,
  height,
  material = GLAZED_TILE_MATERIAL,
  transparent,
  opacity = 1,
}: EavesRoofProps) {
  const mat = useMemo<Record<string, unknown>>(() => {
    if ('textureType' in material && (material as PBRMaterialProps).textureType) {
      return getMaterialProps({ ...(material as PBRMaterialProps), transparent, opacity })
    }
    return { ...material, transparent, opacity }
  }, [material, transparent, opacity])

  // 屋顶主体：金字塔形（4 面 coneGeometry）
  const radius = Math.max(width, depth) * 0.72
  // 4 个翘角的位置
  const corners: [number, number][] = [
    [width / 2, depth / 2],
    [-width / 2, depth / 2],
    [width / 2, -depth / 2],
    [-width / 2, -depth / 2],
  ]

  return (
    <group>
      {/* 屋顶主体：金字塔 */}
      <mesh position={[0, height / 2, 0]} castShadow>
        <coneGeometry args={[radius, height, 4]} />
        <meshStandardMaterial {...mat} />
      </mesh>
      {/* 屋脊：沿对角线的小立方体，强化屋脊线 */}
      {corners.map(([x, z], i) => (
        <mesh
          key={i}
          position={[x * 0.85, height * 0.45, z * 0.85]}
          rotation={[0, Math.atan2(x, z), Math.PI / 6]}
          castShadow
        >
          <boxGeometry args={[0.12, height * 0.5, 0.12]} />
          <meshStandardMaterial {...mat} />
        </mesh>
      ))}
      {/* 翘角：4 个角向上翘起的小板（飞檐特征） */}
      {corners.map(([x, z], i) => (
        <mesh
          key={`eave-${i}`}
          position={[x * 0.95, height * 0.15, z * 0.95]}
          rotation={[
            z > 0 ? -Math.PI / 5 : Math.PI / 5,
            Math.atan2(x, z),
            x > 0 ? Math.PI / 5 : -Math.PI / 5,
          ]}
          castShadow
        >
          <boxGeometry args={[0.3, 0.05, 0.6]} />
          <meshStandardMaterial {...mat} />
        </mesh>
      ))}
    </group>
  )
}

// --- 琉璃瓦屋顶（重檐版，双层屋顶） ---
interface DoubleEavesRoofProps extends PartProps {
  width: number
  depth: number
  lowerHeight: number
  upperHeight: number
  material?: PBRMaterialProps | Record<string, unknown>
}

/**
 * 重檐屋顶：下层大屋顶 + 上层小屋顶（庑殿顶/重檐歇山顶）
 * 用于宫殿/宗门/仙府等高级建筑
 */
export function DoubleEavesRoof({
  width,
  depth,
  lowerHeight,
  upperHeight,
  material = GLAZED_TILE_MATERIAL,
  ...rest
}: DoubleEavesRoofProps) {
  return (
    <group>
      {/* 下层屋顶 */}
      <group position={[0, 0, 0]}>
        <EavesRoof width={width} depth={depth} height={lowerHeight} material={material} {...rest} />
      </group>
      {/* 上层屋顶（缩小 + 抬高） */}
      <group position={[0, lowerHeight * 0.6, 0]}>
        <EavesRoof
          width={width * 0.7}
          depth={depth * 0.7}
          height={upperHeight}
          material={material}
          {...rest}
        />
      </group>
    </group>
  )
}

// --- 金色宝顶（修仙建筑顶部） ---
interface GoldenPinnacleProps extends PartProps {
  height?: number
  radius?: number
}

/**
 * 金色宝顶：圆锥顶 + 宝珠
 * 修仙/宗门/仙府建筑的标志性装饰
 */
export function GoldenPinnacle({
  height = 1.0,
  radius = 0.35,
  transparent,
  opacity = 1,
}: GoldenPinnacleProps) {
  const mat = useMemo<PBRMaterialProps>(
    () => ({ ...GOLDEN_MATERIAL, transparent, opacity }),
    [transparent, opacity],
  )
  return (
    <group>
      {/* 基座（莲花座简化版） */}
      <mesh position={[0, 0.08, 0]} castShadow>
        <cylinderGeometry args={[radius * 1.3, radius * 1.5, 0.16, 16]} />
        <meshStandardMaterial {...mat} />
      </mesh>
      {/* 主尖（圆锥） */}
      <mesh position={[0, height * 0.45 + 0.16, 0]} castShadow>
        <coneGeometry args={[radius, height * 0.9, 16]} />
        <meshStandardMaterial {...mat} />
      </mesh>
      {/* 宝珠（球形顶饰） */}
      <mesh position={[0, height + 0.16, 0]} castShadow>
        <sphereGeometry args={[radius * 0.5, 16, 12]} />
        <meshStandardMaterial {...mat} emissive="#ffd700" emissiveIntensity={0.3} />
      </mesh>
    </group>
  )
}

// --- 灯笼（装饰） ---
interface LanternProps extends PartProps {
  scale?: number
}

/**
 * 红灯笼：球形 + 顶部小盖 + 吊绳
 * 用于城镇/宫殿/宗门装饰
 */
export function Lantern({ scale = 1, transparent, opacity = 1 }: LanternProps) {
  const mat = useMemo<PBRMaterialProps>(
    () => ({
      color: '#c41e3a',
      metalness: 0.1,
      roughness: 0.6,
      envMapIntensity: 0.6,
      emissive: '#c41e3a',
      emissiveIntensity: 0.4,
      transparent,
      opacity,
    }),
    [transparent, opacity],
  )
  const capMat = useMemo<PBRMaterialProps>(
    () => ({ ...WOOD_MATERIAL, transparent, opacity }),
    [transparent, opacity],
  )
  return (
    <group scale={[scale, scale, scale]}>
      {/* 顶盖 */}
      <mesh position={[0, 0.25, 0]} castShadow>
        <cylinderGeometry args={[0.12, 0.18, 0.08, 8]} />
        <meshStandardMaterial {...capMat} />
      </mesh>
      {/* 灯笼主体（扁球） */}
      <mesh castShadow>
        <sphereGeometry args={[0.2, 12, 8]} />
        <meshStandardMaterial {...mat} />
      </mesh>
      {/* 底盖 */}
      <mesh position={[0, -0.25, 0]} castShadow>
        <cylinderGeometry args={[0.12, 0.18, 0.08, 8]} />
        <meshStandardMaterial {...capMat} />
      </mesh>
      {/* 吊绳 */}
      <mesh position={[0, 0.4, 0]}>
        <cylinderGeometry args={[0.01, 0.01, 0.3, 4]} />
        <meshStandardMaterial color="#2a2a2a" />
      </mesh>
    </group>
  )
}

// --- 城墙带垛口 ---
interface CityWallProps extends PartProps {
  width: number
  height: number
  depth?: number
  battlementCount?: number
}

/**
 * 城墙：主体 + 顶部垛口（锯齿状）
 * 用于 city/kingdom 的城墙
 */
export function CityWall({
  width,
  height,
  depth = 0.4,
  battlementCount = 8,
  transparent,
  opacity = 1,
}: CityWallProps) {
  const mat = useMemo<Record<string, unknown>>(
    () => getMaterialProps({ ...STONE_MATERIAL, transparent, opacity }),
    [transparent, opacity],
  )
  const battlementW = width / (battlementCount * 2 - 1)
  const battlements = Array.from({ length: battlementCount }, (_, i) => i)

  return (
    <group>
      {/* 墙体主体 */}
      <mesh position={[0, height / 2, 0]} castShadow receiveShadow>
        <boxGeometry args={[width, height, depth]} />
        <meshStandardMaterial {...mat} />
      </mesh>
      {/* 垛口（顶部锯齿） */}
      {battlements.map((i) => (
        <mesh
          key={i}
          position={[-width / 2 + battlementW * (i * 2 + 0.5), height + 0.1, 0]}
          castShadow
        >
          <boxGeometry args={[battlementW, 0.2, depth]} />
          <meshStandardMaterial {...mat} />
        </mesh>
      ))}
    </group>
  )
}

// ============================================================================
// 三、材质工具函数
// ============================================================================

/**
 * 解析材质中的纹理引用
 * 将 textureType 转换为实际的 map + roughnessMap 对象
 */
function resolveTextures(props: PBRMaterialProps): Record<string, unknown> {
  if (!props.textureType) return {}
  const { map, roughnessMap } = getTexturePair(props.textureType)
  return { map, roughnessMap }
}

/**
 * 创建带建造状态覆盖的材质 props
 * 建造中：透明 + 半透明
 */
export function withBuildState(base: PBRMaterialProps, isBuilding: boolean): PBRMaterialProps {
  if (!isBuilding) return base
  return {
    ...base,
    transparent: true,
    opacity: 0.4,
  }
}

/**
 * 获取完整的材质属性（含纹理解析），用于直接展开到 meshStandardMaterial
 * 会处理 textureType → map + roughnessMap 的转换
 */
export function getMaterialProps(base: PBRMaterialProps): Record<string, unknown> {
  const textures = resolveTextures(base)
  // 移除 textureType 避免传入 meshStandardMaterial（它不认识这个属性）
  const { textureType: _, ...rest } = base
  return { ...rest, ...textures }
}

/**
 * 等级缩放因子：每级 +10% 体积
 */
export function levelScaleFactor(level: number): number {
  return 1 + (level - 1) * 0.1
}

// ============================================================================
// 四、建筑细节装饰组件（v2 几何增强）
// ============================================================================

// --- 窗棂（十字格窗） ---
interface WindowLatticeProps extends PartProps {
  width?: number
  height?: number
}

/**
 * 中式十字格窗棂
 * 用于凡人茅屋/集镇铺面/城楼等建筑
 */
export function WindowLattice({ width = 0.4, height = 0.35, transparent, opacity = 1 }: WindowLatticeProps) {
  const frameMat = useMemo<Record<string, unknown>>(
    () => getMaterialProps({ ...WOOD_MATERIAL, transparent, opacity }),
    [transparent, opacity],
  )
  const frameThick = 0.03

  return (
    <group>
      {/* 外框 */}
      <mesh position={[0, 0, 0]}>
        <boxGeometry args={[width + frameThick * 2, height + frameThick * 2, frameThick]} />
        <meshStandardMaterial {...frameMat} />
      </mesh>
      {/* 内框镂空（深色） */}
      <mesh position={[0, 0, frameThick / 2 + 0.001]}>
        <boxGeometry args={[width, height, frameThick * 0.5]} />
        <meshStandardMaterial color="#1a1a1a" roughness={1} metalness={0} />
      </mesh>
      {/* 十字格：横 */}
      <mesh position={[0, 0, frameThick]}>
        <boxGeometry args={[width, frameThick * 0.8, frameThick * 0.5]} />
        <meshStandardMaterial {...frameMat} />
      </mesh>
      {/* 十字格：竖 */}
      <mesh position={[0, 0, frameThick]}>
        <boxGeometry args={[frameThick * 0.8, height, frameThick * 0.5]} />
        <meshStandardMaterial {...frameMat} />
      </mesh>
    </group>
  )
}

// --- 门框 + 门环 ---
interface DoorFrameProps extends PartProps {
  width?: number
  height?: number
}

/**
 * 中式双开门 + 门环
 * 用于凡人茅屋/集镇铺面/城楼等
 */
export function DoorFrame({ width = 0.6, height = 0.9, transparent, opacity = 1 }: DoorFrameProps) {
  const woodMat = useMemo<Record<string, unknown>>(
    () => getMaterialProps({ ...WOOD_MATERIAL, transparent, opacity }),
    [transparent, opacity],
  )
  const hw = width / 2
  const hh = height / 2
  const frameThick = 0.04

  return (
    <group>
      {/* 门框两侧立柱 */}
      <mesh position={[-hw - frameThick / 2, 0, 0]}>
        <boxGeometry args={[frameThick, height, frameThick]} />
        <meshStandardMaterial {...woodMat} />
      </mesh>
      <mesh position={[hw + frameThick / 2, 0, 0]}>
        <boxGeometry args={[frameThick, height, frameThick]} />
        <meshStandardMaterial {...woodMat} />
      </mesh>
      {/* 门框上横梁 */}
      <mesh position={[0, hh + frameThick / 2, 0]}>
        <boxGeometry args={[width + frameThick * 2, frameThick, frameThick]} />
        <meshStandardMaterial {...woodMat} />
      </mesh>
      {/* 左门扇 */}
      <mesh position={[-hw / 2, 0, frameThick / 2]}>
        <boxGeometry args={[hw - 0.01, height - frameThick, frameThick * 0.5]} />
        <meshStandardMaterial {...woodMat} />
      </mesh>
      {/* 右门扇 */}
      <mesh position={[hw / 2, 0, frameThick / 2]}>
        <boxGeometry args={[hw - 0.01, height - frameThick, frameThick * 0.5]} />
        <meshStandardMaterial {...woodMat} />
      </mesh>
      {/* 门环（左） */}
      <mesh position={[-hw / 2, hh * 0.3, frameThick]}>
        <torusGeometry args={[0.06, 0.015, 8, 8]} />
        <meshStandardMaterial color="#d4a017" metalness={0.8} roughness={0.3} />
      </mesh>
      {/* 门环（右） */}
      <mesh position={[hw / 2, hh * 0.3, frameThick]}>
        <torusGeometry args={[0.06, 0.015, 8, 8]} />
        <meshStandardMaterial color="#d4a017" metalness={0.8} roughness={0.3} />
      </mesh>
    </group>
  )
}

// --- 屋脊吻兽（鸱吻） ---
interface RoofRidgeProps extends PartProps {
  width: number
}

/**
 * 屋脊两端吻兽装饰
 * 用于宫殿/宗门/仙府等高级建筑
 */
export function RoofRidge({ width, transparent, opacity = 1 }: RoofRidgeProps) {
  const mat = useMemo<Record<string, unknown>>(
    () => getMaterialProps({ ...GLAZED_TILE_MATERIAL, transparent, opacity }),
    [transparent, opacity],
  )

  return (
    <group>
      {/* 左吻兽 */}
      <group position={[-width / 2, 0, 0]}>
        <mesh position={[0, 0.15, 0]} castShadow>
          <boxGeometry args={[0.25, 0.3, 0.2]} />
          <meshStandardMaterial {...mat} />
        </mesh>
        {/* 吻兽尾部上翘 */}
        <mesh position={[-0.15, 0.35, 0]} rotation={[0, 0, Math.PI / 6]} castShadow>
          <boxGeometry args={[0.18, 0.08, 0.15]} />
          <meshStandardMaterial {...mat} />
        </mesh>
      </group>
      {/* 右吻兽 */}
      <group position={[width / 2, 0, 0]}>
        <mesh position={[0, 0.15, 0]} castShadow>
          <boxGeometry args={[0.25, 0.3, 0.2]} />
          <meshStandardMaterial {...mat} />
        </mesh>
        <mesh position={[0.15, 0.35, 0]} rotation={[0, 0, -Math.PI / 6]} castShadow>
          <boxGeometry args={[0.18, 0.08, 0.15]} />
          <meshStandardMaterial {...mat} />
        </mesh>
      </group>
      {/* 屋脊中央宝珠 */}
      <mesh position={[0, 0.2, 0]} castShadow>
        <sphereGeometry args={[0.12, 8, 6]} />
        <meshStandardMaterial color="#ffd700" metalness={0.9} roughness={0.15} emissive="#ffd700" emissiveIntensity={0.2} />
      </mesh>
    </group>
  )
}

// --- 台阶 ---
interface StepsProps extends PartProps {
  width: number
  depth: number
  stepCount?: number
  stepHeight?: number
}

/**
 * 台阶/楼梯
 * 用于宫殿/宗门/城楼等高台基建筑
 */
export function Steps({ width, depth, stepCount = 3, stepHeight = 0.15, transparent, opacity = 1 }: StepsProps) {
  const mat = useMemo<Record<string, unknown>>(
    () => getMaterialProps({ ...STONE_MATERIAL, transparent, opacity }),
    [transparent, opacity],
  )
  const stepDepth = depth / stepCount

  return (
    <group>
      {Array.from({ length: stepCount }, (_, i) => (
        <mesh
          key={i}
          position={[0, i * stepHeight, depth / 2 - i * stepDepth - stepDepth / 2]}
          castShadow
          receiveShadow
        >
          <boxGeometry args={[width, stepHeight, stepDepth]} />
          <meshStandardMaterial {...mat} />
        </mesh>
      ))}
    </group>
  )
}

// --- 风铃（塔檐悬挂） ---
interface WindChimeProps extends PartProps {
  scale?: number
}

/**
 * 风铃装饰
 * 用于宝塔/仙府等高级建筑檐角悬挂
 */
export function WindChime({ scale = 1, transparent, opacity = 1 }: WindChimeProps) {
  return (
    <group scale={[scale, scale, scale]}>
      {/* 吊绳 */}
      <mesh position={[0, 0.15, 0]}>
        <cylinderGeometry args={[0.01, 0.01, 0.3, 4]} />
        <meshStandardMaterial color="#4a3728" />
      </mesh>
      {/* 铃铛 */}
      <mesh position={[0, -0.1, 0]} castShadow>
        <cylinderGeometry args={[0.04, 0.06, 0.15, 8]} />
        <meshStandardMaterial
          color="#d4a017"
          metalness={0.8}
          roughness={0.25}
          transparent={transparent}
          opacity={opacity}
        />
      </mesh>
      {/* 铃舌 */}
      <mesh position={[0, -0.2, 0]}>
        <sphereGeometry args={[0.025, 4, 4]} />
        <meshStandardMaterial
          color="#8b7355"
          metalness={0.6}
          roughness={0.4}
          transparent={transparent}
          opacity={opacity}
        />
      </mesh>
    </group>
  )
}

// 防止 THREE 被误判为未使用（材质预设中保留供未来扩展）
void THREE
