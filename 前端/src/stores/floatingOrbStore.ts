import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import { random } from '@/lib/utils'

export type OrbType = 'summary' | 'suggestion' | 'tag' | 'insight' | 'notification'

export interface FloatingOrb {
  id: string
  type: OrbType
  title: string           // 简短标题（收起时显示）
  content: string         // 完整内容（展开时显示）
  keyPoints?: string[]    // 要点列表（摘要类用）
  source: string          // 来源页面标识（knowledge/home/ai/...）
  sourceId?: number       // 关联的条目ID
  icon: string            // 图标 emoji
  color?: string          // 主题色（可选）
  favorited: boolean      // 是否已收藏
  expanded: boolean       // 是否展开
  createdAt: number       // 创建时间戳
  lastInteractedAt: number // 最后交互时间戳
}

interface FloatingOrbState {
  orbs: FloatingOrb[]
  favorites: FloatingOrb[]

  // 添加 orb
  addOrb: (orb: Omit<FloatingOrb, 'id' | 'createdAt' | 'lastInteractedAt' | 'expanded' | 'favorited'>) => string

  // 删除 orb
  removeOrb: (id: string) => void

  // 切换展开/收起
  toggleExpand: (id: string) => void

  // 收藏/取消收藏
  toggleFavorite: (id: string) => void

  // 更新交互时间（用于3小时自动清理）
  touchOrb: (id: string) => void

  // 清理过期 orb（超过3小时未交互且未收藏）
  cleanupExpired: () => void

  // 清除所有非收藏 orb
  clearAll: () => void
}

const AUTO_EXPIRE_MS = 3 * 60 * 60 * 1000 // 3 小时

export const useFloatingOrbStore = create<FloatingOrbState>()(
  persist(
    (set, get) => ({
      orbs: [],
      favorites: [],

      addOrb: (orbData) => {
        const id = random.uid('orb_')
        const now = Date.now()
        const orb: FloatingOrb = {
          ...orbData,
          id,
          favorited: false,
          expanded: true, // 新创建默认展开
          createdAt: now,
          lastInteractedAt: now,
        }
        set(state => ({ orbs: [orb, ...state.orbs] }))
        return id
      },

      removeOrb: (id) => {
        set(state => ({ orbs: state.orbs.filter(o => o.id !== id) }))
      },

      toggleExpand: (id) => {
        set(state => ({
          orbs: state.orbs.map(o =>
            o.id === id ? { ...o, expanded: !o.expanded, lastInteractedAt: Date.now() } : o
          ),
        }))
      },

      toggleFavorite: (id) => {
        const state = get()
        const orb = state.orbs.find(o => o.id === id)
        if (!orb) return

        if (orb.favorited) {
          // 取消收藏：从 favorites 移除
          set({
            favorites: state.favorites.filter(f => f.id !== id),
            orbs: state.orbs.map(o => o.id === id ? { ...o, favorited: false } : o),
          })
        } else {
          // 加入收藏
          const favOrb = { ...orb, favorited: true }
          set({
            favorites: [favOrb, ...state.favorites],
            orbs: state.orbs.map(o => o.id === id ? { ...o, favorited: true } : o),
          })
        }
      },

      touchOrb: (id) => {
        set(state => ({
          orbs: state.orbs.map(o =>
            o.id === id ? { ...o, lastInteractedAt: Date.now() } : o
          ),
        }))
      },

      cleanupExpired: () => {
        const now = Date.now()
        set(state => ({
          orbs: state.orbs.filter(o => o.favorited || (now - o.lastInteractedAt) < AUTO_EXPIRE_MS),
        }))
      },

      clearAll: () => {
        set(state => ({
          orbs: state.orbs.filter(o => o.favorited), // 保留收藏的
        }))
      },
    }),
    {
      name: 'nexterm-floating-orbs',
      partialize: (state) => ({ favorites: state.favorites }), // 只持久化收藏，活跃 orb 不持久化（重启清空）
    }
  )
)
