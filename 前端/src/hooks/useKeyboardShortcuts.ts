import { useEffect, useCallback } from 'react'

export interface ShortcutConfig {
  key: string
  ctrl?: boolean
  shift?: boolean
  alt?: boolean
  /** metaKey = Cmd on Mac, Win on Windows */
  meta?: boolean
  action: () => void
  /** 描述，用于快捷键帮助面板 */
  description?: string
  /** 是否在输入框内也触发（默认 false，输入框内不触发） */
  allowInInput?: boolean
  /** 优先级，数字越小越优先，默认 0 */
  priority?: number
}

// 忽略快捷键的元素标签
const INPUT_TAGS = new Set(['INPUT', 'TEXTAREA', 'SELECT'])

/** 格式化快捷键为显示文本 */
export function formatShortcut(config: ShortcutConfig): string {
  const parts: string[] = []
  if (config.ctrl || config.meta) parts.push('Ctrl')
  if (config.alt) parts.push('Alt')
  if (config.shift) parts.push('Shift')
  parts.push(config.key.toUpperCase())
  return parts.join('+')
}

// ============================================================
// 全局快捷键注册中心
// ============================================================
type ShortcutEntry = ShortcutConfig & { id: string }

let globalShortcuts: ShortcutEntry[] = []
let globalListener: ((e: KeyboardEvent) => void) | null = null

function ensureGlobalListener() {
  if (globalListener) return

  globalListener = (event: KeyboardEvent) => {
    const target = event.target as HTMLElement
    const isInput = INPUT_TAGS.has(target.tagName) || target.isContentEditable

    // 按优先级排序
    const sorted = [...globalShortcuts].sort((a, b) => (b.priority ?? 0) - (a.priority ?? 0))

    for (const shortcut of sorted) {
      // 输入框内默认不触发快捷键
      if (isInput && !shortcut.allowInInput) continue

      const ctrlOrMeta = shortcut.ctrl || shortcut.meta
      const matchesCtrl = ctrlOrMeta ? (event.ctrlKey || event.metaKey) : (!event.ctrlKey && !event.metaKey)

      if (
        event.key.toLowerCase() === shortcut.key.toLowerCase() &&
        matchesCtrl &&
        (!!shortcut.alt === event.altKey) &&
        (!!shortcut.shift === event.shiftKey)
      ) {
        event.preventDefault()
        event.stopPropagation()
        shortcut.action()
        return
      }
    }
  }

  document.addEventListener('keydown', globalListener)
}

/** 注册全局快捷键（不依赖 React 组件生命周期） */
export function registerGlobalShortcut(id: string, config: ShortcutConfig): () => void {
  ensureGlobalListener()
  globalShortcuts = globalShortcuts.filter((s) => s.id !== id)
  globalShortcuts.push({ ...config, id })
  return () => {
    globalShortcuts = globalShortcuts.filter((s) => s.id !== id)
  }
}

/** 获取所有已注册的全局快捷键 */
export function getGlobalShortcuts(): ShortcutEntry[] {
  return [...globalShortcuts]
}

// ============================================================
// React Hook: 组件级快捷键
// ============================================================

/**
 * 注册组件级快捷键（组件卸载时自动清理）
 *
 * @example
 * useKeyboardShortcuts([
 *   { key: 'k', ctrl: true, action: () => openSearch(), description: '搜索' },
 *   { key: 'Escape', action: () => closeModal(), allowInInput: true },
 * ])
 */
export function useKeyboardShortcuts(shortcuts: ShortcutConfig[]) {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement
      const isInput = INPUT_TAGS.has(target.tagName) || target.isContentEditable

      for (const shortcut of shortcuts) {
        if (isInput && !shortcut.allowInInput) continue

        const ctrlOrMeta = shortcut.ctrl || shortcut.meta
        const matchesCtrl = ctrlOrMeta ? (event.ctrlKey || event.metaKey) : (!event.ctrlKey && !event.metaKey)

        if (
          event.key.toLowerCase() === shortcut.key.toLowerCase() &&
          matchesCtrl &&
          (!!shortcut.alt === event.altKey) &&
          (!!shortcut.shift === event.shiftKey)
        ) {
          event.preventDefault()
          shortcut.action()
          break
        }
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [shortcuts])
}

/**
 * 注册单个快捷键的便捷 Hook
 *
 * @example
 * useShortcut('k', { ctrl: true }, () => openSearch())
 */
export function useShortcut(
  key: string,
  modifiers: { ctrl?: boolean; shift?: boolean; alt?: boolean; meta?: boolean; allowInInput?: boolean },
  action: () => void,
  deps: unknown[] = [],
) {
  const stableAction = useCallback(action, deps)
  const config: ShortcutConfig = { key, ...modifiers, action: stableAction }

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement
      const isInput = INPUT_TAGS.has(target.tagName) || target.isContentEditable
      if (isInput && !config.allowInInput) return

      const ctrlOrMeta = config.ctrl || config.meta
      const matchesCtrl = ctrlOrMeta ? (event.ctrlKey || event.metaKey) : (!event.ctrlKey && !event.metaKey)

      if (
        event.key.toLowerCase() === key.toLowerCase() &&
        matchesCtrl &&
        (!!config.alt === event.altKey) &&
        (!!config.shift === event.shiftKey)
      ) {
        event.preventDefault()
        stableAction()
      }
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [key, config.ctrl, config.shift, config.alt, config.meta, config.allowInInput, stableAction])
}