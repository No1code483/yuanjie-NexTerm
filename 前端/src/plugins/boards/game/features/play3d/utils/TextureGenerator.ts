/**
 * TextureGenerator - 程序化PBR纹理生成器
 *
 * 使用 Canvas API 生成中式建筑所需的纹理贴图：
 *   - 砖墙纹理（brick）— 青砖交错排列
 *   - 木纹纹理（wood）— 纵向木纹+年轮+木节
 *   - 石纹纹理（stone）— 花岗岩质感+裂纹
 *   - 琉璃瓦片纹理（glazedTile）— 青绿琉璃鱼鳞瓦
 *   - 灰瓦纹理（greyTile）— 普通灰瓦
 *   - 茅草纹理（thatch）— 草纤维+束带
 *   - 土墙纹理（mud）— 夯土+草筋
 *
 * 每个纹理同时生成对应的粗糙度贴图（roughness map），
 * 用于 PBR 材质的 roughnessMap 属性。
 *
 * 纹理尺寸：512×512（平衡质量与性能）
 *
 * change-id: game-3d-rebuild-refactor-modeling-opt
 */
import * as THREE from 'three'

const TEX_SIZE = 512
const textureCache = new Map<string, THREE.CanvasTexture>()

// ============================================================================
// 一、Canvas 工具函数
// ============================================================================

function createCanvas(size: number = TEX_SIZE): [HTMLCanvasElement, CanvasRenderingContext2D] {
  const canvas = document.createElement('canvas')
  canvas.width = size
  canvas.height = size
  const ctx = canvas.getContext('2d')!
  return [canvas, ctx]
}

function canvasToTexture(
  canvas: HTMLCanvasElement,
  wrapS: THREE.Wrapping = THREE.RepeatWrapping,
  wrapT: THREE.Wrapping = THREE.RepeatWrapping,
): THREE.CanvasTexture {
  const tex = new THREE.CanvasTexture(canvas)
  tex.wrapS = wrapS
  tex.wrapT = wrapT
  tex.colorSpace = THREE.SRGBColorSpace
  tex.minFilter = THREE.LinearMipmapLinearFilter
  tex.magFilter = THREE.LinearFilter
  tex.generateMipmaps = true
  return tex
}

function seededRandom(seed: number): () => number {
  let s = seed
  return () => {
    s = (s * 16807 + 0) % 2147483647
    return (s - 1) / 2147483646
  }
}

// ============================================================================
// 二、纹理生成函数
// ============================================================================

/** 砖墙纹理 — 中式青砖交错排列 */
function generateBrick(): [HTMLCanvasElement, HTMLCanvasElement] {
  const [canvas, ctx] = createCanvas()
  const [roughCanvas, rctx] = createCanvas()
  const rng = seededRandom(42)

  ctx.fillStyle = '#7a8a8a'
  ctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)
  rctx.fillStyle = '#808080'
  rctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)

  const brickW = TEX_SIZE / 8
  const brickH = TEX_SIZE / 16
  const mortarW = 4

  for (let row = 0; row < 16; row++) {
    const offsetX = row % 2 === 0 ? 0 : brickW / 2
    for (let col = -1; col < 9; col++) {
      const x = col * brickW + offsetX
      const y = row * brickH
      const shade = 0.85 + rng() * 0.15
      const r = Math.floor(100 * shade)
      const g = Math.floor(110 * shade)
      const b = Math.floor(115 * shade)

      ctx.fillStyle = `rgb(${r},${g},${b})`
      ctx.fillRect(x + mortarW / 2, y + mortarW / 2, brickW - mortarW, brickH - mortarW)

      for (let i = 0; i < 20; i++) {
        const nx = x + mortarW + rng() * (brickW - mortarW)
        const ny = y + mortarW + rng() * (brickH - mortarW)
        const ns = 0.9 + rng() * 0.1
        ctx.fillStyle = `rgba(${Math.floor(r * ns)},${Math.floor(g * ns)},${Math.floor(b * ns)},0.3)`
        ctx.fillRect(nx, ny, 2, 2)
      }

      rctx.fillStyle = `rgb(${Math.floor(100 + rng() * 40)},${Math.floor(100 + rng() * 40)},${Math.floor(100 + rng() * 40)})`
      rctx.fillRect(x + mortarW / 2, y + mortarW / 2, brickW - mortarW, brickH - mortarW)
    }
  }

  rctx.fillStyle = '#d0d0d0'
  for (let row = 0; row <= 16; row++) rctx.fillRect(0, row * brickH, TEX_SIZE, mortarW)
  for (let col = 0; col <= 8; col++) rctx.fillRect(col * brickW, 0, mortarW, TEX_SIZE)

  return [canvas, roughCanvas]
}

