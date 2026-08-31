import { t } from "i18next";
import React, { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import { LinuxShell } from '@/lib/linuxShell';
import styles from './Terminal.module.css';
const ANSI_COLORS: Record<number, string> = {
  30: '#000000',
  31: '#FF006E',
  32: '#00FF88',
  33: '#FFD700',
  34: '#00F0FF',
  35: '#B026FF',
  36: '#00F0FF',
  37: '#E0E0E0',
  90: '#888888',
  91: '#FF6B8A',
  92: '#80FFBB',
  93: '#FFE566',
  94: '#66D9FF',
  95: '#D08BFF',
  96: '#80FFFF',
  97: '#FFFFFF'
};
function renderLineToHtml(line: string): React.ReactNode {
  const parts: React.ReactNode[] = [];
  let currentColor: string | null = null;
  let buffer = '';
  let bold = false;
  let i = 0;
  while (i < line.length) {
    if (line[i] === '\x1b' && line[i + 1] === '[') {
      if (buffer) {
        parts.push(<span key={parts.length} style={bold ? {
          color: currentColor || '#E0E0E0',
          fontWeight: 700
        } : {
          color: currentColor || '#E0E0E0'
        }}>
            {buffer}
          </span>);
        buffer = '';
      }
      const end = line.indexOf('m', i);
      if (end === -1) {
        buffer += line[i];
        i++;
        continue;
      }
      const code = line.slice(i + 2, end);
      i = end + 1;
      if (code === '0') {
        bold = false;
        currentColor = null;
      } else if (code === '1') {
        bold = true;
      } else {
        const c = parseInt(code);
        if (ANSI_COLORS[c]) currentColor = ANSI_COLORS[c];
      }
      continue;
    }
    buffer += line[i];
    i++;
  }
  if (buffer) {
    parts.push(<span key={parts.length} style={bold ? {
      color: currentColor || '#E0E0E0',
      fontWeight: 700
    } : {
      color: currentColor || '#E0E0E0'
    }}>
        {buffer}
      </span>);
  }
  return <>{parts}</>;
}
export default function LinuxTerminal() {
  const shell = useMemo(() => new LinuxShell(), []);
  const [history, setHistory] = useState<{
    type: 'system' | 'command' | 'output' | 'error';
    content: string;
  }[]>(() => {
    const welcome = ['Welcome to NexTerm Linux Environment v1.0', 'Kernel: Linux 6.1.0-nexterm (x86_64)', '', 'Type "help" for a list of available commands.', ''];
    return welcome.map(line => ({
      type: 'system' as const,
      content: line
    }));
  });
  const [input, setInput] = useState('');
  const [histIdx, setHistIdx] = useState(-1);
  const terminalRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [cmds, setCmds] = useState<string[]>([]);
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [history]);
  useEffect(() => {
    inputRef.current?.focus();
  }, []);
  const execute = useCallback((cmd: string) => {
    const newHistory = [...history, {
      type: 'command' as const,
      content: `${shell.getPrompt()}${cmd}`
    }];
    if (cmd === 'clear') {
      setHistory([]);
      setInput('');
      setHistIdx(-1);
      return;
    }
    const output = shell.execute(cmd);
    if (output === '__CLEAR__') {
      setHistory([]);
      setInput('');
      setHistIdx(-1);
      return;
    }
    if (output) {
      const lines = output.split('\n');
      for (const line of lines) {
        newHistory.push({
          type: 'output' as const,
          content: line
        });
      }
    } else if (cmd && !['cd', 'touch', 'mkdir', 'mv', 'cp', 'chmod', 'hostname'].includes(cmd.split(' ')[0])) {
      newHistory.push({
        type: 'output' as const,
        content: ''
      });
    }
    setHistory(newHistory);
    setInput('');
    setHistIdx(-1);
  }, [history, shell]);
  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      const cmd = input.trim();
      if (cmd) setCmds(prev => [...prev.slice(-499), cmd]);
      execute(cmd);
      return;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (cmds.length === 0) return;
      const newIdx = histIdx === -1 ? cmds.length - 1 : Math.max(0, histIdx - 1);
      setHistIdx(newIdx);
      setInput(cmds[newIdx]);
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (histIdx === -1) return;
      const newIdx = histIdx + 1;
      if (newIdx >= cmds.length) {
        setHistIdx(-1);
        setInput('');
      } else {
        setHistIdx(newIdx);
        setInput(cmds[newIdx]);
      }
      return;
    }
    if (e.key === 'Tab') {
      e.preventDefault();
      const completions = shell.getTabCompletions(input.trim());
      if (completions.length === 1) {
        const parts = input.trim().split(' ');
        parts[parts.length - 1] = completions[0] + (input.trim().includes(' ') ? '' : ' ');
        setInput(parts.join(' '));
      } else if (completions.length > 1) {
        const h = [...history, {
          type: 'output' as const,
          content: completions.join('  ')
        }];
        setHistory(h);
      }
    }
  }, [input, cmds, histIdx, execute, shell, history]);
  return <div className={styles.terminalContainer} onClick={() => inputRef.current?.focus()}>
      <div className={styles.tabContainer}>
        <button className={`${styles.tab} ${styles.tabActive}`}>
          <span className={styles.tabIcon}>🐧</span>
          {t("components.LinuxTerminal.k1")}
        </button>
      </div>
      <div className={styles.terminalBody} ref={terminalRef}>
        {history.map((line, i) => <div key={i} className={styles[line.type] || styles.output}>
            {line.type === 'command' || line.type === 'system' ? renderLineToHtml(line.content) : <span style={{
          color: line.type === 'error' ? '#FF006E' : '#B3B9C4'
        }}>{line.content || ' '}</span>}
          </div>)}
        <div className={styles.inputLine}>
          <span className={styles.prompt}>{shell.getPrompt()}</span>
          <input ref={inputRef} type="text" className={styles.commandInput} value={input} onChange={e => setInput(e.target.value)} onKeyDown={handleKeyDown} spellCheck={false} autoComplete="off" autoFocus />
        </div>
      </div>
    </div>;
}