/**
 * Camera3D - 相机轨道控制 + 预设切换插值（Task 8.3）
 *
 * 参考 05_3D场景设计.md §4.2 OrbitControls 配置 + §4.3 预设切换
 *
 * 实现：
 *   - Drei <OrbitControls> 注册为默认相机控制
 *   - 监听 gameStore.cameraPreset 变化
 *   - useFrame 中 lerp 插值平滑过渡（约 1 秒到位）
 *   - 阻尼启用，旋转/缩放有惯性
 *
 * 限制：
 *   - minDistance=5 防止穿入建筑
 *   - maxDistance=50 防止过远失焦
 *   - maxPolarAngle=π/2.1 防止视角穿到地面以下
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useEffect, useRef } from 'react'
import { useFrame, useThree } from '@react-three/fiber'
import { OrbitControls } from '@react-three/drei'
import type { OrbitControls as OrbitControlsImpl } from 'three-stdlib'
import * as THREE from 'three'
import { useGame3DStore } from '../stores/gameStore'
import { CAMERA_PRESETS } from '../hooks/useCamera'

/** 插值系数：0.08 表示每帧靠近目标 8%，约 1 秒到位（60fps） */
const LERP_FACTOR = 0.08
/** 到达阈值：距离小于此值时停止插值 */
const ARRIVAL_THRESHOLD = 0.1

export function Camera3D() {
  const controlsRef = useRef<OrbitControlsImpl | null>(null)
  const { camera } = useThree()
  const cameraPreset = useGame3DStore((s) => s.cameraPreset)
  // 放置/移动模式下禁用旋转，避免左键拖拽与点击放置冲突
  const placementMode = useGame3DStore((s) => s.placementMode)

  /** 当前目标位置（切换中非空，到位后置 null） */
  const targetPos = useRef<THREE.Vector3 | null>(null)
  /** 当前视线焦点（切换中非空，到位后置 null） */
  const targetLookAt = useRef<THREE.Vector3 | null>(null)

  // 监听预设变化，设置插值目标
  useEffect(() => {
    const preset = CAMERA_PRESETS[cameraPreset]
    if (!preset) return
    targetPos.current = preset.position.clone()
    targetLookAt.current = preset.target.clone()
  }, [cameraPreset])

  // 每帧插值，直到到达目标
  useFrame(() => {
    if (!targetPos.current || !targetLookAt.current || !controlsRef.current) return
    camera.position.lerp(targetPos.current, LERP_FACTOR)
    controlsRef.current.target.lerp(targetLookAt.current, LERP_FACTOR)
    controlsRef.current.update()
    // 到达阈值内停止插值
    if (camera.position.distanceTo(targetPos.current) < ARRIVAL_THRESHOLD) {
      targetPos.current = null
      targetLookAt.current = null
    }
  })

  return (
    <OrbitControls
      ref={controlsRef}
      makeDefault
      enableRotate={placementMode === 'idle'}
      enableZoom
      enablePan
      minDistance={5}
      maxDistance={50}
      panSpeed={0.5}
      rotateSpeed={0.8}
      zoomSpeed={1.0}
      maxPolarAngle={Math.PI / 2.1}
      minPolarAngle={0}
      enableDamping
      dampingFactor={0.08}
      target={[0, 0, 0]}
    />
  )
}
