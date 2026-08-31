import { t } from "i18next";
import { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import '@xterm/xterm/css/xterm.css';
import { listen } from '@tauri-apps/api/event';
import { ipc } from '@/lib/ipc';
const SESSION_STORAGE_KEY = 'nexterm_terminal_session';
interface NexTermTerminalProps {
  onCommand?: (command: string) => void;
  theme?: 'default' | 'dark';
  themeName?: string;
  fontSize?: number;
  showScanline?: boolean;
  sessionType?: 'terminal' | 'cmd' | 'powershell' | 'wsl';
  autoConnect?: boolean;
  enablePersistence?: boolean;
  externalSessionId?: string | null;
  onSessionCreated?: (sessionId: string) => void;
}
interface TerminalSessionInfo {
  sessionId: string;
  sessionType: string;
  connectedAt: string;
  lastActivity: string;
}
interface TerminalThemeData {
  name: string;
  foreground: string;
  background: string;
  cursor: string;
  cursor_accent: string;
  selection: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  bright_black: string;
  bright_red: string;
  bright_green: string;
  bright_yellow: string;
  bright_blue: string;
  bright_magenta: string;
  bright_cyan: string;
  bright_white: string;
}
const DEFAULT_THEME: TerminalThemeData = {
  name: 'Dark+',
  foreground: '#D4D4D4',
  background: '#1E1E1E',
  cursor: '#FFFFFF',
  cursor_accent: '#1E1E1E',
  selection: '#264F78',
  black: '#000000',
  red: '#CD3131',
  green: '#0DBC79',
  yellow: '#E5E510',
  blue: '#2472C8',
  magenta: '#BC3FBC',
  cyan: '#11A8CD',
  white: '#E5E5E5',
  bright_black: '#666666',
  bright_red: '#F14C4C',
  bright_green: '#23D18B',
  bright_yellow: '#F5F543',
  bright_blue: '#3B8EEA',
  bright_magenta: '#D670D6',
  bright_cyan: '#29B8DB',
  bright_white: '#FFFFFF'
};
export default function NexTermTerminal({
  onCommand,
  themeName,
  fontSize = 14,
  showScanline = true,
  sessionType = 'cmd',
  autoConnect = false,
  enablePersistence = true,
  externalSessionId = null,
  onSessionCreated
}: NexTermTerminalProps) {
  const terminalRef = useRef<HTMLDivElement>(null);
  const terminal = useRef<Terminal | null>(null);
  const fitAddon = useRef<FitAddon | null>(null);
  const searchAddon = useRef<SearchAddon | null>(null);
  const [isReady, setIsReady] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(() => {
    if (typeof window !== 'undefined' && enablePersistence) {
      try {
        const saved = sessionStorage.getItem(SESSION_STORAGE_KEY);
        if (saved) {
          const sessionInfo: TerminalSessionInfo = JSON.parse(saved);
          console.log('[Terminal] 📦 发现持久化会话:', sessionInfo.sessionId);
          return sessionInfo.sessionId;
        }
      } catch (e) {
        console.warn('[Terminal] ⚠️ 读取持久化会话失败:', e);
      }
    }
    return null;
  });
  const [isConnected, setIsConnected] = useState(() => {
    return sessionId !== null;
  });
  const unlistenRef = useRef<(() => void) | null>(null);

  // 搜索状态
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [searchText, setSearchText] = useState('');
  const [searchMatchIndex, setSearchMatchIndex] = useState(0);
  const [searchMatchCount, setSearchMatchCount] = useState(0);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // 主题状态
  const [themes, setThemes] = useState<TerminalThemeData[]>([]);
  const [currentTheme, setCurrentTheme] = useState<TerminalThemeData>(DEFAULT_THEME);

  // 加载主题列表
  useEffect(() => {
    if (!window.__TAURI__) return;
    ipc.invoke<{
      code: number;
      data: TerminalThemeData[];
    }>('terminal_get_themes').then(res => {
      if (res.data && Array.isArray(res.data)) {
        setThemes(res.data);
        if (themeName) {
          const found = res.data.find(t => t.name === themeName);
          if (found) setCurrentTheme(found);
        }
      }
    }).catch(() => {});
  }, [themeName]);

  // 切换主题
  const applyTheme = useCallback((t: TerminalThemeData) => {
    setCurrentTheme(t);
    if (terminal.current) {
      terminal.current.options.theme = {
        foreground: t.foreground,
        background: t.background,
        cursor: t.cursor,
        cursorAccent: t.cursor_accent,
        selectionBackground: t.selection,
        black: t.black,
        red: t.red,
        green: t.green,
        yellow: t.yellow,
        blue: t.blue,
        magenta: t.magenta,
        cyan: t.cyan,
        white: t.white,
        brightBlack: t.bright_black,
        brightRed: t.bright_red,
        brightGreen: t.bright_green,
        brightYellow: t.bright_yellow,
        brightBlue: t.bright_blue,
        brightMagenta: t.bright_magenta,
        brightCyan: t.bright_cyan,
        brightWhite: t.bright_white
      };
    }
  }, []);
  const saveSessionToStorage = useCallback((sid: string) => {
    if (!enablePersistence || typeof window === 'undefined') return;
    try {
      const sessionInfo: TerminalSessionInfo = {
        sessionId: sid,
        sessionType: sessionType,
        connectedAt: new Date().toISOString(),
        lastActivity: new Date().toISOString()
      };
      sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(sessionInfo));
      console.log('[Terminal] 💾 会话已保存到 sessionStorage');
    } catch (e) {
      console.error('[Terminal] ❌ 保存会话失败:', e);
    }
  }, [enablePersistence, sessionType]);
  const clearSessionFromStorage = useCallback(() => {
    if (typeof window === 'undefined') return;
    try {
      sessionStorage.removeItem(SESSION_STORAGE_KEY);
      console.log('[Terminal] 🗑️ 持久化会话已清除');
    } catch (e) {
      console.warn('[Terminal] ⚠️ 清除持久化会话失败:', e);
    }
  }, []);
  const updateLastActivity = useCallback(() => {
    if (!sessionId || !enablePersistence || typeof window === 'undefined') return;
    try {
      const saved = sessionStorage.getItem(SESSION_STORAGE_KEY);
      if (saved) {
        const sessionInfo: TerminalSessionInfo = JSON.parse(saved);
        sessionInfo.lastActivity = new Date().toISOString();
        sessionStorage.setItem(SESSION_STORAGE_KEY, JSON.stringify(sessionInfo));
      }
    } catch (e) {
      // 静默失败
    }
  }, [sessionId, enablePersistence]);
  const connectToBackend = useCallback(async () => {
    try {
      if (!window.__TAURI__) {
        console.warn('[Terminal] ⚠️ 当前不在 Tauri 环境中，使用模拟模式');
        return;
      }
      console.log('[Terminal] 🔗 正在连接后端 PTY 会话...');
      const result = await ipc.invoke<string>('terminal_create_session', {
        session_type: sessionType
      });
      if (result.data) {
        setSessionId(result.data);
        setIsConnected(true);
        saveSessionToStorage(result.data);
        onSessionCreated?.(result.data);
        console.log(`[Terminal] ✅ PTY 会话已创建: ${result.data}`);
        const unlisten = await listen<{
          session_id: string;
          data: string;
        }>('terminal-output', event => {
          if (event.payload.session_id === result.data && terminal.current) {
            terminal.current.write(event.payload.data);
            updateLastActivity();
          }
        });
        unlistenRef.current = unlisten;
        console.log('[Terminal] 🎧 已监听 terminal-output 事件');
      }
    } catch (error) {
      console.error('[Terminal] ❌ 连接后端失败:', error);
    }
  }, [sessionType, saveSessionToStorage, updateLastActivity]);
  const disconnectFromBackend = useCallback(async () => {
    try {
      if (sessionId && window.__TAURI__) {
        await ipc.invoke('terminal_kill_session', {
          session_id: sessionId
        });
        console.log(`[Terminal] ✅ PTY 会话已终止: ${sessionId}`);
      }
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
      setSessionId(null);
      setIsConnected(false);
      clearSessionFromStorage();
    } catch (error) {
      console.error('[Terminal] ❌ 断开连接失败:', error);
    }
  }, [sessionId]);
  useEffect(() => {
    if (terminalRef.current && !terminal.current) {
      terminal.current = new Terminal({
        fontSize: fontSize,
        fontFamily: 'Consolas, monospace',
        theme: {
          foreground: currentTheme.foreground,
          background: currentTheme.background,
          cursor: currentTheme.cursor,
          cursorAccent: currentTheme.cursor_accent,
          selectionBackground: currentTheme.selection,
          black: currentTheme.black,
          red: currentTheme.red,
          green: currentTheme.green,
          yellow: currentTheme.yellow,
          blue: currentTheme.blue,
          magenta: currentTheme.magenta,
          cyan: currentTheme.cyan,
          white: currentTheme.white,
          brightBlack: currentTheme.bright_black,
          brightRed: currentTheme.bright_red,
          brightGreen: currentTheme.bright_green,
          brightYellow: currentTheme.bright_yellow,
          brightBlue: currentTheme.bright_blue,
          brightMagenta: currentTheme.bright_magenta,
          brightCyan: currentTheme.bright_cyan,
          brightWhite: currentTheme.bright_white
        },
        cursorBlink: true,
        allowTransparency: true
      });
      fitAddon.current = new FitAddon();
      terminal.current.loadAddon(fitAddon.current);
      searchAddon.current = new SearchAddon();
      terminal.current.loadAddon(searchAddon.current);
      terminal.current.open(terminalRef.current);
      fitAddon.current.fit();
      terminal.current.onData(data => {
        if (isConnected && sessionId && window.__TAURI__) {
          ipc.invoke('terminal_write_input', {
            session_id: sessionId,
            input: data
          }).catch(err => {
            console.error('[Terminal] ❌ 写入输入失败:', err);
          });
        } else if (onCommand) {
          onCommand(data);
        }
      });
      setIsReady(true);
    }
    return () => {
      disconnectFromBackend();
      if (terminal.current) {
        terminal.current.dispose();
        terminal.current = null;
      }
    };
  }, [fontSize, onCommand, isConnected, sessionId, disconnectFromBackend]);
  useEffect(() => {
    if (autoConnect && isReady && !isConnected) {
      connectToBackend();
    }
  }, [autoConnect, isReady, isConnected, connectToBackend]);
  useEffect(() => {
    const restoreSession = async () => {
      if (!isReady || !sessionId || !window.__TAURI__) return;
      console.log('[Terminal] 🔄 尝试恢复持久化会话:', sessionId);
      try {
        const unlisten = await listen<{
          session_id: string;
          data: string;
        }>('terminal-output', event => {
          if (event.payload.session_id === sessionId && terminal.current) {
            terminal.current.write(event.payload.data);
            updateLastActivity();
          }
        });
        unlistenRef.current = unlisten;
        setIsConnected(true);
        console.log('[Terminal] ✅ 持久化会话已恢复，开始监听输出');
      } catch (error) {
        console.error('[Terminal] ❌ 恢复会话失败:', error);
        clearSessionFromStorage();
        setSessionId(null);
      }
    };
    if (enablePersistence && sessionId && !isConnected) {
      restoreSession();
    }
  }, [isReady, sessionId, isConnected, enablePersistence, updateLastActivity, clearSessionFromStorage]);
  useEffect(() => {
    if (!externalSessionId || !isReady || !window.__TAURI__) return;
    const listenToExternal = async () => {
      console.log('[Terminal] 🔗 使用外部会话 ID:', externalSessionId);
      setSessionId(externalSessionId);
      setIsConnected(true);
      const unlisten = await listen<{
        session_id: string;
        data: string;
      }>('terminal-output', event => {
        if (event.payload.session_id === externalSessionId && terminal.current) {
          terminal.current.write(event.payload.data);
        }
      });
      unlistenRef.current = unlisten;
      console.log('[Terminal] 🎧 已监听外部会话 terminal-output 事件');
    };
    listenToExternal();
  }, [externalSessionId, isReady]);
  useEffect(() => {
    if (!isReady || !terminalRef.current || !fitAddon.current) return;
    fitAddon.current.fit();
    const resizeObserver = new ResizeObserver(() => {
      if (!fitAddon.current || !terminal.current) return;
      fitAddon.current.fit();
      if (isConnected && sessionId && window.__TAURI__) {
        const cols = terminal.current.cols;
        const rows = terminal.current.rows;
        if (cols > 0 && rows > 0) {
          ipc.invoke('terminal_resize', {
            session_id: sessionId,
            cols: cols,
            rows: rows
          }).catch(err => {
            console.error('[Terminal] ❌ 调整大小失败:', err);
          });
        }
      }
    });
    resizeObserver.observe(terminalRef.current);
    return () => resizeObserver.disconnect();
  }, [isReady, isConnected, sessionId]);

  // 搜索功能
  const performSearch = useCallback((text: string) => {
    if (!searchAddon.current || !text) {
      setSearchMatchIndex(0);
      setSearchMatchCount(0);
      return;
    }
    const result = searchAddon.current.findNext(text);
    if (result) {
      setSearchMatchIndex(1);
      setSearchMatchCount(1);
    } else {
      setSearchMatchIndex(0);
      setSearchMatchCount(0);
    }
  }, []);
  const findNext = useCallback(() => {
    if (!searchAddon.current || !searchText) return;
    const result = searchAddon.current.findNext(searchText);
    if (result) {
      setSearchMatchIndex(prev => prev + 1);
      setSearchMatchCount(prev => Math.max(prev, prev + 1));
    }
  }, [searchText]);
  const findPrevious = useCallback(() => {
    if (!searchAddon.current || !searchText) return;
    const result = searchAddon.current.findPrevious(searchText);
    if (result) {
      setSearchMatchIndex(prev => Math.max(1, prev - 1));
      setSearchMatchCount(prev => Math.max(prev, 1));
    }
  }, [searchText]);
  const toggleSearch = useCallback(() => {
    setIsSearchOpen(prev => {
      if (!prev) {
        setTimeout(() => searchInputRef.current?.focus(), 50);
      } else {
        setSearchText('');
        setSearchMatchIndex(0);
        setSearchMatchCount(0);
      }
      return !prev;
    });
  }, []);
  const closeSearch = useCallback(() => {
    setIsSearchOpen(false);
    setSearchText('');
    setSearchMatchIndex(0);
    setSearchMatchCount(0);
  }, []);
  const handleSearchChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const text = e.target.value;
    setSearchText(text);
    performSearch(text);
  }, [performSearch]);
  const handleSearchKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      e.shiftKey ? findPrevious() : findNext();
    } else if (e.key === 'Escape') {
      closeSearch();
      terminal.current?.focus();
    }
  }, [findNext, findPrevious, closeSearch]);

  // 全局键盘快捷键 Ctrl+Shift+F 打开搜索
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.key === 'F') {
        e.preventDefault();
        toggleSearch();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleSearch]);
  const getTerminalStyle = () => ({
    width: '100%',
    height: '100%',
    border: '1.5px solid #00F0FF',
    backgroundColor: '#0A0014',
    position: 'relative' as const
  });
  const getScanlineStyle = () => ({
    position: 'absolute' as const,
    top: 0,
    left: 0,
    width: '100%',
    height: '100%',
    background: 'linear-gradient(to bottom, transparent 50%, rgba(0, 240, 255, 0.03) 50%)',
    backgroundSize: '100% 4px',
    animation: 'nt-scanline 8s linear infinite',
    pointerEvents: 'none' as const
  });
  const getStatusStyle = () => ({
    position: 'absolute' as const,
    top: '5px',
    right: '10px',
    padding: '2px 8px',
    borderRadius: '3px',
    fontSize: '11px',
    fontFamily: 'monospace',
    backgroundColor: isConnected ? '#0a1a2e' : '#2e0a0a',
    color: isConnected ? '#00F0FF' : '#FF006E',
    border: `1px solid ${isConnected ? '#00F0FF' : '#FF006E'}`,
    zIndex: 10
  });
  const getControlsStyle = () => ({
    position: 'absolute' as const,
    bottom: '5px',
    right: '10px',
    display: 'flex',
    gap: '8px',
    zIndex: 10
  });
  const getButtonStyle = (isConnectedBtn: boolean) => ({
    padding: '4px 12px',
    fontSize: '11px',
    fontFamily: 'monospace',
    backgroundColor: isConnectedBtn ? '#003300' : '#330000',
    color: isConnectedBtn ? '#00FF00' : '#FF0000',
    border: `1px solid ${isConnectedBtn ? '#00FF00' : '#FF0000'}`,
    borderRadius: '3px',
    cursor: 'pointer' as const,
    opacity: window?.__TAURI__ ? 1 : 0.5
  });
  return <div style={getTerminalStyle()}>
      <div ref={terminalRef} style={{
      width: '100%',
      height: '100%'
    }} />
      {showScanline && <div style={getScanlineStyle()} />}
      
      {isReady && <>
          {/* 顶栏：主题选择 + 状态 */}
          <div style={{
        position: 'absolute' as const,
        top: '4px',
        right: '8px',
        display: 'flex',
        alignItems: 'center',
        gap: '8px',
        zIndex: 10
      }}>
            {themes.length > 0 && <select value={currentTheme.name} onChange={e => {
          const t = themes.find(t => t.name === e.target.value);
          if (t) applyTheme(t);
        }} style={{
          backgroundColor: currentTheme.background,
          color: currentTheme.foreground,
          border: `1px solid ${currentTheme.cyan}`,
          borderRadius: '3px',
          fontSize: '11px',
          fontFamily: 'monospace',
          padding: '2px 4px',
          cursor: 'pointer',
          outline: 'none'
        }}>
                {themes.map(t => <option key={t.name} value={t.name}>{t.name}</option>)}
              </select>}
            <div style={getStatusStyle()}>
              {isConnected ? 'CONNECTED' : 'DISCONNECTED'}
            </div>
          </div>

          {isSearchOpen && <div style={{
        position: 'absolute',
        top: '5px',
        right: '130px',
        display: 'flex',
        alignItems: 'center',
        gap: '4px',
        backgroundColor: '#0A0014',
        border: '1px solid #00F0FF',
        borderRadius: '4px',
        padding: '3px 8px',
        zIndex: 20,
        fontFamily: 'monospace',
        fontSize: '12px'
      }}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#00F0FF" strokeWidth="2">
                <circle cx="11" cy="11" r="8" />
                <line x1="21" y1="21" x2="16.65" y2="16.65" />
              </svg>
              <input ref={searchInputRef} type="text" value={searchText} onChange={handleSearchChange} onKeyDown={handleSearchKeyDown} placeholder={t("components.NexTermTerminal.k1")} style={{
          backgroundColor: 'transparent',
          border: 'none',
          color: '#00F0FF',
          outline: 'none',
          width: '160px',
          fontFamily: 'monospace',
          fontSize: '12px'
        }} />
              {searchMatchCount > 0 && <span style={{
          color: '#6A6A8A',
          fontSize: '11px',
          minWidth: '30px'
        }}>
                  {searchMatchIndex}/{searchMatchCount}
                </span>}
              {searchText && searchMatchCount === 0 && <span style={{
          color: '#FF006E',
          fontSize: '11px'
        }}>{t("components.NexTermTerminal.k2")}</span>}
              <button onClick={findPrevious} title={t("components.NexTermTerminal.k3")} style={{
          backgroundColor: 'transparent',
          border: 'none',
          color: '#00F0FF',
          cursor: 'pointer',
          fontSize: '14px',
          padding: '0 2px'
        }}>▲</button>
              <button onClick={findNext} title={t("components.NexTermTerminal.k4")} style={{
          backgroundColor: 'transparent',
          border: 'none',
          color: '#00F0FF',
          cursor: 'pointer',
          fontSize: '14px',
          padding: '0 2px'
        }}>▼</button>
              <button onClick={closeSearch} title={t("components.NexTermTerminal.k5")} style={{
          backgroundColor: 'transparent',
          border: 'none',
          color: '#FF006E',
          cursor: 'pointer',
          fontSize: '14px',
          padding: '0 2px'
        }}>✕</button>
            </div>}
          
          {window?.__TAURI__ && <div style={getControlsStyle()}>
              {!isConnected ? <button style={getButtonStyle(true)} onClick={connectToBackend} title={t("components.NexTermTerminal.k6")}>
                  🔗 Connect
                </button> : <button style={getButtonStyle(false)} onClick={disconnectFromBackend} title={t("components.NexTermTerminal.k7")}>
                  ✖ Disconnect
                </button>}
            </div>}
        </>}
    </div>;
}