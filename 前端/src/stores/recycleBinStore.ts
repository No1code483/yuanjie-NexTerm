import { create } from 'zustand'

// 对齐后端 models/recycle.rs

export type RecyclableType = 'todo' | 'knowledge_item' | 'chat_session' | 'ai_model' | 'file'

export interface RecycledItem {
  id: number
  originalId: number
  type: RecyclableType
  title: string
  description?: string
  deletedAt: string
  deletedBy: number // userId
  restoreable: boolean
  permanentDeleteAt?: string // 自动永久删除时间(可选)
}

interface RecycleBinState {
  items: RecycledItem[]
  selectedItems: Set<number>
  isLoading: boolean
  
  // Actions
  loadItems: (items: RecycledItem[]) => void
  addItem: (item: RecycledItem) => void
  removeItem: (id: number) => void
  selectItem: (id: number) => void
  deselectItem: (id: number) => void
  selectAll: () => void
  clearSelection: () => void
  restoreSelected: () => number[] // Returns restored IDs
  permanentlyDeleteSelected: () => number[] // Returns deleted IDs
  emptyBin: () => void
  filterByType: (type: RecyclableType | 'all') => RecycledItem[]
  
  // UI state
  setLoading: (loading: boolean) => void
}

type RecycleBinActions = {
  loadItems: (items: RecycledItem[]) => void
  addItem: (item: RecycledItem) => void
  removeItem: (id: number) => void
  selectItem: (id: number) => void
  deselectItem: (id: number) => void
  selectAll: () => void
  clearSelection: () => void
  restoreSelected: () => number[]
  permanentlyDeleteSelected: () => number[]
  emptyBin: () => void
  filterByType: (type: RecyclableType | 'all') => RecycledItem[]
  setLoading: (loading: boolean) => void
}

export const useRecycleBinStore = create<RecycleBinState & RecycleBinActions>()((set, get) => ({
  items: [],
  selectedItems: new Set(),
  isLoading: false,

  loadItems: (items) => set({ items }),

  addItem: (item) =>
    set((state) => ({ items: [item, ...state.items] })),

  removeItem: (id) =>
    set((state) => ({
      items: state.items.filter((item) => item.id !== id),
      selectedItems: (() => {
        const newSet = new Set(state.selectedItems)
        newSet.delete(id)
        return newSet
      })()
    })),

  selectItem: (id) =>
    set((state) => {
      const newSet = new Set(state.selectedItems)
      newSet.add(id)
      return { selectedItems: newSet }
    }),

  deselectItem: (id) =>
    set((state) => {
      const newSet = new Set(state.selectedItems)
      newSet.delete(id)
      return { selectedItems: newSet }
    }),

  selectAll: () =>
    set((state) => ({
      selectedItems: new Set(state.items.map(item => item.id))
    })),

  clearSelection: () => set({ selectedItems: new Set() }),

  restoreSelected: () => {
    const { selectedItems } = get()
    const restoredIds = Array.from(selectedItems)
    
    set((state) => ({
      items: state.items.filter((item) => !selectedItems.has(item.id)),
      selectedItems: new Set()
    }))
    
    // TODO: 调用后端 API 恢复数据
    // ipc.invoke('recycle_restore', { ids: restoredIds })
    
    return restoredIds
  },

  permanentlyDeleteSelected: () => {
    const { selectedItems } = get()
    const deletedIds = Array.from(selectedItems)
    
    set((state) => ({
      items: state.items.filter((item) => !selectedItems.has(item.id)),
      selectedItems: new Set()
    }))
    
    // TODO: 调用后端 API 永久删除
    // ipc.invoke('recycle_permanent_delete', { ids: deletedIds })
    
    return deletedIds
  },

  emptyBin: () => {
    const { items } = get()
    const allIds = items.map(item => item.id)
    
    set({ items: [], selectedItems: new Set() })
    
    // TODO: 调用后端 API 清空回收站
    // ipc.invoke('recycle_empty')
    
    return allIds
  },

  filterByType: (type) => {
    const { items } = get()
    if (type === 'all') return items
    return items.filter(item => item.type === type)
  },

  setLoading: (loading) => set({ isLoading: loading })
}))
