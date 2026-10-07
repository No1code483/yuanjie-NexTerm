import { create } from 'zustand'

export type ToastType = 'info' | 'success' | 'warning' | 'error'

export interface Toast {
  id: string
  type: ToastType
  title: string
  message?: string
  duration?: number
  action?: { label: string; onClick: () => void }
}

interface NotifState {
  toasts: Toast[]
  toastCounter: number

  addToast: (toast: Omit<Toast, 'id'>) => string
  removeToast: (id: string) => void
  clearAllToasts: () => void
}

export const useNotifStore = create<NotifState>((set, get) => ({
  toasts: [],
  toastCounter: 0,

  addToast: (toast) => {
    const counter = get().toastCounter + 1
    const id = `toast-${counter}`
    const newToast: Toast = { ...toast, id }
    set(state => ({
      toasts: [...state.toasts, newToast],
      toastCounter: counter,
    }))

    const duration = toast.duration ?? 4000
    if (duration > 0) {
      setTimeout(() => {
        set(state => ({
          toasts: state.toasts.filter(t => t.id !== id),
        }))
      }, duration)
    }

    return id
  },

  removeToast: (id) => {
    set(state => ({
      toasts: state.toasts.filter(t => t.id !== id),
    }))
  },

  clearAllToasts: () => set({ toasts: [] }),
}))