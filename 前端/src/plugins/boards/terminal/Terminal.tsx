// boards.terminal L1 前端半体（终端本体「一切皆插件」拆分后的壳 + 核心数据层）。
// 保留：路由分发（/terminal/linux 内嵌 LinuxTerminal）、terminalStore 绑定与派生（跨域共享的
// 会话/块/标签数据）、整体布局骨架、端子插件启停门控。
// 功能域物理迁出至 L2：terminal.mux（标签页与分屏）/ terminal.tools（命令辅助工具）/
// terminal.console（终端命令行）；三者为必备/必备/可选，L1 经 hook 组合其渲染函数。
import { useState, useRef, useEffect, useCallback } from 'react';
import { useLocation } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { useTerminalStore, type TabData } from './stores/terminalStore';
import LinuxTerminal from '@/components/LinuxTerminal';
import { useMux } from './features/mux/useMux';
import { useTools } from './features/tools/useTools';
import { useConsole } from './features/console/useConsole';
import styles from './Terminal.module.css';

export default function Terminal() {
  const location = useLocation();
  const pathname = location.pathname;
  const isLinuxPage = pathname === '/terminal/linux';
  if (isLinuxPage) {
    return <LinuxTerminal />;
  }
  const tabs = useTerminalStore(s => s.tabs);
  const activeTabId = useTerminalStore(s => s.activeTabId);
  const commandHistory = useTerminalStore(s => {
    const tab = s.tabs.find((t: TabData) => t.id === s.activeTabId);
    return tab?.commandHistory ?? [];
  });
  const userCommands = useTerminalStore(s => {
    const tab = s.tabs.find((t: TabData) => t.id === s.activeTabId);
    return tab?.userCommands ?? [];
  });
  const terminalMode = useTerminalStore(s => {
    const tab = s.tabs.find((t: TabData) => t.id === s.activeTabId);
    return tab?.terminalMode ?? 'builtin';
  });
  const blocks = useTerminalStore(s => {
    const tab = s.tabs.find((t: TabData) => t.id === s.activeTabId);
    return tab?.blocks ?? [];
  });
  const {
    currentCommand,
    setCommandHistory,
    setCurrentCommand,
    setTerminalMode,
    prependUserCommand,
    clearHistory,
    createTab,
    closeTab,
    switchTab,
    updateTabSessionId,
    saveLayoutToBackend,
    loadLayoutFromBackend,
    _layoutLoaded,
    splitPane,
    closePane,
    focusPane,
    updatePaneSessionId,
    setPaneCommandHistory,
    prependPaneUserCommand,
    setPaneTerminalMode,
    addBlock,
    toggleBlockCollapse,
    setBlocks
  } = useTerminalStore();

  // terminal.tools（可选子插件）启停门控——null 为加载中（放行，与 Layout enabledIds 口径一致）
  const [toolsEnabled, setToolsEnabled] = useState<boolean | null>(null);
  useEffect(() => {
    invoke<Array<{ id: string }>>('plugin:kernel|kernel_dispatch', {
      cmd: 'kernel:plugin:get_enabled',
      args: {},
    }).then((list) => {
      setToolsEnabled(list.some((p) => p.id === 'terminal.tools'));
    }).catch(() => setToolsEnabled(true));
  }, []);
  const toolsOn = toolsEnabled !== false;

  const terminalRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement | HTMLTextAreaElement>(null);
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [commandHistory]);
  useEffect(() => {
    if (inputRef.current && !isLinuxPage) {
      inputRef.current.focus();
    }
  }, [pathname]);

  // 功能域 hook 依赖序：mux（无跨域依赖）→ tools（依赖 mux 分屏）→ console（依赖 tools 高亮/行样式）
  const mux = useMux({
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
  });

  const tools = useTools({
    tabs,
    activeTabId,
    commandHistory,
    setCurrentCommand,
    inputRef,
    terminalRef,
    handleSplitPane: mux.handleSplitPane,
    enabled: toolsOn,
  });

  const consoleApi = useConsole({
    activeTabId,
    commandHistory,
    userCommands,
    terminalMode,
    blocks,
    currentCommand,
    setCommandHistory,
    setCurrentCommand,
    setTerminalMode,
    clearHistory,
    prependUserCommand,
    addBlock,
    setBlocks,
    toggleBlockCollapse,
    updateTabSessionId,
    isLinuxPage,
    terminalRef,
    inputRef,
    getCopyModeLineClass: tools.getCopyModeLineClass,
    renderLineWithHighlight: tools.renderLineWithHighlight,
    searchActive: tools.searchActive,
  });

  // 键盘总控：按域委派（tools → console → mux，与原单体 if/else 链的前后顺序等价）
  const handleKeyPress = useCallback((e: React.KeyboardEvent) => {
    if (tools.handleKey(e)) return;
    if (consoleApi.handleKey(e)) return;
    mux.handleKey(e);
  }, [tools.handleKey, consoleApi.handleKey, mux.handleKey]);

  const renderMainContent = () => {
    const activeTab = tabs.find((t: TabData) => t.id === activeTabId);
    if (!activeTab) return null;
    if (activeTab.splitDirection !== 'none' && activeTab.panes.length > 1) {
      return mux.renderSplitView(activeTab);
    }
    const isPtyTab = activeTab.type === 'cmd' || activeTab.type === 'powershell' || activeTab.type === 'wsl';
    if (isPtyTab) {
      return consoleApi.renderPtyView(activeTab);
    }
    const windowNode = consoleApi.renderBuiltinWindow({
      onKeyDown: handleKeyPress,
      searchBar: tools.renderSearchBar(),
      copyModeActive: tools.copyModeVisible,
    });
    return tools.wrapCopyMode(windowNode);
  };

  return <div className={styles.terminalContainer} onContextMenu={tools.handleContextMenu}>
      {mux.renderTabBar()}
      {renderMainContent()}
      {tools.renderLauncherOverlay()}
      {tools.renderQuickSelectOverlay()}
      {tools.renderContextMenu()}
      {tools.renderCommandSearch()}
      {consoleApi.renderErrorOverlays()}
    </div>;
}
