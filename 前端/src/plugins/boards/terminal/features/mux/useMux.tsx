// terminal.mux L2 功能域（终端本体「一切皆插件」拆分）。
// 承载标签页（Tab 栏 / 新建 Tab 菜单 / WSL 检测）与分屏（pane / 拖拽 resize / 布局持久化）。
// 局部 state + handlers + JSX 物理落在本目录；跨域共享的会话/标签数据经 L1 Terminal 壳经 props 注入。
import { t } from "i18next";
import { useState, useRef, useEffect, useCallback, Fragment } from 'react';
import { terminal } from '../../ipc';
import type { TerminalLine, TabData, PaneData, TerminalMode } from '../../stores/terminalStore';
import NexTermTerminal from '../../NexTermTerminal';
import TerminalPane from '../../TerminalPane';
import styles from '../../Terminal.module.css';

interface WslStatus {
  installed: boolean;
  running: boolean;
  default_distro: string | null;
  distributions: {
    name: string;
    running: boolean;
    version: number;
    default: boolean;
  }[];
  wsl_version: string | null;
}

interface MuxDeps {
  tabs: TabData[];
  activeTabId: string;
  currentCommand: string;
  createTab: (type?: 'builtin' | 'cmd' | 'powershell' | 'wsl') => string;
  closeTab: (tabId: string) => void;
  switchTab: (tabId: string) => void;
  updateTabSessionId: (tabId: string, sessionId: string) => void;
  updatePaneSessionId: (tabId: string, paneId: string, sessionId: string) => void;
  setPaneCommandHistory: (tabId: string, paneId: string, history: TerminalLine[]) => void;
  prependPaneUserCommand: (tabId: string, paneId: string, command: string) => void;
  setPaneTerminalMode: (tabId: string, paneId: string, mode: TerminalMode) => void;
  setCurrentCommand: (cmd: string) => void;
  splitPane: (tabId: string, direction: 'horizontal' | 'vertical') => void;
  closePane: (tabId: string, paneId: string) => void;
  focusPane: (tabId: string, paneId: string) => void;
  saveLayoutToBackend: () => Promise<void>;
  loadLayoutFromBackend: () => Promise<boolean>;
  _layoutLoaded: boolean;
}

