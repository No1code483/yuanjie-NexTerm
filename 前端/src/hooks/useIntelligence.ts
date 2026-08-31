/**
 * useIntelligence Hook - 底层智能被动 AI 控制
 *
 * 提供：
 * 1. aiOn - 全局 AI 主开关状态
 * 2. featureOn(key) - 检查特定功能开关
 * 3. llmConfig - 当前 LLM 配置（provider/endpoint/model）
 * 4. isLoading - 设置是否加载完成
 *
 * 使用方式：
 * const { aiOn, featureOn, llmConfig } = useIntelligence()
 * if (aiOn && featureOn('smart_complete')) { // auto-trigger AI }
 */

import { useState, useEffect, useCallback } from 'react'
import { ipc } from '@/lib/ipc'

interface LLMConfig {
  provider: string
  endpoint: string
  model: string
  apiKey: string
}

interface UseIntelligenceReturn {
  aiOn: boolean
  featureOn: (key: string) => boolean
  llmConfig: LLMConfig
  llmConfigured: boolean
  isLoading: boolean
}

export function useIntelligence(): UseIntelligenceReturn {
  const [toggles, setToggles] = useState<Record<string, boolean>>({
    master_switch: true,
    auto_suggest: true,
    context_collection: true,
    activity_tracking: true,
    focus_detection: false,
    smart_complete: true,
    auto_classify: true,
    news_summary: false,
    quote_check: false,
    command_suggest: true,
    game_recommend: false,
    search_enhance: true,
    timer_smart: true,
    floating_ball: false,
  })
  const [llmConfig, setLlmConfig] = useState<LLMConfig>({
    provider: 'ollama',
    endpoint: 'http://localhost:11434',
    model: '',
    apiKey: '',
  })
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
ipc.invoke<any>('get_system_config', { key: 'intelligence_frontend_settings' }).then(res => {
      if (res?.code === 0 && res?.data?.config_value) {
        try {
          const parsed = JSON.parse(res.data.config_value)
          if (parsed.toggles) {
            setToggles(prev => ({ ...prev, ...parsed.toggles }))
          }
          if (parsed.llmProvider) setLlmConfig(c => ({ ...c, provider: parsed.llmProvider }))
          if (parsed.llmEndpoint) setLlmConfig(c => ({ ...c, endpoint: parsed.llmEndpoint }))
          if (parsed.llmModel) setLlmConfig(c => ({ ...c, model: parsed.llmModel }))
          if (parsed.llmApiKey) setLlmConfig(c => ({ ...c, apiKey: parsed.llmApiKey }))
        } catch (e) {
          console.error('解析底层智能设置失败:', e)
        }
      }
      setIsLoading(false)
    }).catch(() => {
      setIsLoading(false)
    })
  }, [])

  const aiOn = toggles.master_switch !== false

  const featureOn = useCallback((key: string): boolean => {
    if (!aiOn) return false
    return toggles[key] !== false
  }, [aiOn, toggles])

  const llmConfigured = llmConfig.provider.length > 0 && llmConfig.endpoint.length > 0

  return { aiOn, featureOn, llmConfig, llmConfigured, isLoading }
}