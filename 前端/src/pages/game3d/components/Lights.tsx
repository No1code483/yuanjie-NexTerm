/**
 * Lights - 3D 场景三光源体系（Task 8.2）
 *
 * 参考 05_3D场景设计.md §5 光照系统：
 *   - 方向光（太阳）：唯一投射阴影，模拟午后日光
 *   - 环境光：低强度填充，保留明暗对比
 *   - 半球光：天地过渡，弥补方向光硬朗
 *
 * 阴影：PCFSoftShadowMap（由 <Canvas shadows> 开启）
 * 阴影贴图：2048×2048，视锥紧贴场景边界 -40~40
 *
 * change-id: game-3d-rebuild-refactor
 */

/**
 * 灯光组件：返回三光源的 fragment
 *
 * 配置参数来源 05 文档 §5.1-§5.3
 */
export function Lights() {
  return (
    <>
      {/* 太阳方向光：主光源 + 阴影投射 */}
      <directionalLight
        position={[20, 30, 20]}
        intensity={1.5}
        color="#fff5e6"
        castShadow
        shadow-mapSize={[2048, 2048]}
        shadow-camera-near={0.5}
        shadow-camera-far={100}
        shadow-camera-left={-40}
        shadow-camera-right={40}
        shadow-camera-top={40}
        shadow-camera-bottom={-40}
        shadow-bias={-0.0005}
      />
      {/* 环境光：基础填充 */}
      <ambientLight intensity={0.3} color="#ffffff" />
      {/* 半球光：天地过渡 */}
      <hemisphereLight args={['#87CEEB', '#3a3a3a', 0.4]} />
    </>
  )
}
