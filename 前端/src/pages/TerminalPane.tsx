import { t } from "i18next";
import { useState, useRef, useEffect, useCallback } from 'react';
import { ipc } from '@/lib/ipc';
import { type TerminalLine, type TerminalMode } from '@/stores/terminalStore';
import styles from './Terminal.module.css';
const ALL_COMMANDS = ['help', 'man', 'ls', 'pwd', 'cd', 'mkdir', 'rmdir', 'cat', 'echo', 'date', 'whoami', 'clear', 'version', 'env', 'sysinfo', 'which', 'history', 'tree', 'cmd', 'powershell', 'linux', 'wsl', 'exit', 'grep', 'find', 'wc', 'head', 'tail', 'cp', 'mv', 'rm', 'touch', 'clearscrollback', 'reset', 'fontsize', 'fullscreen', 'reload', 'scroll', 'search', 'hide', 'quit', 'alwaysontop', 'll', 'la', 'cls', 'dir', 'type', 'copy', 'move', 'del', 'erase', 'ren', 'md', 'rd', 'minimize', 'top', 'yuan', 'chmod', 'file', 'stat', 'du', 'sed', 'sort', 'uniq', 'cut', 'diff', 'tr', 'awk', 'tee', 'xargs', 'printf', 'ps', 'kill', 'killall', 'bg', 'fg', 'jobs', 'nice', 'renice', 'nohup', 'uname', 'hostname', 'uptime', 'free', 'df', 'lscpu', 'lsblk', 'lspci', 'lsusb', 'ping', 'curl', 'wget', 'ifconfig', 'ip', 'ss', 'netstat', 'nslookup', 'dig', 'traceroute', 'nc', 'apt', 'apt-get', 'dpkg', 'sudo', 'su', 'passwd', 'useradd', 'groups', 'nmap', 'tcpdump', 'tar', 'gzip', 'gunzip', 'zip', 'unzip', 'systemctl', 'service', 'journalctl', 'locate', 'updatedb', 'whereis', 'alias', 'unalias', 'export', 'source', 'type', 'cal', 'bc', 'sleep', 'watch', 'seq', 'base64', 'yes', 'neofetch', 'crontab', 'docker', 'mount', 'umount', 'shutdown', 'reboot', 'hostnamectl'];
interface TerminalPaneProps {
  paneId: string;
  commandHistory: TerminalLine[];
  userCommands: string[];
  currentCommand: string;
  terminalMode: TerminalMode;
  onHistoryChange: (history: TerminalLine[]) => void;
  onUserCommandAdd: (cmd: string) => void;
  onCurrentCommandChange: (cmd: string) => void;
  onModeChange: (mode: TerminalMode) => void;
  onClearHistory: () => void;
  isActive: boolean;
  onFocus: () => void;
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
export default function TerminalPane({
  paneId,
  commandHistory,
  userCommands,
  currentCommand,
  terminalMode,
  onHistoryChange,
  onUserCommandAdd,
  onCurrentCommandChange,
  onModeChange,
  onClearHistory,
  isActive,
  onFocus
}: TerminalPaneProps) {
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [isExecuting, setIsExecuting] = useState(false);
  const [completions, setCompletions] = useState<string[]>([]);
  const [showCompletions, setShowCompletions] = useState(false);
  const [completionIndex, setCompletionIndex] = useState(0);
  const terminalRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [tabGhostText, setTabGhostText] = useState('');
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [commandHistory]);
  useEffect(() => {
    if (inputRef.current && isActive) {
      inputRef.current.focus();
    }
  }, [isActive, paneId]);
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
  const handleYuanOutput = useCallback((output: string, history: TerminalLine[]) => {
    if (output === '__YUAN_INTERACTIVE__' || output.startsWith('__YUAN_INTERACTIVE__')) {
      history.push({
        type: 'success',
        content: t("TerminalPane.k4")
      });
      window.location.href = '/terminal/yuancode';
    } else if (output.startsWith('__YUAN_OPEN__')) {
      const path = output.replace(/^__YUAN_OPEN__:?/, '');
      history.push({
        type: 'success',
        content: t("TerminalPane.k5", {
          arg0: path || t("TerminalPane.k6")
        })
      });
      window.location.href = path ? `/terminal/yuancode?open=${encodeURIComponent(path)}` : '/terminal/yuancode';
    } else if (output.startsWith('__YUAN_RUN__')) {
      const lang = output.replace(/^__YUAN_RUN__:?/, '');
      history.push({
        type: 'success',
        content: t("TerminalPane.k7", {
          arg0: lang || 'python'
        })
      });
      window.location.href = `/terminal/yuancode?run=${encodeURIComponent(lang || 'python')}`;
    } else if (output.startsWith('__YUAN_AI__')) {
      const prompt = output.replace(/^__YUAN_AI__:?/, '');
      history.push({
        type: 'success',
        content: t("TerminalPane.k8", {
          arg0: prompt.slice(0, 50),
          arg1: prompt.length > 50 ? '...' : ''
        })
      });
      window.location.href = `/terminal/yuancode?ai=${encodeURIComponent(prompt)}`;
    } else if (output.startsWith('__YUAN_AGENT__')) {
      const goal = output.replace(/^__YUAN_AGENT__:?/, '');
      history.push({
        type: 'success',
        content: t("TerminalPane.k9", {
          arg0: goal.slice(0, 60)
        })
      });
      window.location.href = `/terminal/yuancode?agent=${encodeURIComponent(goal)}`;
    } else if (output.startsWith('__YUAN_SHELL__')) {
      const cmd = output.replace(/^__YUAN_SHELL__:?/, '');
      history.push({
        type: 'success',
        content: t("TerminalPane.k10", {
          arg0: cmd.slice(0, 60)
        })
      });
      window.location.href = `/terminal/yuancode?shell=${encodeURIComponent(cmd)}`;
    } else if (output.startsWith('__YUAN_GIT__')) {
      const op = output.replace(/^__YUAN_GIT__:?/, '');
      history.push({
        type: 'success',
        content: t("TerminalPane.k11", {
          op: op
        })
      });
      window.location.href = `/terminal/yuancode?git=${encodeURIComponent(op)}`;
    } else if (output.startsWith('__YUAN_WEB__')) {
      const query = output.replace(/^__YUAN_WEB__:?/, '');
      history.push({
        type: 'success',
        content: t("TerminalPane.k12", {
          arg0: query.slice(0, 50)
        })
      });
      window.location.href = `/terminal/yuancode?web=${encodeURIComponent(query)}`;
    } else if (output === '__YUAN_SKILL_LIST__') {
      history.push({
        type: 'success',
        content: t("TerminalPane.k13")
      });
      window.location.href = '/terminal/yuancode?skills=true';
    } else if (output.startsWith('__YUAN_SKILL__')) {
      const name = output.replace(/^__YUAN_SKILL__:?/, '');
      history.push({
        type: 'success',
        content: t("TerminalPane.k14", {
          name: name
        })
      });
      window.location.href = `/terminal/yuancode?skill=${encodeURIComponent(name)}`;
    } else {
      history.push({
        type: 'output',
        content: output
      });
    }
    onHistoryChange(history);
    onCurrentCommandChange('');
    setIsExecuting(false);
  }, [onHistoryChange, onCurrentCommandChange]);
  const handleLinuxOutput = useCallback((output: string, history: TerminalLine[]) => {
    if (output === '__LINUX_INTERACTIVE__' || output.startsWith('__LINUX_INTERACTIVE__')) {
      history.push({
        type: 'success',
        content: t("TerminalPane.k15")
      });
      window.location.href = '/terminal/linux';
    } else if (output === '__LINUX_VERSIONS__') {
      history.push({
        type: 'success',
        content: t("TerminalPane.k16")
      });
      window.location.href = '/terminal/linux?tab=versions';
    } else if (output.startsWith('__LINUX_DOWNLOAD__')) {
      const version = output.replace(/^__LINUX_DOWNLOAD__:?/, '');
      history.push({
        type: 'success',
        content: t("TerminalPane.k17", {
          version: version
        })
      });
      window.location.href = `/terminal/linux?download=${encodeURIComponent(version)}`;
    } else if (output === '__LINUX_LS__') {
      history.push({
        type: 'success',
        content: t("TerminalPane.k18")
      });
      window.location.href = '/terminal/linux?tab=explorer';
    } else if (output.startsWith('__LINUX_LS__:')) {
      const path = output.replace(/^__LINUX_LS__:?/, '');
      history.push({
        type: 'success',
        content: t("TerminalPane.k19", {
          path: path
        })
      });
      window.location.href = `/terminal/linux?tab=explorer&path=${encodeURIComponent(path)}`;
    } else if (output.startsWith('__LINUX_SEARCH__')) {
      const query = output.replace(/^__LINUX_SEARCH__:?/, '');
      history.push({
        type: 'success',
        content: t("TerminalPane.k20", {
          arg0: query.slice(0, 50)
        })
      });
      window.location.href = `/terminal/linux?search=${encodeURIComponent(query)}`;
    } else if (output === '__LINUX_INFO__') {
      history.push({
        type: 'success',
        content: t("TerminalPane.k21")
      });
      window.location.href = '/terminal/linux?tab=info';
    } else {
      history.push({
        type: 'output',
        content: output
      });
    }
    onHistoryChange(history);
    onCurrentCommandChange('');
    setIsExecuting(false);
  }, [onHistoryChange, onCurrentCommandChange]);
  const executeBuiltinCommand = useCallback(async (command: string) => {
    const newHistory = [...commandHistory];
    const cmd = command.trim().toLowerCase();
    const cmdName = cmd.split(' ')[0];
    newHistory.push({
      type: 'command',
      content: `${getPrompt()}${command}`
    });
    if (cmdName === 'clear') {
      onClearHistory();
      onCurrentCommandChange('');
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
        content: t("Terminal.k5")
      });
      newHistory.push({
        type: 'system',
        content: ''
      });
      onModeChange('cmd');
      onHistoryChange(newHistory);
      onCurrentCommandChange('');
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
        content: t("Terminal.k5")
      });
      newHistory.push({
        type: 'system',
        content: ''
      });
      onModeChange('powershell');
      onHistoryChange(newHistory);
      onCurrentCommandChange('');
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
        onModeChange('builtin');
      } else {
        newHistory.push({
          type: 'error',
          content: t("Terminal.k9")
        });
      }
      onHistoryChange(newHistory);
      onCurrentCommandChange('');
      setHistoryIndex(-1);
      return;
    }
    setIsExecuting(true);
    try {
      const result = await ipc.invoke<{
        output: string;
        exit_code: number;
      }>('terminal_execute_builtin', {
        command
      });
      if (result.code === 0 && result.data) {
        const output = result.data.output;
        if (output === '__CLEAR_SCROLLBACK__') {
          onClearHistory();
          onCurrentCommandChange('');
          setIsExecuting(false);
          return;
        }
        if (output === '__FONTSIZE_INCREASE__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k10")
          });
        } else if (output === '__FONTSIZE_DECREASE__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k11")
          });
        } else if (output === '__FONTSIZE_RESET__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k12")
          });
        } else if (output === '__FULLSCREEN_TOGGLE__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k13")
          });
        } else if (output === '__RELOAD_CONFIG__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k14")
          });
        } else if (output === '__SCROLL_TOP__') {
          if (terminalRef.current) terminalRef.current.scrollTop = 0;
          newHistory.push({
            type: 'success',
            content: t("Terminal.k15")
          });
        } else if (output === '__SCROLL_BOTTOM__') {
          if (terminalRef.current) terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
          newHistory.push({
            type: 'success',
            content: t("Terminal.k16")
          });
        } else if (output === '__WINDOW_HIDE__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k19")
          });
        } else if (output === '__APP_QUIT__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k20")
          });
        } else if (output === '__ALWAYS_ON_TOP__') {
          newHistory.push({
            type: 'success',
            content: t("Terminal.k21")
          });
        } else if (output.startsWith('__YUAN_')) {
          handleYuanOutput(output, newHistory);
          return;
        } else if (output.startsWith('__LINUX_')) {
          handleLinuxOutput(output, newHistory);
          return;
        } else if (output.startsWith('__SCROLL_UP_')) {
          const match = output.match(/__SCROLL_UP_(\d+)__/);
          const lines = match ? parseInt(match[1], 10) : 1;
          if (terminalRef.current) terminalRef.current.scrollTop -= lines * 18;
          newHistory.push({
            type: 'success',
            content: t("Terminal.k17", {
              lines: lines
            })
          });
        } else if (output.startsWith('__SCROLL_DOWN_')) {
          const match = output.match(/__SCROLL_DOWN_(\d+)__/);
          const lines = match ? parseInt(match[1], 10) : 1;
          if (terminalRef.current) terminalRef.current.scrollTop += lines * 18;
          newHistory.push({
            type: 'success',
            content: t("Terminal.k18", {
              lines: lines
            })
          });
        } else {
          addOutputLines(newHistory, output, result.data.exit_code);
        }
      } else {
        newHistory.push({
          type: 'error',
          content: t("Terminal.k22", {
            message: result.message
          })
        });
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
    } finally {
      setIsExecuting(false);
    }
    onHistoryChange(newHistory);
    onCurrentCommandChange('');
    setHistoryIndex(-1);
  }, [commandHistory, terminalMode, onHistoryChange, onCurrentCommandChange, onModeChange, onClearHistory, getPrompt, addOutputLines, handleYuanOutput, handleLinuxOutput]);
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
      onModeChange('builtin');
      onHistoryChange(newHistory);
      onCurrentCommandChange('');
      return;
    }
    newHistory.push({
      type: 'system',
      content: t("Terminal.k26", {
        modeLabel: modeLabel,
        command: command
      })
    });
    onHistoryChange(newHistory);
    onCurrentCommandChange('');
  }, [commandHistory, terminalMode, onHistoryChange, onCurrentCommandChange, onModeChange]);
  const executeCommand = useCallback((command: string) => {
    if (command.trim() === '' || isExecuting) return;
    onUserCommandAdd(command);
    setShowCompletions(false);
    if (terminalMode === 'builtin') {
      executeBuiltinCommand(command);
    } else {
      executeSystemCommand(command);
    }
  }, [terminalMode, isExecuting, executeBuiltinCommand, executeSystemCommand, onUserCommandAdd]);
  const fetchCompletions = useCallback(async (input: string) => {
    if (input.trim().length < 1) {
      setShowCompletions(false);
      return;
    }
    const localMatch = ALL_COMMANDS.filter(cmd => cmd.startsWith(input.toLowerCase()));
    if (localMatch.length > 0) {
      setCompletions(localMatch);
      setCompletionIndex(0);
      setShowCompletions(true);
      return;
    }
    try {
      const result = await ipc.invoke<{
        completions: string[];
        prefix: string;
      }>('term_v2_complete', {
        input
      });
      if (result.code === 0 && result.data && result.data.completions.length > 0) {
        setCompletions(result.data.completions);
        setCompletionIndex(0);
        setShowCompletions(true);
      }
    } catch {
      setShowCompletions(false);
    }
  }, []);
  const handleKeyPress = useCallback((e: React.KeyboardEvent) => {
    if (showCompletions) {
      if (e.key === 'Escape') {
        e.preventDefault();
        setShowCompletions(false);
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setCompletionIndex(prev => (prev + 1) % completions.length);
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setCompletionIndex(prev => (prev - 1 + completions.length) % completions.length);
        return;
      }
      if (e.key === 'Tab' || e.key === 'Enter') {
        e.preventDefault();
        onCurrentCommandChange(completions[completionIndex]);
        setShowCompletions(false);
        return;
      }
    }
    if (e.key === 'Enter') {
      if (currentCommand.trim() === '') {
        const newHistory = [...commandHistory];
        newHistory.push({
          type: 'command',
          content: getPrompt()
        });
        onHistoryChange(newHistory);
      } else {
        executeCommand(currentCommand);
      }
    } else if (e.key === 'ArrowUp' && !showCompletions) {
      e.preventDefault();
      if (historyIndex < userCommands.length - 1) {
        const newIndex = historyIndex + 1;
        setHistoryIndex(newIndex);
        onCurrentCommandChange(userCommands[newIndex]);
      }
    } else if (e.key === 'ArrowDown' && !showCompletions) {
      e.preventDefault();
      if (historyIndex > 0) {
        const newIndex = historyIndex - 1;
        setHistoryIndex(newIndex);
        onCurrentCommandChange(userCommands[newIndex]);
      } else if (historyIndex === 0) {
        setHistoryIndex(-1);
        onCurrentCommandChange('');
      }
    } else if (e.key === 'Tab' && !showCompletions) {
      e.preventDefault();
      // Tab 只接受 ghost text 补全
      if (tabGhostText) {
        onCurrentCommandChange(currentCommand + tabGhostText);
        setTabGhostText('');
      } else if (completions.length > 0) {
        onCurrentCommandChange(completions[0]);
        setShowCompletions(false);
      } else {
        const match = ALL_COMMANDS.find(cmd => cmd.startsWith(currentCommand.toLowerCase()));
        if (match) onCurrentCommandChange(match);
      }
    } else if (e.key === 'Escape') {
      setTabGhostText('');
    } else if (e.key === 'l' && e.ctrlKey) {
      e.preventDefault();
      onClearHistory();
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
        onModeChange('builtin');
        onHistoryChange(newHistory);
      } else {
        const newHistory = [...commandHistory];
        newHistory.push({
          type: 'system',
          content: '^C'
        });
        onHistoryChange(newHistory);
      }
    } else if (e.key === 'a' && e.ctrlKey) {
      e.preventDefault();
      inputRef.current?.select();
    } else if (e.key === 'u' && e.ctrlKey) {
      e.preventDefault();
      onCurrentCommandChange('');
    }
  }, [currentCommand, commandHistory, historyIndex, userCommands, terminalMode, showCompletions, completions, completionIndex, executeCommand, getPrompt, onHistoryChange, onCurrentCommandChange, onModeChange, onClearHistory, fetchCompletions]);
  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    onCurrentCommandChange(val);
    setTabGhostText('');
    if (val.trim().length > 0) {
      fetchCompletions(val);
    } else {
      setShowCompletions(false);
    }
  }, [onCurrentCommandChange, fetchCompletions]);
  return <div className={styles.paneContainer} onClick={() => {
    onFocus();
    inputRef.current?.focus();
  }}>
      <div className={styles.terminalWindow} ref={terminalRef}>
        {commandHistory.map((line, index) => <div key={index} className={styles.terminalLine}>
            <span className={styles[line.type]}>{line.content}</span>
          </div>)}
        <div className={styles.inputLine}>
          <span className={styles.inputPrompt}>{getPrompt()}</span>
          <div className={styles.inputWrapper}>
            <input ref={inputRef} type="text" value={currentCommand} onChange={handleInputChange} onKeyDown={handleKeyPress} className={styles.inputField} placeholder={isExecuting ? t("Terminal.k42") : ''} autoFocus={isActive} spellCheck={false} disabled={isExecuting} />
            {showCompletions && completions.length > 0 && <div className={styles.completionPopup}>
                {completions.slice(0, 10).map((comp, i) => <div key={comp} className={`${styles.completionItem} ${i === completionIndex ? styles.completionItemActive : ''}`} onMouseDown={() => {
              onCurrentCommandChange(comp);
              setShowCompletions(false);
            }}>
                    <span className={styles.completionIcon}>{comp.startsWith(currentCommand) ? '>' : ' '}</span>
                    {comp}
                  </div>)}
                {completions.length > 10 && <div className={styles.completionMore}>{t("TerminalPane.k22")} {completions.length - 10} {t("Knowledge.k179")}</div>}
              </div>}
          </div>
        </div>
      </div>
      <div className={styles.paneStatusBar}>
        <span className={styles.paneStatusLeft}>
          <span className={styles.paneModeBadge}>{terminalMode.toUpperCase()}</span>
          <span>{paneId}</span>
        </span>
        <span className={styles.paneStatusRight}>
          {commandHistory.length} lines
        </span>
      </div>
    </div>;
}
export { INITIAL_HISTORY };