/** 木纹纹理 — 纵向木纹+年轮+木节 */
function generateWood(): [HTMLCanvasElement, HTMLCanvasElement] {
  const [canvas, ctx] = createCanvas()
  const [roughCanvas, rctx] = createCanvas()
  const rng = seededRandom(137)

  ctx.fillStyle = '#6b2a1a'
  ctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)

  for (let i = 0; i < 60; i++) {
    const x = rng() * TEX_SIZE
    const w = 2 + rng() * 8
    const shade = 0.85 + rng() * 0.3
    ctx.fillStyle = `rgba(${Math.floor(130 * shade)},${Math.floor(55 * shade)},${Math.floor(30 * shade)},${0.05 + rng() * 0.15})`
    ctx.fillRect(x - w / 2, 0, w, TEX_SIZE)
  }

  for (let i = 0; i < 15; i++) {
    const y = rng() * TEX_SIZE
    ctx.fillStyle = `rgba(0,0,0,${0.02 + rng() * 0.06})`
    ctx.fillRect(0, y, TEX_SIZE, 1 + rng() * 3)
  }

  for (let i = 0; i < 3; i++) {
    const kx = TEX_SIZE * 0.2 + rng() * TEX_SIZE * 0.6
    const ky = TEX_SIZE * 0.2 + rng() * TEX_SIZE * 0.6
    const kr = 8 + rng() * 20
    const gradient = ctx.createRadialGradient(kx, ky, 0, kx, ky, kr)
    gradient.addColorStop(0, 'rgba(40,15,5,0.6)')
    gradient.addColorStop(0.5, 'rgba(60,25,10,0.3)')
    gradient.addColorStop(1, 'rgba(107,42,26,0)')
    ctx.fillStyle = gradient
    ctx.beginPath()
    ctx.ellipse(kx, ky, kr, kr * 0.6, 0, 0, Math.PI * 2)
    ctx.fill()
  }

  rctx.fillStyle = '#909090'
  rctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)
  for (let i = 0; i < 30; i++) {
    const x = rng() * TEX_SIZE
    rctx.fillStyle = `rgba(255,255,255,${0.05 + rng() * 0.1})`
    rctx.fillRect(x - 2, 0, 4 + rng() * 6, TEX_SIZE)
  }

  return [canvas, roughCanvas]
}

/** 石纹纹理 — 花岗岩质感+裂纹 */
function generateStone(): [HTMLCanvasElement, HTMLCanvasElement] {
  const [canvas, ctx] = createCanvas()
  const [roughCanvas, rctx] = createCanvas()
  const rng = seededRandom(256)

  ctx.fillStyle = '#8a8a85'
  ctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)

  for (let i = 0; i < 200; i++) {
    const x = rng() * TEX_SIZE
    const y = rng() * TEX_SIZE
    const r = 5 + rng() * 30
    const g = Math.floor(140 * (0.88 + rng() * 0.24))
    const gradient = ctx.createRadialGradient(x, y, 0, x, y, r)
    gradient.addColorStop(0, `rgba(${g},${g},${g - 5},${0.05 + rng() * 0.15})`)
    gradient.addColorStop(1, 'rgba(0,0,0,0)')
    ctx.fillStyle = gradient
    ctx.fillRect(x - r, y - r, r * 2, r * 2)
  }

  for (let i = 0; i < 8; i++) {
    const sx = rng() * TEX_SIZE
    const sy = rng() * TEX_SIZE
    const len = 20 + rng() * 60
    const angle = rng() * Math.PI * 2
    ctx.strokeStyle = 'rgba(60,60,55,0.3)'
    ctx.lineWidth = 0.5 + rng() * 1.5
    ctx.beginPath()
    ctx.moveTo(sx, sy)
    for (let j = 0; j < 5; j++) {
      const a = angle + (rng() - 0.5) * 0.5
      ctx.lineTo(sx + Math.cos(a) * len * (j / 5), sy + Math.sin(a) * len * (j / 5))
    }
    ctx.stroke()
  }

  rctx.fillStyle = '#c0c0c0'
  rctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)
  for (let i = 0; i < 100; i++) {
    const x = rng() * TEX_SIZE
    const y = rng() * TEX_SIZE
    const r = 3 + rng() * 15
    rctx.fillStyle = `rgba(255,255,255,${0.05 + rng() * 0.15})`
    rctx.fillRect(x - r / 2, y - r / 2, r, r)
  }

  return [canvas, roughCanvas]
}

