/**
 * EnvironmentDecor - 场景环境装饰（树木、石头、竹林）
 *
 * 修仙风格环境元素：
 *   - 松树/竹：中式修仙场景常见植被
 *   - 太湖石：园林假山石
 *   - 散布于地图边缘，不阻挡建筑区域
 *
 * 设计原则：
 *   - 低多边形风格，与程序化建筑视觉一致
 *   - 所有装饰物 receiveShadow + castShadow
 *   - 数量控制在 50 个以内，避免性能问题
 *
 * change-id: game-3d-rebuild-refactor-modeling-opt
 */
import { useMemo } from 'react'

const MAP_SIZE = 64

function seededRandom(seed: number): () => number {
  let s = seed
  return () => {
    s = (s * 16807 + 0) % 2147483647
    return (s - 1) / 2147483646
  }
}

/** 单棵松树 */
function PineTree({ position, scale = 1 }: { position: [number, number, number]; scale?: number }) {
  return (
    <group position={position} scale={[scale, scale, scale]}>
      <mesh position={[0, 0.5, 0]} castShadow>
        <cylinderGeometry args={[0.08, 0.12, 1.0, 6]} />
        <meshStandardMaterial color="#5a3a1a" roughness={0.9} />
      </mesh>
      <mesh position={[0, 1.1, 0]} castShadow>
        <coneGeometry args={[0.5, 0.8, 6]} />
        <meshStandardMaterial color="#2d5a1a" roughness={0.9} />
      </mesh>
      <mesh position={[0, 1.5, 0]} castShadow>
        <coneGeometry args={[0.35, 0.6, 6]} />
        <meshStandardMaterial color="#3a6b2a" roughness={0.9} />
      </mesh>
      <mesh position={[0, 1.85, 0]} castShadow>
        <coneGeometry args={[0.2, 0.5, 6]} />
        <meshStandardMaterial color="#4a8b3a" roughness={0.9} />
      </mesh>
    </group>
  )
}

/** 单根竹子 */
function Bamboo({ position, height = 2.5 }: { position: [number, number, number]; height?: number }) {
  return (
    <group position={position}>
      <mesh position={[0, height / 2, 0]} castShadow>
        <cylinderGeometry args={[0.06, 0.08, height, 8]} />
        <meshStandardMaterial color="#4a7a2a" roughness={0.7} metalness={0.05} />
      </mesh>
      {Array.from({ length: Math.floor(height / 0.5) }, (_, i) => (
        <mesh key={i} position={[0, (i + 1) * 0.5, 0]}>
          <cylinderGeometry args={[0.09, 0.09, 0.04, 8]} />
          <meshStandardMaterial color="#3a6a1a" roughness={0.7} />
        </mesh>
      ))}
      <mesh position={[0, height + 0.2, 0]} castShadow>
        <coneGeometry args={[0.25, 0.5, 6]} />
        <meshStandardMaterial color="#3a7a1a" roughness={0.8} />
      </mesh>
    </group>
  )
}

/** 太湖石 */
function TaihuRock({ position, scale = 1 }: { position: [number, number, number]; scale?: number }) {
  return (
    <group position={position} scale={[scale, scale, scale]}>
      <mesh position={[0, 0.3, 0]} castShadow receiveShadow>
        <sphereGeometry args={[0.4, 5, 4]} />
        <meshStandardMaterial color="#7a7a7a" roughness={0.8} metalness={0.1} />
      </mesh>
      <mesh position={[0.2, 0.5, -0.1]} castShadow>
        <sphereGeometry args={[0.25, 5, 4]} />
        <meshStandardMaterial color="#8a8a8a" roughness={0.75} metalness={0.1} />
      </mesh>
      <mesh position={[-0.15, 0.55, 0.15]} castShadow>
        <sphereGeometry args={[0.2, 5, 4]} />
        <meshStandardMaterial color="#6a6a6a" roughness={0.85} metalness={0.1} />
      </mesh>
      <mesh position={[0, 0.05, 0]} receiveShadow>
        <cylinderGeometry args={[0.35, 0.45, 0.1, 6]} />
        <meshStandardMaterial color="#5a5a5a" roughness={0.9} />
      </mesh>
    </group>
  )
}

/**
 * 场景环境装饰组件
 * 在地图边缘和空地散布树木、竹子和石头
 */
export function EnvironmentDecor() {
  const decorations = useMemo(() => {
    const rng = seededRandom(2024)
    const items: { type: 'pine' | 'bamboo' | 'rock'; position: [number, number, number]; scale: number }[] = []

    for (let i = 0; i < 20; i++) {
      const edge = Math.floor(rng() * 4)
      let x: number, z: number
      const margin = 2 + rng() * 6
      switch (edge) {
        case 0: x = margin; z = rng() * MAP_SIZE; break
        case 1: x = MAP_SIZE - margin; z = rng() * MAP_SIZE; break
        case 2: x = rng() * MAP_SIZE; z = margin; break
        default: x = rng() * MAP_SIZE; z = MAP_SIZE - margin; break
      }
      items.push({ type: 'pine', position: [x, 0, z], scale: 0.5 + rng() * 0.8 })
    }

    for (let i = 0; i < 12; i++) {
      const corner = Math.floor(rng() * 4)
      let x: number, z: number
      const clusterR = 3
      switch (corner) {
        case 0: x = clusterR + rng() * 4; z = clusterR + rng() * 4; break
        case 1: x = MAP_SIZE - clusterR - rng() * 4; z = clusterR + rng() * 4; break
        case 2: x = clusterR + rng() * 4; z = MAP_SIZE - clusterR - rng() * 4; break
        default: x = MAP_SIZE - clusterR - rng() * 4; z = MAP_SIZE - clusterR - rng() * 4; break
      }
      items.push({ type: 'bamboo', position: [x, 0, z], scale: 0.6 + rng() * 0.5 })
    }

    for (let i = 0; i < 8; i++) {
      const x = 8 + rng() * (MAP_SIZE - 16)
      const z = 8 + rng() * (MAP_SIZE - 16)
      items.push({ type: 'rock', position: [x, 0, z], scale: 0.5 + rng() * 0.7 })
    }

    return items
  }, [])

  return (
    <group>
      {decorations.map((item, i) => {
        switch (item.type) {
          case 'pine':
            return <PineTree key={`pine-${i}`} position={item.position} scale={item.scale} />
          case 'bamboo':
            return <Bamboo key={`bamboo-${i}`} position={item.position} height={2.0 + item.scale * 1.5} />
          case 'rock':
            return <TaihuRock key={`rock-${i}`} position={item.position} scale={item.scale} />
        }
      })}
    </group>
  )
}