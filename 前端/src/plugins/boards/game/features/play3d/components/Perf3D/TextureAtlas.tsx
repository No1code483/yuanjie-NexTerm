/**
 * TextureAtlas - 纹理图集合并（Phase 4 §2.3.3）
 *
 * 规范：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §Phase 4
 *
 * 目标：
 *   - 将 TextureGenerator 生成的 7 张 512×512 程序化纹理合并为单张图集
 *   - 同图集同时合并颜色贴图与粗糙度贴图（两张 atlas）
 *   - 所有建筑可共享同一材质，仅通过 UV offset 选择子纹理
 *   - 减少 GPU 纹理绑定次数与 draw call 中的纹理切换开销
 *
 * 布局：
 *   - 4 列 × 2 行 = 8 个槽位（7 个纹理 + 1 个空位）
 *   - 单槽 512×512，图集总尺寸 2048×1024
 *   - 行 0: brick, wood, stone, glazedTile
 *   - 行 1: greyTile, thatch, mud, (spare)
 *
 * 用法：
 * ```tsx
 * const { atlasMap, atlasRoughness, layout } = useTextureAtlas()
 * // 共享同一材质，仅几何 UV 不同：
 * applyAtlasUV(geometry, 'brick', layout)
 * material.map = atlasMap
 * material.roughnessMap = atlasRoughness
 * ```
 *
 * 注：当前建筑组件使用各自独立的 CanvasTexture（已缓存），
 *      本图集为 PoC 与未来批渲染（InstancedMesh / 合并几何）铺路。
 *      实际 draw call 减少需配合几何合并或 InstancedMesh 改造。
 *
 * change-id: deliver-roadmap-55
 */
import { useEffect, useState } from 'react'
import * as THREE from 'three'
import {
  getTexturePair,
  type TextureType,
} from '../../utils/TextureGenerator'

// ============================================================================
// 一、图集布局定义
// ============================================================================

const ATLAS_COLS = 4
const ATLAS_ROWS = 2
const SLOT_SIZE = 512
const ATLAS_W = ATLAS_COLS * SLOT_SIZE // 2048
const ATLAS_H = ATLAS_ROWS * SLOT_SIZE // 1024

/** 纹理类型在图集中的槽位（按生成顺序排列） */
const TEXTURE_SLOTS: TextureType[] = [
  'brick', 'wood', 'stone', 'glazedTile',
  'greyTile', 'thatch', 'mud',
]

export interface AtlasLayout {
  cols: number
  rows: number
  /** 每种纹理类型对应的 UV 矩形 [u0, v0, u1, v1] */
  slots: Record<TextureType, [number, number, number, number]>
}

/**
 * 计算指定槽位的 UV 矩形。
 *
 * CanvasTexture 默认 flipY=true，画布顶部对应 UV v=1。
 * 因此 row=0（画布顶部）的槽位 UV v 范围为 [0.5, 1.0]，
 * row=1（画布底部）的槽位 UV v 范围为 [0.0, 0.5]。
 */
function computeSlotUV(col: number, row: number): [number, number, number, number] {
  const u0 = col / ATLAS_COLS
  const u1 = (col + 1) / ATLAS_COLS
  // flipY=true → 画布 row 0（顶部）映射到 UV v 高位
  const v1 = 1 - row / ATLAS_ROWS
  const v0 = 1 - (row + 1) / ATLAS_ROWS
  return [u0, v0, u1, v1]
}

/** 构建图集布局（纯计算，无副作用） */
function buildLayout(): AtlasLayout {
  const slots = {} as Record<TextureType, [number, number, number, number]>
  TEXTURE_SLOTS.forEach((type, idx) => {
    const col = idx % ATLAS_COLS
    const row = Math.floor(idx / ATLAS_COLS)
    slots[type] = computeSlotUV(col, row)
  })
  return { cols: ATLAS_COLS, rows: ATLAS_ROWS, slots }
}

// ============================================================================
// 二、图集纹理构建（带缓存）
// ============================================================================

interface AtlasBundle {
  atlasMap: THREE.CanvasTexture
  atlasRoughness: THREE.CanvasTexture
  layout: AtlasLayout
}

let atlasCache: AtlasBundle | null = null

/**
 * 构建（或返回缓存的）纹理图集。
 *
 * 实现：创建一张 2048×1024 画布，将 7 张 512×512 子纹理 drawImage 到对应槽位。
 * 颜色贴图与粗糙度贴图分别构建，得到两张 atlas。
 */
