import { t } from "i18next";
import { useState, useRef, useEffect, useCallback, useMemo, Fragment } from 'react';
import { useLocation } from 'react-router-dom';
import { ipc, terminal, intelligence } from '@/lib/ipc';
import { copy } from '@/lib/utils';
import { useIntelligence } from '@/hooks/useIntelligence';
import { useTerminalStore, type TerminalLine, type TabData, type PaneData } from '@/stores/terminalStore';
import LinuxTerminal from '@/components/LinuxTerminal';
import NexTermTerminal from '@/components/NexTermTerminal';
import TerminalPane from '@/pages/TerminalPane';
import { LauncherEntry, QuickSelectMatch, QUICK_SELECT_PATTERNS, computeQuickSelectLabels, LAUNCHER_ENTRIES, splitByLinks, renderAnsiText } from './terminal/types';
import styles from './Terminal.module.css';
const ALL_COMMANDS = ['help', 'ls', 'pwd', 'cd', 'mkdir', 'cat', 'echo', 'date', 'whoami', 'clear', 'version', 'env', 'sysinfo', 'history', 'tree', 'cmd', 'powershell', 'exit', 'grep', 'find', 'wc', 'head', 'tail', 'cp', 'mv', 'rm', 'touch', 'clearscrollback', 'reset', 'fontsize', 'fullscreen', 'reload', 'scroll', 'search', 'hide', 'quit', 'alwaysontop', 'll', 'la', 'cls', 'dir', 'type', 'copy', 'move', 'del', 'erase', 'ren', 'md', 'rd', 'minimize', 'top'];
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
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [isMultiLine, setIsMultiLine] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [aiSuggestedCmd, setAiSuggestedCmd] = useState('');
  const [aiSuggestLoading, setAiSuggestLoading] = useState(false);
  const suggestTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hashTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Task 4.2: 命令搜索面板
  const [cmdSearchVisible, setCmdSearchVisible] = useState(false);
  const [cmdSearchQuery, setCmdSearchQuery] = useState('');
  const [cmdSearchIndex, setCmdSearchIndex] = useState(0);
  const cmdSearchInputRef = useRef<HTMLInputElement>(null);

  // Task 4.3: # AI 建议
  const [hashSuggestLoading, setHashSuggestLoading] = useState(false);
  const [hashSuggestedCmd, setHashSuggestedCmd] = useState('');

  // Task 4.5: 错误右键 AI 解释
  const [errorContextMenu, setErrorContextMenu] = useState<{
    x: number;
    y: number;
    line: TerminalLine;
  } | null>(null);
  const [aiErrorPopup, setAiErrorPopup] = useState<{
    x: number;
    y: number;
    explanation: string;
    loading: boolean;
  } | null>(null);
  const errorContextMenuRef = useRef<HTMLDivElement>(null);
  const aiErrorPopupRef = useRef<HTMLDivElement>(null);
  const [launcherVisible, setLauncherVisible] = useState(false);
  const [launcherSearch, setLauncherSearch] = useState('');
  const [launcherIndex, setLauncherIndex] = useState(0);
  const launcherInputRef = useRef<HTMLInputElement>(null);
  const launcherListRef = useRef<HTMLDivElement>(null);
  const [quickSelectVisible, setQuickSelectVisible] = useState(false);
  const [quickSelectMatches, setQuickSelectMatches] = useState<QuickSelectMatch[]>([]);
  const [quickSelectInput, setQuickSelectInput] = useState('');
  const qsInputRef = useRef<HTMLInputElement>(null);
  const [copyModeVisible, setCopyModeVisible] = useState(false);
  const [copyModeCursorRow, setCopyModeCursorRow] = useState(0);
  const [copyModeSelectionStart, setCopyModeSelectionStart] = useState<number | null>(null);
  const [copyModeSelectionMode, setCopyModeSelectionMode] = useState<'char' | 'line' | null>(null);
  const copyModeContainerRef = useRef<HTMLDivElement>(null);
  const cmGFlag = useRef(false);
  const tabCompleteIndex = useRef(-1);
  const lastTabPrefix = useRef('');
  const [searchVisible, setSearchVisible] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchMatches, setSearchMatches] = useState<{
    lineIndex: number;
    matchIndex: number;
    text: string;
  }[]>([]);
  const [searchActiveIndex, setSearchActiveIndex] = useState(0);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [tabGhostText, setTabGhostText] = useState('');
  const [showNewTabMenu, setShowNewTabMenu] = useState(false);
  const [wslStatus, setWslStatus] = useState<{
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
  } | null>(null);
  const newTabMenuRef = useRef<HTMLDivElement>(null);
  const [paneSizes, setPaneSizes] = useState<number[]>([1, 1]);
  const isResizing = useRef(false);
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);
  const {
    aiOn,
    featureOn
  } = useIntelligence();
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
  useEffect(() => {
    if (launcherVisible && launcherInputRef.current) {
      launcherInputRef.current.focus();
    }
  }, [launcherVisible]);
  useEffect(() => {
    if (quickSelectVisible && qsInputRef.current) {
      qsInputRef.current.focus();
    }
  }, [quickSelectVisible]);
  useEffect(() => {
    if (searchVisible && searchInputRef.current) {
      searchInputRef.current.focus();
    }
  }, [searchVisible]);
  useEffect(() => {
    if (cmdSearchVisible && cmdSearchInputRef.current) {
      cmdSearchInputRef.current.focus();
    }
  }, [cmdSearchVisible]);
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
      if (contextMenuRef.current && !contextMenuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
      if (errorContextMenuRef.current && !errorContextMenuRef.current.contains(e.target as Node)) {
        setErrorContextMenu(null);
      }
      if (aiErrorPopupRef.current && !aiErrorPopupRef.current.contains(e.target as Node)) {
        setAiErrorPopup(null);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);
  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const activeTab = tabs.find((t: TabData) => t.id === activeTabId);
    if (!activeTab || activeTab.splitDirection !== 'none') return;
    setContextMenu({
      x: e.clientX,
      y: e.clientY
    });
  }, [activeTabId, tabs]);

  // Task 4.5: 检测错误行
  const isErrorLine = useCallback((line: TerminalLine): boolean => {
    if (line.type === 'error') return true;
    const patterns = /error|Error|failed|Failed|cannot|not found|permission denied|denied|refused|timed out|timeout/i;
    return patterns.test(line.content);
  }, []);

  // Task 4.5: 错误行右键处理
  const handleLineContextMenu = useCallback((e: React.MouseEvent, line: TerminalLine) => {
    if (!isErrorLine(line)) return;
    e.preventDefault();
    e.stopPropagation();
    setErrorContextMenu({
      x: e.clientX,
      y: e.clientY,
      line
    });
  }, [isErrorLine]);

  // Task 4.5: AI 解释错误
  const handleAiExplainError = useCallback(async (line: TerminalLine, x: number, y: number) => {
    setErrorContextMenu(null);
    setAiErrorPopup({
      x,
      y,
      explanation: '',
      loading: true
    });
    try {
      const res = await ipc.invoke<string>('intelligence_query_local_llm', {
        prompt: `Explain this terminal error in Chinese and suggest how to fix it:\n\n${line.content}\n\nProvide a concise explanation and fix suggestions.`
      });
      if (res?.code === 0 && res?.data) {
        setAiErrorPopup(prev => prev ? {
          ...prev,
          explanation: res.data!,
          loading: false
        } : null);
      } else {
        setAiErrorPopup(prev => prev ? {
          ...prev,
          explanation: t("Terminal.k1"),
          loading: false
        } : null);
      }
    } catch {
      setAiErrorPopup(prev => prev ? {
        ...prev,
        explanation: t("Terminal.k2"),
        loading: false
      } : null);
    }
  }, []);

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
  const filteredLauncherEntries = (() => {
    if (!launcherSearch.trim()) return LAUNCHER_ENTRIES;
    const q = launcherSearch.toLowerCase();
    return LAUNCHER_ENTRIES.filter(e => e.command.toLowerCase().includes(q) || e.description.toLowerCase().includes(q));
  })();
  const addOutputLines = useCallback((newHistory: TerminalLine[], output: string, exitCode: number) => {
    if (output) {
      const lines = output.split('\n');
      for (const line of lines) {
        const type: TerminalLine['type'] = exitCode === 0 ? 'output' : 'error';
        newHistory.push({
          type,
          content: line
        });
      }
    }
  }, []);
  const executeBuiltinCommand = useCallback(async (command: string) => {
    const newHistory = [...commandHistory];
    const cmd = command.trim().toLowerCase();
    const cmdName = cmd.split(' ')[0];
    newHistory.push({
      type: 'command',
      content: `user@nexterm:~$ ${command}`
    });
    if (cmdName === 'clear') {
      clearHistory();
      setBlocks([]);
      setCurrentCommand('');
      setHistoryIndex(-1);
      return;
    }
    if (cmdName === 'cmd') {
      newHistory.push({
        type: 'system',
        content: ''
      });
      newHistory.push({
        type: 'success',
        content: t("Terminal.k3")
      });
      newHistory.push({
        type: 'system',
        content: t("Terminal.k4")
      });
      newHistory.push({
        type: 'system',
        content: t("Terminal.k5")
      });
      newHistory.push({
        type: 'system',
        content: ''
      });
      setTerminalMode('cmd');
      setCommandHistory(newHistory);
      setCurrentCommand('');
      setHistoryIndex(-1);
      return;
    }
    if (cmdName === 'powershell') {
      newHistory.push({
        type: 'system',
        content: ''
      });
      newHistory.push({
        type: 'success',
        content: t("Terminal.k6")
      });
      newHistory.push({
        type: 'system',
        content: t("Terminal.k7")
      });
      newHistory.push({
        type: 'system',
        content: t("Terminal.k5")
      });
      newHistory.push({
        type: 'system',
        content: ''
      });
      setTerminalMode('powershell');
      setCommandHistory(newHistory);
      setCurrentCommand('');
      setHistoryIndex(-1);
      return;
    }
    if (cmdName === 'exit') {
      if (terminalMode !== 'builtin') {
        newHistory.push({
          type: 'system',
          content: ''
        });
        newHistory.push({
          type: 'success',
          content: t("Terminal.k8")
        });
        newHistory.push({
          type: 'system',
          content: ''
        });
        setTerminalMode('builtin');
      } else {
        newHistory.push({
          type: 'error',
          content: t("Terminal.k9")
        });
      }
      setCommandHistory(newHistory);
      setCurrentCommand('');
      setHistoryIndex(-1);
      return;
    }
    const blockId = `block_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;
    let blockOutput = '';
    let blockExitCode = 0;
    setIsExecuting(true);
    try {
      const result = await (terminal as any).executeBuiltin(command);
      if (result.code === 0 && result.data) {
        const output = result.data.output;
        blockExitCode = result.data.exit_code ?? 0;
        if (output === '__CLEAR_SCROLLBACK__') {
          clearHistory();
          setBlocks([]);
          setCurrentCommand('');
          setHistoryIndex(-1);
          return;
        }
        if (output === '__FONTSIZE_INCREASE__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k10")
          });
          blockOutput = t("Terminal.k10");
        } else if (output === '__FONTSIZE_DECREASE__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k11")
          });
          blockOutput = t("Terminal.k11");
        } else if (output === '__FONTSIZE_RESET__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k12")
          });
          blockOutput = t("Terminal.k12");
        } else if (output === '__FULLSCREEN_TOGGLE__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k13")
          });
          blockOutput = t("Terminal.k13");
        } else if (output === '__RELOAD_CONFIG__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k14")
          });
          blockOutput = t("Terminal.k14");
        } else if (output === '__SCROLL_TOP__') {
          if (terminalRef.current) {
            terminalRef.current.scrollTop = 0;
          }
          newHistory.push({
            type: 'success',
            content: t("Terminal.k15")
          });
          blockOutput = t("Terminal.k15");
        } else if (output === '__SCROLL_BOTTOM__') {
          if (terminalRef.current) {
            terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
          }
          newHistory.push({
            type: 'success',
            content: t("Terminal.k16")
          });
          blockOutput = t("Terminal.k16");
        } else if (output.startsWith('__SCROLL_UP_')) {
          const match = output.match(/__SCROLL_UP_(\d+)__/);
          const lines = match ? parseInt(match[1], 10) : 1;
          if (terminalRef.current) {
            terminalRef.current.scrollTop -= lines * 18;
          }
          newHistory.push({
            type: 'success',
            content: t("Terminal.k17", {
              lines: lines
            })
          });
          blockOutput = t("Terminal.k17", {
            lines: lines
          });
        } else if (output.startsWith('__SCROLL_DOWN_')) {
          const match = output.match(/__SCROLL_DOWN_(\d+)__/);
          const lines = match ? parseInt(match[1], 10) : 1;
          if (terminalRef.current) {
            terminalRef.current.scrollTop += lines * 18;
          }
          newHistory.push({
            type: 'success',
            content: t("Terminal.k18", {
              lines: lines
            })
          });
          blockOutput = t("Terminal.k18", {
            lines: lines
          });
        } else if (output === '__WINDOW_HIDE__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k19")
          });
          blockOutput = t("Terminal.k19");
        } else if (output === '__APP_QUIT__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k20")
          });
          blockOutput = t("Terminal.k20");
        } else if (output === '__ALWAYS_ON_TOP__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k21")
          });
          blockOutput = t("Terminal.k21");
        } else {
          addOutputLines(newHistory, output, blockExitCode);
          blockOutput = output;
        }
      } else {
        newHistory.push({
          type: 'error',
          content: t("Terminal.k22", {
            message: result.message
          })
        });
        blockOutput = t("Terminal.k22", {
          message: result.message
        });
        blockExitCode = 1;
      }
    } catch {
      newHistory.push({
        type: 'error',
        content: t("Terminal.k23", {
          command: command
        })
      });
      newHistory.push({
        type: 'output',
        content: t("Terminal.k24")
      });
      blockOutput = t("Terminal.k25", {
        command: command
      });
      blockExitCode = 1;
    } finally {
      setIsExecuting(false);
    }
    addBlock({
      id: blockId,
      command,
      output: blockOutput,
      timestamp: Date.now(),
      collapsed: false,
      exitCode: blockExitCode
    });
    setCommandHistory(newHistory);
    setCurrentCommand('');
    setHistoryIndex(-1);
  }, [commandHistory, terminalMode, addOutputLines, clearHistory, setCommandHistory, setCurrentCommand, setTerminalMode, addBlock, setBlocks]);
  const executeSystemCommand = useCallback((command: string) => {
    const newHistory = [...commandHistory];
    const modeLabel = terminalMode === 'cmd' ? 'CMD' : 'PowerShell';
    const prompt = terminalMode === 'cmd' ? 'C:\\Users\\user>' : 'PS C:\\Users\\user>';
    newHistory.push({
      type: 'command',
      content: `${prompt} ${command}`
    });
    if (command.trim().toLowerCase() === 'exit') {
      newHistory.push({
        type: 'system',
        content: ''
      });
      newHistory.push({
        type: 'success',
        content: t("Terminal.k8")
      });
      newHistory.push({
        type: 'system',
        content: ''
      });
      setTerminalMode('builtin');
      setCommandHistory(newHistory);
      setCurrentCommand('');
      return;
    }
    newHistory.push({
      type: 'system',
      content: t("Terminal.k26", {
        modeLabel: modeLabel,
        command: command
      })
    });
    newHistory.push({
      type: 'system',
      content: t("Terminal.k27")
    });
    addBlock({
      id: `block_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      command,
      output: t("Terminal.k28", {
        modeLabel: modeLabel,
        command: command
      }),
      timestamp: Date.now(),
      collapsed: false
    });
    setCommandHistory(newHistory);
    setCurrentCommand('');
  }, [commandHistory, terminalMode, setCommandHistory, setCurrentCommand, setTerminalMode, addBlock]);
  const executeCommand = useCallback((command: string) => {
    if (command.trim() === '' || isExecuting) return;
    prependUserCommand(command);
    if (terminalMode === 'builtin') {
      executeBuiltinCommand(command);
    } else {
      executeSystemCommand(command);
    }
  }, [terminalMode, isExecuting, executeBuiltinCommand, executeSystemCommand, prependUserCommand]);
  const handleAiSuggest = useCallback(async (hint: string) => {
    setAiSuggestLoading(true);
    setAiSuggestedCmd('');
    try {
      const res = await intelligence.terminalComplete(hint, undefined, []);
      if (res?.data && res.data.length > 0) {
        setAiSuggestedCmd(res.data[0].command);
      }
    } catch {/* silent */} finally {
      setAiSuggestLoading(false);
    }
  }, []);

  // === Passive AI: auto-suggest command when typing natural language (debounced 1s) ===
  useEffect(() => {
    if (!aiOn || !featureOn('command_suggest') || !currentCommand.trim() || isLinuxPage) {
      setAiSuggestedCmd('');
      return;
    }
    const lower = currentCommand.trim().toLowerCase();
    if (ALL_COMMANDS.includes(lower) || ALL_COMMANDS.some(cmd => lower.startsWith(cmd + ' '))) {
      setAiSuggestedCmd('');
      return;
    }
    if (suggestTimerRef.current) clearTimeout(suggestTimerRef.current);
    suggestTimerRef.current = setTimeout(() => {
      handleAiSuggest(currentCommand);
    }, 1000);
    return () => {
      if (suggestTimerRef.current) clearTimeout(suggestTimerRef.current);
    };
  }, [currentCommand, aiOn, featureOn, isLinuxPage, handleAiSuggest]);

  // === Task 4.3: # 自然语言触发 AI 命令建议 ===
  useEffect(() => {
    if (!currentCommand.startsWith('#')) {
      setHashSuggestedCmd('');
      setHashSuggestLoading(false);
      return;
    }
    const prompt = currentCommand.slice(1).trim();
    if (!prompt) {
      setHashSuggestedCmd('');
      setHashSuggestLoading(false);
      return;
    }
    if (hashTimerRef.current) clearTimeout(hashTimerRef.current);
    setHashSuggestLoading(true);
    setHashSuggestedCmd('');
    hashTimerRef.current = setTimeout(async () => {
      try {
        const res = await intelligence.terminalComplete(prompt, undefined, []);
        if (res?.data && res.data.length > 0) {
          setHashSuggestedCmd(res.data[0].command);
        }
      } catch {/* silent */} finally {
        setHashSuggestLoading(false);
      }
    }, 800);
    return () => {
      if (hashTimerRef.current) clearTimeout(hashTimerRef.current);
    };
  }, [currentCommand]);
  const getPrompt = useCallback(() => {
    switch (terminalMode) {
      case 'cmd':
        return 'C:\\Users\\user> ';
      case 'powershell':
        return 'PS C:\\Users\\user> ';
      default:
        return 'user@nexterm:~$ ';
    }
  }, [terminalMode]);

  // === Task 4.2: 命令搜索面板 ===
  const getAllCommands = useCallback((): {
    command: string;
    timestamp: number;
    tabName: string;
  }[] => {
    const results: {
      command: string;
      timestamp: number;
      tabName: string;
    }[] = [];
    for (const tab of tabs) {
      for (const cmd of tab.userCommands) {
        if (!results.some(r => r.command === cmd)) {
          results.push({
            command: cmd,
            timestamp: Date.now(),
            tabName: tab.title
          });
        }
      }
    }
    return results;
  }, [tabs]);
  const fuzzyMatch = useCallback((text: string, query: string): boolean => {
    const lowerText = text.toLowerCase();
    const lowerQuery = query.toLowerCase();
    let qi = 0;
    for (let i = 0; i < lowerText.length && qi < lowerQuery.length; i++) {
      if (lowerText[i] === lowerQuery[qi]) qi++;
    }
    return qi === lowerQuery.length;
  }, []);
  const filteredCommands = useMemo(() => {
    const all = getAllCommands();
    if (!cmdSearchQuery.trim()) return all;
    const q = cmdSearchQuery.trim();
    return all.filter(c => fuzzyMatch(c.command, q));
  }, [getAllCommands, cmdSearchQuery, fuzzyMatch]);
  const handleCmdSearchKey = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      setCmdSearchVisible(false);
      setCmdSearchQuery('');
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setCmdSearchIndex(prev => Math.min(prev + 1, filteredCommands.length - 1));
      return;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      setCmdSearchIndex(prev => Math.max(prev - 1, 0));
      return;
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      const cmd = filteredCommands[cmdSearchIndex];
      if (cmd) {
        setCurrentCommand(cmd.command);
        setCmdSearchVisible(false);
        setCmdSearchQuery('');
        setTimeout(() => inputRef.current?.focus(), 0);
      }
    }
  }, [filteredCommands, cmdSearchIndex, setCurrentCommand]);
  const openLauncher = useCallback(() => {
    setLauncherSearch('');
    setLauncherIndex(0);
    setLauncherVisible(true);
  }, []);
  const closeLauncher = useCallback(() => {
    setLauncherVisible(false);
    setLauncherSearch('');
    setLauncherIndex(0);
    setTimeout(() => inputRef.current?.focus(), 0);
  }, []);
  const handleLauncherKey = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      closeLauncher();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      setLauncherIndex(prev => Math.min(prev + 1, filteredLauncherEntries.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setLauncherIndex(prev => Math.max(prev - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const entry = filteredLauncherEntries[launcherIndex];
      if (entry) {
        setCurrentCommand(entry.command);
        closeLauncher();
        setTimeout(() => inputRef.current?.focus(), 0);
      }
    }
  }, [filteredLauncherEntries, launcherIndex, closeLauncher, setCurrentCommand]);
  useEffect(() => {
    setLauncherIndex(0);
    if (launcherListRef.current) {
      launcherListRef.current.scrollTop = 0;
    }
  }, [launcherSearch]);
  const scanQuickSelectMatches = useCallback(() => {
    const matches: QuickSelectMatch[] = [];
    const outputLines: {
      index: number;
      content: string;
    }[] = [];
    commandHistory.forEach((line: TerminalLine, i: number) => {
      if (line.type === 'output' || line.type === 'error') {
        outputLines.push({
          index: i,
          content: line.content
        });
      }
    });
    const seen = new Set<string>();
    for (const {
      index,
      content
    } of outputLines) {
      for (const pattern of QUICK_SELECT_PATTERNS) {
        const regex = new RegExp(pattern.regex.source, pattern.regex.flags);
        let match: RegExpExecArray | null;
        while ((match = regex.exec(content)) !== null) {
          const key = `${index}:${match.index}:${match[0]}`;
          if (seen.has(key)) continue;
          seen.add(key);
          matches.push({
            label: '',
            text: match[0],
            lineIndex: index,
            startIndex: match.index,
            patternName: pattern.name
          });
        }
      }
    }
    const labels = computeQuickSelectLabels(matches.length);
    matches.forEach((m, i) => {
      m.label = labels[i] || '';
    });
    return matches;
  }, [commandHistory]);
  const openQuickSelect = useCallback(() => {
    const matches = scanQuickSelectMatches();
    if (matches.length === 0) return;
    setQuickSelectMatches(matches);
    setQuickSelectInput('');
    setQuickSelectVisible(true);
  }, [scanQuickSelectMatches]);
  const closeQuickSelect = useCallback(() => {
    setQuickSelectVisible(false);
    setQuickSelectInput('');
    setQuickSelectMatches([]);
    setTimeout(() => inputRef.current?.focus(), 0);
  }, []);
  const handleQuickSelectKey = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      closeQuickSelect();
      return;
    }
    if (e.key === 'Backspace') {
      e.preventDefault();
      setQuickSelectInput(prev => prev.slice(0, -1));
      return;
    }
    if (e.key.length === 1 && /[a-zA-Z]/.test(e.key)) {
      e.preventDefault();
      const newInput = quickSelectInput + e.key.toLowerCase();
      const match = quickSelectMatches.find(m => m.label === newInput);
      if (match) {
        copy(match.text).catch(() => {});
        if (e.key === e.key.toUpperCase()) {
          setCurrentCommand(match.text);
        }
        closeQuickSelect();
      } else {
        const prefixMatches = quickSelectMatches.filter(m => m.label.startsWith(newInput));
        if (prefixMatches.length > 0) {
          setQuickSelectInput(newInput);
        }
      }
    }
  }, [quickSelectInput, quickSelectMatches, closeQuickSelect, setCurrentCommand]);
  const enterCopyMode = useCallback(() => {
    const outputRowCount = commandHistory.length;
    if (outputRowCount === 0) return;
    setCopyModeCursorRow(Math.max(0, outputRowCount - 1));
    setCopyModeSelectionStart(null);
    setCopyModeSelectionMode(null);
    setCopyModeVisible(true);
  }, [commandHistory]);
  const exitCopyMode = useCallback(() => {
    setCopyModeVisible(false);
    setCopyModeSelectionStart(null);
    setCopyModeSelectionMode(null);
    setTimeout(() => inputRef.current?.focus(), 0);
  }, []);
  const copyModeCopy = useCallback(() => {
    if (copyModeSelectionStart === null || copyModeCursorRow === copyModeSelectionStart) {
      const line = commandHistory[copyModeCursorRow];
      if (line) {
        copy(line.content).catch(() => {});
      }
    } else {
      const start = Math.min(copyModeSelectionStart, copyModeCursorRow);
      const end = Math.max(copyModeSelectionStart, copyModeCursorRow);
      const lines = commandHistory.slice(start, end + 1);
      const text = lines.map((l: TerminalLine) => l.content).join('\n');
      copy(text).catch(() => {});
    }
    exitCopyMode();
  }, [copyModeCursorRow, copyModeSelectionStart, commandHistory, exitCopyMode]);
  const handleCopyModeKey = useCallback((e: React.KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const maxRow = commandHistory.length - 1;
    if (e.key === 'Escape' || e.key === 'c' && e.ctrlKey || e.key === 'q') {
      exitCopyMode();
      return;
    }
    if (e.key === 'y') {
      copyModeCopy();
      return;
    }
    if (e.key === 'v' && !e.ctrlKey) {
      if (copyModeSelectionStart === null) {
        setCopyModeSelectionStart(copyModeCursorRow);
        setCopyModeSelectionMode('char');
      } else {
        setCopyModeSelectionStart(null);
        setCopyModeSelectionMode(null);
      }
      return;
    }
    if (e.key === 'V' && e.shiftKey && !e.ctrlKey) {
      if (copyModeSelectionStart === null) {
        setCopyModeSelectionStart(copyModeCursorRow);
        setCopyModeSelectionMode('line');
      } else {
        setCopyModeSelectionStart(null);
        setCopyModeSelectionMode(null);
      }
      return;
    }
    if (e.key === 'j' || e.key === 'ArrowDown') {
      setCopyModeCursorRow(prev => Math.min(prev + 1, maxRow));
    } else if (e.key === 'k' || e.key === 'ArrowUp') {
      setCopyModeCursorRow(prev => Math.max(prev - 1, 0));
    } else if (e.key === 'h' || e.key === 'ArrowLeft') {
      setCopyModeCursorRow(prev => Math.max(prev - 1, 0));
    } else if (e.key === 'l' || e.key === 'ArrowRight') {
      setCopyModeCursorRow(prev => Math.min(prev + 1, maxRow));
    } else if (e.key === 'g' && !e.shiftKey) {
      if (cmGFlag.current) {
        setCopyModeCursorRow(0);
        cmGFlag.current = false;
      } else {
        cmGFlag.current = true;
        setTimeout(() => {
          cmGFlag.current = false;
        }, 500);
      }
    } else if (e.key === 'G' && e.shiftKey) {
      setCopyModeCursorRow(maxRow);
    } else if (e.key === '0' || e.key === 'Home') {
      setCopyModeCursorRow(0);
    } else if (e.key === '$' || e.key === 'End') {
      setCopyModeCursorRow(maxRow);
    } else if (e.key === 'w') {
      setCopyModeCursorRow(prev => Math.min(prev + 5, maxRow));
    } else if (e.key === 'b') {
      setCopyModeCursorRow(prev => Math.max(prev - 5, 0));
    } else if (e.key === 'd' && e.ctrlKey) {
      setCopyModeCursorRow(prev => Math.min(prev + 15, maxRow));
    } else if (e.key === 'u' && e.ctrlKey) {
      setCopyModeCursorRow(prev => Math.max(prev - 15, 0));
    }
  }, [commandHistory, exitCopyMode, copyModeCopy, copyModeCursorRow, copyModeSelectionStart]);
  const scanSearchMatches = useCallback((query: string, history: TerminalLine[]) => {
    if (!query.trim()) return [];
    const matches: {
      lineIndex: number;
      matchIndex: number;
      text: string;
    }[] = [];
    const lowerQuery = query.toLowerCase();
    for (let i = 0; i < history.length; i++) {
      const content = history[i].content.toLowerCase();
      let fromIndex = 0;
      while (true) {
        const idx = content.indexOf(lowerQuery, fromIndex);
        if (idx === -1) break;
        matches.push({
          lineIndex: i,
          matchIndex: idx,
          text: history[i].content.slice(idx, idx + query.length)
        });
        fromIndex = idx + 1;
      }
    }
    return matches;
  }, []);
  const openSearch = useCallback(() => {
    setSearchVisible(true);
    setSearchQuery('');
    setSearchMatches([]);
    setSearchActiveIndex(0);
  }, []);
  const closeSearch = useCallback(() => {
    setSearchVisible(false);
    setSearchQuery('');
    setSearchMatches([]);
    setSearchActiveIndex(0);
    setTimeout(() => inputRef.current?.focus(), 0);
  }, []);
  const updateSearch = useCallback((query: string) => {
    setSearchQuery(query);
    const matches = scanSearchMatches(query, commandHistory);
    setSearchMatches(matches);
    setSearchActiveIndex(matches.length > 0 ? 0 : -1);
    if (matches.length > 0) {
      scrollToMatchLine(matches[0].lineIndex);
    }
  }, [commandHistory, scanSearchMatches]);
  const scrollToMatchLine = useCallback((lineIndex: number) => {
    if (!terminalRef.current) return;
    const lineEl = terminalRef.current.children[lineIndex] as HTMLElement | undefined;
    if (lineEl) {
      lineEl.scrollIntoView({
        block: 'center',
        behavior: 'smooth'
      });
    }
  }, []);
  const navigateSearch = useCallback((direction: 'next' | 'prev') => {
    if (searchMatches.length === 0) return;
    let newIndex: number;
    if (direction === 'next') {
      newIndex = (searchActiveIndex + 1) % searchMatches.length;
    } else {
      newIndex = (searchActiveIndex - 1 + searchMatches.length) % searchMatches.length;
    }
    setSearchActiveIndex(newIndex);
    scrollToMatchLine(searchMatches[newIndex].lineIndex);
  }, [searchMatches, searchActiveIndex, scrollToMatchLine]);
  const handleSearchKey = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      closeSearch();
      return;
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      if (e.shiftKey) {
        navigateSearch('prev');
      } else {
        navigateSearch('next');
      }
      return;
    }
    if (e.key === 'f' && e.ctrlKey) {
      e.preventDefault();
      return;
    }
  }, [closeSearch, navigateSearch]);
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
  const handleKeyPress = useCallback((e: React.KeyboardEvent) => {
    if ((e.key === 'p' || e.key === 'P') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      openLauncher();
      return;
    }
    if (e.key === ' ' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      openQuickSelect();
      return;
    }
    if ((e.key === 'x' || e.key === 'X') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      enterCopyMode();
      return;
    }
    if ((e.key === 'f' || e.key === 'F') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      openSearch();
      return;
    }
    if ((e.key === 'r' || e.key === 'R') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      setCmdSearchVisible(true);
      setCmdSearchQuery('');
      setCmdSearchIndex(0);
      return;
    }
    if (e.key === 'Enter') {
      if (e.shiftKey) {
        e.preventDefault();
        setIsMultiLine(true);
        setCurrentCommand(currentCommand + '\n');
        return;
      }
      setIsMultiLine(false);
      if (currentCommand.trim() === '') {
        const newHistory = [...commandHistory];
        newHistory.push({
          type: 'command',
          content: `${getPrompt()}`
        });
        setCommandHistory(newHistory);
      } else {
        executeCommand(currentCommand);
      }
    } else if (e.key === 'Escape' && isMultiLine) {
      e.preventDefault();
      setIsMultiLine(false);
      setCurrentCommand('');
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (historyIndex < userCommands.length - 1) {
        const newIndex = historyIndex + 1;
        setHistoryIndex(newIndex);
        setCurrentCommand(userCommands[newIndex]);
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndex > 0) {
        const newIndex = historyIndex - 1;
        setHistoryIndex(newIndex);
        setCurrentCommand(userCommands[newIndex]);
      } else if (historyIndex === 0) {
        setHistoryIndex(-1);
        setCurrentCommand('');
      }
    } else if (e.key === 'Tab') {
      e.preventDefault();
      // Tab 只接受 ghost text 补全
      if (tabGhostText) {
        setCurrentCommand(currentCommand + tabGhostText);
        setTabGhostText('');
      } else {
        // 无 ghost text 时执行原来的 Tab 补全逻辑
        const prefix = currentCommand.toLowerCase().trim();
        if (!prefix) return;
        if (prefix !== lastTabPrefix.current) {
          tabCompleteIndex.current = -1;
          lastTabPrefix.current = prefix;
        }
        const historyMatches = userCommands.filter((cmd: string) => cmd.toLowerCase().startsWith(prefix));
        if (historyMatches.length > 0) {
          tabCompleteIndex.current = (tabCompleteIndex.current + 1) % historyMatches.length;
          setCurrentCommand(historyMatches[tabCompleteIndex.current]);
        } else {
          const match = ALL_COMMANDS.find(cmd => cmd.startsWith(prefix));
          if (match) {
            setCurrentCommand(match);
          } else if (aiOn && featureOn('command_suggest')) {
            setAiSuggestedCmd('');
            setAiSuggestLoading(true);
            intelligence.terminalComplete(currentCommand).then(res => {
              if (res?.code === 0 && res?.data && res.data.length > 0) {
                const aiCmd = res.data[0].command.trim();
                if (aiCmd && !aiCmd.toLowerCase().startsWith(prefix)) {
                  setAiSuggestedCmd('');
                } else {
                  setAiSuggestedCmd(aiCmd);
                }
              }
            }).catch(() => {}).finally(() => setAiSuggestLoading(false));
          }
        }
      }
    } else if (e.key === 'l' && e.ctrlKey) {
      e.preventDefault();
      clearHistory();
      setBlocks([]);
    } else if (e.key === 'K' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      clearHistory();
      setBlocks([]);
    } else if (e.key === 'c' && e.ctrlKey && currentCommand === '') {
      e.preventDefault();
      if (terminalMode !== 'builtin') {
        const newHistory = [...commandHistory];
        newHistory.push({
          type: 'system',
          content: '^C'
        });
        newHistory.push({
          type: 'success',
          content: t("Terminal.k8")
        });
        newHistory.push({
          type: 'system',
          content: ''
        });
        setTerminalMode('builtin');
        setCommandHistory(newHistory);
      } else {
        const newHistory = [...commandHistory];
        newHistory.push({
          type: 'system',
          content: '^C'
        });
        setCommandHistory(newHistory);
      }
    } else if (e.key === 'a' && e.ctrlKey) {
      e.preventDefault();
      inputRef.current?.select();
    } else if (e.key === 'Escape') {
      setTabGhostText('');
    } else if (e.key === 'u' && e.ctrlKey) {
      e.preventDefault();
      setCurrentCommand('');
      setTabGhostText('');
    } else if (e.key === '=' && e.ctrlKey) {
      e.preventDefault();
      executeCommand('fontsize +');
    } else if (e.key === '-' && e.ctrlKey) {
      e.preventDefault();
      executeCommand('fontsize -');
    } else if (e.key === '0' && e.ctrlKey) {
      e.preventDefault();
      executeCommand('fontsize 0');
    } else if (e.key === 'Enter' && e.altKey) {
      e.preventDefault();
      executeCommand('fullscreen');
    } else if (e.key === 'PageUp' && e.shiftKey) {
      e.preventDefault();
      executeCommand('scroll up 10');
    } else if (e.key === 'PageDown' && e.shiftKey) {
      e.preventDefault();
      executeCommand('scroll down 10');
    } else if ((e.key === 't' || e.key === 'T') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      setShowNewTabMenu(true);
    } else if ((e.key === 'w' || e.key === 'W') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      const activeTab = tabs.find((t: TabData) => t.id === activeTabId);
      if (activeTab && activeTab.splitDirection !== 'none') {
        handleClosePane(activeTab.activePaneId);
      } else {
        handleCloseTab(activeTabId);
      }
    } else if (e.key === '\\' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      handleSplitPane('vertical');
    } else if (e.key === '-' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      handleSplitPane('horizontal');
    } else if (e.key === '[' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      const tab = tabs.find((t: TabData) => t.id === activeTabId);
      if (tab && tab.splitDirection !== 'none' && tab.panes.length > 1) {
        const idx = tab.panes.findIndex((p: PaneData) => p.id === tab.activePaneId);
        const prevIdx = (idx - 1 + tab.panes.length) % tab.panes.length;
        focusPane(activeTabId, tab.panes[prevIdx].id);
      }
    } else if (e.key === ']' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      const tab = tabs.find((t: TabData) => t.id === activeTabId);
      if (tab && tab.splitDirection !== 'none' && tab.panes.length > 1) {
        const idx = tab.panes.findIndex((p: PaneData) => p.id === tab.activePaneId);
        const nextIdx = (idx + 1) % tab.panes.length;
        focusPane(activeTabId, tab.panes[nextIdx].id);
      }
    } else if (e.key === 'Tab' && e.ctrlKey) {
      e.preventDefault();
      const idx = tabs.findIndex((t: TabData) => t.id === activeTabId);
      if (e.shiftKey) {
        const prevIdx = (idx - 1 + tabs.length) % tabs.length;
        switchTab(tabs[prevIdx].id);
      } else {
        const nextIdx = (idx + 1) % tabs.length;
        switchTab(tabs[nextIdx].id);
      }
    } else if ((e.key === 'c' || e.key === 'C') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      const _outputs = commandHistory.filter((l: TerminalLine) => l.type === 'output' || l.type === 'error');
      const lastOutput = _outputs[_outputs.length - 1];
      if (lastOutput) {
        copy(lastOutput.content).catch(() => {});
      } else {
        const allText = commandHistory.map((l: TerminalLine) => l.content).join('\n');
        copy(allText).catch(() => {});
      }
    } else if ((e.key === 'v' || e.key === 'V') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      navigator.clipboard.readText().then(text => {
        setCurrentCommand(currentCommand + text);
      }).catch(() => {});
    }
  }, [currentCommand, commandHistory, historyIndex, userCommands, terminalMode, executeCommand, getPrompt, setCommandHistory, setCurrentCommand, setTerminalMode, clearHistory, openLauncher, openQuickSelect, enterCopyMode, openSearch, tabs, activeTabId, createTab, closeTab, switchTab, handleSplitPane, handleClosePane, focusPane, handleCloseTab]);
  const getCopyModeLineClass = (index: number) => {
    if (!copyModeVisible) return '';
    const isCursor = index === copyModeCursorRow;
    if (isCursor && copyModeSelectionStart === null) return ` ${styles.cmCursorLine}`;
    if (copyModeSelectionStart !== null) {
      const selStart = Math.min(copyModeSelectionStart, copyModeCursorRow);
      const selEnd = Math.max(copyModeSelectionStart, copyModeCursorRow);
      if (index >= selStart && index <= selEnd) {
        return index === copyModeCursorRow ? ` ${styles.cmSelectionLine} ${styles.cmCursorLine}` : ` ${styles.cmSelectionLine}`;
      }
      if (isCursor) return ` ${styles.cmCursorLine}`;
    }
    return '';
  };
  const renderLineWithHighlight = (content: string, lineIndex: number) => {
    if (!searchVisible || !searchQuery.trim()) return content;
    const lineMatches = searchMatches.filter(m => m.lineIndex === lineIndex);
    if (lineMatches.length === 0) return content;
    const parts: Array<{
      text: string;
      highlighted: boolean;
      active: boolean;
    }> = [];
    let lastIndex = 0;
    const sorted = [...lineMatches].sort((a, b) => a.matchIndex - b.matchIndex);
    for (const m of sorted) {
      if (m.matchIndex > lastIndex) {
        parts.push({
          text: content.slice(lastIndex, m.matchIndex),
          highlighted: false,
          active: false
        });
      }
      const globalIdx = searchMatches.indexOf(m);
      parts.push({
        text: content.slice(m.matchIndex, m.matchIndex + m.text.length),
        highlighted: true,
        active: globalIdx === searchActiveIndex
      });
      lastIndex = m.matchIndex + m.text.length;
    }
    if (lastIndex < content.length) {
      parts.push({
        text: content.slice(lastIndex),
        highlighted: false,
        active: false
      });
    }
    return parts.map((p, i) => {
      if (p.highlighted) {
        return <span key={i} className={`${styles.searchHighlight} ${p.active ? styles.searchHighlightActive : ''}`}>
            {p.text}
          </span>;
      }
      return <span key={i}>{p.text}</span>;
    });
  };
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
  const renderLineContent = (line: TerminalLine, index: number) => {
    if (searchVisible && searchQuery.trim()) {
      return renderLineWithHighlight(line.content, index);
    }
    const parts = splitByLinks(line.content);
    if (parts.length <= 1) return renderAnsiText(line.content);
    return parts.map((p, i) => p.isLink ? <a key={i} href={p.text} target="_blank" rel="noopener noreferrer" className={styles.outputLink} onClick={e => {
      e.stopPropagation();
      ipc.invoke('system_open_url', {
        url: p.text
      }).catch(() => {
        window.open(p.text, '_blank', 'noopener,noreferrer');
      });
    }}>
            {p.text}
          </a> : <span key={i}>{renderAnsiText(p.text)}</span>);
  };
  const renderTerminalContent = () => {
    const activeTab = tabs.find((t: TabData) => t.id === activeTabId);
    if (!activeTab) return null;
    if (activeTab.splitDirection !== 'none' && activeTab.panes.length > 1) {
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
    }
    const isPtyTab = activeTab.type === 'cmd' || activeTab.type === 'powershell' || activeTab.type === 'wsl';
    if (isPtyTab) {
      return <div className={styles.ptyTerminalWrapper}>
          <NexTermTerminal sessionType={activeTab!.type as 'cmd' | 'powershell' | 'wsl'} autoConnect={!activeTab!.sessionId} externalSessionId={activeTab!.sessionId} onSessionCreated={sid => {
          updateTabSessionId(activeTabId, sid);
        }} fontSize={14} />
        </div>;
    }
    // 将 blocks 按命令匹配到对应位置
    const renderTerminalContent = () => {
      const elements: React.ReactNode[] = [];
      let blockIndex = 0;
      
      commandHistory.forEach((line: TerminalLine, index: number) => {
        elements.push(
          <div key={`line-${index}`} className={`${styles.terminalLine}${getCopyModeLineClass(index)}`} onContextMenu={e => handleLineContextMenu(e, line)}>
            <span className={styles[line.type]}>
              {renderLineContent(line, index)}
            </span>
          </div>
        );
        
        // 如果当前行是命令类型，检查是否有对应的 block
        if (line.type === 'command' && blockIndex < blocks.length) {
          const block = blocks[blockIndex];
          // 匹配 block 的命令（去掉 prompt 部分，支持多种终端模式）
          const lineCommand = line.content
            .replace(/^user@nexterm:~\$ /, '')
            .replace(/^C:\\Users\\user> /, '')
            .replace(/^PS C:\\Users\\user> /, '')
            .trim();
          if (block.command.trim() === lineCommand) {
            elements.push(
              <div key={`block-${block.id}`} className={styles.blockWrapper} id={`block-${block.id}`}>
                <div className={styles.blockHeader} onClick={() => toggleBlockCollapse(block.id)}>
                  <span className={styles.blockNumber}>#{blockIndex + 1}</span>
                  <span className={styles.blockCommand}>{block.command}</span>
                  <span className={styles.blockTimestamp}>
                    {new Date(block.timestamp).toLocaleTimeString()}
                  </span>
                  {block.exitCode !== undefined && block.exitCode !== 0 && <span className={styles.blockExitCode}>exit:{block.exitCode}</span>}
                  <button className={styles.blockCopyBtn} onClick={e => {
                    e.stopPropagation();
                    copy(block.output).catch(() => {});
                  }} title={t("Terminal.k41")}>
                    📋
                  </button>
                  <span className={styles.blockToggle}>
                    {block.collapsed ? '▶' : '▼'}
                  </span>
                </div>
                <div className={`${styles.blockOutput} ${block.collapsed ? styles.blockOutputCollapsed : ''}`}>
                  {block.collapsed ? block.output.split('\n').slice(0, 1).map((line, i) => <div key={i} className={`${styles.blockOutputLine} ${block.exitCode !== 0 ? styles.blockOutputError : ''}`}>
                          {line || '\u00A0'}
                        </div>) : block.output.split('\n').map((line, i) => <div key={i} className={`${styles.blockOutputLine} ${block.exitCode !== 0 ? styles.blockOutputError : ''}`}>
                          {line || '\u00A0'}
                        </div>)}
                </div>
              </div>
            );
            blockIndex++;
          }
        }
      });
      
      // 渲染剩余的 blocks（未匹配到命令行的）
      while (blockIndex < blocks.length) {
        const block = blocks[blockIndex];
        elements.push(
          <div key={`block-${block.id}`} className={styles.blockWrapper} id={`block-${block.id}`}>
            <div className={styles.blockHeader} onClick={() => toggleBlockCollapse(block.id)}>
              <span className={styles.blockNumber}>#{blockIndex + 1}</span>
              <span className={styles.blockCommand}>{block.command}</span>
              <span className={styles.blockTimestamp}>
                {new Date(block.timestamp).toLocaleTimeString()}
              </span>
              {block.exitCode !== undefined && block.exitCode !== 0 && <span className={styles.blockExitCode}>exit:{block.exitCode}</span>}
              <button className={styles.blockCopyBtn} onClick={e => {
                e.stopPropagation();
                copy(block.output).catch(() => {});
              }} title={t("Terminal.k41")}>
                📋
              </button>
              <span className={styles.blockToggle}>
                {block.collapsed ? '▶' : '▼'}
              </span>
            </div>
            <div className={`${styles.blockOutput} ${block.collapsed ? styles.blockOutputCollapsed : ''}`}>
              {block.collapsed ? block.output.split('\n').slice(0, 1).map((line, i) => <div key={i} className={`${styles.blockOutputLine} ${block.exitCode !== 0 ? styles.blockOutputError : ''}`}>
                      {line || '\u00A0'}
                    </div>) : block.output.split('\n').map((line, i) => <div key={i} className={`${styles.blockOutputLine} ${block.exitCode !== 0 ? styles.blockOutputError : ''}`}>
                      {line || '\u00A0'}
                    </div>)}
            </div>
          </div>
        );
        blockIndex++;
      }
      
      return elements;
    };
    
    const content = <div className={styles.terminalWindow} ref={terminalRef} onClick={() => !copyModeVisible && inputRef.current?.focus()}>
        {renderTerminalContent()}
        {!copyModeVisible && <div className={`${styles.inputLine} ${isMultiLine ? styles.inputLineMulti : ''}`}>
            <span className={styles.inputPrompt}>{getPrompt()}</span>
            <div style={{ position: 'relative', flex: 1 }}>
              {isMultiLine ? <textarea ref={inputRef as React.RefObject<HTMLTextAreaElement>} value={currentCommand} onChange={e => {
                const val = e.target.value;
                setCurrentCommand(val);
                // 自动生成 ghost text
                const prefix = val.toLowerCase().trim();
                if (prefix.length >= 2) {
                  const historyMatch = userCommands.find((cmd: string) => cmd.toLowerCase().startsWith(prefix));
                  if (historyMatch) {
                    setTabGhostText(historyMatch.slice(val.length));
                  } else {
                    const cmdMatch = ALL_COMMANDS.find(cmd => cmd.startsWith(prefix));
                    setTabGhostText(cmdMatch ? cmdMatch.slice(val.length) : '');
                  }
                } else {
                  setTabGhostText('');
                }
              }} onKeyDown={handleKeyPress} className={styles.inputFieldMulti} placeholder={isExecuting ? t("Terminal.k42") : t("Terminal.k43")} autoFocus spellCheck={false} disabled={isExecuting} rows={Math.min(currentCommand.split('\n').length + 1, 12)} /> : <input ref={inputRef as React.RefObject<HTMLInputElement>} type="text" value={currentCommand} onChange={e => {
                const val = e.target.value;
                setCurrentCommand(val);
                // 自动生成 ghost text
                const prefix = val.toLowerCase().trim();
                if (prefix.length >= 2) {
                  const historyMatch = userCommands.find((cmd: string) => cmd.toLowerCase().startsWith(prefix));
                  if (historyMatch) {
                    setTabGhostText(historyMatch.slice(val.length));
                  } else {
                    const cmdMatch = ALL_COMMANDS.find(cmd => cmd.startsWith(prefix));
                    setTabGhostText(cmdMatch ? cmdMatch.slice(val.length) : '');
                  }
                } else {
                  setTabGhostText('');
                }
              }} onKeyDown={handleKeyPress} className={styles.inputField} placeholder={isExecuting ? t("Terminal.k42") : t("Terminal.k44")} autoFocus spellCheck={false} disabled={isExecuting} />}
              {tabGhostText && !isMultiLine && <span style={{
                position: 'absolute',
                left: 0,
                top: 0,
                pointerEvents: 'none',
                color: 'rgba(0, 240, 255, 0.4)',
                whiteSpace: 'pre',
                fontFamily: 'var(--nt-font-mono)',
                fontSize: 'inherit'
              }}>
                <span style={{ visibility: 'hidden' }}>{currentCommand}</span>
                <span>{tabGhostText}</span>
              </span>}
            </div>
          </div>}
        {aiSuggestLoading && <div className={styles.aiChecking}>{t("Terminal.k45")}</div>}
        {!aiSuggestLoading && aiSuggestedCmd && <div className={styles.aiGhostSuggestion}>
            <span className={styles.aiGhostLabel}>{t("profile.QuotePanel.k15")}</span>
            <span className={styles.aiGhostCmd}>{aiSuggestedCmd}</span>
            <button className={styles.aiGhostApply} onClick={() => {
          setCurrentCommand(aiSuggestedCmd);
          setAiSuggestedCmd('');
        }}>
              ↩
            </button>
          </div>}
        {/* Task 4.3: # AI 命令建议 */}
        {hashSuggestLoading && <div className={styles.aiHashLoading}>{t("Terminal.k46")}</div>}
        {!hashSuggestLoading && hashSuggestedCmd && <div className={styles.aiHashResult}>
            <span className={styles.aiHashResultCmd}>{hashSuggestedCmd}</span>
            <button className={styles.aiHashResultBtn} onClick={() => {
          setCurrentCommand(hashSuggestedCmd);
          setHashSuggestedCmd('');
        }}>
              {t("Terminal.k47")}
            </button>
          </div>}
        {searchVisible && <div className={styles.searchBarContainer}>
            <div className={styles.searchBar}>
              <span className={styles.searchIcon}>🔍</span>
              <input ref={searchInputRef} type="text" value={searchQuery} onChange={e => updateSearch(e.target.value)} onKeyDown={handleSearchKey} className={styles.searchInput} placeholder={t("Terminal.k48")} spellCheck={false} />
              <span className={styles.searchCount}>
                {searchMatches.length > 0 ? `${searchActiveIndex + 1}/${searchMatches.length}` : searchQuery ? t("components.NexTermTerminal.k2") : ''}
              </span>
              <span className={styles.searchHint}>
                {t("Terminal.k49")}
              </span>
              <button className={styles.searchCloseBtn} onClick={closeSearch}>✕</button>
            </div>
          </div>}
      </div>;
    if (copyModeVisible) {
      return <div className={styles.copyModeContainer} ref={copyModeContainerRef} onKeyDown={handleCopyModeKey} tabIndex={0}>
          {content}
          <div className={styles.copyModeStatus}>
            <span className={styles.cmStatusIcon}>📋 COPY MODE</span>
            <span className={styles.cmStatusInfo}>
              {copyModeSelectionStart !== null ? t("Terminal.k50", {
              arg0: copyModeSelectionMode === 'line' ? 'LINE' : 'CHAR'
            }) : t("Terminal.k51")}
            </span>
            <span className={styles.cmStatusKeys}>
              {t("Terminal.k52")}
            </span>
            <span className={styles.cmStatusPos}>
              {t("components.CodePreview.k1")} {copyModeCursorRow + 1}/{commandHistory.length}
              {copyModeSelectionStart !== null && <span> {t("Terminal.k53")} {Math.abs(copyModeCursorRow - copyModeSelectionStart) + 1} {t("components.CodePreview.k1")}</span>}
            </span>
          </div>
        </div>;
    }
    return content;
  };
  const renderLauncherOverlay = () => {
    if (!launcherVisible) return null;
    const grouped = new Map<string, LauncherEntry[]>();
    for (const entry of filteredLauncherEntries) {
      const g = grouped.get(entry.category) || [];
      g.push(entry);
      grouped.set(entry.category, g);
    }
    let globalIdx = 0;
    return <div className={styles.launcherOverlay} onClick={closeLauncher}>
        <div className={styles.launcherPanel} onClick={e => e.stopPropagation()}>
          <div className={styles.launcherHeader}>
            <span className={styles.launcherIcon}>⚡</span>
            <input ref={launcherInputRef} type="text" value={launcherSearch} onChange={e => setLauncherSearch(e.target.value)} onKeyDown={handleLauncherKey} className={styles.launcherInput} placeholder={t("Terminal.k54")} spellCheck={false} />
            <span className={styles.launcherHint}>{t("Terminal.k55")}</span>
          </div>
          <div className={styles.launcherList} ref={launcherListRef}>
            {filteredLauncherEntries.length === 0 ? <div className={styles.launcherEmpty}>{t("Terminal.k56")}</div> : Array.from(grouped.entries()).map(([category, entries]) => <div key={category} className={styles.launcherGroup}>
                  <div className={styles.launcherGroupTitle}>{category}</div>
                  {entries.map(entry => {
              const idx = globalIdx++;
              return <div key={entry.command} className={`${styles.launcherItem} ${idx === launcherIndex ? styles.launcherItemActive : ''}`} onMouseEnter={() => setLauncherIndex(idx)} onClick={() => {
                setCurrentCommand(entry.command);
                closeLauncher();
              }}>
                        <span className={styles.launcherCmd}>{entry.command}</span>
                        <span className={styles.launcherDesc}>{entry.description}</span>
                      </div>;
            })}
                </div>)}
          </div>
        </div>
      </div>;
  };
  const renderQuickSelectOverlay = () => {
    if (!quickSelectVisible) return null;
    const prefixMatches = quickSelectMatches.filter(m => m.label.startsWith(quickSelectInput));
    const exactMatch = quickSelectMatches.find(m => m.label === quickSelectInput);
    return <div className={styles.launcherOverlay} onClick={closeQuickSelect}>
        <div className={styles.launcherPanel} onClick={e => e.stopPropagation()}>
          <div className={styles.launcherHeader}>
            <span className={styles.launcherIcon}>🎯</span>
            <input ref={qsInputRef} type="text" value={quickSelectInput} readOnly onKeyDown={handleQuickSelectKey} className={styles.launcherInput} placeholder={quickSelectInput ? t("Terminal.k57", {
            quickSelectInput: quickSelectInput
          }) : t("Terminal.k58")} autoFocus spellCheck={false} />
            <span className={styles.launcherHint}>
              {quickSelectMatches.length} {t("Terminal.k59")}
            </span>
          </div>
          <div className={styles.launcherList}>
            {exactMatch ? <div className={styles.qsExactMatch}>
                <span className={styles.qsExactLabel}>{exactMatch.label}</span>
                <span className={styles.qsExactText}>{t("Terminal.k60")} {exactMatch.text}</span>
              </div> : <div className={styles.qsGroupedList}>
                {(() => {
              const grouped = new Map<string, QuickSelectMatch[]>();
              for (const m of prefixMatches) {
                const g = grouped.get(m.patternName) || [];
                g.push(m);
                grouped.set(m.patternName, g);
              }
              return Array.from(grouped.entries()).map(([patternName, entries]) => <div key={patternName} className={styles.launcherGroup}>
                      <div className={styles.launcherGroupTitle}>{patternName} ({entries.length})</div>
                      {entries.map(m => <div key={`${m.lineIndex}:${m.startIndex}`} className={styles.qsMatchItem}>
                          <span className={styles.qsMatchLabel}>{m.label}</span>
                          <span className={styles.qsMatchText} title={m.text}>
                            {m.text.length > 80 ? m.text.slice(0, 80) + '...' : m.text}
                          </span>
                        </div>)}
                    </div>);
            })()}
              </div>}
          </div>
        </div>
      </div>;
  };
  return <div className={styles.terminalContainer} onContextMenu={handleContextMenu}>
      {renderTabBar()}
      {renderTerminalContent()}
      {renderLauncherOverlay()}
      {renderQuickSelectOverlay()}
      {contextMenu && <div ref={contextMenuRef} className={styles.contextMenu} style={{
      left: contextMenu.x,
      top: contextMenu.y
    }}>
          <button className={styles.contextMenuItem} onClick={() => {
        handleSplitPane('vertical');
        setContextMenu(null);
      }}>
            {t("Terminal.k61")}
          </button>
          <button className={styles.contextMenuItem} onClick={() => {
        handleSplitPane('horizontal');
        setContextMenu(null);
      }}>
            {t("Terminal.k62")}
          </button>
        </div>}
      {/* Task 4.2: 命令搜索面板 */}
      {cmdSearchVisible && <div className={styles.commandSearchOverlay} onClick={() => setCmdSearchVisible(false)}>
          <div className={styles.commandSearchPanel} onClick={e => e.stopPropagation()}>
            <div className={styles.commandSearchHeader}>
              <span className={styles.commandSearchIcon}>⌘</span>
              <input ref={cmdSearchInputRef} type="text" value={cmdSearchQuery} onChange={e => {
            setCmdSearchQuery(e.target.value);
            setCmdSearchIndex(0);
          }} onKeyDown={handleCmdSearchKey} className={styles.commandSearchInput} placeholder={t("Terminal.k63")} spellCheck={false} />
              <span className={styles.commandSearchHint}>{t("Terminal.k64")}</span>
            </div>
            <div className={styles.commandSearchList}>
              {filteredCommands.length === 0 ? <div className={styles.commandSearchEmpty}>{t("Terminal.k56")}</div> : filteredCommands.map((cmd, idx) => <div key={`${cmd.command}-${idx}`} className={`${styles.commandSearchItem} ${idx === cmdSearchIndex ? styles.commandSearchItemActive : ''}`} onMouseEnter={() => setCmdSearchIndex(idx)} onClick={() => {
            setCurrentCommand(cmd.command);
            setCmdSearchVisible(false);
            setCmdSearchQuery('');
            setTimeout(() => inputRef.current?.focus(), 0);
          }}>
                    <span className={styles.commandSearchItemCmd}>{cmd.command}</span>
                    <span className={styles.commandSearchItemTab}>{cmd.tabName}</span>
                  </div>)}
            </div>
          </div>
        </div>}
      {/* Task 4.5: 错误右键菜单 */}
      {errorContextMenu && <div ref={errorContextMenuRef} className={styles.errorContextMenu} style={{
      left: errorContextMenu.x,
      top: errorContextMenu.y
    }}>
          <button className={styles.errorContextMenuItem} onClick={() => handleAiExplainError(errorContextMenu.line, errorContextMenu.x, errorContextMenu.y)}>
            {t("Terminal.k65")}
          </button>
        </div>}
      {/* Task 4.5: AI 错误解释弹窗 */}
      {aiErrorPopup && <div ref={aiErrorPopupRef} className={styles.aiErrorPopup} style={{
      left: aiErrorPopup.x + 10,
      top: aiErrorPopup.y + 10
    }}>
          <div className={styles.aiErrorPopupHeader}>
            <span>{t("Terminal.k66")}</span>
            <button className={styles.aiErrorPopupClose} onClick={() => setAiErrorPopup(null)}>✕</button>
          </div>
          <div className={styles.aiErrorPopupBody}>
            {aiErrorPopup.loading ? <div className={styles.aiErrorPopupLoading}>{t("Terminal.k67")}</div> : aiErrorPopup.explanation}
          </div>
        </div>}
    </div>;
}