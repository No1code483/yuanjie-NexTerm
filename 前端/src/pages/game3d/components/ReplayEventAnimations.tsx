/**
 * ReplayEventAnimations - 时间轴回放事件动画组件集合（T1.6）
 *
 * 5 种事件动画（spec 11_时间轴回放.md §7）：
 *   - EventBuild     破土生长：土黄色粒子向上喷射
 *   - EventComplete  金光脉冲：金色光环向外扩散
 *   - EventUpgrade   光柱升腾：蓝紫色垂直光柱
 *   - EventDemolish  碎裂消散：红色碎片向外飞散
 *   - EventMove      抬升平移：抬升 + 残影位移
 *
 * 所有动画使用 useFrame 驱动 + 时间衰减，约 1.5 秒完成。
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo, useRef } from 'react'
import { useFrame } from '@react-three/fiber'
import * as THREE from 'three'

/** 动画总时长（秒）。 */
const DURATION_S = 1.5

/** 标准化进度 t = elapsed / DURATION（0 → 1）。 */
function progress(elapsed: number): number {
  return Math.min(1, elapsed / DURATION_S)
}

// ============================================================================
// 1. EventBuild - 破土生长：土黄色粒子向上喷射
// ============================================================================

interface EventBuildProps {
  position: [number, number, number]
  name?: string
}

export function EventBuild({ position, name }: EventBuildProps) {
  const groupRef = useRef<THREE.Group>(null)
  const nameRef = useRef<THREE.Mesh>(null)
  const startRef = useRef(performance.now() / 1000)

  // 预生成 12 个粒子的随机水平偏移
  const particles = useMemo(
    () =>
      Array.from({ length: 12 }, () => ({
        offsetX: (Math.random() - 0.5) * 1.5,
        offsetZ: (Math.random() - 0.5) * 1.5,
        heightScale: 0.8 + Math.random() * 1.2,
      })),
    [],
  )

  useFrame(() => {
    const elapsed = performance.now() / 1000 - startRef.current
    const t = progress(elapsed)
    if (groupRef.current) {
      // 整体透明度衰减
      groupRef.current.children.forEach((child, i) => {
        const mesh = child as THREE.Mesh
        const mat = mesh.material as THREE.MeshBasicMaterial
        if (mat) {
          mat.opacity = Math.max(0, 1 - t)
        }
        // 粒子向上移动
        if (i > 0 && particles[i - 1]) {
          const p = particles[i - 1]
          mesh.position.y = p.heightScale * t * 3
          mesh.position.x = p.offsetX * t
          mesh.position.z = p.offsetZ * t
        }
      })
    }
    if (nameRef.current) {
      const mat = nameRef.current.material as THREE.MeshBasicMaterial
      if (mat) mat.opacity = Math.max(0, 1 - t)
    }
  })

  return (
    <group ref={groupRef} position={position}>
      {/* 中心土堆 */}
      <mesh position={[0, 0.2, 0]}>
        <coneGeometry args={[0.8, 0.6, 6]} />
        <meshBasicMaterial color="#8b6f47" transparent opacity={1} />
      </mesh>
      {/* 12 个土黄色粒子 */}
      {particles.map((_, i) => (
        <mesh key={i}>
          <sphereGeometry args={[0.15, 6, 6]} />
          <meshBasicMaterial color="#a67c52" transparent opacity={1} />
        </mesh>
      ))}
      {/* 名称标签：初始 opacity 1.0，由 useFrame 更新衰减 */}
      {name && (
        <mesh ref={nameRef} position={[0, 3, 0]}>
          <planeGeometry args={[2, 0.4]} />
          <meshBasicMaterial color="#fbbf24" transparent opacity={1} side={2} />
        </mesh>
      )}
    </group>
  )
}

// ============================================================================
// 2. EventComplete - 金光脉冲：金色光环向外扩散
// ============================================================================

interface EventCompleteProps {
  position: [number, number, number]
}

