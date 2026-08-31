/**
 * SpiritParticles - 灵气粒子与场景光点
 *
 * 修仙风格特效：
 *   - 灵气粒子：半透明发光球体，缓慢上升 + 水平漂移，循环再生
 *   - 场景光点：固定位置的小型发光点，散布于场景中，模拟灵气充盈
 *
 * 技术实现：
 *   - useFrame 驱动粒子动画（无外部依赖）
 *   - 所有粒子 depthWrite=false 避免深度排序问题
 *   - 粒子数量控制在 60 个以内，性能友好
 *
 * change-id: game-3d-rebuild-refactor-modeling-opt
 */
import { useRef, useMemo } from 'react'
import { useFrame } from '@react-three/fiber'
import * as THREE from 'three'

const MAP_SIZE = 64
const PARTICLE_COUNT = 40
const LIGHT_POINT_COUNT = 20

interface Particle {
  position: THREE.Vector3
  baseY: number
  speed: number
  phase: number
  amplitude: number
  size: number
}

/** 粒子系统 */
function FloatingParticles() {
  const meshRef = useRef<THREE.InstancedMesh>(null)
  const dummy = useMemo(() => new THREE.Object3D(), [])

  const particles = useMemo<Particle[]>(() => {
    const items: Particle[] = []
    for (let i = 0; i < PARTICLE_COUNT; i++) {
      const seed = (i * 137 + 42) % 2147483647
      const rng = () => {
        const s = (seed * 16807 + i * 7) % 2147483647
        return ((s - 1) / 2147483646 + 0.5) % 1
      }
      items.push({
        position: new THREE.Vector3(rng() * MAP_SIZE, 1 + rng() * 8, rng() * MAP_SIZE),
        baseY: 1 + rng() * 8,
        speed: 0.2 + rng() * 0.6,
        phase: rng() * Math.PI * 2,
        amplitude: 0.3 + rng() * 1.5,
        size: 0.08 + rng() * 0.2,
      })
    }
    return items
  }, [])

  useFrame((_, _delta) => {
    if (!meshRef.current) return
    const time = performance.now() * 0.001

    for (let i = 0; i < PARTICLE_COUNT; i++) {
      const p = particles[i]
      const y = p.baseY + Math.sin(time * p.speed + p.phase) * p.amplitude
      const x = p.position.x + Math.cos(time * p.speed * 0.7 + p.phase) * 0.5
      const z = p.position.z + Math.sin(time * p.speed * 0.5 + p.phase + 1) * 0.5

      dummy.position.set(x, y, z)
      dummy.scale.setScalar(p.size)
      dummy.updateMatrix()
      meshRef.current.setMatrixAt(i, dummy.matrix)
    }
    meshRef.current.instanceMatrix.needsUpdate = true
  })

  return (
    <instancedMesh ref={meshRef} args={[undefined, undefined, PARTICLE_COUNT]}>
      <sphereGeometry args={[1, 6, 4]} />
      <meshBasicMaterial
        color="#88ccff"
        transparent
        opacity={0.4}
        depthWrite={false}
        blending={THREE.AdditiveBlending}
      />
    </instancedMesh>
  )
}

/** 场景光点（固定位置发光点） */
function AmbientLightPoints() {
  const points = useMemo(() => {
    const items: [number, number, number, number][] = []
    const rng = (seed: number) => {
      const s = (seed * 16807 + 0) % 2147483647
      return (s - 1) / 2147483646
    }
    for (let i = 0; i < LIGHT_POINT_COUNT; i++) {
      const seed = i * 7919 + 137
      const x = rng(seed) * MAP_SIZE
      const z = rng(seed + 1) * MAP_SIZE
      const y = 0.5 + rng(seed + 2) * 5
      const size = 0.05 + rng(seed + 3) * 0.15
      items.push([x, y, z, size])
    }
    return items
  }, [])

  return (
    <group>
      {points.map(([x, y, z, size], i) => (
        <mesh key={i} position={[x, y, z]}>
          <sphereGeometry args={[size, 4, 4]} />
          <meshBasicMaterial
            color={i % 3 === 0 ? '#ffd700' : i % 3 === 1 ? '#88ccff' : '#aaddaa'}
            transparent
            opacity={0.3 + (i % 5) * 0.08}
            depthWrite={false}
          />
        </mesh>
      ))}
    </group>
  )
}

/**
 * 灵气粒子系统
 * 包含浮动粒子和场景光点两个子系统
 */
export function SpiritParticles() {
  return (
    <group>
      <FloatingParticles />
      <AmbientLightPoints />
    </group>
  )
}