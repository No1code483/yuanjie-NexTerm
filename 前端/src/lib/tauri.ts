// 检查是否在Tauri环境中（必须在运行时调用，不能作为模块级常量）
export function checkIsTauri(): boolean {
  if (typeof window === 'undefined') return false
  // Tauri 2.x 使用 __TAURI_INTERNALS__ 而非 __TAURI__
  return '__TAURI_INTERNALS__' in window || '__TAURI__' in window
}

// 系统信息接口
export interface SystemInfo {
  os: string
  arch: string
  version: string
  hostname: string
  memory: {
    total: number
    used: number
    free: number
  }
  cpu: {
    cores: number
    usage: number
  }
}

// 窗口控制
export const windowControl = {
  minimize: async () => {
    if (checkIsTauri()) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().minimize()
    }
  },
  maximize: async () => {
    if (checkIsTauri()) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().toggleMaximize()
    }
  },
  unmaximize: async () => {
    if (checkIsTauri()) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().unmaximize()
    }
  },
  isMaximized: async (): Promise<boolean> => {
    if (checkIsTauri()) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      return await getCurrentWindow().isMaximized()
    }
    return false
  },
  startDragging: async () => {
    if (checkIsTauri()) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().startDragging()
    }
  },
  close: async () => {
    if (checkIsTauri()) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().close()
    }
  },
  setAlwaysOnTop: async (alwaysOnTop: boolean) => {
    if (checkIsTauri()) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().setAlwaysOnTop(alwaysOnTop)
    }
  },
  setTitle: async (title: string) => {
    if (checkIsTauri()) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      await getCurrentWindow().setTitle(title)
    }
  },
  setSize: async (width: number, height: number) => {
    if (checkIsTauri()) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      const window = getCurrentWindow()
      const { LogicalSize } = await import('@tauri-apps/api/window')
      await window.setSize(new LogicalSize(width, height))
    }
  },
  setPosition: async (x: number, y: number) => {
    if (checkIsTauri()) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      const window = getCurrentWindow()
      const { LogicalPosition } = await import('@tauri-apps/api/window')
      await window.setPosition(new LogicalPosition(x, y))
    }
  }
}

// 文件系统操作（简化版本）
export const fileSystem = {
  readTextFile: async (_path: string): Promise<string> => {
    if (checkIsTauri()) {
      console.log('文件系统功能在Tauri 2.x中需要插件支持')
    }
    return Promise.resolve('')
  },
  writeTextFile: async (_path: string, _contents: string): Promise<void> => {
    if (checkIsTauri()) {
      console.log('文件系统功能在Tauri 2.x中需要插件支持')
    }
  }
}

// 目录操作（简化版本）
export const directory = {
  appDir: async (): Promise<string> => {
    if (checkIsTauri()) {
      console.log('目录操作功能在Tauri 2.x中需要插件支持')
    }
    return Promise.resolve('')
  }
}

// 剪贴板操作（通过后端命令）
export const clipboard = {
  readText: async (): Promise<string> => {
    if (checkIsTauri()) {
      const { invoke } = await import('@tauri-apps/api/core')
      const res = await invoke<{data?: string}>('clipboard_read_text')
      return res.data || ''
    }
    return navigator.clipboard?.readText() || Promise.resolve('')
  },
  writeText: async (text: string): Promise<void> => {
    if (checkIsTauri()) {
      const { invoke } = await import('@tauri-apps/api/core')
      await invoke<void>('clipboard_write_text', { text })
      return
    }
    await navigator.clipboard?.writeText(text)
  }
}

// 系统信息
export const system = {
  getSystemInfo: async (): Promise<SystemInfo> => {
    return {
      os: checkIsTauri() ? 'Windows' : 'Browser',
      arch: checkIsTauri() ? 'x64' : 'unknown',
      version: checkIsTauri() ? '10.0.19041' : '1.0.0',
      hostname: checkIsTauri() ? 'nexterm-pc' : 'localhost',
      memory: {
        total: checkIsTauri() ? 8589934592 : 0,
        used: checkIsTauri() ? 4294967296 : 0,
        free: checkIsTauri() ? 4294967296 : 0
      },
      cpu: {
        cores: checkIsTauri() ? 8 : 0,
        usage: checkIsTauri() ? 25.5 : 0
      }
    }
  }
}

// 进程管理（简化版本）
export const process = {
  spawn: async (_command: string, _args?: string[]): Promise<number> => {
    if (checkIsTauri()) {
      console.log('进程管理功能在Tauri 2.x中需要插件支持')
    }
    return Promise.resolve(0)
  }
}

// 通知系统（简化版本）
export const notification = {
  show: async (_title: string, _body?: string): Promise<void> => {
    if (checkIsTauri()) {
      console.log('通知功能在Tauri 2.x中需要插件支持')
    }
  }
}

// 对话框（简化版本）
export const dialog = {
  open: async (_options?: any): Promise<string | string[] | null> => {
    if (checkIsTauri()) {
      console.log('对话框功能在Tauri 2.x中需要插件支持')
    }
    return Promise.resolve(null)
  }
}

// 存储系统（使用localStorage作为统一接口）
export const storage = {
  set: async (key: string, value: any): Promise<void> => {
    localStorage.setItem(key, JSON.stringify(value))
  },
  get: async <T = any>(key: string): Promise<T | null> => {
    const value = localStorage.getItem(key)
    return value ? JSON.parse(value) : null
  },
  remove: async (key: string): Promise<void> => {
    localStorage.removeItem(key)
  },
  clear: async (): Promise<void> => {
    localStorage.clear()
  }
}

// 网络请求（使用fetch作为统一接口）
export const http = {
  get: async (url: string, options?: any): Promise<any> => {
    const response = await fetch(url, options)
    return await response.json()
  },
  post: async (url: string, data?: any, options?: any): Promise<any> => {
    const response = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers
      },
      body: JSON.stringify(data),
      ...options
    })
    return await response.json()
  }
}

// 扩展系统相关
export const extensions = {
  install: async (_path: string): Promise<void> => {
    if (checkIsTauri()) {
      console.log('扩展安装功能在Tauri 2.x中需要插件支持')
    }
  }
}

// 工具函数：退出应用
export const exitApp = async (): Promise<void> => {
  if (checkIsTauri()) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    const window = getCurrentWindow()
    await window.close()
  }
}
