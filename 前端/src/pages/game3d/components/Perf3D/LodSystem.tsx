/**
 * LodSystem - 动态 LOD 系统（Phase 4 §2.3.1）
 *
 * 规范：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §Phase 4
 *
 * 实现：
 * - 基于 drei <Detailed> 的距离驱动 LOD 切换
 * - 提供 3 级细节：近（高细节）、中（标准）、远（简化/SimpleBuilding）
 * - 自适应：可通过 props 自定义各级距离阈值与渲染内容
 *
 * 用法：
 * ```tsx
 * <LodSystem
 *   position={[x, y, z]}
 *   high={<ProceduralBuilding ... />}
 *   medium={<ProceduralBuilding ... lowDetail />}
 *   low={<SimpleBuilding ... />}
 * />
 * ```
 *
 * 注：Detailed 是 @react-three/drei 内置组件，本组件是对其的封装 + 文档化，
 *      方便 Scene 统一替换 Building 的渲染入口。
 */
import { Detailed } from '@react-three/drei'
import type { ReactElement } from 'react'

export interface LodSystemProps {
  /** 模型世界坐标 */
  position?: [number, number, number]
  /** 高细节阈值（< 此距离显示 high） */
  highThreshold?: number
  /** 中细节阈值（< 此距离显示 medium，否则 low） */
  mediumThreshold?: number
  /** 高细节渲染内容（近景） */
  high: ReactElement
  /** 中细节渲染内容（中景） */
  medium: ReactElement
  /** 低细节渲染内容（远景） */
  low: ReactElement
}

/** 默认 LOD 阈值（基于 game3d 地图 64x64 的尺度） */
const DEFAULT_HIGH = 15
const DEFAULT_MEDIUM = 35

export function LodSystem({
  position = [0, 0, 0],
  highThreshold = DEFAULT_HIGH,
  mediumThreshold = DEFAULT_MEDIUM,
  high,
  medium,
  low,
}: LodSystemProps) {
  // Detailed 接受 distances 数组（升序），按距离切换 children
  // children 顺序对应 [distances[0], distances[1], distances[2], null (默认)]
  return (
    <Detailed position={position} distances={[0, highThreshold, mediumThreshold]}>
      {high as any}
      {medium as any}
      {low as any}
    </Detailed>
  )
}
