// Home 模块共享类型定义

export interface NewsItem {
  id: number
  title: string
  url?: string
  source?: string
  summary?: string
  content?: string
  category?: string
  published_at?: string
  fetched_at: number
  is_read: boolean
  is_favorite: boolean
  ai_summary?: string
}

export interface TodoItem {
  id: number
  title: string
  description?: string
  priority: 'low' | 'medium' | 'high'
  due_date?: string
  completed: boolean
  createdAt?: string
}

export interface JournalEntry {
  id: number
  date: string
  content: string | null
}

export interface FetchStage {
  timeRange: [number, number]
  percent: [number, number]
  stage: string
  detail: string
}

export interface RefreshProgress {
  stage: string
  percent: number
  elapsed: number
  isWarning: boolean
}

export interface TodoEnhanceResult {
  suggest_priority: string
  suggest_estimate_minutes: number
  suggest_category: string | null
  suggestions: string[]
}

export interface JournalFillResult {
  template: string
  suggestions: string[]
}