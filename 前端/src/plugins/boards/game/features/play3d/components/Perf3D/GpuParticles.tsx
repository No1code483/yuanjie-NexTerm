/**
 * GpuParticles - GPU 粒子 PoC（Phase 4 §2.3.5）
 *
 * 规范：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §Phase 4
 *
 * 目标：
 *   - 验证 GPU 粒子方案可行性（PoC）
 *   - 粒子位置更新全部在顶点着色器中完成，CPU 零开销
 *   - 对比现有 SpiritParticles（CPU useFrame 更新 InstancedMatrix）的性能差异
 *
 * 实现：
 *   - BufferGeometry 携带 aOrigin / aVelocity / aLife 自定义属性
 *   - 顶点着色器根据 uTime 计算当前位置（mod 生命实现循环）
 *   - 片元着色器绘制圆形粒子 + 软边缘 + 加性混合
 *   - 仅一个 draw call，无 CPU 每帧更新
 *
 * 限制（PoC）：
 *   - 仅演示连续上升 + 水平漂移的灵气粒子模式
 *   - 未集成到 SpiritParticles（保留原 CPU 版本作为对照）
 *   - 未做距离剔除（可外包 ParticleDistanceLOD）
 *
 * 用法：
 * ```tsx
 * <GpuParticles count={200} areaSize={64} />
 * ```
 *
 * change-id: deliver-roadmap-55
 */
import { useMemo, useRef } from 'react'
import { useFrame } from '@react-three/fiber'
import * as THREE from 'three'

export interface GpuParticlesProps {
  /** 粒子数量（默认 200，PoC 规模） */
  count?: number
  /** 粒子分布区域边长（默认 64，匹配 game3d 地图） */
  areaSize?: number
  /** 粒子上升高度（默认 8） */
  riseHeight?: number
  /** 粒子大小（默认 0.15） */
  particleSize?: number
  /** 粒子颜色（默认 #88ccff，灵气蓝） */
  color?: string
}

// ============================================================================
// 着色器
// ============================================================================

const vertexShader = /* glsl */ `
  attribute vec3 aOrigin;     // 粒子初始位置
  attribute vec3 aVelocity;   // 粒子速度（x/z 水平漂移，y 上升）
  attribute float aLife;      // 粒子生命时长（秒）
  attribute float aPhase;     // 相位偏移（让粒子运动不同步）

  uniform float uTime;
  uniform float uSize;

  varying float vAlpha;       // 生命周期内的透明度（淡入淡出）

  void main() {
    // 当前生命进度（0~1，循环）
    float t = mod(uTime + aPhase, aLife) / aLife;

    // 位置 = 初始 + 速度 × 进度
    vec3 pos = aOrigin + aVelocity * t * aLife;

    // 透明度：前 20% 淡入，后 30% 淡出
    if (t < 0.2) {
      vAlpha = t / 0.2;
    } else if (t > 0.7) {
      vAlpha = (1.0 - t) / 0.3;
    } else {
      vAlpha = 1.0;
    }

    vec4 mvPosition = modelViewMatrix * vec4(pos, 1.0);
    gl_Position = projectionMatrix * mvPosition;

    // 点大小随距离衰减（透视）
    gl_PointSize = uSize * (300.0 / -mvPosition.z);
  }
`

const fragmentShader = /* glsl */ `
  uniform vec3 uColor;
  varying float vAlpha;

  void main() {
    // 圆形粒子：gl_PointCoord 范围 [0,1]，中心 (0.5, 0.5)
    vec2 coord = gl_PointCoord - vec2(0.5);
    float dist = length(coord);
    if (dist > 0.5) discard; // 超出半径丢弃

    // 软边缘：从中心 1.0 衰减到边缘 0.0
    float soft = 1.0 - smoothstep(0.0, 0.5, dist);
    gl_FragColor = vec4(uColor, soft * vAlpha * 0.6);
  }
`

// ============================================================================
// 组件
// ============================================================================

export function GpuParticles({
  count = 200,
  areaSize = 64,
  riseHeight = 8,
  particleSize = 0.15,
  color = '#88ccff',
}: GpuParticlesProps) {
  const materialRef = useRef<THREE.ShaderMaterial>(null)

  // 粒子属性（仅初始化一次）
  const { geometry, uniforms } = useMemo(() => {
    const origins = new Float32Array(count * 3)
    const velocities = new Float32Array(count * 3)
    const lives = new Float32Array(count)
    const phases = new Float32Array(count)

    // 简易确定性随机（与 SpiritParticles 风格一致）
    const rng = (seed: number) => {
      const s = (seed * 16807 + 0) % 2147483647
      return (s - 1) / 2147483646
    }

    for (let i = 0; i < count; i++) {
      const seed = i * 137 + 42
      // 初始位置：随机分布在 areaSize × riseHeight × areaSize 空间
      origins[i * 3] = rng(seed) * areaSize
      origins[i * 3 + 1] = rng(seed + 1) * riseHeight
      origins[i * 3 + 2] = rng(seed + 2) * areaSize

      // 速度：缓慢上升 + 水平漂移
      velocities[i * 3] = (rng(seed + 3) - 0.5) * 0.3
      velocities[i * 3 + 1] = 0.2 + rng(seed + 4) * 0.4
      velocities[i * 3 + 2] = (rng(seed + 5) - 0.5) * 0.3

      // 生命：4~8 秒
      lives[i] = 4 + rng(seed + 6) * 4
      // 相位：0~life 错开起始时间
      phases[i] = rng(seed + 7) * lives[i]
    }

    const geo = new THREE.BufferGeometry()
    geo.setAttribute('aOrigin', new THREE.BufferAttribute(origins, 3))
    geo.setAttribute('aVelocity', new THREE.BufferAttribute(velocities, 3))
    geo.setAttribute('aLife', new THREE.BufferAttribute(lives, 1))
    geo.setAttribute('aPhase', new THREE.BufferAttribute(phases, 1))
    // Points 需要 position 属性（占位，实际位置在着色器中计算）
    geo.setAttribute('position', new THREE.BufferAttribute(new Float32Array(count * 3), 3))

    const uni = {
      uTime: { value: 0 },
      uSize: { value: particleSize },
      uColor: { value: new THREE.Color(color) },
    }

    return { geometry: geo, uniforms: uni }
  }, [count, areaSize, riseHeight, particleSize, color])

  // 每帧仅更新 uTime uniform（CPU 开销极低）
  useFrame(() => {
    if (materialRef.current) {
      materialRef.current.uniforms.uTime.value = performance.now() * 0.001
    }
  })

  return (
    <points geometry={geometry}>
      <shaderMaterial
        ref={materialRef}
        vertexShader={vertexShader}
        fragmentShader={fragmentShader}
        uniforms={uniforms}
        transparent
        depthWrite={false}
        blending={THREE.AdditiveBlending}
      />
    </points>
  )
}
