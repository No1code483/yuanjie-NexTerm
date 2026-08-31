import { useState, useEffect, useCallback, useRef } from 'react'

/**
 * 用户统计数据 Hook
 * - 登录次数、使用时长：localStorage 累计（跨会话持久化）
 * - 简历数、语录数：由外部组件传入（来自后端 API）
 * - 连续登录天数：基于 lastLoginDate 计算
 */

export interface UserStats {
  loginCount: number
  totalUsageSecs: number
  streakDays: number
  lastLoginDate: string | null // YYYY-MM-DD
  firstLoginAt: number | null // 首次登录时间戳
}

const STATS_KEY = 'nt-user-stats'

const DEFAULT_STATS: UserStats = {
  loginCount: 0,
  totalUsageSecs: 0,
  streakDays: 0,
  lastLoginDate: null,
  firstLoginAt: null,
}

function loadStats(): UserStats {
  try {
    const raw = localStorage.getItem(STATS_KEY)
    if (!raw) return { ...DEFAULT_STATS }
    const parsed = JSON.parse(raw)
    return { ...DEFAULT_STATS, ...parsed }
  } catch {
    return { ...DEFAULT_STATS }
  }
}

function saveStats(stats: UserStats) {
  try {
    localStorage.setItem(STATS_KEY, JSON.stringify(stats))
  } catch {
    // ignore quota errors
  }
}

function todayStr(): string {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

function isYesterday(prev: string, today: string): boolean {
  const p = new Date(prev + 'T00:00:00')
  const t = new Date(today + 'T00:00:00')
  const diff = (t.getTime() - p.getTime()) / 86400000
  return diff === 1
}

/**
 * @param userId 当前用户 ID，变化时触发登录计数
 * @param enabled 是否启用（临时账号可禁用）
 */
export function useUserStats(userId: number | undefined, enabled: boolean = true) {
  const [stats, setStats] = useState<UserStats>(loadStats)
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const countedRef = useRef(false)

  // 登录计数 + 连续登录天数计算（userId 变化或组件首次挂载时触发一次）
  useEffect(() => {
    if (!enabled || !userId || countedRef.current) return
    countedRef.current = true

    setStats(prev => {
      const today = todayStr()
      let streak = prev.streakDays
      if (prev.lastLoginDate === today) {
        // 今日已登录，保持不变
      } else if (prev.lastLoginDate && isYesterday(prev.lastLoginDate, today)) {
        streak = prev.streakDays + 1
      } else {
        streak = 1
      }
      const next: UserStats = {
        ...prev,
        loginCount: prev.loginCount + 1,
        streakDays: streak,
        lastLoginDate: today,
        firstLoginAt: prev.firstLoginAt ?? Date.now(),
      }
      saveStats(next)
      return next
    })
  }, [userId, enabled])

  // 使用时长累计：每 60 秒 +60
  useEffect(() => {
    if (!enabled) return
    timerRef.current = setInterval(() => {
      setStats(prev => {
        const next = { ...prev, totalUsageSecs: prev.totalUsageSecs + 60 }
        saveStats(next)
        return next
      })
    }, 60000)
    return () => {
      if (timerRef.current) clearInterval(timerRef.current)
    }
  }, [enabled])

  // 手动累加（供外部调用，例如 AI 调用次数等后续扩展）
  const increment = useCallback((field: keyof UserStats, delta: number = 1) => {
    setStats(prev => {
      const next = { ...prev, [field]: (prev[field] as number) + delta } as UserStats
      saveStats(next)
      return next
    })
  }, [])

  return { stats, increment }
}

/**
 * 格式化使用时长（秒 → 人类可读）
 */
export function formatUsageDuration(secs: number): string {
  if (secs < 60) return `${secs}s`
  const m = Math.floor(secs / 60)
  if (m < 60) return `${m}m`
  const h = Math.floor(m / 60)
  const restM = m % 60
  if (h < 24) return `${h}h ${restM}m`
  const d = Math.floor(h / 24)
  return `${d}d ${h % 24}h`
}

/**
 * 生成 ASCII 进度条 [████████░░] 80%
 */
export function asciiProgress(percent: number, width: number = 10): string {
  const clamped = Math.max(0, Math.min(100, Math.round(percent)))
  const filled = Math.round((clamped / 100) * width)
  const empty = width - filled
  return `[${'█'.repeat(filled)}${'░'.repeat(empty)}] ${clamped}%`
}
