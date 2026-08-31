/**
 * useEngineEvents — 订阅引擎事件流
 * 监听后端 EventBus 通过 Tauri `yuan-engine-event` 转发的事件
 * 对标 Codex-rs Event/EventMsg 体系
 */
import { useState, useEffect, useCallback, useRef } from 'react'

// 引擎事件类型 — 对齐后端 YuanEvent 枚举
export interface EngineStreamDelta {
  type: 'StreamDelta'
  session_id: string
  turn_id: string
  delta: string
  sequence: number
}

export interface EngineStreamEnd {
  type: 'StreamEnd'
  session_id: string
  turn_id: string
}

export interface EngineTurnStarted {
  type: 'TurnStarted'
  session_id: string
  turn_id: string
  turn_number: number
}

export interface EngineTurnCompleted {
  type: 'TurnCompleted'
  session_id: string
  turn_id: string
  token_usage?: { input_tokens: number; output_tokens: number }
}

export interface EngineToolCall {
  type: 'ToolCallStarted' | 'ToolCallCompleted'
  session_id: string
  turn_id: string
  tool_name: string
  tool_input?: string
  tool_output?: string
  duration_ms?: number
}

export interface EngineError {
  type: 'Error'
  session_id: string
  error: string
}

export interface EngineCompaction {
  type: 'CompactionTriggered'
  session_id: string
  reason: string
  tokens_before: number
  tokens_after: number
}

export interface EngineGoalUpdated {
  type: 'GoalUpdated'
  session_id: string
  goal_id: string
  progress: number
  status: string
}

export type EngineEvent =
  | EngineStreamDelta
  | EngineStreamEnd
  | EngineTurnStarted
  | EngineTurnCompleted
  | EngineToolCall
  | EngineError
  | EngineCompaction
  | EngineGoalUpdated
  | { type: string; [key: string]: unknown }

export interface TurnStreamState {
  turnId: string
  content: string
  isStreaming: boolean
  sequence: number
}

export interface UseEngineEventsReturn {
  /** 当前会话 ID */
  sessionId: string | null
  /** 最近事件列表 */
  events: EngineEvent[]
  /** 当前流式回合状态 */
  currentStream: TurnStreamState | null
  /** 是否正在流式输出 */
  isStreaming: boolean
  /** 回合历史 */
  turns: Array<{ id: string; number: number; userInput: string; response?: string; status: string }>
  /** 订阅引擎事件 */
  subscribe: (sid: string) => Promise<void>
  /** 取消订阅 */
  unsubscribe: () => void
  /** 清空事件 */
  clearEvents: () => void
  /** 添加回合记录 */
  addTurn: (id: string, number: number, userInput: string) => void
  /** 更新回合响应 */
  updateTurnResponse: (turnId: string, response: string, status: string) => void
}

export function useEngineEvents(): UseEngineEventsReturn {
  const [sessionId, setSessionId] = useState<string | null>(null)
  const [events, setEvents] = useState<EngineEvent[]>([])
  const [currentStream, setCurrentStream] = useState<TurnStreamState | null>(null)
  const [isStreaming, setIsStreaming] = useState(false)
  const [turns, setTurns] = useState<UseEngineEventsReturn['turns']>([])
  const unlistenRef = useRef<(() => void) | null>(null)

  // 订阅引擎事件
  const subscribe = useCallback(async (sid: string) => {
    // 先取消之前的订阅
    if (unlistenRef.current) {
      unlistenRef.current()
      unlistenRef.current = null
    }

    setSessionId(sid)
    setEvents([])
    setCurrentStream(null)
    setIsStreaming(false)
    setTurns([])

    try {
      // 调用后端订阅命令
      const { invoke } = await import('@tauri-apps/api/core')
      const { listen } = await import('@tauri-apps/api/event')

      await invoke('engine_subscribe_events', { sessionId: sid })

      // 监听 Tauri 事件
      const unlisten = await listen<EngineEvent>('yuan-engine-event', (event) => {
        const payload = event.payload
        setEvents(prev => [...prev.slice(-99), payload])

        // 处理流式事件
        if (payload.type === 'StreamDelta') {
          const delta = payload as EngineStreamDelta
          setCurrentStream(prev => {
            if (!prev || prev.turnId !== delta.turn_id) {
              return { turnId: delta.turn_id, content: delta.delta, isStreaming: true, sequence: delta.sequence }
            }
            return { ...prev, content: prev.content + delta.delta, sequence: delta.sequence }
          })
          setIsStreaming(true)
        } else if (payload.type === 'StreamEnd') {
          const end = payload as EngineStreamEnd
          setCurrentStream(prev => {
            if (prev && prev.turnId === end.turn_id) {
              // 完成流式，更新回合记录
              setTurns(prevTurns => prevTurns.map(t =>
                t.id === end.turn_id ? { ...t, response: prev.content, status: 'completed' } : t
              ))
              return { ...prev, isStreaming: false }
            }
            return prev
          })
          setIsStreaming(false)
        } else if (payload.type === 'TurnStarted') {
          const started = payload as EngineTurnStarted
          setTurns(prev => {
            if (prev.some(t => t.id === started.turn_id)) return prev
            return [...prev, { id: started.turn_id, number: started.turn_number, userInput: '', status: 'in_progress' }]
          })
        } else if (payload.type === 'TurnCompleted') {
          const completed = payload as EngineTurnCompleted
          setTurns(prev => prev.map(t =>
            t.id === completed.turn_id ? { ...t, status: 'completed' } : t
          ))
        } else if (payload.type === 'Error') {
          const err = payload as EngineError
          setTurns(prev => prev.map(t =>
            t.status === 'in_progress' ? { ...t, status: 'error' } : t
          ))
          console.error('[引擎事件] 错误:', err.error)
        }
      })

      unlistenRef.current = unlisten
    } catch (e) {
      console.warn('[引擎事件] 订阅失败:', e)
    }
  }, [])

  // 取消订阅
  const unsubscribe = useCallback(() => {
    if (unlistenRef.current) {
      unlistenRef.current()
      unlistenRef.current = null
    }
    setSessionId(null)
    setIsStreaming(false)
  }, [])

  // 清空事件
  const clearEvents = useCallback(() => {
    setEvents([])
  }, [])

  // 添加回合记录
  const addTurn = useCallback((id: string, number: number, userInput: string) => {
    setTurns(prev => [...prev, { id, number, userInput, status: 'in_progress' }])
  }, [])

  // 更新回合响应
  const updateTurnResponse = useCallback((turnId: string, response: string, status: string) => {
    setTurns(prev => prev.map(t => t.id === turnId ? { ...t, response, status } : t))
  }, [])

  // 组件卸载时清理
  useEffect(() => {
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current()
      }
    }
  }, [])

  return {
    sessionId,
    events,
    currentStream,
    isStreaming,
    turns,
    subscribe,
    unsubscribe,
    clearEvents,
    addTurn,
    updateTurnResponse,
  }
}
