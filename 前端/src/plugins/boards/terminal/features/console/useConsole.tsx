// terminal.console L2 功能域（终端本体「一切皆插件」拆分）。
// 承载终端命令行：命令执行引擎（内建/系统命令）、Block 输出块渲染与折叠复制、
// 命令历史回放/Tab 补全/ghost text、AI 命令建议、`#` 自然语言触发、错误行右键 AI 解释。
// 局部 state + handlers + JSX 物理落在本目录；跨域共享的会话数据经 L1 Terminal 壳经 props 注入。
// 本插件为必备子插件（随父停用，不单独门控）。
import { t } from "i18next";
import { useState, useRef, useEffect, useCallback } from 'react';
import { terminal } from '../../ipc';
import { intelligence, ipc } from '@/lib/ipc';
import { systemtools } from '@/plugins/customs/systemtools';
import { copy } from '@/lib/utils';
import { useIntelligence } from '@/hooks/useIntelligence';
import type { TerminalLine, TerminalBlock, TabData, TerminalMode } from '../../stores/terminalStore';
import NexTermTerminal from '../../NexTermTerminal';
import { splitByLinks, renderAnsiText } from '../../terminal/types';
import styles from '../../Terminal.module.css';

const ALL_COMMANDS = ['help', 'ls', 'pwd', 'cd', 'mkdir', 'cat', 'echo', 'date', 'whoami', 'clear', 'version', 'env', 'sysinfo', 'history', 'tree', 'cmd', 'powershell', 'exit', 'grep', 'find', 'wc', 'head', 'tail', 'cp', 'mv', 'rm', 'touch', 'clearscrollback', 'reset', 'fontsize', 'fullscreen', 'reload', 'scroll', 'search', 'hide', 'quit', 'alwaysontop', 'll', 'la', 'cls', 'dir', 'type', 'copy', 'move', 'del', 'erase', 'ren', 'md', 'rd', 'minimize', 'top'];

interface ConsoleDeps {
  activeTabId: string;
  commandHistory: TerminalLine[];
  userCommands: string[];
  terminalMode: TerminalMode;
  blocks: TerminalBlock[];
  currentCommand: string;
  setCommandHistory: (history: TerminalLine[]) => void;
  setCurrentCommand: (cmd: string) => void;
  setTerminalMode: (mode: TerminalMode) => void;
  clearHistory: () => void;
  prependUserCommand: (command: string) => void;
  addBlock: (block: TerminalBlock) => void;
  setBlocks: (blocks: TerminalBlock[]) => void;
  toggleBlockCollapse: (blockId: string) => void;
  updateTabSessionId: (tabId: string, sessionId: string) => void;
  isLinuxPage: boolean;
  terminalRef: React.RefObject<HTMLDivElement | null>;
  inputRef: React.RefObject<HTMLInputElement | HTMLTextAreaElement | null>;
  getCopyModeLineClass: (index: number) => string;
  renderLineWithHighlight: (content: string, lineIndex: number) => React.ReactNode;
  searchActive: boolean;
}

