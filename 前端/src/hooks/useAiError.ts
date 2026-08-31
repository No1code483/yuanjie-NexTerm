/**
 * useAiError Hook - AI 错误降级处理
 *
 * 提供统一的 AI 调用错误状态管理：
 * - error: 最近一次错误消息（null 表示无错误）
 * - loading: 是否正在加载中
 * - wrap(fn): 包装异步调用，自动捕获错误
 * - clear: 清除错误状态
 */

import { useState, useCallback } from 'react'

interface UseAiErrorReturn {
  error: string | null
  loading: boolean
  wrap: <T>(fn: () => Promise<T>) => Promise<T | null>
  clear: () => void
}

export function useAiError(): UseAiErrorReturn {
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  const wrap = useCallback(async <T>(fn: () => Promise<T>): Promise<T | null> => {
    setLoading(true)
    setError(null)
    try {
      const result = await fn()
      return result
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      setError(msg.length > 80 ? msg.slice(0, 80) + '...' : msg)
      return null
    } finally {
      setLoading(false)
    }
  }, [])

  const clear = useCallback(() => setError(null), [])

  return { error, loading, wrap, clear }
}