export function EventComplete({ position }: EventCompleteProps) {
  const ring1Ref = useRef<THREE.Mesh>(null)
  const ring2Ref = useRef<THREE.Mesh>(null)
  const startRef = useRef(performance.now() / 1000)

  useFrame(() => {
    const elapsed = performance.now() / 1000 - startRef.current
    const t = progress(elapsed)
    if (ring1Ref.current) {
      const scale = 1 + t * 5
      ring1Ref.current.scale.set(scale, scale, scale)
      const mat = ring1Ref.current.material as THREE.MeshBasicMaterial
      mat.opacity = Math.max(0, 0.8 * (1 - t))
    }
    if (ring2Ref.current) {
      const scale = 1 + t * 3
      ring2Ref.current.scale.set(scale, scale, scale)
      const mat = ring2Ref.current.material as THREE.MeshBasicMaterial
      mat.opacity = Math.max(0, 0.6 * (1 - t))
    }
  })

  return (
    <group position={position}>
      <mesh ref={ring1Ref} position={[0, 0.1, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[0.5, 0.8, 32]} />
        <meshBasicMaterial color="#ffd700" transparent opacity={0.8} side={2} />
      </mesh>
      <mesh ref={ring2Ref} position={[0, 0.1, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[0.3, 0.6, 32]} />
        <meshBasicMaterial color="#ffea80" transparent opacity={0.6} side={2} />
      </mesh>
    </group>
  )
}

// ============================================================================
// 3. EventUpgrade - 光柱升腾：蓝紫色垂直光柱
// ============================================================================

interface EventUpgradeProps {
  position: [number, number, number]
}

export function EventUpgrade({ position }: EventUpgradeProps) {
  const beamRef = useRef<THREE.Mesh>(null)
  const startRef = useRef(performance.now() / 1000)

  useFrame(() => {
    const elapsed = performance.now() / 1000 - startRef.current
    const t = progress(elapsed)
    if (beamRef.current) {
      // 光柱高度从 0 升到 8，再衰减
      const height = t < 0.5 ? t * 16 : (1 - t) * 16
      beamRef.current.scale.y = Math.max(0.01, height)
      const mat = beamRef.current.material as THREE.MeshBasicMaterial
      mat.opacity = Math.max(0, 0.6 * (1 - t))
    }
  })

  return (
    <group position={position}>
      <mesh ref={beamRef} position={[0, 4, 0]}>
        <cylinderGeometry args={[0.4, 0.4, 1, 16, 1, true]} />
        <meshBasicMaterial
          color="#8b5cf6"
          transparent
          opacity={0.6}
          side={THREE.DoubleSide}
          blending={THREE.AdditiveBlending}
        />
      </mesh>
    </group>
  )
}

// ============================================================================
// 4. EventDemolish - 碎裂消散：红色碎片向外飞散
// ============================================================================

interface EventDemolishProps {
  position: [number, number, number]
}

export function EventDemolish({ position }: EventDemolishProps) {
  const groupRef = useRef<THREE.Group>(null)
  const startRef = useRef(performance.now() / 1000)

  // 预生成 8 个碎片的初始方向
  const fragments = useMemo(
    () =>
      Array.from({ length: 8 }, (_, i) => {
        const angle = (i / 8) * Math.PI * 2
        return {
          dirX: Math.cos(angle),
          dirZ: Math.sin(angle),
          dirY: 0.5 + Math.random() * 0.5,
        }
      }),
    [],
  )

  useFrame(() => {
    const elapsed = performance.now() / 1000 - startRef.current
    const t = progress(elapsed)
    if (groupRef.current) {
      groupRef.current.children.forEach((child, i) => {
        if (i >= fragments.length) return
        const f = fragments[i]
        const mesh = child as THREE.Mesh
        mesh.position.x = f.dirX * t * 3
        mesh.position.y = f.dirY * t * 3 - t * t * 2 // 重力下落
        mesh.position.z = f.dirZ * t * 3
        mesh.rotation.x = t * Math.PI * 2
        mesh.rotation.z = t * Math.PI * 3
        const mat = mesh.material as THREE.MeshBasicMaterial
        mat.opacity = Math.max(0, 1 - t)
      })
    }
  })

  return (
    <group ref={groupRef} position={position}>
      {fragments.map((_, i) => (
        <mesh key={i}>
          <boxGeometry args={[0.3, 0.3, 0.3]} />
          <meshBasicMaterial color="#f87171" transparent opacity={1} />
        </mesh>
      ))}
    </group>
  )
}

// ============================================================================
// 5. EventMove - 抬升平移：抬升 + 残影位移轨迹
// ============================================================================

interface EventMoveProps {
  position: [number, number, number]
}

export function EventMove({ position }: EventMoveProps) {
  const ghostRef = useRef<THREE.Mesh>(null)
  const startRef = useRef(performance.now() / 1000)

  useFrame(() => {
    const elapsed = performance.now() / 1000 - startRef.current
    const t = progress(elapsed)
    if (ghostRef.current) {
      // 残影：向上抬升 + 水平位移
      ghostRef.current.position.y = 1 + t * 1.5
      ghostRef.current.position.x = t * 2
      const mat = ghostRef.current.material as THREE.MeshBasicMaterial
      mat.opacity = Math.max(0, 0.6 * (1 - t))
    }
  })

  return (
    <group position={position}>
      <mesh ref={ghostRef} position={[0, 1, 0]}>
        <boxGeometry args={[1.5, 1.2, 1.5]} />
        <meshBasicMaterial
          color="#6bb6ff"
          transparent
          opacity={0.6}
          wireframe
        />
      </mesh>
    </group>
  )
}
