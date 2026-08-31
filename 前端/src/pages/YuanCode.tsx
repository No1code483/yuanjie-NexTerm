import { t } from "i18next";
import { useState, useCallback, useEffect, useRef } from 'react';
import { ipc, yuanCode } from '@/lib/ipc';
import SplitPane from '@/components/ui/SplitPane';
import FileTree from './yuan-code/FileTree';
import EditorPanel from './yuan-code/EditorPanel';
import RunPanel from './yuan-code/RunPanel';
import AgentPanel from './yuan-code/AgentPanel';
import AgentDialog from './yuan-code/AgentDialog';
import SandboxPanel from './yuan-code/SandboxPanel';
import SkillsPanel from './yuan-code/SkillsPanel';
import SettingsPanel from './yuan-code/SettingsPanel';
import SnippetPanel from './yuan-code/SnippetPanel';
import SymbolSearch from './yuan-code/SymbolSearch';
import GitPanel from './yuan-code/GitPanel';
import { useEngineEvents } from './yuan-code/useEngineEvents';
import ConfirmDialog from '@/components/ConfirmDialog';
import styles from './YuanCode.module.css';
export interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  children: FileNode[] | null;
  expanded: boolean;
}
export interface OpenTab {
  path: string;
  name: string;
  content: string;
  language: string;
  is_dirty: boolean;
}
export interface EditorSettings {
  fontSize: number;
  tabSize: number;
  showMinimap: boolean;
  wordWrap: 'off' | 'on' | 'wordWrapColumn';
  autoSave: boolean;
  formatOnSave: boolean;
}
const DEFAULT_EDITOR_SETTINGS: EditorSettings = {
  fontSize: 14,
  tabSize: 4,
  showMinimap: true,
  wordWrap: 'off',
  autoSave: false,
  formatOnSave: false
};
function detectLanguage(fileName: string): string {
  const ext = fileName.split('.').pop()?.toLowerCase() || '';
  const map: Record<string, string> = {
    py: 'python',
    js: 'javascript',
    ts: 'typescript',
    tsx: 'typescript',
    jsx: 'javascript',
    rs: 'rust',
    go: 'go',
    java: 'java',
    c: 'c',
    cpp: 'c++',
    h: 'c',
    hpp: 'c++',
    cs: 'csharp',
    rb: 'ruby',
    php: 'php',
    swift: 'swift',
    kt: 'kotlin',
    lua: 'lua',
    sql: 'sql',
    html: 'html',
    css: 'css',
    scss: 'scss',
    less: 'less',
    json: 'json',
    xml: 'xml',
    yaml: 'yaml',
    yml: 'yaml',
    toml: 'toml',
    md: 'markdown',
    txt: 'text',
    sh: 'sh',
    bash: 'sh',
    fish: 'sh',
    zsh: 'sh',
    ps1: 'powershell',
    bat: 'bat',
    cmd: 'bat'
  };
  return map[ext] || 'text';
}
// T2.12 XSS 审查：TOP_TABS[].icon 为代码内常量 SVG 字符串，非用户输入，理论安全
const TOP_TABS = [{
  id: 0,
  icon: '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M1.5 2h13l-6.5 7L14.5 15H1.5L8 9 1.5 2z"/></svg>',
  label: t("components.ShortcutPanel.k21")
}, {
  id: 1,
  icon: '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1a7 7 0 100 14A7 7 0 008 1zm0 1.2a5.8 5.8 0 110 11.6A5.8 5.8 0 018 2.2zM7.4 4.6h1.2v4.8H7.4V4.6zm0 6h1.2v1.2H7.4v-1.2z"/></svg>',
  label: 'Agent'
}, {
  id: 2,
  icon: '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M2 3h12a1 1 0 011 1v8a1 1 0 01-1 1H2a1 1 0 01-1-1V4a1 1 0 011-1zm1 2v6h10V5H3z"/><path d="M4 7h8v1H4V7zm0 2h5v1H4V9z"/></svg>',
  label: t("yuan-code.SettingsPanel.k3")
}, {
  id: 3,
  icon: '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1L2 5v6l6 4 6-4V5L8 1zm0 1.8L12.5 5.5v4.8L8 12l-4.5-1.7V5.5L8 2.8zM7 6h2v4H7V6zM7 4h2v1H7V4z"/></svg>',
  label: t("layout.k26")
}, {
  id: 4,
  icon: '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M8 2a6 6 0 100 12A6 6 0 008 2zm0 1.5a4.5 4.5 0 110 9 4.5 4.5 0 010-9zM7.5 5.5h1v2h2v1h-2v2h-1v-2h-2v-1h2v-2z"/></svg>',
  label: t("common.settings")
}, {
  id: 5,
  icon: '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M3 2h10a1 1 0 011 1v10a1 1 0 01-1 1H3a1 1 0 01-1-1V3a1 1 0 011-1zm1 2v8h8V4H4zm1 1h2v1H5V5zm0 2h4v1H5V7zm0 2h3v1H5V9zm5-4h1v5h-1V5z"/></svg>',
  label: t("YuanCode.k1")
}, {
  id: 6,
  icon: '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1a3 3 0 00-3 3v1H3a1 1 0 00-1 1v7a1 1 0 001 1h10a1 1 0 001-1V6a1 1 0 00-1-1h-2V4a3 3 0 00-3-3zM6 4a2 2 0 014 0v1H6V4zm-3 3h10v7H3V7zm3 1.5a.5.5 0 01.5.5v1.5h1.5a.5.5 0 010 1H6.5V13a.5.5 0 01-1 0v-1.5H4a.5.5 0 010-1h1.5V9a.5.5 0 01.5-.5z"/></svg>',
  label: 'Git'
}] as const;
export default function YuanCode() {
  const [activeTopTab, setActiveTopTab] = useState(0);
  const [workspacePath, setWorkspacePath] = useState('');
  const [fileTree, setFileTree] = useState<FileNode[]>([]);
  const [openTabs, setOpenTabs] = useState<OpenTab[]>([]);
  const [activeTabIndex, setActiveTabIndex] = useState(-1);

  // ===== 确认弹窗（关闭未保存文件） =====
  const [confirmDialog, setConfirmDialog] = useState<{
    isOpen: boolean;
    targetName: string;
    onConfirm: () => void | Promise<void>;
  }>({
    isOpen: false,
    targetName: '',
    onConfirm: () => {}
  });
  const [showRunPanel, setShowRunPanel] = useState(false);
  const [runLang, setRunLang] = useState('python');
  const [runOutput, setRunOutput] = useState('');
  const [runError, setRunError] = useState('');
  const [runRunning, setRunRunning] = useState(false);
  const [runDurationMs, setRunDurationMs] = useState<number | null>(null);
  const [runTruncated, setRunTruncated] = useState(false);
  const [executionId, setExecutionId] = useState<string | null>(null);
  const [editorSettings, setEditorSettings] = useState<EditorSettings>(DEFAULT_EDITOR_SETTINGS);
  const [editorRef, setEditorRef] = useState<any>(null);

  // D1.1：Yuan Code 编排模型 ID（来自统一模型管理，用于代码补全/分析/inline edit）
  // 从 localStorage 读取上次选择，SettingsPanel 拉取模型列表后会自动同步
  const [yuanModelId, setYuanModelId] = useState<number | null>(() => {
    const saved = localStorage.getItem('yuancode.orchestrator_model_id');
    return saved ? Number(saved) : null;
  });
  const handleOrchestratorModelIdChange = useCallback((id: number | null) => {
    setYuanModelId(id);
    if (id !== null) {
      localStorage.setItem('yuancode.orchestrator_model_id', String(id));
    } else {
      localStorage.removeItem('yuancode.orchestrator_model_id');
    }
  }, []);

  // Agent dialog state
  const [agentDialogOpen, setAgentDialogOpen] = useState(false);
  const activeTab = openTabs[activeTabIndex] || null;
  const [workspaceError, setWorkspaceError] = useState('');
  const [gitBranch, setGitBranch] = useState('');

  // 引擎会话
  const engineEvents = useEngineEvents();
  const sessionIdRef = useRef<string | null>(null);

  // 组件卸载时清理引擎会话
  useEffect(() => {
    return () => {
      const sid = sessionIdRef.current;
      if (sid) {
        engineEvents.unsubscribe();
        ipc.invoke('engine_destroy_session', {
          sessionId: sid
        }).catch(() => {});
      }
    };
  }, [engineEvents]);

  // Ctrl+. 快捷键打开 Agent Dialog
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === '.') {
        e.preventDefault();
        setAgentDialogOpen(prev => !prev);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // 获取 Git 分支名
  useEffect(() => {
    if (!workspacePath) return;
    ipc.invoke<{
      branch: string;
    }>('git_status', {
      workspace_path: workspacePath
    }).then(res => {
      if (res.code === 0 && res.data) {
        setGitBranch(res.data.branch);
      } else {
        setGitBranch('');
      }
    }).catch(() => setGitBranch(''));
  }, [workspacePath]);
  const listFiles = useCallback(async (path: string) => {
    setWorkspaceError('');
    try {
      const res = await ipc.invoke<FileNode[]>('yuan_list_files', {
        request: {
          workspace_path: path,
          exclude_patterns: ['.git', 'node_modules', 'target', '__pycache__', '.venv', 'venv', '.idea', '.vscode', 'dist', 'build']
        }
      });
      if (res.code === 0 && res.data) {
        const initExpand = (nodes: FileNode[]): FileNode[] => nodes.map(n => ({
          name: n.name,
          path: n.path,
          is_dir: n.is_dir,
          expanded: !!n.is_dir && !!n.children && n.children.length <= 5,
          children: n.children ? initExpand(n.children) : null
        }));
        setFileTree(initExpand(res.data));
      } else {
        setWorkspaceError(res.message || t("YuanCode.k2"));
      }
    } catch (e) {
      setWorkspaceError(e instanceof Error ? e.message : t("YuanCode.k3"));
    }
  }, []);
  const toggleDir = useCallback((node: FileNode) => {
    const update = (nodes: FileNode[]): FileNode[] => nodes.map(n => {
      if (n.path === node.path) return {
        ...n,
        expanded: !n.expanded
      };
      if (n.children) return {
        ...n,
        children: update(n.children)
      };
      return n;
    });
    setFileTree(prev => update(prev));
  }, []);
  const openFile = useCallback(async (node: FileNode) => {
    try {
      const existingIdx = openTabs.findIndex(t => t.path === node.path);
      if (existingIdx >= 0) {
        setActiveTabIndex(existingIdx);
        return;
      }

      // 文件树存储的是相对路径，需要拼接 workspacePath 变成绝对路径
      const normalized = node.path.replace(/\\/g, '/');
      const fullPath = workspacePath ? `${workspacePath.replace(/\\/g, '/')}/${normalized}` : normalized;
      const res = await ipc.invoke<{
        name: string;
        content: string;
        language: string;
      }>('yuan_read_file', {
        request: {
          path: fullPath
        }
      });
      if (res.code === 0 && res.data) {
        const lang = detectLanguage(node.name);
        const newTab: OpenTab = {
          path: node.path,
          name: node.name,
          content: res.data.content || '',
          language: res.data.language || lang,
          is_dirty: false
        };
        setOpenTabs(prev => {
          const nextIdx = prev.length;
          setTimeout(() => setActiveTabIndex(nextIdx), 0);
          return [...prev, newTab];
        });
      } else {
        console.warn('[YuanCode] 文件读取失败:', res.message);
      }
    } catch (e) {
      console.error('[YuanCode] openFile 异常:', e);
    }
  }, [openTabs, workspacePath]);
  const closeTab = useCallback(async (index: number) => {
    const tab = openTabs[index];
    if (tab?.is_dirty) {
      setConfirmDialog({
        isOpen: true,
        targetName: t("YuanCode.k4", {
          name: tab.name
        }),
        onConfirm: () => {
          setOpenTabs(prev => {
            const next = prev.filter((_, i) => i !== index);
            if (activeTabIndex >= next.length) {
              setActiveTabIndex(Math.max(0, next.length - 1));
            } else if (index < activeTabIndex) {
              setActiveTabIndex(activeTabIndex - 1);
            }
            return next;
          });
        }
      });
      return;
    }
    setOpenTabs(prev => {
      const next = prev.filter((_, i) => i !== index);
      if (activeTabIndex >= next.length) {
        setActiveTabIndex(Math.max(0, next.length - 1));
      } else if (index < activeTabIndex) {
        setActiveTabIndex(activeTabIndex - 1);
      }
      return next;
    });
  }, [openTabs, activeTabIndex]);
  const handleOpenWorkspace = useCallback(async () => {
    if (!workspacePath.trim()) return;
    await listFiles(workspacePath.trim());

    // 创建引擎会话
    try {
      const oldSessionId = sessionIdRef.current;
      if (oldSessionId) {
        engineEvents.unsubscribe();
        // 销毁旧会话
        await ipc.invoke('engine_destroy_session', {
          sessionId: oldSessionId
        }).catch(() => {});
      }
      const res = await ipc.invoke<string>('engine_create_session', {
        model: 'gpt-4o',
        workspacePath: workspacePath.trim(),
        tokenBudget: 200000,
        sandboxEnabled: true
      });
      if (res.code === 0 && res.data) {
        sessionIdRef.current = res.data;
        await engineEvents.subscribe(res.data);
      }
    } catch {
      // 引擎不可用时静默处理
    }
  }, [workspacePath, listFiles, engineEvents]);
  const handleContentChange = useCallback((content: string) => {
    setOpenTabs(prev => prev.map((t, i) => i === activeTabIndex ? {
      ...t,
      content,
      is_dirty: true
    } : t));
  }, [activeTabIndex]);
  const handleSave = useCallback(async () => {
    if (!activeTab) return;
    let content = activeTab.content;

    // 保存时格式化
    if (editorSettings.formatOnSave) {
      try {
        const formatRes = await ipc.invoke<any>('yuan_format_code', {
          request: {
            file_path: activeTab.path,
            language: activeTab.language,
            content
          }
        });
        if (formatRes?.data?.changed) {
          content = formatRes.data.formatted;
          // 更新 tab 内容
          setOpenTabs(prev => prev.map((t, i) => i === activeTabIndex ? {
            ...t,
            content,
            is_dirty: true
          } : t));
        }
      } catch {
        // 格式化失败，继续保存原始内容
      }
    }
    const res = await ipc.invoke('yuan_write_file', {
      request: {
        path: activeTab.path,
        content
      }
    });
    if (res.code === 0) {
      setOpenTabs(prev => prev.map((t, i) => i === activeTabIndex ? {
        ...t,
        is_dirty: false
      } : t));
    }
  }, [activeTab, activeTabIndex, editorSettings.formatOnSave]);
  const handleRun = useCallback(async () => {
    const code = activeTab?.content || '';
    if (!code.trim()) return;
    setShowRunPanel(true);
    setRunRunning(true);
    setRunOutput('');
    setRunError('');
    setRunTruncated(false);
    const id = `exec_${Date.now()}`;
    setExecutionId(id);
    const lang = activeTab?.language || runLang;
    try {
      const res = await ipc.invoke<{
        stdout: string;
        stderr: string;
        exit_code: number;
        duration_ms: number;
        stdout_truncated?: boolean;
        stderr_truncated?: boolean;
        timed_out?: boolean;
      }>('yuan_execute', {
        request: {
          code,
          language: lang,
          timeout_seconds: 30
        }
      });
      setRunRunning(false);
      setExecutionId(null);
      if (res.code === 0 && res.data) {
        setRunOutput(res.data.stdout || '');
        setRunError(res.data.stderr || '');
        setRunDurationMs(res.data.duration_ms);
        setRunTruncated(!!(res.data.stdout_truncated || res.data.stderr_truncated || res.data.timed_out));
      } else {
        setRunError(res.message || t("YuanCode.k5"));
        setRunDurationMs(null);
      }
    } catch {
      setRunRunning(false);
      setExecutionId(null);
      setRunError(t("YuanCode.k6"));
    }
  }, [activeTab, runLang]);
  const handleCancelRun = useCallback(async () => {
    if (executionId) {
      await yuanCode.ioCancel(executionId);
    }
    setRunRunning(false);
    setExecutionId(null);
  }, [executionId]);
  const handleForceCancelRun = useCallback(async () => {
    if (executionId) {
      await yuanCode.ioKill(executionId);
    }
    setRunRunning(false);
    setExecutionId(null);
  }, [executionId]);
  const handleSendStdin = useCallback(async (input: string) => {
    if (executionId) {
      await yuanCode.ioStdin(executionId, input);
    }
  }, [executionId]);
  const handleCreateFile = useCallback(async () => {
    const name = window.prompt(t("YuanCode.k7"));
    if (!name) return;
    const parentPath = activeTab?.path ? activeTab.path.split('/').slice(0, -1).join('/') : workspacePath;
    if (!parentPath) return;
    await ipc.invoke('yuan_create_item', {
      request: {
        parent_path: parentPath,
        name,
        is_dir: false
      }
    });
    listFiles(workspacePath);
  }, [activeTab, workspacePath, listFiles]);
  const handleCreateDir = useCallback(async () => {
    const name = window.prompt(t("YuanCode.k8"));
    if (!name) return;
    const parentPath = activeTab?.path ? activeTab.path.split('/').slice(0, -1).join('/') : workspacePath;
    if (!parentPath) return;
    await ipc.invoke('yuan_create_item', {
      request: {
        parent_path: parentPath,
        name,
        is_dir: true
      }
    });
    listFiles(workspacePath);
  }, [activeTab, workspacePath, listFiles]);
  const handleRefresh = useCallback(() => {
    if (workspacePath) listFiles(workspacePath);
  }, [workspacePath, listFiles]);
  const handleWorkspaceKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleOpenWorkspace();
  }, [handleOpenWorkspace]);

  // Agent 文件修改回调：更新 openTabs 中的文件内容
  const handleAgentUpdateFileContent = useCallback((path: string, content: string) => {
    setOpenTabs(prev => prev.map(t => t.path === path ? {
      ...t,
      content,
      is_dirty: true
    } : t));
  }, []);
  const renderCodeEditor = () => <>
      <div className={styles.workspaceBar}>
        <input className={styles.workspaceInput} value={workspacePath} onChange={e => {
        setWorkspacePath(e.target.value);
        setWorkspaceError('');
      }} onKeyDown={handleWorkspaceKeyDown} placeholder={t("YuanCode.k9")} />
        <button className={styles.openBtn} onClick={handleOpenWorkspace}>{t("common.open")}</button>
      </div>
      {workspaceError && <div style={{
      padding: '6px 16px',
      color: '#FF4444',
      fontSize: '12px',
      background: 'rgba(255,0,0,0.08)',
      borderBottom: '1px solid rgba(255,68,68,0.3)'
    }}>
          {workspaceError}
        </div>}

      {/* 主编辑区：水平分割 — 文件树侧边栏 | 编辑器 */}
      <SplitPane direction="horizontal" storageKey="yc_sidebar" defaultFirstSize={240} firstMinSize={100} firstMaxSize={480} secondMinSize={300} firstCollapsible={true} secondCollapsible={false} first={<FileTree fileTree={fileTree} activeFilePath={activeTab?.path || null} onToggleDir={toggleDir} onOpenFile={openFile} onCreateFile={handleCreateFile} onCreateDir={handleCreateDir} onRefresh={handleRefresh} />} second={<EditorPanel openTabs={openTabs} activeTabIndex={activeTabIndex} activeTab={activeTab} onTabChange={setActiveTabIndex} onTabClose={closeTab} onContentChange={handleContentChange} onSave={handleSave} onRun={handleRun} editorSettings={editorSettings} workspacePath={workspacePath} onEditorMount={setEditorRef} yuanModelId={yuanModelId} />} />

      {/* 运行面板（底部可折叠区域） */}
      {showRunPanel && <RunPanel visible={showRunPanel} code={activeTab?.content || ''} language={activeTab?.language || runLang} output={runOutput} error={runError} running={runRunning} durationMs={runDurationMs} truncated={runTruncated} onLanguageChange={setRunLang} onRun={handleRun} onCancel={handleCancelRun} onForceCancel={handleForceCancelRun} onSendStdin={handleSendStdin} onClose={() => setShowRunPanel(false)} />}

      <div className={styles.statusBar}>
        <div className={styles.statusLeft}>
          <span>{activeTab?.path || t("YuanCode.k10")}</span>
          {activeTab && <span className={styles.statusLang}>{activeTab.language}</span>}
          {gitBranch && <span className={styles.statusGit}>git:{gitBranch}</span>}
        </div>
        <div className={styles.statusRight}>
          <span>{workspacePath || t("YuanCode.k11")}</span>
          <span>Tab {activeTabIndex + 1}/{openTabs.length}</span>
        </div>
      </div>
    </>;
  return <div className={styles.container}>
      <div className={styles.topTabBar}>
        {TOP_TABS.map(tab => <div key={tab.id} className={`${styles.topTab} ${activeTopTab === tab.id ? styles.topTabActive : ''}`} onClick={() => setActiveTopTab(tab.id)}>
            <span className={styles.topTabIcon} dangerouslySetInnerHTML={{
          __html: tab.icon
        }} />
            <span>{tab.label}</span>
          </div>)}
      </div>

      {activeTopTab === 0 && renderCodeEditor()}
      {activeTopTab === 1 && <AgentPanel engineEvents={engineEvents} sessionId={sessionIdRef.current} />}
      {activeTopTab === 2 && <SandboxPanel />}
      {activeTopTab === 3 && <SkillsPanel />}
      {activeTopTab === 4 && <SettingsPanel editorSettings={editorSettings} onEditorSettingsChange={setEditorSettings} onOrchestratorModelIdChange={handleOrchestratorModelIdChange} />}
      {activeTopTab === 5 && <SnippetPanel editor={editorRef} workspacePath={workspacePath} />}
      {activeTopTab === 6 && <GitPanel workspacePath={workspacePath} />}

      {/* 符号搜索弹窗 */}
      <SymbolSearch editor={editorRef} onNavigate={(_file, _line) => {
      // 符号导航回调
    }} />

      {/* 确认弹窗（关闭未保存文件） */}
      <ConfirmDialog isOpen={confirmDialog.isOpen} onClose={() => setConfirmDialog({
      ...confirmDialog,
      isOpen: false
    })} onConfirm={confirmDialog.onConfirm} targetName={confirmDialog.targetName} type="warning" message={t("YuanCode.k12", {
      targetName: confirmDialog.targetName
    })} />

      {/* Agent Dialog */}
      <AgentDialog isOpen={agentDialogOpen} onClose={() => setAgentDialogOpen(false)} fileTree={fileTree} openTabs={openTabs} workspacePath={workspacePath} onUpdateFileContent={handleAgentUpdateFileContent} />
    </div>;
}