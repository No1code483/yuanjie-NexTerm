/**
 * ScaffoldFader - 脚手架淡出（Task 9.6）
 *
 * 触发时机：建筑 status 从 'building' → 'completed' 转换的瞬间
 * 由父组件 <Building /> 挂载，独立 useFrame 推进动画
 *
 * 视觉：
 *   - 黄色（#fbbf24）线框立方体，比建筑略大
 *   - 从 initialOpacity 线性淡出到 0，模拟"脚手架拆除"过程
 *   - 持续约 1.5 秒
 *
 * 几何：
 *   - 立方体 12 条边（lineSegments）
 *   - 4 个顶面 + 4 个底面 + 4 条竖边
 *
 * 说明：
 *   - House3D 等建筑组件内部 status='building' 时也显示脚手架（硬切换）
 *   - 本组件负责"完成瞬间的过渡脚手架"，与建筑内部脚手架互补：
 *     切换瞬间建筑内部脚手架消失（因 status='completed'），
 *     本组件立刻接管显示外置脚手架并平滑淡出
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo, useRef } from 'react'
import { useFrame } from '@react-three/fiber'
import * as THREE from 'three'

interface ScaffoldFaderProps {
  /** 建筑包围盒尺寸 [w, h, d] */
  size: [number, number, number]
  /** 持续时间（秒，默认 1.5） */
  duration?: number
  /** 初始透明度（默认 0.5） */
  initialOpacity?: number
  /** 完成回调 */
  onComplete?: () => void
}

export function ScaffoldFader({
  size,
  duration = 1.5,
  initialOpacity = 0.5,
  onComplete,
}: ScaffoldFaderProps) {
  const materialRef = useRef<THREE.LineBasicMaterial>(null)
  const elapsedRef = useRef(0)
  const completedRef = useRef(false)

  // 线段几何：立方体 12 条边，比建筑略大 0.15 米留空隙
  const geometry = useMemo(() => {
    const [w, h, d] = size
    const hw = w / 2 + 0.15
    const hh = h / 2 + 0.15
    const hd = d / 2 + 0.15
    const cy = h + 0.15 // 中心抬升到建筑高度一半
    const verts: [number, number, number][] = [
      [-hw, cy - hh, -hd], [hw, cy - hh, -hd],
      [-hw, cy + hh, -hd], [hw, cy + hh, -hd],
      [-hw, cy - hh, hd], [hw, cy - hh, hd],
      [-hw, cy + hh, hd], [hw, cy + hh, hd],
    ]
    // 12 条边：底面 4 + 顶面 4 + 4 竖边
    const edges: [number, number][] = [
      [0, 1], [1, 3], [3, 2], [2, 0], // 底面
      [4, 5], [5, 7], [7, 6], [6, 4], // 顶面
      [0, 4], [1, 5], [2, 6], [3, 7], // 4 竖边
    ]
    const positions = new Float32Array(edges.length * 2 * 3)
    edges.forEach(([a, b], i) => {
      const va = verts[a]
      const vb = verts[b]
      positions[i * 6] = va[0]
      positions[i * 6 + 1] = va[1]
      positions[i * 6 + 2] = va[2]
      positions[i * 6 + 3] = vb[0]
      positions[i * 6 + 4] = vb[1]
      positions[i * 6 + 5] = vb[2]
    })
    const g = new THREE.BufferGeometry()
    g.setAttribute('position', new THREE.BufferAttribute(positions, 3))
    return g
  }, [size])

  useFrame((_, delta) => {
    if (!materialRef.current || completedRef.current) return
    const dt = Math.min(delta, 0.05)
    elapsedRef.current += dt
    const t = elapsedRef.current / duration

    if (t >= 1) {
      materialRef.current.opacity = 0
      completedRef.current = true
      onComplete?.()
      return
    }
    materialRef.current.opacity = initialOpacity * (1 - t)
  })

  return (
    <lineSegments geometry={geometry}>
      <lineBasicMaterial
        ref={materialRef}
        color="#fbbf24"
        transparent
        opacity={initialOpacity}
        depthWrite={false}
      />
    </lineSegments>
  )
}
