/**
 * 补全记忆模块
 * 对标 VSCode suggestMemory.ts
 * 记录用户选择的补全项，提升常用补全的排序
 */

const STORAGE_KEY = 'nexterm_completion_memory'

interface MemoryEntry {
  label: string
  language: string
  count: number
  lastUsed: number
}

interface CompletionMemoryData {
  entries: Record<string, MemoryEntry>
}

function getKey(label: string, language: string): string {
  return `${language}::${label}`
}

class CompletionMemory {
  private entries: Record<string, MemoryEntry> = {}
  private loaded = false

  private load(): void {
    if (this.loaded) return
    this.loaded = true
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      if (raw) {
        const data: CompletionMemoryData = JSON.parse(raw)
        this.entries = data.entries || {}
        // 清理超过 30 天未使用的条目
        const now = Date.now()
        const thirtyDays = 30 * 24 * 60 * 60 * 1000
        for (const key of Object.keys(this.entries)) {
          if (now - this.entries[key].lastUsed > thirtyDays) {
            delete this.entries[key]
          }
        }
      }
    } catch {
      this.entries = {}
    }
  }

  private save(): void {
    try {
      const data: CompletionMemoryData = { entries: this.entries }
      localStorage.setItem(STORAGE_KEY, JSON.stringify(data))
    } catch {
      // localStorage 不可用
    }
  }

  /** 记录一次补全选择 */
  record(label: string, language: string): void {
    this.load()
    const key = getKey(label, language)
    const existing = this.entries[key]
    if (existing) {
      existing.count++
      existing.lastUsed = Date.now()
    } else {
      this.entries[key] = {
        label,
        language,
        count: 1,
        lastUsed: Date.now(),
      }
    }
    this.save()
  }

  /** 获取补全的排序权重（0-100，越高越靠前） */
  getScore(label: string, language: string): number {
    this.load()
    const key = getKey(label, language)
    const entry = this.entries[key]
    if (!entry) return 0

    // 使用次数 + 最近使用时间衰减
    const recency = Math.max(0, 1 - (Date.now() - entry.lastUsed) / (7 * 24 * 60 * 60 * 1000))
    return Math.min(100, Math.round(entry.count * 10 * recency))
  }

  /** 获取所有记忆条目 */
  getAll(): MemoryEntry[] {
    this.load()
    return Object.values(this.entries).sort((a, b) => b.count - a.count)
  }

  /** 清除所有记忆 */
  clear(): void {
    this.entries = {}
    this.save()
  }
}

export const completionMemory = new CompletionMemory()