/**
 * CompletionBurst - 建造完成金光粒子爆发（Task 9.6）
 *
 * 触发时机：建筑 status 从 'building' → 'completed' 转换的瞬间
 * 由父组件 <Building /> 挂载，独立 useFrame 推进动画
 *
 * 视觉：
 *   - 金色（#ffd700）粒子从建筑中心向四周 + 向上飞散
 *   - 持续约 2 秒，前 30% 满亮度，后 70% 线性淡出
 *   - 受重力影响下落，模拟"金光绽放" + "金粉飘散"
 *
 * 实现：
 *   - Points + BufferGeometry + PointsMaterial
 *   - AdditiveBlending 让粒子发光叠加
 *   - 不写深度，避免遮挡建筑
 *
 * 参考 05_3D场景设计.md §15 完成动画
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo, useRef } from 'react'
import { useFrame } from '@react-three/fiber'
import * as THREE from 'three'

interface CompletionBurstProps {
  /** 粒子数量（默认 60） */
  count?: number
  /** 持续时间（秒，默认 2.0） */
  duration?: number
  /** 粒子颜色（默认金色 #ffd700） */
  color?: string
  /** 粒子发射中心高度（建筑高度的一半，默认 1.5） */
  originY?: number
  /** 完成回调（动画播完后由父组件卸载） */
  onComplete?: () => void
}

export function CompletionBurst({
  count = 60,
  duration = 2.0,
  color = '#ffd700',
  originY = 1.5,
  onComplete,
}: CompletionBurstProps) {
  // 初始位置 + 速度（球面随机方向，向上偏置）
  const { positions, velocities } = useMemo(() => {
    const positions = new Float32Array(count * 3)
    const velocities = new Float32Array(count * 3)
    for (let i = 0; i < count; i++) {
      // 球面随机方向（限制 phi 范围让粒子偏向上半空间）
      const theta = Math.random() * Math.PI * 2
      const phi = Math.acos(Math.random() * 0.7 + 0.1)
      const speed = 1.5 + Math.random() * 2.0
      velocities[i * 3] = Math.sin(phi) * Math.cos(theta) * speed
      velocities[i * 3 + 1] = Math.cos(phi) * speed + 1.0 // 上浮偏置
      velocities[i * 3 + 2] = Math.sin(phi) * Math.sin(theta) * speed
      // 初始位置在发射中心
      positions[i * 3] = 0
      positions[i * 3 + 1] = originY
      positions[i * 3 + 2] = 0
    }
    return { positions, velocities }
  }, [count, originY])

  const geometryRef = useRef<THREE.BufferGeometry>(null)
  const materialRef = useRef<THREE.PointsMaterial>(null)
  const elapsedRef = useRef(0)
  const completedRef = useRef(false)

  useFrame((_, delta) => {
    if (!geometryRef.current || !materialRef.current) return
    if (completedRef.current) return

    // 防止暂停后 delta 突变（标签页切换返回时 delta 可能很大）
    const dt = Math.min(delta, 0.05)
    elapsedRef.current += dt
    const t = elapsedRef.current / duration

    if (t >= 1) {
      completedRef.current = true
      materialRef.current.opacity = 0
      onComplete?.()
      return
    }

    // 更新粒子位置（速度 × dt，受重力下落）
    const posAttr = geometryRef.current.attributes.position as THREE.BufferAttribute
    const arr = posAttr.array as Float32Array
    const gravity = 1.5
    for (let i = 0; i < count; i++) {
      arr[i * 3] += velocities[i * 3] * dt
      arr[i * 3 + 1] += (velocities[i * 3 + 1] - gravity * t) * dt
      arr[i * 3 + 2] += velocities[i * 3 + 2] * dt
    }
    posAttr.needsUpdate = true

    // 透明度：前 30% 满亮度，后 70% 线性淡出
    const opacity = t < 0.3 ? 1 : Math.max(0, 1 - (t - 0.3) / 0.7)
    materialRef.current.opacity = opacity
  })

  return (
    <points>
      <bufferGeometry ref={geometryRef}>
        <bufferAttribute
          attach="attributes-position"
          args={[positions, 3]}
          count={count}
        />
      </bufferGeometry>
      <pointsMaterial
        ref={materialRef}
        color={color}
        size={0.18}
        transparent
        opacity={1}
        sizeAttenuation
        depthWrite={false}
        blending={THREE.AdditiveBlending}
      />
    </points>
  )
}
