import { t } from "i18next";
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { ipc } from '@/lib/ipc';
export interface TerminalLine {
  type: 'prompt' | 'command' | 'output' | 'error' | 'success' | 'warning' | 'info' | 'system' | 'mascot';
  content: string;
}
export interface TerminalBlock {
  id: string;
  command: string;
  output: string;
  timestamp: number;
  collapsed: boolean;
  exitCode?: number;
}
export type TerminalMode = 'builtin' | 'cmd' | 'powershell' | 'wsl';
export type SplitDirection = 'none' | 'horizontal' | 'vertical';
export interface PaneData {
  id: string;
  type: 'builtin' | 'cmd' | 'powershell' | 'wsl';
  sessionId: string | null;
  commandHistory: TerminalLine[];
  userCommands: string[];
  terminalMode: TerminalMode;
  blocks: TerminalBlock[];
}
export interface TabData {
  id: string;
  title: string;
  type: 'builtin' | 'cmd' | 'powershell' | 'wsl';
  sessionId: string | null;
  commandHistory: TerminalLine[];
  userCommands: string[];
  terminalMode: TerminalMode;
  wslDistro?: string | null;
  blocks: TerminalBlock[];
  splitDirection: SplitDirection;
  panes: PaneData[];
  activePaneId: string;
}
export interface TabLayoutInput {
  tab_id: string;
  tab_type: string;
  title: string;
  sort_order: number;
  is_active: boolean;
  pane_data: string | null;
}
const MASCOT = ['', '   ╔══════════════════════╗', '   ║                      ║', t("TerminalPane.k1"), t("TerminalPane.k2"), '   ║                      ║', '   ╚══════════════════════╝', ''].join('\n');
const INITIAL_HISTORY: TerminalLine[] = [{
  type: 'mascot',
  content: MASCOT
}, {
  type: 'output',
  content: ''
}, {
  type: 'output',
  content: t("TerminalPane.k3")
}, {
  type: 'system',
  content: ''
}];
function generateTabId(): string {
  return `tab_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}
function createDefaultTab(type: 'builtin' | 'cmd' | 'powershell' | 'wsl' = 'builtin'): TabData {
  const titles: Record<string, string> = {
    builtin: t("stores.terminalStore.k1"),
    cmd: 'CMD',
    powershell: 'PowerShell',
    wsl: 'WSL'
  };
  return {
    id: generateTabId(),
    title: titles[type],
    type,
    sessionId: null,
    commandHistory: type === 'builtin' ? [...INITIAL_HISTORY.map(h => ({
      ...h
    }))] : [{
      type: 'mascot',
      content: '   ╔═══════╗'
    }, {
      type: 'mascot',
      content: '   ║NexTerm║'
    }, {
      type: 'mascot',
      content: '   ╚═══════╝'
    }, {
      type: 'output',
      content: ''
    }, {
      type: 'output',
      content: t("stores.terminalStore.k2", {
        type: titles[type]
      })
    }, {
      type: 'output',
      content: ''
    }],
    userCommands: [],
    terminalMode: type,
    blocks: [],
    splitDirection: 'none' as SplitDirection,
    panes: [],
    activePaneId: ''
  };
}
interface TerminalState {
  tabs: TabData[];
  activeTabId: string;
  currentCommand: string;
  getActiveTab: () => TabData | undefined;
  createTab: (type?: 'builtin' | 'cmd' | 'powershell' | 'wsl') => string;
  closeTab: (tabId: string) => void;
  switchTab: (tabId: string) => void;
  updateActiveTabTitle: (title: string) => void;
  updateTabSessionId: (tabId: string, sessionId: string) => void;
  setCommandHistory: (history: TerminalLine[]) => void;
  appendCommandHistory: (lines: TerminalLine[]) => void;
  setUserCommands: (commands: string[]) => void;
  prependUserCommand: (command: string) => void;
  setCurrentCommand: (cmd: string) => void;
  setTerminalMode: (mode: TerminalMode) => void;
  resetTerminal: () => void;
  clearHistory: () => void;
  _layoutLoaded: boolean;
  setLayoutLoaded: () => void;
  saveLayoutToBackend: () => Promise<void>;
  loadLayoutFromBackend: () => Promise<boolean>;
  splitPane: (tabId: string, direction: 'horizontal' | 'vertical') => void;
  closePane: (tabId: string, paneId: string) => void;
  focusPane: (tabId: string, paneId: string) => void;
  getActivePane: () => PaneData | undefined;
  updatePaneSessionId: (tabId: string, paneId: string, sessionId: string) => void;
  setPaneCommandHistory: (tabId: string, paneId: string, history: TerminalLine[]) => void;
  appendPaneCommandHistory: (tabId: string, paneId: string, lines: TerminalLine[]) => void;
  setPaneUserCommands: (tabId: string, paneId: string, commands: string[]) => void;
  prependPaneUserCommand: (tabId: string, paneId: string, command: string) => void;
  setPaneTerminalMode: (tabId: string, paneId: string, mode: TerminalMode) => void;
  addBlock: (block: TerminalBlock) => void;
  appendToBlock: (blockId: string, output: string) => void;
  toggleBlockCollapse: (blockId: string) => void;
  setBlocks: (blocks: TerminalBlock[]) => void;
}
export const useTerminalStore = create<TerminalState>()(persist((set, get) => {
  const defaultTab = createDefaultTab('builtin');
  return {
    tabs: [defaultTab],
    activeTabId: defaultTab.id,
    currentCommand: '',
    getActiveTab: () => {
      const {
        tabs,
        activeTabId
      } = get();
      return tabs.find(t => t.id === activeTabId);
    },
    createTab: (type = 'builtin') => {
      const tab = createDefaultTab(type);
      set(state => ({
        tabs: [...state.tabs, tab],
        activeTabId: tab.id,
        currentCommand: ''
      }));
      return tab.id;
    },
    closeTab: (tabId: string) => {
      set(state => {
        if (state.tabs.length <= 1) return state;
        const newTabs = state.tabs.filter(t => t.id !== tabId);
        let newActiveId = state.activeTabId;
        if (state.activeTabId === tabId) {
          const idx = state.tabs.findIndex(t => t.id === tabId);
          newActiveId = newTabs[Math.min(idx, newTabs.length - 1)].id;
        }
        return {
          tabs: newTabs,
          activeTabId: newActiveId,
          currentCommand: ''
        };
      });
    },
    switchTab: (tabId: string) => {
      const {
        tabs
      } = get();
      if (tabs.some(t => t.id === tabId)) {
        set({
          activeTabId: tabId,
          currentCommand: ''
        });
      }
    },
    updateActiveTabTitle: (title: string) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === state.activeTabId ? {
          ...t,
          title
        } : t)
      }));
    },
    updateTabSessionId: (tabId: string, sessionId: string) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === tabId ? {
          ...t,
          sessionId
        } : t)
      }));
    },
    setCommandHistory: history => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === state.activeTabId ? {
          ...t,
          commandHistory: history
        } : t)
      }));
    },
    appendCommandHistory: lines => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === state.activeTabId ? {
          ...t,
          commandHistory: [...t.commandHistory, ...lines]
        } : t)
      }));
    },
    setUserCommands: commands => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === state.activeTabId ? {
          ...t,
          userCommands: commands
        } : t)
      }));
    },
    prependUserCommand: command => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === state.activeTabId ? {
          ...t,
          userCommands: [command, ...t.userCommands].slice(0, 50)
        } : t)
      }));
    },
    setCurrentCommand: cmd => set({
      currentCommand: cmd
    }),
    setTerminalMode: mode => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === state.activeTabId ? {
          ...t,
          terminalMode: mode
        } : t)
      }));
    },
    resetTerminal: () => {
      set(state => ({
        tabs: state.tabs.map(tab => tab.id === state.activeTabId ? {
          ...tab,
          commandHistory: [{
            type: 'prompt',
            content: t("Terminal.k40")
          }, {
            type: 'output',
            content: ''
          }, {
            type: 'output',
            content: t("stores.terminalStore.k3")
          }, {
            type: 'output',
            content: ''
          }],
          terminalMode: 'builtin'
        } : tab),
        currentCommand: ''
      }));
    },
    clearHistory: () => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === state.activeTabId ? {
          ...t,
          commandHistory: [...INITIAL_HISTORY.map(h => ({
            ...h
          }))]
        } : t)
      }));
    },
    _layoutLoaded: false,
    setLayoutLoaded: () => set({
      _layoutLoaded: true
    }),
    saveLayoutToBackend: async () => {
      const state = get();
      if (!state._layoutLoaded) return;
      const inputs: TabLayoutInput[] = state.tabs.map((tab, index) => ({
        tab_id: tab.id,
        tab_type: tab.type,
        title: tab.title,
        sort_order: index,
        is_active: tab.id === state.activeTabId,
        pane_data: tab.splitDirection !== 'none' ? JSON.stringify({
          splitDirection: tab.splitDirection,
          panes: tab.panes,
          activePaneId: tab.activePaneId
        }) : null
      }));
      try {
        await ipc.invoke('terminal_save_layout', {
          tabs: inputs
        });
      } catch {
        // 静默失败，后端可能不可用
      }
    },
    loadLayoutFromBackend: async () => {
      try {
        const result = await ipc.invoke<TabLayoutInput[]>('terminal_load_layout', {});
        if (result.code === 0 && result.data && result.data.length > 0) {
          const tabs: TabData[] = result.data.map((item: TabLayoutInput) => {
            let paneRestore: {
              splitDirection: SplitDirection;
              panes: PaneData[];
              activePaneId: string;
            } | null = null;
            if (item.pane_data) {
              try {
                paneRestore = JSON.parse(item.pane_data);
              } catch {/* ignore parse errors */}
            }
            const baseTab: TabData = {
              id: item.tab_id,
              title: item.title,
              type: item.tab_type as 'builtin' | 'cmd' | 'powershell' | 'wsl',
              sessionId: null,
              commandHistory: item.tab_type === 'builtin' ? INITIAL_HISTORY.map(h => ({
                ...h
              })) : [{
                type: 'info',
                content: '╔══════════════════════════════════════╗'
              }, {
                type: 'info',
                content: t("stores.terminalStore.k4", {
                  title: item.title
                })
              }, {
                type: 'info',
                content: '╚══════════════════════════════════════╝'
              }, {
                type: 'output',
                content: ''
              }, {
                type: 'output',
                content: t("stores.terminalStore.k5", {
                  arg0: item.tab_type === 'cmd' ? 'Windows CMD' : item.tab_type === 'powershell' ? 'PowerShell' : 'WSL (Linux)'
                })
              }, {
                type: 'output',
                content: ''
              }],
              userCommands: [],
              terminalMode: item.tab_type as TerminalMode,
              blocks: [],
              splitDirection: 'none' as SplitDirection,
              panes: [],
              activePaneId: ''
            };
            if (paneRestore) {
              baseTab.splitDirection = paneRestore.splitDirection;
              baseTab.panes = paneRestore.panes;
              baseTab.activePaneId = paneRestore.activePaneId;
            }
            return baseTab;
          });
          const activeTab = result.data.find((item: TabLayoutInput) => item.is_active);
          const activeTabId = activeTab ? activeTab.tab_id : tabs[0].id;
          set({
            tabs,
            activeTabId,
            _layoutLoaded: true
          });
          return true;
        }
      } catch {
        // 静默失败，后端可能不可用
      }
      set({
        _layoutLoaded: true
      });
      return false;
    },
    splitPane: (tabId: string, direction: 'horizontal' | 'vertical') => {
      set(state => ({
        tabs: state.tabs.map(t => {
          if (t.id !== tabId || t.splitDirection !== 'none') return t;
          const existingPane: PaneData = {
            id: `pane_${Date.now()}_0`,
            type: t.type,
            sessionId: t.sessionId,
            commandHistory: [...t.commandHistory],
            userCommands: [...t.userCommands],
            terminalMode: t.terminalMode,
            blocks: [...t.blocks]
          };
          const newPane: PaneData = {
            id: `pane_${Date.now()}_1`,
            type: 'builtin',
            sessionId: null,
            commandHistory: [...INITIAL_HISTORY.map(h => ({
              ...h
            }))],
            userCommands: [],
            terminalMode: 'builtin',
            blocks: []
          };
          return {
            ...t,
            splitDirection: direction,
            panes: [existingPane, newPane],
            activePaneId: existingPane.id
          };
        })
      }));
    },
    closePane: (tabId: string, paneId: string) => {
      set(state => ({
        tabs: state.tabs.map(t => {
          if (t.id !== tabId) return t;
          const remaining = t.panes.filter(p => p.id !== paneId);
          if (remaining.length <= 1) {
            return {
              ...t,
              splitDirection: 'none' as SplitDirection,
              panes: [],
              activePaneId: ''
            };
          }
          return {
            ...t,
            panes: remaining,
            activePaneId: remaining[0].id
          };
        })
      }));
    },
    focusPane: (tabId: string, paneId: string) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === tabId ? {
          ...t,
          activePaneId: paneId
        } : t)
      }));
    },
    getActivePane: () => {
      const {
        tabs,
        activeTabId
      } = get();
      const tab = tabs.find(t => t.id === activeTabId);
      if (!tab) return undefined;
      if (tab.splitDirection === 'none') return undefined;
      return tab.panes.find(p => p.id === tab.activePaneId);
    },
    updatePaneSessionId: (tabId: string, paneId: string, sessionId: string) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === tabId ? {
          ...t,
          panes: t.panes.map(p => p.id === paneId ? {
            ...p,
            sessionId
          } : p)
        } : t)
      }));
    },
    setPaneCommandHistory: (tabId: string, paneId: string, history: TerminalLine[]) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === tabId ? {
          ...t,
          panes: t.panes.map(p => p.id === paneId ? {
            ...p,
            commandHistory: history
          } : p)
        } : t)
      }));
    },
    appendPaneCommandHistory: (tabId: string, paneId: string, lines: TerminalLine[]) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === tabId ? {
          ...t,
          panes: t.panes.map(p => p.id === paneId ? {
            ...p,
            commandHistory: [...p.commandHistory, ...lines]
          } : p)
        } : t)
      }));
    },
    setPaneUserCommands: (tabId: string, paneId: string, commands: string[]) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === tabId ? {
          ...t,
          panes: t.panes.map(p => p.id === paneId ? {
            ...p,
            userCommands: commands
          } : p)
        } : t)
      }));
    },
    prependPaneUserCommand: (tabId: string, paneId: string, command: string) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === tabId ? {
          ...t,
          panes: t.panes.map(p => p.id === paneId ? {
            ...p,
            userCommands: [command, ...p.userCommands].slice(0, 50)
          } : p)
        } : t)
      }));
    },
    setPaneTerminalMode: (tabId: string, paneId: string, mode: TerminalMode) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === tabId ? {
          ...t,
          panes: t.panes.map(p => p.id === paneId ? {
            ...p,
            terminalMode: mode
          } : p)
        } : t)
      }));
    },
    addBlock: (block: TerminalBlock) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === state.activeTabId ? {
          ...t,
          blocks: [...t.blocks, block]
        } : t)
      }));
    },
    appendToBlock: (blockId: string, output: string) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === state.activeTabId ? {
          ...t,
          blocks: t.blocks.map(b => b.id === blockId ? {
            ...b,
            output: b.output + output
          } : b)
        } : t)
      }));
    },
    toggleBlockCollapse: (blockId: string) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === state.activeTabId ? {
          ...t,
          blocks: t.blocks.map(b => b.id === blockId ? {
            ...b,
            collapsed: !b.collapsed
          } : b)
        } : t)
      }));
    },
    setBlocks: (blocks: TerminalBlock[]) => {
      set(state => ({
        tabs: state.tabs.map(t => t.id === state.activeTabId ? {
          ...t,
          blocks
        } : t)
      }));
    }
  };
}, {
  name: 'nexterm-terminal',
  partialize: (state: TerminalState) => ({
    tabs: state.tabs,
    activeTabId: state.activeTabId,
    _layoutLoaded: state._layoutLoaded
  })
}));