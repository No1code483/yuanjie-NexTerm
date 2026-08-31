/**
 * Tauri Store 持久化存储适配器
 * 
 * 替代 localStorage 存储敏感数据（如 Token），
 * 数据存储在应用数据目录的 JSON 文件中，不暴露在 WebView 的 localStorage 中，
 * 有效防止 XSS 攻击窃取 Token。
 * 
 * 使用方式:
 * import { tauriStorage } from '@/lib/tauriStorage'
 * // 在 Zustand persist 中使用: storage: tauriStorage
 */

import { load, type Store } from '@tauri-apps/plugin-store'

let _store: Store | null = null
let _initPromise: Promise<Store> | null = null

async function getStore(): Promise<Store> {
  if (_store) return _store
  if (!_initPromise) {
    _initPromise = load('nexterm-auth.dat', { autoSave: 100, defaults: {} })
  }
  _store = await _initPromise
  return _store
}

export const tauriStorage = {
  getItem: async (name: string): Promise<string | null> => {
    try {
      const store = await getStore()
      const val = await store.get<string>(name)
      return val ?? null
    } catch {
      return null
    }
  },

  setItem: async (name: string, value: string): Promise<void> => {
    try {
      const store = await getStore()
      await store.set(name, value)
    } catch {
      console.error('[TauriStorage] 写入失败:', name)
    }
  },

  removeItem: async (name: string): Promise<void> => {
    try {
      const store = await getStore()
      await store.delete(name)
    } catch {
      console.error('[TauriStorage] 删除失败:', name)
    }
  },
}