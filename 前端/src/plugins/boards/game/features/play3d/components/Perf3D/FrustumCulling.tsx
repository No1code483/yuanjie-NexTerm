/**
 * FrustumCulling - 视锥体剔除（Phase 4 §2.3.2）
 *
 * 规范：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §Phase 4
 *
 * 实现：
 * - useFrame 每帧检测 group 包围盒是否在相机视锥体内
 * - 不在视锥内 → visible=false（GPU 跳过绘制，节省 draw call）
 * - 在视锥内 → visible=true（正常渲染）
 * - 节流策略：每 N 帧检测一次（默认 3 帧），避免每帧开销
 *
 * 注：three.js 内部对 Mesh 已有 frustum culling（frustumCulled=true 默认开启），
 *      但对 Group 不生效。本组件通过显式 visible 切换，对包含多个子 mesh 的
 *      复杂组（如建筑群、装饰群）提供更主动的剔除控制。
 *
 * 用法：
 * ```tsx
 * <FrustumCulling>
 *   <group>...建筑群...</group>
 * </FrustumCulling>
 * ```
 */
import { useRef, type ReactNode } from 'react'
import { useFrame, useThree } from '@react-three/fiber'
import * as THREE from 'three'

export interface FrustumCullingProps {
  children: ReactNode
  /** 包围盒扩张量（避免边缘闪烁） */
  margin?: number
  /** 检测频率（每 N 帧检测一次，默认 3） */
  checkEveryNFrames?: number
  /** 初始可见性（默认 true） */
  initialVisible?: boolean
}

const DEFAULT_MARGIN = 1.0
const DEFAULT_CHECK_EVERY = 3

export function FrustumCulling({
  children,
  margin = DEFAULT_MARGIN,
  checkEveryNFrames = DEFAULT_CHECK_EVERY,
  initialVisible = true,
}: FrustumCullingProps) {
  const groupRef = useRef<THREE.Group>(null)
  const { camera } = useThree()
  const frustum = useRef(new THREE.Frustum())
  const projScreenMatrix = useRef(new THREE.Matrix4())
  const box = useRef(new THREE.Box3())
  const frameCounter = useRef(0)
  const visibleRef = useRef(initialVisible)

  useFrame(() => {
    if (!groupRef.current) return
    frameCounter.current += 1
    if (frameCounter.current % checkEveryNFrames !== 0) return

    // 计算组的包围盒
    box.current.setFromObject(groupRef.current).expandByScalar(margin)

    // 更新视锥体
    projScreenMatrix.current.multiplyMatrices(camera.projectionMatrix, camera.matrixWorldInverse)
    frustum.current.setFromProjectionMatrix(projScreenMatrix.current)

    // 检测包围盒与视锥体相交
    const intersects = frustum.current.intersectsBox(box.current)
    if (intersects !== visibleRef.current) {
      visibleRef.current = intersects
      groupRef.current.visible = intersects
    }
  })

  return (
    <group ref={groupRef} visible={initialVisible}>
      {children}
    </group>
  )
}
