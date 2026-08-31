import { 
  NexTermAdapter, 
  ExtensionContext as IExtensionContext,
  ExtensionManager 
} from '../types/extensions'

/**
 * 扩展上下文实现
 */
export class ExtensionContext implements IExtensionContext {
  constructor(
    public adapter: NexTermAdapter,
    public manager: ExtensionManager
  ) {
    this.setupEventListeners()
  }

  private eventListeners: Map<string, Function[]> = new Map()

  async invoke<T = any>(command: string, args?: any): Promise<T> {
    // 这里可以调用Tauri命令或其他系统功能
    // 暂时返回模拟数据
    console.log(`调用命令: ${command}`, args)
    
    // 模拟异步调用
    return new Promise<T>((resolve) => {
      setTimeout(() => {
        resolve({ success: true, command, args } as T)
      }, 100)
    })
  }

  storage = {
    get: async <T = any>(key: string): Promise<T | null> => {
      try {
        const value = localStorage.getItem(`extension_${this.adapter.id}_${key}`)
        return value ? JSON.parse(value) : null
      } catch (error) {
        console.error('存储读取失败:', error)
        return null
      }
    },

    set: async <T = any>(key: string, value: T): Promise<void> => {
      try {
        localStorage.setItem(`extension_${this.adapter.id}_${key}`, JSON.stringify(value))
      } catch (error) {
        console.error('存储写入失败:', error)
        throw error
      }
    },

    delete: async (key: string): Promise<void> => {
      try {
        localStorage.removeItem(`extension_${this.adapter.id}_${key}`)
      } catch (error) {
        console.error('存储删除失败:', error)
        throw error
      }
    }
  }

  emit(event: string, data?: any): void {
    const listeners = this.eventListeners.get(event)
    if (listeners) {
      listeners.forEach(listener => {
        try {
          listener(data)
        } catch (error) {
          console.error(`事件发射失败: ${event}`, error)
        }
      })
    }
  }

  on(event: string, listener: Function): void {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, [])
    }
    this.eventListeners.get(event)!.push(listener)
  }

  off(event: string, listener: Function): void {
    const listeners = this.eventListeners.get(event)
    if (listeners) {
      const index = listeners.indexOf(listener)
      if (index > -1) {
        listeners.splice(index, 1)
      }
    }
  }

  private setupEventListeners(): void {
    // 监听管理器事件
    this.manager.on('extensionMounted', (id: string) => {
      if (id === this.adapter.id) {
        this.emit('mounted')
      }
    })

    this.manager.on('extensionUnmounted', (id: string) => {
      if (id === this.adapter.id) {
        this.emit('unmounted')
      }
    })
  }

  // 销毁上下文
  destroy(): void {
    this.eventListeners.clear()
  }
}

/**
 * 创建扩展上下文工厂函数
 */
export function createExtensionContext(
  adapter: NexTermAdapter, 
  manager: ExtensionManager
): ExtensionContext {
  return new ExtensionContext(adapter, manager)
}