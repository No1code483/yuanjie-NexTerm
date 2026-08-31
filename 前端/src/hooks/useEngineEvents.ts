// 引擎事件监听 Hook — 对接后端 EventBus 流式事件
// 用于 Yuan Code 页面实时接收 AI 会话事件

import { useEffect, useRef, useState, useCallback } from 'react'
import { listen, UnlistenFn } from '@tauri-apps/api/event'

/** 引擎事件类型 */
export interface EngineEvent {
  type: string
  session_id?: string
  turn_id?: string
  [key: string]: unknown
}

/** 流式增量事件 */
export interface StreamDeltaEvent extends EngineEvent {
  type: 'StreamDelta'
  session_id: string
  turn_id: string
  delta: string
  sequence: number
}

/** Turn 开始事件 */
export interface TurnStartedEvent extends EngineEvent {
  type: 'TurnStarted'
  session_id: string
  turn_id: string
  turn_number: number
}

/** Turn 完成事件 */
export interface TurnCompletedEvent extends EngineEvent {
  type: 'TurnCompleted'
  session_id: string
  turn_id: string
  token_usage?: {
    input_tokens: number
    output_tokens: number
    total_tokens: number
  }
}

/** 流结束事件 */
export interface StreamEndEvent extends EngineEvent {
  type: 'StreamEnd'
  session_id: string
  turn_id: string
}

/** 错误事件 */
export interface ErrorEvent extends EngineEvent {
  type: 'Error'
  session_id: string
  error: string
}

/** 工具调用事件 */
export interface ToolCallEvent extends EngineEvent {
  type: 'ToolCallStarted' | 'ToolCallCompleted'
  session_id: string
  turn_id: string
  tool_name: string
  tool_input?: string
  tool_output?: string
  duration_ms?: number
}

/** 事件回调映射 */
export interface EngineEventCallbacks {
  onTurnStarted?: (event: TurnStartedEvent) => void
  onTurnCompleted?: (event: TurnCompletedEvent) => void
  onStreamDelta?: (event: StreamDeltaEvent) => void
  onStreamEnd?: (event: StreamEndEvent) => void
  onError?: (event: ErrorEvent) => void
  onToolCallStarted?: (event: ToolCallEvent) => void
  onToolCallCompleted?: (event: ToolCallEvent) => void
  onCompactionTriggered?: (event: EngineEvent) => void
  onGoalUpdated?: (event: EngineEvent) => void
  onAgentStatusChanged?: (event: EngineEvent) => void
  onSessionStateChanged?: (event: EngineEvent) => void
}

/**
 * 引擎事件监听 Hook
 * 自动订阅后端事件流，组件卸载时自动取消订阅
 */
export function useEngineEvents(callbacks: EngineEventCallbacks) {
  const callbacksRef = useRef(callbacks)
  callbacksRef.current = callbacks

  const [isConnected, setIsConnected] = useState(false)
  const [eventCount, setEventCount] = useState(0)
  const [lastEvent, setLastEvent] = useState<EngineEvent | null>(null)
  const unlistenRef = useRef<UnlistenFn | undefined>(undefined)

  useEffect(() => {
    let cancelled = false

    async function subscribe() {
      try {
        const unlisten = await listen<EngineEvent>('yuan-engine-event', (event) => {
          if (cancelled) return
          const payload = event.payload
          const cbs = callbacksRef.current

          setEventCount(prev => prev + 1)
          setLastEvent(payload)

          switch (payload.type) {
            case 'StreamDelta':
              cbs.onStreamDelta?.(payload as StreamDeltaEvent)
              break
            case 'TurnStarted':
              cbs.onTurnStarted?.(payload as TurnStartedEvent)
              break
            case 'TurnCompleted':
              cbs.onTurnCompleted?.(payload as TurnCompletedEvent)
              break
            case 'StreamEnd':
              cbs.onStreamEnd?.(payload as StreamEndEvent)
              break
            case 'Error':
              cbs.onError?.(payload as ErrorEvent)
              break
            case 'ToolCallStarted':
              cbs.onToolCallStarted?.(payload as ToolCallEvent)
              break
            case 'ToolCallCompleted':
              cbs.onToolCallCompleted?.(payload as ToolCallEvent)
              break
            case 'CompactionTriggered':
              cbs.onCompactionTriggered?.(payload)
              break
            case 'GoalUpdated':
              cbs.onGoalUpdated?.(payload)
              break
            case 'AgentStatusChanged':
              cbs.onAgentStatusChanged?.(payload)
              break
            case 'SessionStateChanged':
              cbs.onSessionStateChanged?.(payload)
              break
          }
        })

        if (cancelled) {
          unlisten()
        } else {
          unlistenRef.current = unlisten
          setIsConnected(true)
        }
      } catch (err) {
        console.error('[useEngineEvents] 订阅失败:', err)
        if (!cancelled) {
          setIsConnected(false)
        }
      }
    }

    subscribe()

    return () => {
      cancelled = true
      if (unlistenRef.current) {
        unlistenRef.current()
        unlistenRef.current = undefined
      }
    }
  }, [])

  const reset = useCallback(() => {
    setEventCount(0)
    setLastEvent(null)
  }, [])

  return {
    isConnected,
    eventCount,
    lastEvent,
    reset,
  }
}

/**
 * 流式文本累积 Hook
 * 自动累积 StreamDelta 事件，重建完整响应文本
 */
export function useStreamAccumulator() {
  const [accumulatedText, setAccumulatedText] = useState('')
  const [isStreaming, setIsStreaming] = useState(false)
  const [currentTurnId, setCurrentTurnId] = useState<string | null>(null)

  const callbacks: EngineEventCallbacks = {
    onTurnStarted: (event) => {
      setAccumulatedText('')
      setIsStreaming(true)
      setCurrentTurnId(event.turn_id)
    },
    onStreamDelta: (event) => {
      setAccumulatedText(prev => prev + event.delta)
    },
    onStreamEnd: () => {
      setIsStreaming(false)
    },
    onError: () => {
      setIsStreaming(false)
    },
  }

  const engine = useEngineEvents(callbacks)

  const reset = useCallback(() => {
    setAccumulatedText('')
    setIsStreaming(false)
    setCurrentTurnId(null)
  }, [])

  return {
    accumulatedText,
    isStreaming,
    currentTurnId,
    reset,
    engine,
  }
}