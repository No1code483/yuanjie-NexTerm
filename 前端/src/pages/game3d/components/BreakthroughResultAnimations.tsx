/**
 * BreakthroughResultAnimations - 突破结果动画（T2.4，02_境界系统设计.md §6 / 12_AI考验机制.md §4.5）
 *
 * 监听 store.breakthroughAnimation 状态，分发到对应粒子动画：
 *   - `success` → BreakthroughSuccess：绿色道光升腾 + 金色粒子爆发（3 秒）
 *   - `failure` → BreakthroughFailure：8 道红色闪电劈下（2 秒，轻雷劫）
 *   - `dropped`  → BreakthroughDropped：16 道红色闪电 + 紫色雷球扩散（4 秒，重雷劫）
 *
 * 触发位置：场景中心 (0, 0, 0)，与祭坛一致。
 * 动画完成后调用 `setBreakthroughAnimation('idle')` 卸载（保留 outcome 显示结果卡）。
 *
 * 渲染位置：Canvas 内（由 Scene.tsx 在 <Buildings /> 之后挂载）。
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useEffect, useMemo, useRef } from 'react'
import { useFrame } from '@react-three/fiber'
import * as THREE from 'three'
import { useGame3DStore } from '../stores/gameStore'

// ============================================================================
// 主组件：监听 animation 状态分发
// ============================================================================

export function BreakthroughResultAnimations() {
  const animation = useGame3DStore((s) => s.breakthroughAnimation)
  const setAnimation = useGame3DStore((s) => s.setBreakthroughAnimation)

  // 用 key 强制每次进入非 idle 状态时重新挂载子组件，确保 useFrame 重新计时
  if (animation === 'idle') return null
  return (
    <BreakthroughAnimationDispatcher
      key={animation}
      kind={animation}
      onComplete={() => setAnimation('idle')}
    />
  )
}

interface DispatcherProps {
  kind: 'success' | 'failure' | 'dropped'
  onComplete: () => void
}

function BreakthroughAnimationDispatcher({ kind, onComplete }: DispatcherProps) {
  if (kind === 'success') return <BreakthroughSuccess onComplete={onComplete} />
  if (kind === 'failure') return <BreakthroughFailure onComplete={onComplete} />
  return <BreakthroughDropped onComplete={onComplete} />
}

// ============================================================================
// 成功动画：绿色道光升腾 + 金色粒子爆发
// ============================================================================

interface SuccessProps {
  onComplete: () => void
}

function BreakthroughSuccess({ onComplete }: SuccessProps) {
  const DURATION_S = 3.0
  const PARTICLE_COUNT = 60

  const beamRef = useRef<THREE.Mesh>(null)
  const geometryRef = useRef<THREE.BufferGeometry>(null)
  const materialRef = useRef<THREE.PointsMaterial>(null)
  const elapsedRef = useRef(0)
  const completedRef = useRef(false)

  // 粒子初始位置（中心点）+ 速度（球面随机方向，向上偏置）
  const { positions, velocities } = useMemo(() => {
    const positions = new Float32Array(PARTICLE_COUNT * 3)
    const velocities = new Float32Array(PARTICLE_COUNT * 3)
    for (let i = 0; i < PARTICLE_COUNT; i++) {
      const theta = Math.random() * Math.PI * 2
      const phi = Math.acos(Math.random() * 0.7 + 0.1)
      const speed = 1.0 + Math.random() * 2.5
      velocities[i * 3] = Math.sin(phi) * Math.cos(theta) * speed
      velocities[i * 3 + 1] = Math.cos(phi) * speed + 1.5
      velocities[i * 3 + 2] = Math.sin(phi) * Math.sin(theta) * speed
      positions[i * 3] = 0
      positions[i * 3 + 1] = 2.0
      positions[i * 3 + 2] = 0
    }
    return { positions, velocities }
  }, [])

  useEffect(() => {
    completedRef.current = false
    elapsedRef.current = 0
  }, [])

  useFrame((_, delta) => {
    if (completedRef.current) return
    const dt = Math.min(delta, 0.05)
    elapsedRef.current += dt
    const t = elapsedRef.current / DURATION_S

    // 光柱：前 50% 高度从 0 升到 12，后 50% 衰减
    if (beamRef.current) {
      const height = t < 0.5 ? t * 24 : (1 - t) * 24
      beamRef.current.scale.y = Math.max(0.01, height)
      const mat = beamRef.current.material as THREE.MeshBasicMaterial
      mat.opacity = Math.max(0, 0.7 * (1 - t))
    }

    // 粒子运动 + 重力下落
    if (geometryRef.current && materialRef.current) {
      const posAttr = geometryRef.current.attributes.position as THREE.BufferAttribute
      const arr = posAttr.array as Float32Array
      const gravity = 1.2
      for (let i = 0; i < PARTICLE_COUNT; i++) {
        arr[i * 3] += velocities[i * 3] * dt
        arr[i * 3 + 1] += (velocities[i * 3 + 1] - gravity * t) * dt
        arr[i * 3 + 2] += velocities[i * 3 + 2] * dt
      }
      posAttr.needsUpdate = true
      // 前 30% 满亮度，后 70% 线性淡出
      materialRef.current.opacity = t < 0.3 ? 1 : Math.max(0, 1 - (t - 0.3) / 0.7)
    }

    if (t >= 1) {
      completedRef.current = true
      onComplete()
    }
  })

  return (
    <group position={[0, 0, 0]}>
      {/* 中心绿色光柱（AdditiveBlending 自发光） */}
      <mesh ref={beamRef} position={[0, 6, 0]}>
        <cylinderGeometry args={[0.3, 0.6, 1, 16, 1, true]} />
        <meshBasicMaterial
          color="#00FF00"
          transparent
          opacity={0.7}
          side={THREE.DoubleSide}
          blending={THREE.AdditiveBlending}
          depthWrite={false}
        />
      </mesh>

      {/* 金色粒子爆发 */}
      <points>
        <bufferGeometry ref={geometryRef}>
          <bufferAttribute
            attach="attributes-position"
            args={[positions, 3]}
            count={PARTICLE_COUNT}
          />
        </bufferGeometry>
        <pointsMaterial
          ref={materialRef}
          color="#ffd700"
          size={0.18}
          transparent
          opacity={1}
          sizeAttenuation
          depthWrite={false}
          blending={THREE.AdditiveBlending}
        />
      </points>
    </group>
  )
}

