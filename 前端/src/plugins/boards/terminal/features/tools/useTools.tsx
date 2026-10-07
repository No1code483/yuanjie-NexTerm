// terminal.tools L2 功能域（终端本体「一切皆插件」拆分）。
// 承载命令辅助工具：命令 Launcher（Ctrl+Shift+P）、QuickSelect（Ctrl+Shift+Space）、
// Copy Mode（Ctrl+Shift+X）、命令搜索面板（Ctrl+Shift+R）、终端内文本搜索（Ctrl+Shift+F）、右键菜单。
// 局部 state + handlers + JSX 物理落在本目录；跨域共享的会话数据经 L1 Terminal 壳经 props 注入。
// 本插件为可选子插件：启用状态经 L1 门控（enabled=false 时全部入口隐藏、快捷键不拦截）。
import { t } from "i18next";
import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import { copy } from '@/lib/utils';
import type { TerminalLine, TabData } from '../../stores/terminalStore';
import { LauncherEntry, QuickSelectMatch, QUICK_SELECT_PATTERNS, computeQuickSelectLabels, LAUNCHER_ENTRIES } from '../../terminal/types';
import styles from '../../Terminal.module.css';

interface ToolsDeps {
  tabs: TabData[];
  activeTabId: string;
  commandHistory: TerminalLine[];
  setCurrentCommand: (cmd: string) => void;
  inputRef: React.RefObject<HTMLInputElement | HTMLTextAreaElement | null>;
  terminalRef: React.RefObject<HTMLDivElement | null>;
  handleSplitPane: (direction: 'horizontal' | 'vertical') => void;
  enabled: boolean;
}

export function useTools(deps: ToolsDeps) {
  const { tabs, activeTabId, commandHistory, setCurrentCommand, inputRef, terminalRef, handleSplitPane, enabled } = deps;

  // Task 4.2: 命令搜索面板
  const [cmdSearchVisible, setCmdSearchVisible] = useState(false);
  const [cmdSearchQuery, setCmdSearchQuery] = useState('');
  const [cmdSearchIndex, setCmdSearchIndex] = useState(0);
  const cmdSearchInputRef = useRef<HTMLInputElement>(null);

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

  const [searchVisible, setSearchVisible] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchMatches, setSearchMatches] = useState<{
    lineIndex: number;
    matchIndex: number;
    text: string;
  }[]>([]);
  const [searchActiveIndex, setSearchActiveIndex] = useState(0);
  const searchInputRef = useRef<HTMLInputElement>(null);

  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);

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
    const handleClickOutside = (e: MouseEvent) => {
      if (contextMenuRef.current && !contextMenuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    if (!enabled) return;
    const activeTab = tabs.find((t: TabData) => t.id === activeTabId);
    if (!activeTab || activeTab.splitDirection !== 'none') return;
    setContextMenu({
      x: e.clientX,
      y: e.clientY
    });
  }, [activeTabId, tabs, enabled]);

  const filteredLauncherEntries = (() => {
    if (!launcherSearch.trim()) return LAUNCHER_ENTRIES;
    const q = launcherSearch.toLowerCase();
    return LAUNCHER_ENTRIES.filter(e => e.command.toLowerCase().includes(q) || e.description.toLowerCase().includes(q));
  })();

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
  }, [filteredCommands, cmdSearchIndex, setCurrentCommand, inputRef]);

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
  }, [inputRef]);
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
  }, [filteredLauncherEntries, launcherIndex, closeLauncher, setCurrentCommand, inputRef]);
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
  }, [inputRef]);
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
  }, [inputRef]);
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
  const scrollToMatchLine = useCallback((lineIndex: number) => {
    if (!terminalRef.current) return;
    const lineEl = terminalRef.current.children[lineIndex] as HTMLElement | undefined;
    if (lineEl) {
      lineEl.scrollIntoView({
        block: 'center',
        behavior: 'smooth'
      });
    }
  }, [terminalRef]);
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
  }, [inputRef]);
  const updateSearch = useCallback((query: string) => {
    setSearchQuery(query);
    const matches = scanSearchMatches(query, commandHistory);
    setSearchMatches(matches);
    setSearchActiveIndex(matches.length > 0 ? 0 : -1);
    if (matches.length > 0) {
      scrollToMatchLine(matches[0].lineIndex);
    }
  }, [commandHistory, scanSearchMatches, scrollToMatchLine]);
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

  const renderLauncherOverlay = () => {
    if (!enabled || !launcherVisible) return null;
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
    if (!enabled || !quickSelectVisible) return null;
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

  const renderContextMenu = () => {
    if (!enabled || !contextMenu) return null;
    return <div ref={contextMenuRef} className={styles.contextMenu} style={{
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
      </div>;
  };

  /** Task 4.2: 命令搜索面板 */
  const renderCommandSearch = () => {
    if (!enabled || !cmdSearchVisible) return null;
    return <div className={styles.commandSearchOverlay} onClick={() => setCmdSearchVisible(false)}>
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
      </div>;
  };

  const renderSearchBar = () => {
    if (!enabled || !searchVisible) return null;
    return <div className={styles.searchBarContainer}>
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
      </div>;
  };

  const wrapCopyMode = (content: React.ReactNode) => {
    if (!enabled || !copyModeVisible) return content;
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
  };

  // 键盘总控中 tools 域：Launcher / QuickSelect / Copy Mode / 文本搜索 / 命令搜索
  const handleKey = useCallback((e: React.KeyboardEvent): boolean => {
    if (!enabled) return false;
    if ((e.key === 'p' || e.key === 'P') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      openLauncher();
      return true;
    }
    if (e.key === ' ' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      openQuickSelect();
      return true;
    }
    if ((e.key === 'x' || e.key === 'X') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      enterCopyMode();
      return true;
    }
    if ((e.key === 'f' || e.key === 'F') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      openSearch();
      return true;
    }
    if ((e.key === 'r' || e.key === 'R') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      setCmdSearchVisible(true);
      setCmdSearchQuery('');
      setCmdSearchIndex(0);
      return true;
    }
    return false;
  }, [enabled, openLauncher, openQuickSelect, enterCopyMode, openSearch]);

  const searchActive = searchVisible && searchQuery.trim().length > 0;

  return {
    renderLauncherOverlay,
    renderQuickSelectOverlay,
    renderContextMenu,
    renderCommandSearch,
    renderSearchBar,
    wrapCopyMode,
    handleKey,
    handleContextMenu,
    getCopyModeLineClass,
    renderLineWithHighlight,
    searchActive,
    copyModeVisible,
  };
}
