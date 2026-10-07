/**
 * SkyFog - 天空盒 + 雾效 + 环境贴图（Task 8.2）
 *
 * 参考 05_3D场景设计.md §7 天空与环境：
 *   - <Sky>：Drei 程序化天空，基于 Preetham 天空模型
 *   - <Environment preset="city">：HDR 环境贴图，驱动 PBR 反射
 *   - <fog>：线性雾效，远处建筑淡入雾色
 *
 * 太阳方向 [20, 30, 20] 与 Lights.tsx 方向光一致
 *
 * 注意：Environment preset 由 Drei 从 CDN 加载，Tauri 桌面端需联网。
 *      若离线场景报错，可将 preset 替换为本地 HDR 文件路径。
 *
 * change-id: game-3d-rebuild-refactor
 */
import { Sky, Environment } from '@react-three/drei'

/** 太阳方向（与方向光保持一致） */
const SUN_POSITION: [number, number, number] = [20, 30, 20]

/**
 * 天空 + 雾效 + 环境贴图组件
 * - Sky：程序化天空，distance 450000 模拟远距离球
 * - Environment：HDR 反射贴图，不作为背景（仅反射用）
 * - fog：线性雾，30 近距起雾，80 远距完全雾化
 */
export function SkyFog() {
  return (
    <>
      <Sky
        distance={450000}
        sunPosition={SUN_POSITION}
        turbidity={8}
        rayleigh={1}
        mieCoefficient={0.005}
        mieDirectionalG={0.8}
      />
      <Environment preset="city" background={false} />
      {/* 线性雾：匹配天空底部色 #c4d7e8 */}
      <fog attach="fog" args={['#c4d7e8', 30, 80]} />
    </>
  )
}