export function buildTextureAtlas(): AtlasBundle {
  if (atlasCache) return atlasCache

  const layout = buildLayout()

  const mapCanvas = document.createElement('canvas')
  mapCanvas.width = ATLAS_W
  mapCanvas.height = ATLAS_H
  const mapCtx = mapCanvas.getContext('2d')!

  const roughCanvas = document.createElement('canvas')
  roughCanvas.width = ATLAS_W
  roughCanvas.height = ATLAS_H
  const roughCtx = roughCanvas.getContext('2d')!

  // 填充背景为中性灰，避免空槽位采样异常
  mapCtx.fillStyle = '#808080'
  mapCtx.fillRect(0, 0, ATLAS_W, ATLAS_H)
  roughCtx.fillStyle = '#808080'
  roughCtx.fillRect(0, 0, ATLAS_W, ATLAS_H)

  TEXTURE_SLOTS.forEach((type, idx) => {
    const col = idx % ATLAS_COLS
    const row = Math.floor(idx / ATLAS_COLS)
    const x = col * SLOT_SIZE
    // 画布 y 向下为正，row 0 在顶部
    const y = row * SLOT_SIZE

    const { map, roughnessMap } = getTexturePair(type)
    // CanvasTexture.image 即原始 HTMLCanvasElement
    const srcMap = map.image as HTMLCanvasElement
    const srcRough = roughnessMap.image as HTMLCanvasElement

    mapCtx.drawImage(srcMap, x, y, SLOT_SIZE, SLOT_SIZE)
    roughCtx.drawImage(srcRough, x, y, SLOT_SIZE, SLOT_SIZE)
  })

  const atlasMap = new THREE.CanvasTexture(mapCanvas)
  atlasMap.colorSpace = THREE.SRGBColorSpace
  atlasMap.wrapS = THREE.ClampToEdgeWrapping
  atlasMap.wrapT = THREE.ClampToEdgeWrapping
  atlasMap.minFilter = THREE.LinearMipmapLinearFilter
  atlasMap.magFilter = THREE.LinearFilter
  atlasMap.generateMipmaps = true

  const atlasRoughness = new THREE.CanvasTexture(roughCanvas)
  atlasRoughness.colorSpace = THREE.NoColorSpace
  atlasRoughness.wrapS = THREE.ClampToEdgeWrapping
  atlasRoughness.wrapT = THREE.ClampToEdgeWrapping
  atlasRoughness.minFilter = THREE.LinearMipmapLinearFilter
  atlasRoughness.magFilter = THREE.LinearFilter
  atlasRoughness.generateMipmaps = true

  atlasCache = { atlasMap, atlasRoughness, layout }
  return atlasCache
}

/** 释放图集缓存（切换场景或热重载时调用） */
export function disposeTextureAtlas(): void {
  if (!atlasCache) return
  atlasCache.atlasMap.dispose()
  atlasCache.atlasRoughness.dispose()
  atlasCache = null
}

// ============================================================================
// 三、UV 变换工具
// ============================================================================

/**
 * 将几何体的 UV 属性从 [0,1] 重映射到图集中指定纹理类型的槽位。
 *
 * 作用：让使用同一 atlas 材质的多个几何体，通过 UV offset 选中不同子纹理，
 *      从而共享材质、减少 draw call。
 */
export function applyAtlasUV(
  geometry: THREE.BufferGeometry,
  type: TextureType,
  layout: AtlasLayout,
): void {
  const uv = geometry.attributes.uv
  if (!uv) return
  const [u0, v0, u1, v1] = layout.slots[type]
  const uScale = u1 - u0
  const vScale = v1 - v0
  const arr = uv.array as Float32Array
  for (let i = 0; i < arr.length; i += 2) {
    arr[i] = u0 + arr[i] * uScale
    arr[i + 1] = v0 + arr[i + 1] * vScale
  }
  uv.needsUpdate = true
}

/**
 * 返回指定纹理类型在图集中的 UV 变换参数。
 * 用于 ShaderMaterial 或手动 UV 计算场景。
 */
export function getAtlasTransform(
  type: TextureType,
  layout: AtlasLayout,
): { offset: [number, number]; scale: [number, number] } {
  const [u0, v0, u1, v1] = layout.slots[type]
  return {
    offset: [u0, v0],
    scale: [u1 - u0, v1 - v0],
  }
}

// ============================================================================
// 四、React Hook
// ============================================================================

/**
 * useTextureAtlas - 在组件中获取图集纹理与布局。
 *
 * 首次挂载时构建图集（触发 TextureGenerator 生成 7 张子纹理 + 合并），
 * 卸载时不释放（图集为全局缓存，跨组件共享）。
 */
export function useTextureAtlas(): AtlasBundle | null {
  const [bundle, setBundle] = useState<AtlasBundle | null>(null)

  useEffect(() => {
    // 在 effect 中构建，避免 SSR 或首屏同步阻塞
    setBundle(buildTextureAtlas())
  }, [])

  return bundle
}