export function useMux(deps: MuxDeps) {
  const {
    tabs,
    activeTabId,
    currentCommand,
    createTab,
    closeTab,
    switchTab,
    updateTabSessionId,
    updatePaneSessionId,
    setPaneCommandHistory,
    prependPaneUserCommand,
    setPaneTerminalMode,
    setCurrentCommand,
    splitPane,
    closePane,
    focusPane,
    saveLayoutToBackend,
    loadLayoutFromBackend,
    _layoutLoaded,
  } = deps;

  const [showNewTabMenu, setShowNewTabMenu] = useState(false);
  const [wslStatus, setWslStatus] = useState<WslStatus | null>(null);
  const newTabMenuRef = useRef<HTMLDivElement>(null);
  const [paneSizes, setPaneSizes] = useState<number[]>([1, 1]);
  const isResizing = useRef(false);

  useEffect(() => {
    loadLayoutFromBackend();
    const checkWsl = async () => {
      try {
        if (window.__TAURI__) {
          const result = await terminal.detectWsl();
          if (result?.data) {
            setWslStatus(result.data);
          }
        }
      } catch {
        setWslStatus(null);
      }
    };
    checkWsl();
    const handleClickOutside = (e: MouseEvent) => {
      if (newTabMenuRef.current && !newTabMenuRef.current.contains(e.target as Node)) {
        setShowNewTabMenu(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  useEffect(() => {
    if (_layoutLoaded) {
      saveLayoutToBackend();
    }
  }, [tabs, activeTabId, _layoutLoaded, saveLayoutToBackend]);
  useEffect(() => {
    const handleBeforeUnload = () => {
      saveLayoutToBackend();
    };
    window.addEventListener('beforeunload', handleBeforeUnload);
    return () => window.removeEventListener('beforeunload', handleBeforeUnload);
  }, [saveLayoutToBackend]);

  /// 创建 WSL Tab（支持指定发行版）
  const createWslTab = useCallback(async (distro?: string) => {
    const tabId = createTab('wsl');
    try {
      const result = await terminal.createWslSession(distro);
      if (result?.data) {
        updateTabSessionId(tabId, result.data);
      }
    } catch {
      // 后端不可用时使用默认 session
    }
  }, [createTab, updateTabSessionId]);

  const handleCloseTab = useCallback(async (tabId: string) => {
    const tab = tabs.find((t: TabData) => t.id === tabId);
    if (tab?.sessionId) {
      await terminal.closeSession(tab.sessionId).catch(() => {});
    }
    if (tab?.panes) {
      for (const pane of tab.panes) {
        if (pane.sessionId) {
          await terminal.closeSession(pane.sessionId).catch(() => {});
        }
      }
    }
    closeTab(tabId);
  }, [tabs, closeTab]);

  const handleSplitPane = useCallback((direction: 'horizontal' | 'vertical') => {
    splitPane(activeTabId, direction);
  }, [activeTabId, splitPane]);

  const handleClosePane = useCallback((paneId: string) => {
    const tab = tabs.find((t: TabData) => t.id === activeTabId);
    const pane = tab?.panes.find((p: PaneData) => p.id === paneId);
    if (pane?.sessionId) {
      terminal.closeSession(pane.sessionId).catch(() => {});
    }
    closePane(activeTabId, paneId);
  }, [activeTabId, tabs, closePane]);

  const handleResizeStart = useCallback((e: React.MouseEvent, paneIdx: number) => {
    e.preventDefault();
    isResizing.current = true;
    const startX = e.clientX;
    const startY = e.clientY;
    const startSizes = [...paneSizes];
    const handleMouseMove = (moveEvent: MouseEvent) => {
      if (!isResizing.current) return;
      const tab = tabs.find((t: TabData) => t.id === activeTabId);
      const isHorizontal = tab?.splitDirection === 'horizontal';
      const delta = isHorizontal ? moveEvent.clientX - startX : moveEvent.clientY - startY;
      const totalSize = startSizes[paneIdx - 1] + startSizes[paneIdx];
      const minSize = 0.2;
      const newSizes = [...startSizes];
      const normalizedDelta = delta / 100;
      newSizes[paneIdx - 1] = Math.max(minSize, Math.min(totalSize - minSize, startSizes[paneIdx - 1] + normalizedDelta));
      newSizes[paneIdx] = totalSize - newSizes[paneIdx - 1];
      setPaneSizes(newSizes);
    };
    const handleMouseUp = () => {
      isResizing.current = false;
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [paneSizes, activeTabId, tabs]);

  const renderTabBar = () => <div className={styles.tabContainer}>
      {tabs.map((tab: TabData) => {
      const typeIcon = tab.type === 'cmd' ? '🖥️' : tab.type === 'powershell' ? '💙' : tab.type === 'wsl' ? '🐧' : '⚡';
      return <div key={tab.id} className={`${styles.tab} ${tab.id === activeTabId ? styles.tabActive : ''}`} onClick={() => switchTab(tab.id)} title={tab.title}>
            <span className={styles.tabIcon}>{typeIcon}</span>
            <span className={styles.tabTitle}>{tab.title}</span>
            {tabs.length > 1 && <button className={styles.tabClose} onClick={e => {
          e.stopPropagation();
          handleCloseTab(tab.id);
        }} title={t("Terminal.k29")}>
                ✕
              </button>}
          </div>;
    })}
      <div className={styles.tabNewWrapper} ref={newTabMenuRef}>
        <button className={styles.tabNew} onClick={() => setShowNewTabMenu(!showNewTabMenu)} title={t("Terminal.k30")}>
          +
        </button>
        {showNewTabMenu && <div className={styles.tabNewMenu}>
            <button className={styles.tabNewMenuItem} onClick={() => {
          createTab('builtin');
          setShowNewTabMenu(false);
        }}>
              {t("Terminal.k31")}
            </button>
            <button className={styles.tabNewMenuItem} onClick={() => {
          createTab('cmd');
          setShowNewTabMenu(false);
        }}>
              🖥️ Windows CMD
            </button>
            <button className={styles.tabNewMenuItem} onClick={() => {
          createTab('powershell');
          setShowNewTabMenu(false);
        }}>
              PowerShell
            </button>

            {/* WSL 子菜单：发行版选择器 */}
            <div className={styles.wslSubMenu}>
              <span className={styles.wslSubMenuLabel}>
                {wslStatus?.installed ? <>WSL {wslStatus.wsl_version ? `v${wslStatus.wsl_version}` : ''}</> : 'WSL'}
                <span className={styles.wslArrow}>▸</span>
              </span>
              <div className={styles.wslSubMenuPanel}>
                {wslStatus?.installed ? <>
                    {wslStatus.distributions.length > 0 ? wslStatus.distributions.map((d: {
                name: string;
                running: boolean;
                version: number;
                default: boolean;
              }) => <button key={d.name} className={`${styles.tabNewMenuItem} ${styles.distroItem}`} onClick={() => {
                createWslTab(d.name);
                setShowNewTabMenu(false);
              }}>
                          <span className={styles.distroIcon}>{d.running ? '●' : '○'}</span>
                          <span className={styles.distroName}>{d.name}</span>
                          <span className={styles.distroMeta}>
                            WSL{d.version} {d.default ? t("Terminal.k32") : ''}
                          </span>
                        </button>) : <div className={styles.wslEmptyHint}>{t("Terminal.k33")}</div>}
                    <div className={styles.wslDivider} />
                    <button className={`${styles.tabNewMenuItem} ${styles.distroItem}`} onClick={() => {
                createWslTab();
                setShowNewTabMenu(false);
              }}>
                      <span className={styles.distroIcon}>⊕</span>
                      <span className={styles.distroName}>{t("Terminal.k34")}</span>
                    </button>
                  </> : <div className={styles.wslNotInstalled}>
                    <div className={styles.wslNotInstalledTitle}>{t("Terminal.k35")}</div>
                    <div className={styles.wslNotInstalledHint}>
                      {t("Terminal.k36")}
                    </div>
                    <code className={styles.wslInstallCmd}>wsl --install</code>
                    <a className={styles.wslInstallLink} href="https://docs.microsoft.com/windows/wsl/install" target="_blank" rel="noreferrer">
                      {t("Terminal.k37")}
                    </a>
                  </div>}
              </div>
            </div>
          </div>}
      </div>
    </div>;

  const renderSplitView = (activeTab: TabData) => {
    const isHorizontal = activeTab.splitDirection === 'horizontal';
    return <div className={`${styles.splitContainer} ${isHorizontal ? styles.splitHorizontal : styles.splitVertical}`}>
        {activeTab.panes.map((pane: PaneData, paneIdx: number) => {
        const isActive = pane.id === activeTab.activePaneId;
        const isPty = pane.type === 'cmd' || pane.type === 'powershell' || pane.type === 'wsl';
        return <Fragment key={pane.id}>
              {paneIdx > 0 && <div className={`${styles.resizeHandle} ${!isHorizontal ? styles.resizeHandleVertical : ''}`} onMouseDown={e => handleResizeStart(e, paneIdx)} />}
              <div className={`${styles.paneWrapper} ${isActive ? styles.paneActive : ''}`} style={{
            flex: `${paneSizes[paneIdx] || 1} 1 0`
          }} onClick={() => focusPane(activeTabId, pane.id)}>
              <div className={styles.paneHeader}>
                <span className={styles.paneTitle}>
                  {pane.type === 'cmd' ? '🖥️ CMD' : pane.type === 'powershell' ? '💙 PowerShell' : pane.type === 'wsl' ? '🐧 WSL' : t("Terminal.k38")}
                </span>
                <div className={styles.paneActions}>
                  <button className={styles.paneCloseBtn} onClick={e => {
                  e.stopPropagation();
                  handleClosePane(pane.id);
                }} title={t("Terminal.k39")}>
                    ✕
                  </button>
                </div>
              </div>
              <div className={styles.paneContent}>
                {isPty ? <NexTermTerminal sessionType={pane.type as 'cmd' | 'powershell' | 'wsl'} autoConnect={!pane.sessionId} externalSessionId={pane.sessionId} onSessionCreated={sid => {
                updatePaneSessionId(activeTabId, pane.id, sid);
              }} fontSize={14} /> : <TerminalPane paneId={pane.id} commandHistory={pane.commandHistory} userCommands={pane.userCommands} currentCommand={currentCommand} terminalMode={pane.terminalMode} onHistoryChange={history => setPaneCommandHistory(activeTabId, pane.id, history)} onUserCommandAdd={cmd => prependPaneUserCommand(activeTabId, pane.id, cmd)} onCurrentCommandChange={cmd => setCurrentCommand(cmd)} onModeChange={mode => setPaneTerminalMode(activeTabId, pane.id, mode)} onClearHistory={() => {
                setPaneCommandHistory(activeTabId, pane.id, [{
                  type: 'prompt',
                  content: t("Terminal.k40")
                }, {
                  type: 'output',
                  content: ''
                }, {
                  type: 'output',
                  content: t("Terminal.k24")
                }, {
                  type: 'output',
                  content: ''
                }]);
              }} isActive={isActive} onFocus={() => focusPane(activeTabId, pane.id)} />}
              </div>
            </div>
            </Fragment>;
      })}
      </div>;
  };

  // 键盘总控中 mux 域：新建/关闭 Tab、分屏、pane 切换、Tab 切换
  const handleKey = useCallback((e: React.KeyboardEvent): boolean => {
    if ((e.key === 't' || e.key === 'T') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      setShowNewTabMenu(true);
      return true;
    }
    if ((e.key === 'w' || e.key === 'W') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      const activeTab = tabs.find((tb: TabData) => tb.id === activeTabId);
      if (activeTab && activeTab.splitDirection !== 'none') {
        handleClosePane(activeTab.activePaneId);
      } else {
        handleCloseTab(activeTabId);
      }
      return true;
    }
    if (e.key === '\\' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      handleSplitPane('vertical');
      return true;
    }
    if (e.key === '-' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      handleSplitPane('horizontal');
      return true;
    }
    if (e.key === '[' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      const tab = tabs.find((tb: TabData) => tb.id === activeTabId);
      if (tab && tab.splitDirection !== 'none' && tab.panes.length > 1) {
        const idx = tab.panes.findIndex((p: PaneData) => p.id === tab.activePaneId);
        const prevIdx = (idx - 1 + tab.panes.length) % tab.panes.length;
        focusPane(activeTabId, tab.panes[prevIdx].id);
      }
      return true;
    }
    if (e.key === ']' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      const tab = tabs.find((tb: TabData) => tb.id === activeTabId);
      if (tab && tab.splitDirection !== 'none' && tab.panes.length > 1) {
        const idx = tab.panes.findIndex((p: PaneData) => p.id === tab.activePaneId);
        const nextIdx = (idx + 1) % tab.panes.length;
        focusPane(activeTabId, tab.panes[nextIdx].id);
      }
      return true;
    }
    if (e.key === 'Tab' && e.ctrlKey) {
      e.preventDefault();
      const idx = tabs.findIndex((tb: TabData) => tb.id === activeTabId);
      if (e.shiftKey) {
        const prevIdx = (idx - 1 + tabs.length) % tabs.length;
        switchTab(tabs[prevIdx].id);
      } else {
        const nextIdx = (idx + 1) % tabs.length;
        switchTab(tabs[nextIdx].id);
      }
      return true;
    }
    return false;
  }, [tabs, activeTabId, switchTab, focusPane, handleSplitPane, handleClosePane, handleCloseTab]);

  return {
    renderTabBar,
    renderSplitView,
    handleKey,
    handleSplitPane,
    handleClosePane,
    handleCloseTab,
    createWslTab,
  };
}