// ============================================================================
// 失败动画：8 道红色闪电从天空劈下（轻雷劫）
// ============================================================================

interface FailureProps {
  onComplete: () => void
  /** 闪电数量（dropped 时覆盖为 16） */
  boltCount?: number
  /** 持续时间（秒） */
  duration?: number
}

function BreakthroughFailure({
  onComplete,
  boltCount = 8,
  duration = 2.0,
}: FailureProps) {
  // 每道闪电的水平位置 + 劈下时延（错峰）
  const bolts = useMemo(() => {
    return Array.from({ length: boltCount }, (_, i) => {
      const angle = (i / boltCount) * Math.PI * 2 + Math.random() * 0.3
      const radius = 1.5 + Math.random() * 2.5
      return {
        x: Math.cos(angle) * radius,
        z: Math.sin(angle) * radius,
        delay: (i / boltCount) * 0.7, // 0~0.7 秒错峰
        length: 12, // 闪电从 y=12 劈到 y=0
      }
    })
  }, [boltCount])

  const boltRefs = useRef<(THREE.Mesh | null)[]>([])
  const elapsedRef = useRef(0)
  const completedRef = useRef(false)

  useEffect(() => {
    completedRef.current = false
    elapsedRef.current = 0
  }, [])

  useFrame((_, delta) => {
    if (completedRef.current) return
    const dt = Math.min(delta, 0.05)
    elapsedRef.current += dt
    const elapsed = elapsedRef.current

    bolts.forEach((bolt, i) => {
      const mesh = boltRefs.current[i]
      if (!mesh) return
      // 每道闪电在自己的生命周期内：1.2 秒（前 0.1 秒升起，中间 0.8 秒满亮度，最后 0.3 秒闪烁淡出）
      const localT = (elapsed - bolt.delay) / 1.2
      if (localT < 0) {
        mesh.visible = false
        return
      }
      mesh.visible = true
      const mat = mesh.material as THREE.MeshBasicMaterial
      if (localT > 1) {
        mesh.visible = false
        return
      }
      // 前 10% 升起 → 80% 满亮度（带闪烁）→ 10% 淡出
      if (localT < 0.1) {
        mesh.scale.y = Math.max(0.01, localT / 0.1)
        mat.opacity = 0.9
      } else if (localT < 0.9) {
        mesh.scale.y = 1
        // 闪烁：每 0.05 秒随机透明度
        mat.opacity = 0.5 + Math.random() * 0.5
      } else {
        mesh.scale.y = 1
        mat.opacity = Math.max(0, (1 - localT) / 0.1 * 0.9)
      }
    })

    if (elapsed >= duration) {
      completedRef.current = true
      onComplete()
    }
  })

  return (
    <group position={[0, 0, 0]}>
      {bolts.map((bolt, i) => (
        <mesh
          key={i}
          ref={(el) => {
            boltRefs.current[i] = el
          }}
          position={[bolt.x, bolt.length / 2, bolt.z]}
          visible={false}
        >
          <cylinderGeometry args={[0.05, 0.05, bolt.length, 4, 1, true]} />
          <meshBasicMaterial
            color="#FF0000"
            transparent
            opacity={0.9}
            side={THREE.DoubleSide}
            blending={THREE.AdditiveBlending}
            depthWrite={false}
          />
        </mesh>
      ))}
    </group>
  )
}