export function useConsole(deps: ConsoleDeps) {
  const {
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
    getCopyModeLineClass,
    renderLineWithHighlight,
    searchActive,
  } = deps;

  const [historyIndex, setHistoryIndex] = useState(-1);
  const [isMultiLine, setIsMultiLine] = useState(false);
  const [isExecuting, setIsExecuting] = useState(false);
  const [aiSuggestedCmd, setAiSuggestedCmd] = useState('');
  const [aiSuggestLoading, setAiSuggestLoading] = useState(false);
  const suggestTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hashTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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

  const tabCompleteIndex = useRef(-1);
  const lastTabPrefix = useRef('');
  const [tabGhostText, setTabGhostText] = useState('');

  const { aiOn, featureOn } = useIntelligence();

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
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
  }, [commandHistory, terminalMode, addOutputLines, clearHistory, setCommandHistory, setCurrentCommand, setTerminalMode, addBlock, setBlocks, terminalRef]);
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

  const renderLineContent = (line: TerminalLine, index: number) => {
    if (searchActive) {
      return renderLineWithHighlight(line.content, index);
    }
    const parts = splitByLinks(line.content);
    if (parts.length <= 1) return renderAnsiText(line.content);
    return parts.map((p, i) => p.isLink ? <a key={i} href={p.text} target="_blank" rel="noopener noreferrer" className={styles.outputLink} onClick={e => {
      e.stopPropagation();
      systemtools.openUrl(p.text).catch(() => {
        window.open(p.text, '_blank', 'noopener,noreferrer');
      });
    }}>
            {p.text}
          </a> : <span key={i}>{renderAnsiText(p.text)}</span>);
  };

  const renderPtyView = (activeTab: TabData) => {
    return <div className={styles.ptyTerminalWrapper}>
        <NexTermTerminal sessionType={activeTab.type as 'cmd' | 'powershell' | 'wsl'} autoConnect={!activeTab.sessionId} externalSessionId={activeTab.sessionId} onSessionCreated={sid => {
        updateTabSessionId(activeTabId, sid);
      }} fontSize={14} />
      </div>;
  };

  // 将 blocks 按命令匹配到对应位置
  const renderLines = () => {
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

  const renderBuiltinWindow = (opts: {
    onKeyDown: (e: React.KeyboardEvent) => void;
    searchBar: React.ReactNode;
    copyModeActive: boolean;
  }) => {
    const { onKeyDown, searchBar, copyModeActive } = opts;
    return <div className={styles.terminalWindow} ref={terminalRef} onClick={() => !copyModeActive && inputRef.current?.focus()}>
        {renderLines()}
        {!copyModeActive && <div className={`${styles.inputLine} ${isMultiLine ? styles.inputLineMulti : ''}`}>
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
            }} onKeyDown={onKeyDown} className={styles.inputFieldMulti} placeholder={isExecuting ? t("Terminal.k42") : t("Terminal.k43")} autoFocus spellCheck={false} disabled={isExecuting} rows={Math.min(currentCommand.split('\n').length + 1, 12)} /> : <input ref={inputRef as React.RefObject<HTMLInputElement>} type="text" value={currentCommand} onChange={e => {
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
            }} onKeyDown={onKeyDown} className={styles.inputField} placeholder={isExecuting ? t("Terminal.k42") : t("Terminal.k44")} autoFocus spellCheck={false} disabled={isExecuting} />}
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
        {searchBar}
      </div>;
  };

  // Task 4.5: 错误右键菜单 + AI 错误解释弹窗
  const renderErrorOverlays = () => <>
      {errorContextMenu && <div ref={errorContextMenuRef} className={styles.errorContextMenu} style={{
      left: errorContextMenu.x,
      top: errorContextMenu.y
    }}>
          <button className={styles.errorContextMenuItem} onClick={() => handleAiExplainError(errorContextMenu.line, errorContextMenu.x, errorContextMenu.y)}>
            {t("Terminal.k65")}
          </button>
        </div>}
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
    </>;

  // 键盘总控中 console 域：命令输入 / 历史回放 / Tab 补全 / 清屏 / 复制粘贴 / 字号 / 全屏 / 滚动
  const handleKey = useCallback((e: React.KeyboardEvent): boolean => {
    if (e.key === 'Enter') {
      if (e.shiftKey) {
        e.preventDefault();
        setIsMultiLine(true);
        setCurrentCommand(currentCommand + '\n');
        return true;
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
      return true;
    } else if (e.key === 'Escape' && isMultiLine) {
      e.preventDefault();
      setIsMultiLine(false);
      setCurrentCommand('');
      return true;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (historyIndex < userCommands.length - 1) {
        const newIndex = historyIndex + 1;
        setHistoryIndex(newIndex);
        setCurrentCommand(userCommands[newIndex]);
      }
      return true;
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
      return true;
    } else if (e.key === 'Tab') {
      e.preventDefault();
      // Tab 只接受 ghost text 补全
      if (tabGhostText) {
        setCurrentCommand(currentCommand + tabGhostText);
        setTabGhostText('');
      } else {
        // 无 ghost text 时执行原来的 Tab 补全逻辑
        const prefix = currentCommand.toLowerCase().trim();
        if (!prefix) return true;
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
      return true;
    } else if (e.key === 'l' && e.ctrlKey) {
      e.preventDefault();
      clearHistory();
      setBlocks([]);
      return true;
    } else if (e.key === 'K' && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      clearHistory();
      setBlocks([]);
      return true;
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
      return true;
    } else if (e.key === 'a' && e.ctrlKey) {
      e.preventDefault();
      inputRef.current?.select();
      return true;
    } else if (e.key === 'Escape') {
      setTabGhostText('');
      return true;
    } else if (e.key === 'u' && e.ctrlKey) {
      e.preventDefault();
      setCurrentCommand('');
      setTabGhostText('');
      return true;
    } else if (e.key === '=' && e.ctrlKey) {
      e.preventDefault();
      executeCommand('fontsize +');
      return true;
    } else if (e.key === '-' && e.ctrlKey) {
      e.preventDefault();
      executeCommand('fontsize -');
      return true;
    } else if (e.key === '0' && e.ctrlKey) {
      e.preventDefault();
      executeCommand('fontsize 0');
      return true;
    } else if (e.key === 'Enter' && e.altKey) {
      e.preventDefault();
      executeCommand('fullscreen');
      return true;
    } else if (e.key === 'PageUp' && e.shiftKey) {
      e.preventDefault();
      executeCommand('scroll up 10');
      return true;
    } else if (e.key === 'PageDown' && e.shiftKey) {
      e.preventDefault();
      executeCommand('scroll down 10');
      return true;
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
      return true;
    } else if ((e.key === 'v' || e.key === 'V') && e.ctrlKey && e.shiftKey) {
      e.preventDefault();
      navigator.clipboard.readText().then(text => {
        setCurrentCommand(currentCommand + text);
      }).catch(() => {});
      return true;
    }
    return false;
  }, [currentCommand, commandHistory, historyIndex, userCommands, terminalMode, executeCommand, getPrompt, setCommandHistory, setCurrentCommand, setTerminalMode, clearHistory, setBlocks, inputRef, aiOn, featureOn]);

  return {
    renderPtyView,
    renderBuiltinWindow,
    renderErrorOverlays,
    handleKey,
  };
}
