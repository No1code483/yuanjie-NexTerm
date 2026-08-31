import { create } from 'zustand'

export interface TimerState {
  shortTimers: ShortTimer[]
  longTimers: LongTimer[]
  activeShortTimerId: number | null
  activeLongTimerId: number | null
}

export interface ShortTimer {
  id: number
  name: string
  duration: number // seconds
  remaining: number // seconds
  status: 'running' | 'paused' | 'completed'
  startedAt?: string
  finishedAt?: string
}

export interface LongTimer {
  id: number
  name: string
  targetDate: string // ISO date string
  remainingDays: number
  completed: boolean
}

interface TimerActions {
  // Short timer actions
  addShortTimer: (name: string, duration: number) => void
  startShortTimer: (id: number) => void
  pauseShortTimer: (id: number) => void
  resetShortTimer: (id: number) => void
  deleteShortTimer: (id: number) => void
  tickShortTimers: () => void // Called every second
  
  // Long timer actions
  addLongTimer: (name: string, targetDate: string) => void
  deleteLongTimer: (id: number) => void
  completeLongTimer: (id: number) => void
  updateLongTimers: () => void // Called daily to recalculate days
  
  // Active timer management
  setActiveShortTimer: (id: number | null) => void
  setActiveLongTimer: (id: number | null) => void
}

export const useTimerStore = create<TimerState & TimerActions>()((set) => ({
  shortTimers: [],
  longTimers: [],
  activeShortTimerId: null,
  activeLongTimerId: null,

  // Short timer actions
  addShortTimer: (name, duration) => {
    const id = Date.now()
    const newTimer: ShortTimer = {
      id,
      name,
      duration,
      remaining: duration,
      status: 'paused'
    }
    set((state) => ({ 
      shortTimers: [...state.shortTimers, newTimer],
      activeShortTimerId: id 
    }))
  },

  startShortTimer: (id) =>
    set((state) => ({
      shortTimers: state.shortTimers.map((t) =>
        t.id === id ? { ...t, status: 'running' as const, startedAt: new Date().toISOString() } : t
      ),
      activeShortTimerId: id
    })),

  pauseShortTimer: (id) =>
    set((state) => ({
      shortTimers: state.shortTimers.map((t) =>
        t.id === id && t.status === 'running' ? { ...t, status: 'paused' as const } : t
      )
    })),

  resetShortTimer: (id) =>
    set((state) => ({
      shortTimers: state.shortTimers.map((t) =>
        t.id === id ? { ...t, remaining: t.duration, status: 'paused' as const, startedAt: undefined } : t
      )
    })),

  deleteShortTimer: (id) =>
    set((state) => ({
      shortTimers: state.shortTimers.filter((t) => t.id !== id),
      activeShortTimerId: state.activeShortTimerId === id ? null : state.activeShortTimerId
    })),

  tickShortTimers: () =>
    set((state) => {
      let needsUpdate = false
      const updatedTimers = state.shortTimers.map((t) => {
        if (t.status === 'running' && t.remaining > 0) {
          needsUpdate = true
          const newRemaining = t.remaining - 1
          if (newRemaining <= 0) {
            return { ...t, remaining: 0, status: 'completed' as const, finishedAt: new Date().toISOString() }
          }
          return { ...t, remaining: newRemaining }
        }
        return t
      })
      return needsUpdate ? { shortTimers: updatedTimers } : {}
    }),

  // Long timer actions
  addLongTimer: (name, targetDate) => {
    const id = Date.now()
    const target = new Date(targetDate)
    const now = new Date()
    const remainingDays = Math.ceil((target.getTime() - now.getTime()) / (1000 * 60 * 60 * 24))
    
    const newTimer: LongTimer = {
      id,
      name,
      targetDate,
      remainingDays: Math.max(0, remainingDays),
      completed: false
    }
    set((state) => ({ 
      longTimers: [...state.longTimers, newTimer],
      activeLongTimerId: id 
    }))
  },

  deleteLongTimer: (id) =>
    set((state) => ({
      longTimers: state.longTimers.filter((t) => t.id !== id),
      activeLongTimerId: state.activeLongTimerId === id ? null : state.activeLongTimerId
    })),

  completeLongTimer: (id) =>
    set((state) => ({
      longTimers: state.longTimers.map((t) =>
        t.id === id ? { ...t, completed: true, remainingDays: 0 } : t
      )
    })),

  updateLongTimers: () =>
    set((state) => ({
      longTimers: state.longTimers.map((t) => {
        if (t.completed) return t
        const target = new Date(t.targetDate)
        const now = new Date()
        const remainingDays = Math.ceil((target.getTime() - now.getTime()) / (1000 * 60 * 60 * 24))
        if (remainingDays <= 0) {
          return { ...t, completed: true, remainingDays: 0 }
        }
        return { ...t, remainingDays }
      })
    })),

  // Active timer management
  setActiveShortTimer: (id) => set({ activeShortTimerId: id }),
  setActiveLongTimer: (id) => set({ activeLongTimerId: id })
}))