// ============================================================================
// 跌落动画：16 道红色闪电 + 紫色雷球扩散（重雷劫）
// ============================================================================

function BreakthroughDropped({ onComplete }: { onComplete: () => void }) {
  return (
    <>
      <BreakthroughFailure
        onComplete={() => {
          /* 不在此处完成，由外层雷球控制总时长 */
        }}
        boltCount={16}
        duration={4.0}
      />
      <ThunderSphereExpansion onComplete={onComplete} />
    </>
  )
}

/** 紫色雷球扩散动画（dropped 专用） */
function ThunderSphereExpansion({ onComplete }: { onComplete: () => void }) {
  const DURATION_S = 4.0
  const sphereRef = useRef<THREE.Mesh>(null)
  const elapsedRef = useRef(0)
  const completedRef = useRef(false)

  useEffect(() => {
    completedRef.current = false
    elapsedRef.current = 0
  }, [])

  useFrame((_, delta) => {
    if (completedRef.current) return
    const dt = Math.min(delta, 0.05)
    elapsedRef.current += dt
    const t = elapsedRef.current / DURATION_S

    if (sphereRef.current) {
      // 半径从 0.5 扩散到 6
      const scale = 0.5 + t * 5.5
      sphereRef.current.scale.set(scale, scale, scale)
      const mat = sphereRef.current.material as THREE.MeshBasicMaterial
      // 前 30% 渐显，后 70% 渐隐
      mat.opacity = t < 0.3 ? t / 0.3 * 0.4 : Math.max(0, (1 - t) / 0.7 * 0.4)
    }

    if (t >= 1) {
      completedRef.current = true
      onComplete()
    }
  })

  return (
    <mesh ref={sphereRef} position={[0, 2, 0]}>
      <sphereGeometry args={[1, 16, 16]} />
      <meshBasicMaterial
        color="#8b5cf6"
        transparent
        opacity={0}
        wireframe
        side={THREE.DoubleSide}
        blending={THREE.AdditiveBlending}
        depthWrite={false}
      />
    </mesh>
  )
}
