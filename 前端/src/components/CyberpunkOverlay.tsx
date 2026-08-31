import { useEffect, useState } from 'react'

interface Props {
  /** 是否启用 CRT 扫描线效果，默认 true */
  enabled?: boolean
  /** 扫描线透明度，默认 0.04 */
  intensity?: number
}

/**
 * 赛博朋克 CRT 覆盖层
 * 在应用顶层渲染，提供扫描线 + 屏幕微光效果
 *
 * 使用：<CyberpunkOverlay enabled={true} intensity={0.04} />
 */
export default function CyberpunkOverlay({ enabled = true, intensity = 0.04 }: Props) {
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
  }, [])

  if (!mounted || !enabled) return null

  return (
    <div
      className="cyberpunk-crt"
      style={{
        // 动态强度通过 CSS 自定义属性控制
        // @ts-ignore
        '--crt-intensity': intensity,
      }}
    />
  )
}

/**
 * 赛博朋克数据雨背景
 * 可选的装饰性背景效果
 */
export function CyberpunkRain({ enabled = true, density = 20 }: { enabled?: boolean; density?: number }) {
  const [columns, setColumns] = useState<Array<{ left: string; duration: string; delay: string; chars: string }>>([])

  useEffect(() => {
    if (!enabled) return
    const chars = '01アイウエオカキクケコサシスセソタチツテトナニヌネノ'
    const newCols = Array.from({ length: density }, () => {
      const len = 8 + Math.floor(Math.random() * 20)
      let str = ''
      for (let i = 0; i < len; i++) {
        str += chars[Math.floor(Math.random() * chars.length)]
      }
      return {
        left: `${Math.random() * 100}%`,
        duration: `${6 + Math.random() * 10}s`,
        delay: `${Math.random() * 5}s`,
        chars: str,
      }
    })
    setColumns(newCols)
  }, [enabled, density])

  if (!enabled) return null

  return (
    <div className="cyberpunk-rain-container">
      {columns.map((col, i) => (
        <div
          key={i}
          className="cyberpunk-rain-column"
          style={{
            left: col.left,
            animationDuration: col.duration,
            animationDelay: col.delay,
          }}
        >
          {col.chars.split('').map((c, j) => (
            <div key={j}>{c}</div>
          ))}
        </div>
      ))}
    </div>
  )
}