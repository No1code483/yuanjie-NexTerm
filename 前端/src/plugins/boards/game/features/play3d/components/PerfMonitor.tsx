/**
 * PerfMonitor - 性能监控与降级（Task 8.5 配套）
 *
 * 参考 05_3D场景设计.md §10.1 性能监控与降级：
 *   - PerformanceMonitor 监控 FPS / DrawCall
 *   - onDecline：帧率下降时触发 setLowPerf(true)
 *   - onIncline：帧率恢复时触发 setLowPerf(false)
 *   - flipflops=3：3 次抖动后强制降级
 *
 * 降级链路（按性能恶化递进）：
 *   1. 关闭 SSAO（PostFX 根据 viewMode 自动跳过）
 *   2. 阴影贴图降至 1024（v2）
 *   3. 关闭 Bloom（v2）
 *   4. DPR 降至 1（v2）
 *   5. 关闭阴影（v2）
 *
 * change-id: game-3d-rebuild-refactor
 */
import { PerformanceMonitor } from '@react-three/drei'
import { useGame3DStore } from '../stores/gameStore'

export function PerfMonitor() {
  const setLowPerf = useGame3DStore((s) => s.setLowPerf)

  return (
    <PerformanceMonitor
      onDecline={() => setLowPerf(true)}
      onIncline={() => setLowPerf(false)}
      flipflops={3}
      onFallback={() => setLowPerf(true)}
    />
  )
}
