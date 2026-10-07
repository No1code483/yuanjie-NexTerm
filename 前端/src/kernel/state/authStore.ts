import { create } from 'zustand'
import { persist, createJSONStorage } from 'zustand/middleware'
import type { UserInfo } from '@/types'
import { tauriStorage } from '@/lib/tauriStorage'

interface AuthState {
  user: UserInfo | null
  token: string | null
  isAuthenticated: boolean
  isLoading: boolean

  login: (user: UserInfo, token: string) => void
  logout: () => void
  updateUser: (updates: Partial<UserInfo>) => void
  setLoading: (isLoading: boolean) => void

  hasPermission: (permission: string) => boolean
  hasAnyPermission: (permissions: string[]) => boolean
  hasAllPermissions: (permissions: string[]) => boolean
  isAdmin: () => boolean
  isGuest: () => boolean
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      user: null,
      token: null,
      isAuthenticated: false,
      isLoading: false,

      login: (user: UserInfo, token: string) =>
        set({
          user,
          token,
          isAuthenticated: true,
          isLoading: false
        }),

      logout: () =>
        set({
          user: null,
          token: null,
          isAuthenticated: false,
          isLoading: false
        }),

      updateUser: (updates: Partial<UserInfo>) =>
        set((state) => ({
          user: state.user ? { ...state.user, ...updates } : null
        })),

      setLoading: (isLoading: boolean) => set({ isLoading }),

      hasPermission: (permission: string) => {
        const { user } = get()
        if (!user?.permissions) return false
        return user.permissions.includes(permission)
      },

      hasAnyPermission: (permissions: string[]) => {
        const { user } = get()
        if (!user?.permissions) return false
        return permissions.some((p) => user.permissions!.includes(p))
      },

      hasAllPermissions: (permissions: string[]) => {
        const { user } = get()
        if (!user?.permissions) return false
        return permissions.every((p) => user.permissions!.includes(p))
      },

      isAdmin: () => {
        const { user } = get()
        return user?.role === 'admin'
      },

      isGuest: () => {
        const { user } = get()
        return user?.is_permanent === false
      }
    }),
    {
      name: 'nexterm-auth',
      storage: createJSONStorage(() => tauriStorage),
      partialize: (state: AuthState) => ({
        user: state.user,
        token: state.token,
        // 不持久化 isAuthenticated —— 每次重启都必须重新验证
      })
    }
  )
)
