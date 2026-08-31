/**
 * 万能转接头扩展系统类型定义
 */

export interface NexTermAdapter<T = any> {
  // 基础信息
  id: string
  name: string
  type: string
  version: string
  description: string
  author?: string
  
  // 元数据
  icon?: string
  category?: string
  tags?: string[]
  
  // 功能数据
  data: T
  
  // 生命周期钩子
  onMount?: () => Promise<void> | void
  onUnmount?: () => Promise<void> | void
  onMessage?: (msg: any) => Promise<void> | void
  
  // 权限要求
  permissions?: ExtensionPermission[]
  
  // 配置选项
  config?: ExtensionConfig
}

export interface ExtensionConfig {
  autoStart?: boolean
  enabled?: boolean
  settings?: Record<string, any>
}

export interface ExtensionPermission {
  name: string
  description: string
  required: boolean
}

export interface ExtensionRegistry {
  [id: string]: NexTermAdapter
}

export interface ExtensionManager {
  // 插件管理
  register(adapter: NexTermAdapter): Promise<void>
  unregister(id: string): Promise<void>
  get(id: string): NexTermAdapter | undefined
  getAll(): NexTermAdapter[]
  
  // 生命周期管理
  mount(id: string): Promise<void>
  unmount(id: string): Promise<void>
  
  // 消息通信
  sendMessage(id: string, message: any): Promise<void>
  
  // 配置管理
  updateConfig(id: string, config: Partial<ExtensionConfig>): Promise<void>
  
  // 事件监听
  on(event: 'extensionRegistered', listener: (adapter: NexTermAdapter) => void): void
  on(event: 'extensionUnregistered', listener: (id: string) => void): void
  on(event: 'extensionMounted', listener: (id: string) => void): void
  on(event: 'extensionUnmounted', listener: (id: string) => void): void
  off(event: string, listener: Function): void
}

export interface ExtensionContext {
  adapter: NexTermAdapter
  manager: ExtensionManager
  
  // 工具方法
  invoke<T = any>(command: string, args?: any): Promise<T>
  storage: {
    get<T = any>(key: string): Promise<T | null>
    set<T = any>(key: string, value: T): Promise<void>
    delete(key: string): Promise<void>
  }
  
  // 事件系统
  emit(event: string, data?: any): void
  on(event: string, listener: Function): void
  off(event: string, listener: Function): void
}

// 内置扩展类型定义
export const BUILTIN_EXTENSION_TYPES = {
  UTILITY: 'utility',
  TOOL: 'tool', 
  GAME: 'game',
  AI: 'ai',
  COMMUNICATION: 'communication',
  PRODUCTIVITY: 'productivity',
  SYSTEM: 'system'
} as const

export type ExtensionType = typeof BUILTIN_EXTENSION_TYPES[keyof typeof BUILTIN_EXTENSION_TYPES]

// 扩展事件类型
export const EXTENSION_EVENTS = {
  EXTENSION_READY: 'extension:ready',
  EXTENSION_ERROR: 'extension:error',
  EXTENSION_MESSAGE: 'extension:message',
  EXTENSION_CONFIG_CHANGED: 'extension:configChanged'
} as const