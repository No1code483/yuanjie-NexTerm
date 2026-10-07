/**
 * PostFX - 后处理链（Task 8.5）
 *
 * 参考 05_3D场景设计.md §9 后处理：
 *   - Bloom：辉光，高亮区域柔和光晕
 *   - SSAO：环境光遮蔽，强化体积感
 *   - Vignette：暗角，聚焦视线
 *   - ACESFilmic 色调映射：在 Canvas gl 配置中开启
 *
 * 性能降级（参考 §10.1）：
 *   - isLowPerf=true 时完全关闭后处理
 *   - viewMode='low' 时仅保留 Vignette
 *   - viewMode='medium' 时 Bloom + Vignette
 *   - viewMode='high' 时 Bloom + SSAO + Vignette
 *
 * 注意（postprocessing v2.19 API 变化）：
 *   - EffectComposer 的 disableNormalPass 改为 enableNormalPass（逻辑取反）
 *   - SSAO 新增 world* 距离感知参数（0 表示禁用）
 *   - SSAO 的 color 属性期望 THREE.Color 实例，故去掉 color（用默认黑色）
 *
 * change-id: game-3d-rebuild-refactor
 */
import type { ReactElement } from 'react'
import { EffectComposer, Bloom, SSAO, Vignette } from '@react-three/postprocessing'
import { useGame3DStore } from '../stores/gameStore'

/** Bloom 参数（05 文档 §9.1） */
const BLOOM_PROPS = {
  intensity: 0.8,
  luminanceThreshold: 0.5,
  luminanceSmoothing: 0.4,
  mipmapBlur: true,
  radius: 0.6,
}

/** SSAO 参数（05 文档 §9.2 + postprocessing v2.19 新增 world* 必填项） */
const SSAO_PROPS = {
  radius: 0.1,
  intensity: 10,
  luminanceInfluence: 0.7,
  // postprocessing v2.19 新增的距离感知参数（0 表示禁用距离感知）
  worldDistanceThreshold: 0,
  worldDistanceFalloff: 0,
  worldProximityThreshold: 0,
  worldProximityFalloff: 0,
}

/** Vignette 参数（05 文档 §9.4） */
const VIGNETTE_PROPS = {
  offset: 0.3,
  darkness: 0.6,
}

/**
 * 后处理组件
 *
 * EffectComposer children 不能含 false/null，故用数组构造 effects 列表
 */
export function PostFX() {
  const viewMode = useGame3DStore((s) => s.viewMode)
  const isLowPerf = useGame3DStore((s) => s.isLowPerf)

  // 性能降级：完全关闭后处理
  if (isLowPerf) return null

  // 按 viewMode 构造效果链（避免 false/null 作为 children）
  const effects: ReactElement[] = [
    <Bloom key="bloom" {...BLOOM_PROPS} />,
    <Vignette key="vignette" {...VIGNETTE_PROPS} />,
  ]
  if (viewMode === 'high') {
    effects.splice(1, 0, <SSAO key="ssao" {...SSAO_PROPS} />)
  }

  return (
    <EffectComposer enableNormalPass={viewMode === 'high'}>
      {effects}
    </EffectComposer>
  )
}
