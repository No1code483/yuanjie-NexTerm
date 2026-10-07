/**
 * ParticleDistanceLOD - 粒子系统距离优化（Phase 4 §2.3.4）
 *
 * 规范：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §Phase 4
 *
 * 目标：
 *   - 远距离粒子系统自动隐藏，避免无效 draw call
 *   - 提供两级阈值：near（满渲染）/ medium（简化或隐藏）/ far（完全剔除）
 *   - 与 FrustumCulling 互补：FrustumCulling 处理视野外剔除，本组件处理远距离剔除
 *
 * 实现：
 *   - useFrame 每帧计算相机到粒子组原点的距离
 *   - 节流策略：每 N 帧检测一次（默认 5 帧），避免每帧 sqrt 开销
 *   - 距离 > maxDistance → visible=false（GPU 跳过绘制）
 *   - 距离 ≤ maxDistance → visible=true（正常渲染）
 *   - 可选 onTierChange 回调，让子粒子系统根据 tier 调整粒子数量
 *
 * 与 SpiritParticles / CompletionBurst 的关系：
 *   - 包裹 <SpiritParticles /> 即可启用距离剔除
 *   - CompletionBurst 是短时一次性特效（2 秒），不需要距离剔除
 *
 * 用法：
 * ```tsx
 * <ParticleDistanceLOD position={[32, 4, 32]} maxDistance={60}>
 *   <SpiritParticles />
 * </ParticleDistanceLOD>
 * ```
 *
 * change-id: deliver-roadmap-55
 */
import { useRef, type ReactNode } from 'react'
import { useFrame, useThree } from '@react-three/fiber'
import * as THREE from 'three'

export interface ParticleDistanceLODProps {
  children: ReactNode
  /** 粒子组的世界坐标（用于距离计算） */
  position?: [number, number, number]
  /** 最大可见距离（超过此距离完全剔除，默认 60） */
  maxDistance?: number
  /** 简化阈值（超过此距离触发 tier=medium，默认 35） */
  mediumDistance?: number
  /** 检测频率（每 N 帧检测一次，默认 5） */
  checkEveryNFrames?: number
  /** 初始可见性（默认 true） */
  initialVisible?: boolean
  /**
   * 距离层级变化回调（子粒子系统可据此调整粒子数量）。
   * tier: 'near' | 'medium' | 'far'
   */
  onTierChange?: (tier: 'near' | 'medium' | 'far', distance: number) => void
}

const DEFAULT_MAX_DISTANCE = 60
const DEFAULT_MEDIUM_DISTANCE = 35
const DEFAULT_CHECK_EVERY = 5

export function ParticleDistanceLOD({
  children,
  position = [0, 0, 0],
  maxDistance = DEFAULT_MAX_DISTANCE,
  mediumDistance = DEFAULT_MEDIUM_DISTANCE,
  checkEveryNFrames = DEFAULT_CHECK_EVERY,
  initialVisible = true,
  onTierChange,
}: ParticleDistanceLODProps) {
  const groupRef = useRef<THREE.Group>(null)
  const { camera } = useThree()
  const frameCounter = useRef(0)
  const visibleRef = useRef(initialVisible)
  const tierRef = useRef<'near' | 'medium' | 'far'>(
    initialVisible ? 'near' : 'far',
  )
  // 复用 Vector3 避免每帧分配
  const tmpVec = useRef(new THREE.Vector3(...position))

  // 若 position 变化，同步 tmpVec
  tmpVec.current.set(position[0], position[1], position[2])

  useFrame(() => {
    if (!groupRef.current) return
    frameCounter.current += 1
    if (frameCounter.current % checkEveryNFrames !== 0) return

    const distance = camera.position.distanceTo(tmpVec.current)

    let newTier: 'near' | 'medium' | 'far'
    if (distance > maxDistance) {
      newTier = 'far'
    } else if (distance > mediumDistance) {
      newTier = 'medium'
    } else {
      newTier = 'near'
    }

    // 可见性切换：far 隐藏，near/medium 显示
    const shouldVisible = newTier !== 'far'
    if (shouldVisible !== visibleRef.current) {
      visibleRef.current = shouldVisible
      groupRef.current.visible = shouldVisible
    }

    // 层级变化回调
    if (newTier !== tierRef.current) {
      tierRef.current = newTier
      onTierChange?.(newTier, distance)
    }
  })

  return (
    <group ref={groupRef} position={position} visible={initialVisible}>
      {children}
    </group>
  )
}