/** 琉璃瓦片纹理 — 青绿琉璃鱼鳞瓦 */
function generateGlazedTile(): [HTMLCanvasElement, HTMLCanvasElement] {
  const [canvas, ctx] = createCanvas()
  const [roughCanvas, rctx] = createCanvas()
  const rng = seededRandom(789)

  ctx.fillStyle = '#3a6b5e'
  ctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)
  rctx.fillStyle = '#606060'
  rctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)

  const tileW = TEX_SIZE / 8
  const tileH = TEX_SIZE / 12
  const gap = 2

  for (let row = 0; row < 12; row++) {
    const offsetX = row % 2 === 0 ? 0 : tileW / 2
    for (let col = -1; col < 9; col++) {
      const cx = col * tileW + offsetX + tileW / 2
      const cy = row * tileH + tileH / 2
      const shade = 0.85 + rng() * 0.15
      const r = Math.floor(50 * shade)
      const g = Math.floor(100 * shade)
      const b = Math.floor(90 * shade)

      ctx.fillStyle = `rgb(${r},${g},${b})`
      ctx.beginPath()
      ctx.ellipse(cx, cy, tileW / 2 - gap, tileH / 2 - gap, 0, 0, Math.PI * 2)
      ctx.fill()

      const hlGrad = ctx.createRadialGradient(cx - tileW * 0.15, cy - tileH * 0.2, 0, cx, cy, tileW / 2)
      hlGrad.addColorStop(0, `rgba(${Math.floor(r * 1.5)},${Math.floor(g * 1.3)},${Math.floor(b * 1.3)},0.3)`)
      hlGrad.addColorStop(1, 'rgba(0,0,0,0)')
      ctx.fillStyle = hlGrad
      ctx.beginPath()
      ctx.ellipse(cx, cy, tileW / 2 - gap, tileH / 2 - gap, 0, 0, Math.PI * 2)
      ctx.fill()

      rctx.fillStyle = `rgb(${Math.floor(40 + rng() * 20)},${Math.floor(40 + rng() * 20)},${Math.floor(40 + rng() * 20)})`
      rctx.beginPath()
      rctx.ellipse(cx, cy, tileW / 2 - gap, tileH / 2 - gap, 0, 0, Math.PI * 2)
      rctx.fill()
    }
  }

  rctx.fillStyle = '#c0c0c0'
  for (let row = 0; row <= 12; row++) rctx.fillRect(0, row * tileH - gap, TEX_SIZE, gap * 2)

  return [canvas, roughCanvas]
}

/** 灰瓦纹理 — 普通灰瓦 */
function generateGreyTile(): [HTMLCanvasElement, HTMLCanvasElement] {
  const [canvas, ctx] = createCanvas()
  const [roughCanvas, rctx] = createCanvas()
  const rng = seededRandom(456)

  ctx.fillStyle = '#5a5a55'
  ctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)
  rctx.fillStyle = '#a0a0a0'
  rctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)

  const tileW = TEX_SIZE / 8
  const tileH = TEX_SIZE / 12
  const gap = 2

  for (let row = 0; row < 12; row++) {
    const offsetX = row % 2 === 0 ? 0 : tileW / 2
    for (let col = -1; col < 9; col++) {
      const cx = col * tileW + offsetX + tileW / 2
      const cy = row * tileH + tileH / 2
      const shade = 0.85 + rng() * 0.15
      const g = Math.floor(95 * shade)

      ctx.fillStyle = `rgb(${g},${g},${Math.floor(g * 0.9)})`
      ctx.beginPath()
      ctx.ellipse(cx, cy, tileW / 2 - gap, tileH / 2 - gap, 0, 0, Math.PI * 2)
      ctx.fill()

      rctx.fillStyle = `rgb(${Math.floor(120 + rng() * 30)},${Math.floor(120 + rng() * 30)},${Math.floor(120 + rng() * 30)})`
      rctx.beginPath()
      rctx.ellipse(cx, cy, tileW / 2 - gap, tileH / 2 - gap, 0, 0, Math.PI * 2)
      rctx.fill()
    }
  }

  return [canvas, roughCanvas]
}

