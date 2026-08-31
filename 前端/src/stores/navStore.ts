import { create } from 'zustand'

type NavTab = 'home' | 'ai' | 'knowledge' | 'terminal' | 'xin' | 'game' | 'profile' | 'recycle'

interface NavState {
  activeTab: NavTab
  previousTab: NavTab | null
  sidebarCollapsed: boolean
  
  // Sub-navigation states
  homeSubTab: 'news' | 'todo' | 'log' | 'timer'
  profileSubTab: 'account' | 'resume' | 'setting' | 'logout'
  aiSubTab: 'model' | 'chat' | 'group' | 'agent'
  terminalSubTab: 'terminal' | 'cmd' | 'xincode' | 'linux'
  
  // Actions
  setActiveTab: (tab: NavTab) => void
  toggleSidebar: () => void
  setSidebarCollapsed: (collapsed: boolean) => void
  setHomeSubTab: (tab: 'news' | 'todo' | 'log' | 'timer') => void
  setProfileSubTab: (tab: 'account' | 'resume' | 'setting' | 'logout') => void
  setAiSubTab: (tab: 'model' | 'chat' | 'group' | 'agent') => void
  setTerminalSubTab: (tab: 'terminal' | 'cmd' | 'xincode' | 'linux') => void
}

export const useNavStore = create<NavState>((set) => ({
  activeTab: 'home',
  previousTab: null,
  sidebarCollapsed: false,
  
  homeSubTab: 'news',
  profileSubTab: 'account',
  aiSubTab: 'model',
  terminalSubTab: 'terminal',

  setActiveTab: (tab) =>
    set((state) => ({
      previousTab: state.activeTab,
      activeTab: tab
    })),

  toggleSidebar: () =>
    set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

  setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
  
  setHomeSubTab: (tab) => set({ homeSubTab: tab }),
  setProfileSubTab: (tab) => set({ profileSubTab: tab }),
  setAiSubTab: (tab) => set({ aiSubTab: tab }),
  setTerminalSubTab: (tab) => set({ terminalSubTab: tab })
}))
