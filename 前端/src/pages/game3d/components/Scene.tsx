/**
 * Scene - 场景逻辑根（Task 8.2）
 *
 * 参考 05_3D场景设计.md §3 场景组件树：
 *   <Scene>
 *     <Lights />           灯光（方向光+环境光+半球光）
 *     <SkyFog />           天空 + 雾效 + 环境贴图（异步加载需 Suspense）
 *     <Terrain />          地形网格
 *     <Buildings />        建筑集合（Task 9.x 实现，阶段8 留接口注释）
 *     <Camera3D />         相机轨道控制
 *     <PostFX />           后处理
 *     <PerfMonitor />      性能监控
 *   </Scene>
 *
 * 说明：
 *   - SkyFog 中的 <Environment> 需异步加载 HDR，单独 Suspense 包裹
 *   - 其他组件均为同步，无需 Suspense
 *   - 建筑渲染层在阶段9 实现，本版留接口注释
 *
 * change-id: game-3d-rebuild-refactor
 */
import { Suspense } from 'react'
import { Lights } from './Lights'
import { Terrain } from './Terrain'
import { SkyFog } from './SkyFog'
import { Buildings } from './Buildings'
import { PlacementController } from './PlacementController'
import { Camera3D } from './Camera3D'
import { PostFX } from './PostFX'
import { PerfMonitor } from './PerfMonitor'
import { ReplayEventOverlay } from './ReplayEventOverlay'
import { BreakthroughAltar } from './BreakthroughAltar'
import { BreakthroughResultAnimations } from './BreakthroughResultAnimations'
import { EnvironmentDecor } from './EnvironmentDecor'
import { SpiritParticles } from './SpiritParticles'

export function Scene() {
  return (
    <>
      {/* 天空+雾+环境贴图（异步，单独 Suspense） */}
      <Suspense fallback={null}>
        <SkyFog />
      </Suspense>

      {/* 三光源体系 */}
      <Lights />

      {/* 地形网格 */}
      <Terrain />

      {/* 场景环境装饰：树木/竹子/太湖石（v2 建模优化） */}
      <EnvironmentDecor />

      {/* 灵气粒子 + 场景光点（v2 建模优化） */}
      <SpiritParticles />

      {/* 建筑渲染层：遍历 store.buildings，按 status 渲染（Task 9.1/9.2）。
          T1.6 回放模式下从 replayCurrentScene.buildings 渲染（Buildings 内部处理）。 */}
      <Buildings />

      {/* T1.6 时间轴回放事件动画叠加层（仅 replayMode 时生效） */}
      <ReplayEventOverlay />

      {/* T2.2 突破祭坛：仅在 preview.can_breakthrough 且非回放模式时渲染（内部处理条件） */}
      <BreakthroughAltar />

      {/* T2.4 突破结果动画：监听 store.breakthroughAnimation 分发（idle 时返回 null） */}
      <BreakthroughResultAnimations />

      {/* 预建造放置控制器：鼠标跟随预览 + Raycaster 地块确认（Task 9.3） */}
      <PlacementController />

      {/* 相机轨道控制 + 预设切换 */}
      <Camera3D />

      {/* 后处理（Bloom + SSAO + Vignette） */}
      <PostFX />

      {/* 性能监控与降级 */}
      <PerfMonitor />
    </>
  )
}
