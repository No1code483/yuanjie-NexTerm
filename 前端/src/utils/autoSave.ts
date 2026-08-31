/**
 * 自动保存工具函数
 * 实现所有输入内容的实时自动保存
 */

import React from 'react'

// 存储键名前缀
const STORAGE_PREFIX = 'nt_auto_save_'

/**
 * 获取自动保存的存储键名
 */
function getStorageKey(key: string): string {
  return `${STORAGE_PREFIX}${key}`
}

/**
 * 自动保存配置
 */
interface AutoSaveConfig {
  debounceDelay?: number // 防抖延迟（毫秒）
  maxRetries?: number    // 最大重试次数
  retryDelay?: number    // 重试延迟（毫秒）
}

/**
 * 自动保存管理器
 */
class AutoSaveManager {
  private timers: Map<string, ReturnType<typeof setTimeout>> = new Map()
  private defaultConfig: Required<AutoSaveConfig> = {
    debounceDelay: 500,
    maxRetries: 3,
    retryDelay: 1000
  }

  /**
   * 自动保存数据
   */
  async saveData<T>(key: string, data: T, config: AutoSaveConfig = {}): Promise<boolean> {
    const mergedConfig = { ...this.defaultConfig, ...config }
    
    try {
      // 清除之前的定时器
      this.clearTimer(key)
      
      // 设置新的定时器（防抖）
      return new Promise((resolve) => {
        const timer = setTimeout(async () => {
          try {
            await this.saveToStorage(key, data, mergedConfig)
            resolve(true)
          } catch (error) {
            console.error(`自动保存失败: ${key}`, error)
            resolve(false)
          }
        }, mergedConfig.debounceDelay)
        
        this.timers.set(key, timer)
      })
    } catch (error) {
      console.error(`自动保存设置失败: ${key}`, error)
      return false
    }
  }

  /**
   * 保存到存储（带重试机制）
   */
  private async saveToStorage<T>(
    key: string, 
    data: T, 
    config: Required<AutoSaveConfig>,
    retryCount = 0
  ): Promise<void> {
    try {
      const storageKey = getStorageKey(key)
      const serializedData = JSON.stringify({
        data,
        timestamp: Date.now(),
        version: '1.0'
      })
      
      localStorage.setItem(storageKey, serializedData)
      
      // 触发保存成功事件
      this.dispatchSaveEvent(key, 'success', data)
      
    } catch (error) {
      if (retryCount < config.maxRetries) {
        // 重试
        await new Promise(resolve => setTimeout(resolve, config.retryDelay))
        return this.saveToStorage(key, data, config, retryCount + 1)
      } else {
        // 重试次数用尽，触发失败事件
        this.dispatchSaveEvent(key, 'error', data, error)
        throw error
      }
    }
  }

  /**
   * 加载保存的数据
   */
  loadData<T>(key: string): T | null {
    try {
      const storageKey = getStorageKey(key)
      const stored = localStorage.getItem(storageKey)
      
      if (!stored) {
        return null
      }
      
      const parsed = JSON.parse(stored)
      
      // 验证数据格式
      if (parsed.version === '1.0' && parsed.data !== undefined) {
        return parsed.data as T
      }
      
      return null
    } catch (error) {
      console.error(`加载保存数据失败: ${key}`, error)
      return null
    }
  }

  /**
   * 清除定时器
   */
  private clearTimer(key: string): void {
    const timer = this.timers.get(key)
    if (timer) {
      clearTimeout(timer as unknown as number)
      this.timers.delete(key)
    }
  }

  /**
   * 触发保存事件
   */
  private dispatchSaveEvent<T>(key: string, type: 'success' | 'error', data: T, error?: any): void {
    const event = new CustomEvent('autosave', {
      detail: {
        key,
        type,
        data,
        error,
        timestamp: Date.now()
      }
    })
    
    window.dispatchEvent(event)
  }

  /**
   * 清除所有定时器
   */
  destroy(): void {
    for (const timer of this.timers.values()) {
      clearTimeout(timer as unknown as number)
    }
    this.timers.clear()
  }
}

// 创建全局自动保存管理器实例
const autoSaveManager = new AutoSaveManager()

/**
 * 自动保存钩子
 */
export function useAutoSave<T>(key: string, initialValue: T, config?: AutoSaveConfig) {
  const [value, setValue] = React.useState<T>(() => {
    // 初始化时加载保存的数据
    const saved = autoSaveManager.loadData<T>(key)
    return saved !== null ? saved : initialValue
  })

  // 自动保存效果
  React.useEffect(() => {
    if (value !== initialValue) {
      autoSaveManager.saveData(key, value, config)
    }
  }, [value, key, config, initialValue])

  // 清理效果
  React.useEffect(() => {
    return () => {
      // 组件卸载时保存最终状态
      autoSaveManager.saveData(key, value, { debounceDelay: 0 })
    }
  }, [key, value])

  return [value, setValue] as const
}

/**
 * 自动保存输入框组件（已移动到AutoSaveIndicator组件）
 */

/**
 * 自动保存文本区域组件（已移动到AutoSaveIndicator组件）
 */

/**
 * 批量自动保存管理器
 */
export class BatchAutoSaveManager {
  private saves: Array<{ key: string; data: any }> = []

  /**
   * 添加保存任务
   */
  addSave<T>(key: string, data: T): void {
    this.saves.push({ key, data })
  }

  /**
   * 批量保存
   */
  async saveAll(config?: AutoSaveConfig): Promise<boolean[]> {
    const results = await Promise.all(
      this.saves.map(({ key, data }) => 
        autoSaveManager.saveData(key, data, config)
      )
    )
    
    this.saves = []
    return results
  }

  /**
   * 清除所有保存任务
   */
  clear(): void {
    this.saves = []
  }
}

/**
 * 导出全局自动保存管理器
 */
export default autoSaveManager