/** 茅草纹理 — 草纤维+横向束带 */
function generateThatch(): [HTMLCanvasElement, HTMLCanvasElement] {
  const [canvas, ctx] = createCanvas()
  const [roughCanvas, rctx] = createCanvas()
  const rng = seededRandom(999)

  ctx.fillStyle = '#a09040'
  ctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)

  for (let i = 0; i < 3000; i++) {
    const x = rng() * TEX_SIZE
    const y = rng() * TEX_SIZE
    const len = 3 + rng() * 15
    const angle = -Math.PI / 2 + (rng() - 0.5) * 0.8
    const shade = 0.8 + rng() * 0.4
    ctx.strokeStyle = `rgba(${Math.floor(180 * shade)},${Math.floor(150 * shade)},${Math.floor(70 * shade)},0.4)`
    ctx.lineWidth = 0.5 + rng() * 1.5
    ctx.beginPath()
    ctx.moveTo(x, y)
    ctx.lineTo(x + Math.cos(angle) * len, y + Math.sin(angle) * len)
    ctx.stroke()
  }

  for (let i = 0; i < 12; i++) {
    const y = i * (TEX_SIZE / 12) + TEX_SIZE / 24
    const shade = 0.7 + rng() * 0.15
    ctx.strokeStyle = `rgba(${Math.floor(100 * shade)},${Math.floor(80 * shade)},${Math.floor(40 * shade)},0.3)`
    ctx.lineWidth = 2 + rng() * 3
    ctx.beginPath()
    ctx.moveTo(0, y)
    for (let x = 0; x < TEX_SIZE; x += 10) ctx.lineTo(x, y + (rng() - 0.5) * 4)
    ctx.stroke()
  }

  rctx.fillStyle = '#e0e0e0'
  rctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)

  return [canvas, roughCanvas]
}

/** 土墙纹理 — 夯土+草筋 */
function generateMud(): [HTMLCanvasElement, HTMLCanvasElement] {
  const [canvas, ctx] = createCanvas()
  const [roughCanvas, rctx] = createCanvas()
  const rng = seededRandom(333)

  ctx.fillStyle = '#9b8b6f'
  ctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)

  for (let i = 0; i < 2000; i++) {
    const x = rng() * TEX_SIZE
    const y = rng() * TEX_SIZE
    const shade = 0.85 + rng() * 0.3
    ctx.fillStyle = `rgba(${Math.floor(160 * shade)},${Math.floor(140 * shade)},${Math.floor(110 * shade)},0.5)`
    ctx.fillRect(x, y, 1 + rng() * 3, 1 + rng() * 3)
  }

  for (let i = 0; i < 200; i++) {
    const x = rng() * TEX_SIZE
    const y = rng() * TEX_SIZE
    const len = 5 + rng() * 25
    const angle = rng() * Math.PI * 2
    ctx.strokeStyle = 'rgba(200,180,120,0.2)'
    ctx.lineWidth = 0.5 + rng() * 1
    ctx.beginPath()
    ctx.moveTo(x, y)
    ctx.lineTo(x + Math.cos(angle) * len, y + Math.sin(angle) * len)
    ctx.stroke()
  }

  rctx.fillStyle = '#d0d0d0'
  rctx.fillRect(0, 0, TEX_SIZE, TEX_SIZE)

  return [canvas, roughCanvas]
}

// ============================================================================
// 三、对外接口
// ============================================================================

export type TextureType = 'brick' | 'wood' | 'stone' | 'glazedTile' | 'greyTile' | 'thatch' | 'mud'

const GENERATORS: Record<TextureType, () => [HTMLCanvasElement, HTMLCanvasElement]> = {
  brick: generateBrick,
  wood: generateWood,
  stone: generateStone,
  glazedTile: generateGlazedTile,
  greyTile: generateGreyTile,
  thatch: generateThatch,
  mud: generateMud,
}

/**
 * 获取纹理对（颜色贴图 + 粗糙度贴图）
 * 结果被缓存，多次调用返回同一实例
 */
export function getTexturePair(type: TextureType): {
  map: THREE.CanvasTexture
  roughnessMap: THREE.CanvasTexture
} {
  const mapKey = `${type}_map`
  const roughKey = `${type}_rough`

  if (!textureCache.has(mapKey)) {
    const generator = GENERATORS[type]
    const [colorCanvas, roughCanvas] = generator()
    textureCache.set(mapKey, canvasToTexture(colorCanvas))
    textureCache.set(roughKey, canvasToTexture(roughCanvas))
  }

  return {
    map: textureCache.get(mapKey)!,
    roughnessMap: textureCache.get(roughKey)!,
  }
}

/** 预生成所有纹理（避免首次渲染卡顿） */
export function preloadAllTextures(): void {
  const types: TextureType[] = ['brick', 'wood', 'stone', 'glazedTile', 'greyTile', 'thatch', 'mud']
  for (const type of types) getTexturePair(type)
}

/** 清除纹理缓存 */
export function clearTextureCache(): void {
  for (const tex of textureCache.values()) tex.dispose()
  textureCache.clear()
}