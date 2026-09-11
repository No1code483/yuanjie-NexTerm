import { t } from "i18next";
import { useCallback, useEffect, useRef, useState } from 'react';
import Editor, { type OnMount, type Monaco } from '@monaco-editor/react';
import type { editor, IPosition } from 'monaco-editor';
import type { OpenTab, EditorSettings } from '../YuanCode';
import { getDiagnostics, toMonacoMarkers, getCompletions, getHover, getDefinition } from '@/lib/ipc-lsp';
import { ipc } from '@/lib/ipc';
import { useNetworkStatus } from '@/hooks/useNetworkStatus';
import FindReplace from './FindReplace'; // eslint-disable-line
import Breadcrumb from './Breadcrumb';
import { SnippetStore } from './SnippetStore';
import { completionMemory } from './CompletionMemory';
import { BUILT_IN_SNIPPETS } from './BuiltInSnippets';
import InlineEdit from './InlineEdit';
import styles from '../YuanCode.module.css';
interface EditorPanelProps {
  openTabs: OpenTab[];
  activeTabIndex: number;
  activeTab: OpenTab | null;
  onTabChange: (index: number) => void;
  onTabClose: (index: number) => void;
  onContentChange: (content: string) => void;
  onSave: () => void;
  onRun: () => void;
  editorSettings: EditorSettings;
  workspacePath: string;
  onEditorMount?: (editor: any) => void;
  /** D1.1：Yuan Code 编排模型 ID（来自统一模型管理），用于 inline 补全 */
  yuanModelId?: number | null;
}
const LANGUAGE_MAP: Record<string, string> = {
  python: 'python',
  javascript: 'javascript',
  typescript: 'typescript',
  rust: 'rust',
  go: 'go',
  java: 'java',
  c: 'c',
  cpp: 'cpp',
  csharp: 'csharp',
  ruby: 'ruby',
  php: 'php',
  swift: 'swift',
  kotlin: 'kotlin',
  lua: 'lua',
  scala: 'scala',
  r: 'r',
  sql: 'sql',
  html: 'html',
  css: 'css',
  scss: 'scss',
  less: 'less',
  json: 'json',
  xml: 'xml',
  yaml: 'yaml',
  toml: 'plaintext',
  markdown: 'markdown',
  text: 'plaintext',
  sh: 'shell',
  bash: 'shell',
  powershell: 'powershell',
  bat: 'bat',
  dockerfile: 'dockerfile'
};
const NEXTERM_THEME = {
  /* ★ inherit=true：完整继承 VSCode Dark+ 内置 token 规则（所有语言完美着色） */
  base: 'vs-dark' as const,
  inherit: true,
  /* 仅覆盖需要微调的 UI token（大部分继承自 Dark+，无需重复定义） */
  rules: [
  // 注释保持斜体（Dark+ 默认无 fontStyle）
  {
    token: 'comment',
    foreground: '6A9955',
    fontStyle: 'italic'
  }, {
    token: 'comment.doc',
    foreground: '6A9955',
    fontStyle: 'italic'
  },
  // JSON key 覆盖为浅蓝（Dark+ 默认可能不够明显）
  {
    token: 'string.key.json',
    foreground: '9CDCFE'
  }, {
    token: 'literal.key.json',
    foreground: '9CDCFE'
  }],
  colors: {
    // ===== 编辑器主体 =====
    'editor.background': '#0A0014',
    'editor.foreground': '#D4D4D4',
    'editor.lineHighlightBackground': '#15102E',
    // ===== 选区 — 暗蓝灰（非红色） =====
    'editor.selectionBackground': '#264F78',
    'editor.inactiveSelectionBackground': '#3A3D41',
    'editorCursor.foreground': '#00F0FF',
    'editorCursor.background': '#0A0014',
    'editorLineNumber.foreground': '#3A3A55',
    'editorLineNumber.activeForeground': '#8080A0',
    // ===== 高亮 & 搜索 — 低饱和度灰蓝 =====
    'editor.selectionHighlightBackground': '#264F7840',
    'editor.wordHighlightBackground': '#575C6530',
    'editor.wordHighlightStrongBackground': '#575C6550',
    'editor.findMatchBackground': '#515C6A',
    'editor.findMatchHighlightBackground': '#515C6A40',
    'editor.findRangeHighlightBackground': '#515C6A20',
    // ===== 括号匹配 =====
    'editorBracketMatch.background': 'rgba(0, 240, 255, 0.10)',
    'editorBracketMatch.border': 'rgba(0, 200, 220, 0.35)',
    // ===== 缩进引导线 — VSCode Dark+: #404040 / #707070 =====
    'editorIndentGuide.background': '#404040',
    'editorIndentGuide.activeBackground': '#707070',
    'editorIndentGuide.background1': '#404040',
    'editorIndentGuide.background2': '#404040',
    'editorIndentGuide.background3': '#404040',
    'editorIndentGuide.background4': '#404040',
    'editorIndentGuide.background5': '#404040',
    'editorIndentGuide.background6': '#404040',
    'editorIndentGuide.activeBackground1': '#707070',
    'editorIndentGuide.activeBackground2': '#707070',
    'editorIndentGuide.activeBackground3': '#707070',
    'editorIndentGuide.activeBackground4': '#707070',
    'editorIndentGuide.activeBackground5': '#707070',
    'editorIndentGuide.activeBackground6': '#707070',
    // ===== 括号对引导线 =====
    'editorBracketPairGuide.background1': 'rgba(64, 64, 64, 0.5)',
    'editorBracketPairGuide.background2': 'rgba(64, 64, 64, 0.5)',
    'editorBracketPairGuide.background3': 'rgba(64, 64, 64, 0.5)',
    'editorBracketPairGuide.background4': 'rgba(64, 64, 64, 0.5)',
    'editorBracketPairGuide.background5': 'rgba(64, 64, 64, 0.5)',
    'editorBracketPairGuide.background6': 'rgba(64, 64, 64, 0.5)',
    'editorBracketPairGuide.activeBackground1': '#707070',
    'editorBracketPairGuide.activeBackground2': '#707070',
    'editorBracketPairGuide.activeBackground3': '#707070',
    'editorBracketPairGuide.activeBackground4': '#707070',
    'editorBracketPairGuide.activeBackground5': '#707070',
    'editorBracketPairGuide.activeBackground6': '#707070',
    // ===== Widget / 弹窗 / 浮层 =====
    'editorWidget.background': '#0E0022',
    'editorWidget.border': 'rgba(0, 240, 255, 0.15)',
    'editorSuggestWidget.background': '#0E0022',
    'editorSuggestWidget.border': 'rgba(0, 240, 255, 0.15)',
    'editorSuggestWidget.selectedBackground': 'rgba(0, 240, 255, 0.12)',
    'editorSuggestWidget.highlightForeground': '#00F0FF',
    'editorHoverWidget.background': '#0E0022',
    'editorHoverWidget.border': 'rgba(0, 240, 255, 0.15)',
    // ===== 输入框 =====
    'input.background': '#0E0022',
    'input.foreground': '#D4D4D4',
    'input.border': 'rgba(0, 240, 255, 0.2)',
    'inputOption.activeBorder': '#00F0FF',
    // ===== 滚动条 — 灰色系（与项目其他位置统一 4px 宽度） =====
    'scrollbarSlider.background': 'rgba(80, 85, 100, 0.35)',
    'scrollbarSlider.hoverBackground': 'rgba(90, 95, 115, 0.50)',
    'scrollbarSlider.activeBackground': 'rgba(100, 105, 130, 0.65)',
    'scrollbar.shadow': '#00000000',
    // 编辑器专用滚动条
    'editorScrollbar.background': '#0A001400',
    'editorScrollbar.sliderBackground': 'rgba(80, 85, 100, 0.35)',
    'editorScrollbar.sliderHoverBackground': 'rgba(90, 95, 115, 0.50)',
    'editorScrollbar.sliderActiveBackground': 'rgba(100, 105, 130, 0.65)',
    // ===== Minimap（代码缩略图）— 全部灰色系，无红色 =====
    'minimap.background': '#0A0014',
    'minimap.findMatchHighlight': '#515C6A',
    'minimap.selectionHighlight': 'rgba(100, 105, 130, 0.25)',
    'minimap.errorHighlight': '#515C6A',
    'minimap.warningHighlight': '#515C6A',
    'minimap.infoHighlight': '#515C6A',
    // Minimap 滑块（缩略图上的可见区域指示器）
    'minimapSlider.background': 'rgba(80, 85, 100, 0.35)',
    'minimapSlider.hoverBackground': 'rgba(90, 95, 115, 0.50)',
    'minimapSlider.activeBackground': 'rgba(100, 105, 130, 0.65)',
    // ===== 行内装饰 =====
    'editorOverviewRuler.border': '#1A0A30',
    // Overview Ruler 标记 — 灰色系（非红色）
    'editorOverviewRuler.errorForeground': '#505560',
    'editorOverviewRuler.warningForeground': '#505560',
    'editorOverviewRuler.infoForeground': '#505560',
    'editorOverviewRuler.bracketForeground': '#707070',
    // ===== 行内错误/警告/信息标记（gutter 区域） =====
    'editorError.foreground': '#F44747',
    'editorWarning.foreground': '#D7BA7D',
    // ===== 链接 =====
    'textLink.foreground': '#4FC3FF',
    'textLink.activeForeground': '#00F0FF',
    // ===== Git 装饰（侧边栏颜色条）=====
    'gitDecoration.addedResourceForeground': '#50FA7B',
    'gitDecoration.modifiedResourceForeground': '#FFD700',
    'gitDecoration.deletedResourceForeground': '#FF5555',
    'gitDecoration.conflictedResourceForeground': '#FF6B6B',
    // ===== 标记栏 =====
    'editorMarkerNavigation.background': 'rgba(0, 240, 255, 0.08)',
    'editorMarkerNavigationError.background': 'rgba(255, 68, 68, 0.15)',
    'editorMarkerNavigationWarning.background': 'rgba(255, 170, 0, 0.15)',
    // ===== 折叠区域 =====
    'editor.foldBackground': 'rgba(0, 240, 255, 0.04)',
    'editorGutter.modifiedBackground': 'rgba(255, 215, 0, 0.15)',
    'editorGutter.addedBackground': 'rgba(80, 250, 123, 0.15)',
    'editorGutter.deletedBackground': 'rgba(255, 85, 85, 0.15)'
  }
};
function getMonacoLanguage(language: string): string {
  return LANGUAGE_MAP[language] || 'plaintext';
}

/** LSP 补全类型映射到 Monaco 补全类型 */
function mapLspKind(kind?: string): number {
  switch (kind) {
    case 'Method':
      return 0;
    case 'Function':
      return 1;
    case 'Constructor':
      return 2;
    case 'Field':
      return 3;
    case 'Variable':
      return 4;
    case 'Class':
      return 5;
    case 'Struct':
      return 6;
    case 'Interface':
      return 7;
    case 'Module':
      return 8;
    case 'Property':
      return 9;
    case 'Event':
      return 10;
    case 'Operator':
      return 11;
    case 'Unit':
      return 12;
    case 'Value':
      return 13;
    case 'Constant':
      return 14;
    case 'Enum':
      return 15;
    case 'EnumMember':
      return 16;
    case 'Keyword':
      return 17;
    case 'Text':
      return 18;
    case 'Color':
      return 19;
    case 'File':
      return 20;
    case 'Reference':
      return 21;
    case 'Snippet':
      return 25;
    default:
      return 1;
  }
}
export default function EditorPanel({
  openTabs,
  activeTabIndex,
  activeTab,
  onTabChange,
  onTabClose,
  onContentChange,
  onSave,
  onRun,
  editorSettings,
  workspacePath,
  onEditorMount,
  yuanModelId
}: EditorPanelProps) {
  const editorRef = useRef<any>(null);
  const monacoRef = useRef<any>(null);
  const [showFind, setShowFind] = useState(false);
  const inlineEditSelectionRef = useRef<any>(null);

  // D1.2 流式补全：ghost text widget refs
  const streamBufferRef = useRef<string>('');
  const streamWidgetRef = useRef<any>(null);
  const streamPositionRef = useRef<any>(null);
  const streamUnlistenRef = useRef<Array<(() => void) | undefined>>([]);
  const streamKeyHandlerRef = useRef<((e: KeyboardEvent) => void) | null>(null);

  // A5 Phase 3 Task 2: 云端 AI 离线降级
  // 设计依据：.trae/rules/项目核心设计意图.md §三（Yuan Code 走云端 API，离线不切本地 ollama）
  // 离线时跳过 yuan_inline_complete / 流式补全 IPC 调用（避免触发云端 API 请求），
  // Monaco 编辑器的本地补全（单词/片段/LSP）不受影响。
  // 使用 ref 让 monaco 闭包内始终读取最新 isOnline 值。
  const { isOnline } = useNetworkStatus();
  const isOnlineRef = useRef(isOnline);
  useEffect(() => {
    isOnlineRef.current = isOnline;
  }, [isOnline]);

  // Inline edit state
  const [showInlineEdit, setShowInlineEdit] = useState(false);
  const [inlineEditSelectedCode, setInlineEditSelectedCode] = useState('');

  // 文件树存储相对路径，后端需要绝对路径 — 统一转换
  const resolvedFilePath = activeTab && workspacePath ? `${workspacePath.replace(/\\/g, '/')}/${activeTab.path.replace(/\\/g, '/')}` : activeTab?.path || '';

  // 全局键盘快捷键：Ctrl+F / Ctrl+H
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
        e.preventDefault();
        setShowFind(true);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // LSP 诊断：文件变化时获取诊断信息
  useEffect(() => {
    if (!activeTab || !workspacePath) return;
    const fetchDiagnostics = async () => {
      const diagnostics = await getDiagnostics(resolvedFilePath, workspacePath);
      if (!monacoRef.current || !editorRef.current) return;
      const model = editorRef.current.getModel();
      if (!model) return;
      const markers = toMonacoMarkers(diagnostics);
      monacoRef.current.editor.setModelMarkers(model, 'lsp', markers);
    };
    fetchDiagnostics();
  }, [activeTab?.path, workspacePath]);

  // ===== beforeMount：编辑器创建前定义主题（必须在渲染前完成） =====
  const handleBeforeMount = useCallback((monaco: Monaco) => {
    // 先尝试重新定义（热更新时主题已存在会报错，需先清除）
    try {
      (monaco.editor as any)._definedThemes?.delete('nexterm-dark');
    } catch {/* no-op */}
    try {
      monaco.editor.defineTheme('nexterm-dark', NEXTERM_THEME);
    } catch {
      // 主题已存在，直接覆盖
      try {
        ;
        (monaco as any).editor['_themeService']?._setTheme?.('nexterm-dark', NEXTERM_THEME);
      } catch {}
    }

    // 禁用内置 JS/TS 诊断
    try {
      const ts = monaco.languages.typescript as any;
      ts.javascriptDefaults?.setDiagnosticsOptions?.({
        noSemanticValidation: true,
        noSyntaxValidation: true
      });
      ts.typescriptDefaults?.setDiagnosticsOptions?.({
        noSemanticValidation: true,
        noSyntaxValidation: true
      });
    } catch {}

    // ===== 持久化样式注入：VSCode 方案 — CSS 变量 + box-shadow =====
    // Monaco 虚拟滚动会重建 DOM，必须用 CSS 变量覆盖才能持久生效
    let el = document.getElementById('nexterm-editor-overrides');
    if (!el) {
      el = document.createElement('style');
      el.id = 'nexterm-editor-overrides';
      document.head.appendChild(el);
    }
    el.textContent = [/* ========== 第1层：CSS 变量持久覆盖（Monaco 每次创建元素都会读取这些变量） ========== */
    /* 缩进引导线变量 — 所有 lvl 统一 #404040 */
    '.monaco-editor .core-guide-indent {', '  --indent-color: #404040 !important;', '  --indent-color-active: #707070 !important;', '}', /* 括号对引导线变量 */
    '.monaco-editor .vertical,', '.monaco-editor .horizontal-top,', '.monaco-editor .horizontal-bottom {', '  --guide-color: #404040 !important;', '  --guide-color-active: #707070 !important;', '}', /* ========== 第2层：box-shadow 强制渲染（VSCode 原生方式） ========== */
    /* 普通缩进线 */
    '.monaco-editor .lines-content .core-guide-indent,', '.monaco-editor .lines-content .indent-guide,', '.monaco-editor .lines-content > .cigrd,', '.monaco-editor .cigrd,', '.monaco-editor .indent-guide,', '.monaco-editor .bracket-pair-guide,', '.monaco-editor .vertical:not(.indent-active):not(.bracket-pair-guide-active) {', '  box-shadow: 1px 0 0 0 #404040 inset !important;', '  background: transparent !important;', '}', /* active 行缩进线 */
    '.monaco-editor .lines-content .core-guide-indent.indent-active,', '.monaco-editor .lines-content .indent-guide-active,', '.monaco-editor .active-line .indent-guide,', '.monaco-editor .active-line .cigrd,', '.monaco-editor .bracket-pair-guide-active,', '.monaco-editor .bracket-pair-guide-active.in-active-line,', '.monaco-editor .indent-active,', '.monaco-editor .vertical.indent-active,', '.monaco-editor .vertical.bracket-pair-guide-active {', '  box-shadow: 1px 0 0 0 #707070 inset !important;', '  background: transparent !important;', '}', /* ========== 第3层：所有 lvl-* 变体兜底 ========== */
    '.monaco-editor .core-guide-indent[class*="lvl-"] {', '  --indent-color: #404040 !important;', '  box-shadow: 1px 0 0 0 var(--indent-color, #404040) inset !important;', '}', '.monaco-editor .core-guide-indent[class*="lvl-"].indent-active {', '  --indent-color-active: #707070 !important;', '  box-shadow: 1px 0 0 0 var(--indent-color-active, #707070) inset !important;', '}', /* ========== 第4层：VSCode 原生方式 — --vscode-* CSS 变量 ========== */
    /* 滚动条（对应 VSCode scrollbars.css 中的 var() 引用） */
    ':root, .monaco-editor {', '  --vscode-scrollbarSlider-background: rgba(80, 85, 100, 0.35) !important;', '  --vscode-scrollbarSlider-hoverBackground: rgba(90, 95, 115, 0.50) !important;', '  --vscode-scrollbarSlider-activeBackground: rgba(100, 105, 130, 0.65) !important;', '  --vscode-scrollbar-shadow: transparent !important;', '}', /* Minimap slider（对应 VSCode minimap.css 中的 var() 引用） */
    '.monaco-editor .minimap {', '  --vscode-minimapSlider-background: rgba(80, 85, 100, 0.35) !important;', '  --vscode-minimapSlider-hoverBackground: rgba(90, 95, 115, 0.50) !important;', '  --vscode-minimapSlider-activeBackground: rgba(100, 105, 130, 0.65) !important;', '}', /* ========== 第5层：Webkit scrollbar — VSCode 默认尺寸（垂直14px/水平12px） ========== */
    /* 只设轨道宽度，滑块尺寸由 Monaco 动态计算（与内容比例相关） */
    '.monaco-editor::-webkit-scrollbar { width: 14px !important; height: 12px !important; }', '.monaco-editor::-webkit-scrollbar-track { background: transparent !important; }', '.monaco-editor::-webkit-scrollbar-thumb { background: rgba(80, 85, 100, 0.35) !important; border-radius: 2px; }', '.monaco-editor::-webkit-scrollbar-thumb:hover { background: rgba(90, 95, 115, 0.50) !important; }', '.monaco-editor::-webkit-scrollbar-thumb:active { background: rgba(100, 105, 130, 0.65) !important; }', /* 外层容器滚动条（页面级）— 同 VSCode 尺寸 */
    '.editorBody::-webkit-scrollbar { width: 14px !important; height: 12px !important; }', '.editorBody::-webkit-scrollbar-track { background: transparent; }', '.editorBody::-webkit-scrollbar-thumb { background: rgba(80, 85, 100, 0.35) !important; border-radius: 2px; }'].join('\n');

    // MutationObserver：监听 DOM 变化，对新创建的缩进线立即改色
    if (!(window as any).__nextermIndentObserver) {
      const observer = new MutationObserver(mutations => {
        for (const m of mutations) {
          for (const node of m.addedNodes) {
            if (node.nodeType === 1) {
              const el = node as HTMLElement;
              const cls = el.className || '';
              const isGuide = typeof cls === 'string' && (cls.includes('indent') || cls.includes('cigrd') || cls.includes('guide') || cls.includes('core-guide'));
              if (isGuide) {
                const isActive = typeof cls === 'string' && cls.includes('active');
                el.style.setProperty('box-shadow', isActive ? '1px 0 0 0 #707070 inset' : '1px 0 0 0 #404040 inset', 'important');
                el.style.setProperty('background', 'transparent', 'important');
              }
              // 滚动条滑块：只覆盖颜色，不碰尺寸（Monaco 动态计算滑块长度）
              if (typeof cls === 'string' && cls.includes('slider')) {
                el.style.setProperty('background', 'rgba(80, 85, 100, 0.35)', 'important');
              }
            }
          }
        }
      });
      observer.observe(document.body, {
        childList: true,
        subtree: true
      });
      (window as any).__nextermIndentObserver = observer;
    }
    console.log('[NexTerm] Theme nexterm-dark defined, indent guides forced to gray');
  }, []);
  const handleEditorMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;
    monacoRef.current = monaco;
    onEditorMount?.(editor);

    // 确保主题生效（beforeMount 已定义，这里再确认一次）
    monaco.editor.setTheme('nexterm-dark');

    // 注册内置代码片段
    SnippetStore.registerBuiltIn(BUILT_IN_SNIPPETS);

    // ============ D1.2 流式补全 ============
    // 更新 ghost text widget（显示当前累积的流式文本）
    const updateStreamWidget = () => {
      if (streamWidgetRef.current) {
        editor.removeContentWidget(streamWidgetRef.current);
        streamWidgetRef.current = null;
      }
      if (!streamBufferRef.current) return;
      const widget = {
        getId: () => 'yuan-stream-ghost-widget',
        getDomNode: () => {
          const node = document.createElement('div');
          node.className = 'yuan-stream-ghost';
          node.style.color = 'rgba(0, 255, 0, 0.45)';
          node.style.fontFamily = 'var(--font-mono, Consolas, monospace)';
          node.style.fontSize = '14px';
          node.style.whiteSpace = 'pre';
          node.style.padding = '0 4px';
          node.style.pointerEvents = 'none';
          node.style.background = 'rgba(0, 255, 0, 0.05)';
          node.style.borderLeft = '2px solid rgba(0, 255, 0, 0.4)';
          node.innerText = streamBufferRef.current;
          return node;
        },
        getPosition: () => ({
          position: streamPositionRef.current,
          preference: [monaco.editor.ContentWidgetPositionPreference.EXACT]
        })
      };
      streamWidgetRef.current = widget;
      editor.addContentWidget(widget);
    };

    // 接受流式补全（Tab 键）
    const acceptStreamCompletion = () => {
      if (!streamBufferRef.current || !streamPositionRef.current) return;
      const pos = streamPositionRef.current;
      editor.executeEdits('yuan-stream-accept', [{
        range: new monaco.Range(pos.lineNumber, pos.column, pos.lineNumber, pos.column),
        text: streamBufferRef.current,
        forceMoveMarkers: true
      }]);
      if (streamWidgetRef.current) {
        editor.removeContentWidget(streamWidgetRef.current);
        streamWidgetRef.current = null;
      }
      streamBufferRef.current = '';
      streamPositionRef.current = null;
      streamUnlistenRef.current.forEach(fn => fn?.());
      streamUnlistenRef.current = [];
      // 移除 keydown 监听器
      if (streamKeyHandlerRef.current) {
        document.removeEventListener('keydown', streamKeyHandlerRef.current, true);
        streamKeyHandlerRef.current = null;
      }
    };

    // 取消流式补全（Esc 键）
    const cancelStreamCompletion = () => {
      if (streamWidgetRef.current) {
        editor.removeContentWidget(streamWidgetRef.current);
        streamWidgetRef.current = null;
      }
      streamBufferRef.current = '';
      streamPositionRef.current = null;
      streamUnlistenRef.current.forEach(fn => fn?.());
      streamUnlistenRef.current = [];
      // 移除 keydown 监听器
      if (streamKeyHandlerRef.current) {
        document.removeEventListener('keydown', streamKeyHandlerRef.current, true);
        streamKeyHandlerRef.current = null;
      }
    };

    // 注册 keydown 监听器（仅在流式补全激活时拦截 Tab/Esc）
    const registerStreamKeyHandler = () => {
      if (streamKeyHandlerRef.current) return;
      const handler = (e: KeyboardEvent) => {
        if (e.key === 'Tab' && streamBufferRef.current) {
          e.preventDefault();
          e.stopPropagation();
          acceptStreamCompletion();
        } else if (e.key === 'Escape' && streamBufferRef.current) {
          e.preventDefault();
          e.stopPropagation();
          cancelStreamCompletion();
        }
      };
      streamKeyHandlerRef.current = handler;
      document.addEventListener('keydown', handler, true);
    };

    // Alt+\ 触发流式补全
    editor.addAction({
      id: 'yuan-complete-stream',
      label: t('yuan-code.EditorPanel.streamComplete') || 'AI Stream Complete',
      keybindings: [monaco.KeyMod.Alt | monaco.KeyCode.Backslash],
      run: async (ed: any) => {
        const model = ed.getModel();
        if (!model) return;
        const position = ed.getPosition();
        if (!position) return;

        // A5 Phase 3 Task 2: 离线时不触发云端 AI 流式补全
        if (!isOnlineRef.current) return;

        // 清理上一次的 widget
        cancelStreamCompletion();

        streamPositionRef.current = position;
        streamBufferRef.current = '';

        // 注册 keydown 监听器（拦截 Tab/Esc）
        registerStreamKeyHandler();

        // 准备上下文（前后各 5 行）
        const lineCount = model.getLineCount();
        const lineContent = model.getLineContent(position.lineNumber);
        const beforeCursor = lineContent.substring(0, position.column - 1);
        const afterCursor = lineContent.substring(position.column - 1);
        const contextBeforeLines: string[] = [];
        for (let i = Math.max(1, position.lineNumber - 5); i < position.lineNumber; i++) {
          contextBeforeLines.push(model.getLineContent(i));
        }
        contextBeforeLines.push(beforeCursor);
        const contextAfterLines: string[] = [afterCursor];
        for (let i = position.lineNumber + 1; i <= Math.min(lineCount, position.lineNumber + 5); i++) {
          contextAfterLines.push(model.getLineContent(i));
        }

        // 注册事件监听器
        try {
          const { listen } = await import('@tauri-apps/api/event');
          const unlistenStream = await listen('ai-stream', (event: any) => {
            // 只处理 yuan code 流式（conversation_id === -1）
            if (event.payload?.conversation_id !== -1) return;
            const chunk = event.payload?.chunk || '';
            if (chunk) {
              streamBufferRef.current += chunk;
              updateStreamWidget();
            }
          });
          const unlistenDone = await listen('yuan-code-stream-done', () => {
            // 流式完成，保留 widget 等待用户 Tab/Esc
          });
          streamUnlistenRef.current = [unlistenStream, unlistenDone];
        } catch {
          // 监听失败，静默
        }

        // 调用流式补全命令
        try {
          await ipc.invoke('yuan_complete_stream', {
            request: {
              file_path: resolvedFilePath,
              language: model.getLanguageId(),
              cursor_line: position.lineNumber - 1,
              cursor_column: position.column - 1,
              context_before: contextBeforeLines.join('\n'),
              context_after: contextAfterLines.join('\n'),
              model_id: yuanModelId ?? undefined
            }
          });
        } catch {
          cancelStreamCompletion();
        }
      }
    });

    // ============ D1.2 流式补全 END ============

    editor.addAction({
      id: 'save-file',
      label: t("components.TextEditor.k11"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
      run: () => onSave()
    });
    editor.addAction({
      id: 'run-code',
      label: t("yuan-code.EditorPanel.k1"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter],
      run: () => onRun()
    });
    editor.addAction({
      id: 'close-tab',
      label: t("yuan-code.EditorPanel.k2"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyW],
      run: () => onTabClose(activeTabIndex)
    });
    editor.addAction({
      id: 'go-to-line',
      label: t("yuan-code.EditorPanel.k3"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyG],
      run: () => editor.getAction('editor.action.gotoLine')?.run()
    });

    // 多光标: 向上添加光标
    editor.addAction({
      id: 'add-cursor-above',
      label: t("yuan-code.EditorPanel.k4"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Alt | monaco.KeyCode.UpArrow],
      run: () => editor.getAction('editor.action.insertCursorAbove')?.run()
    });

    // 多光标: 向下添加光标
    editor.addAction({
      id: 'add-cursor-below',
      label: t("yuan-code.EditorPanel.k5"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Alt | monaco.KeyCode.DownArrow],
      run: () => editor.getAction('editor.action.insertCursorBelow')?.run()
    });

    // 多光标: 选中所有匹配项
    editor.addAction({
      id: 'select-all-occurrences',
      label: t("yuan-code.EditorPanel.k6"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyL],
      run: () => editor.getAction('editor.action.selectAllOccurrencesOfFindMatch')?.run()
    });

    // 多光标: 添加下一个匹配项
    editor.addAction({
      id: 'add-next-occurrence',
      label: t("yuan-code.EditorPanel.k7"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyD],
      run: () => editor.getAction('editor.action.addSelectionToNextFindMatch')?.run()
    });

    // 多光标: 撤销上一个光标
    editor.addAction({
      id: 'cursor-undo',
      label: t("yuan-code.EditorPanel.k8"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyU],
      run: () => editor.getAction('cursorUndo')?.run()
    });

    // AI 内联编辑: Ctrl+K
    editor.addAction({
      id: 'ai-inline-edit',
      label: t("yuan-code.EditorPanel.k9"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK],
      run: () => {
        const selection = editor.getSelection();
        if (!selection || selection.isEmpty()) {
          // No selection - could show a hint
          return;
        }
        const selectedCode = editor.getModel()?.getValueInRange(selection);
        if (selectedCode) {
          inlineEditSelectionRef.current = selection;
          setInlineEditSelectedCode(selectedCode);
          setShowInlineEdit(true);
        }
      }
    });

    // LSP + 代码片段 补全提供器（含补全记忆 + Markdown 详情）
    const completionProvider = monaco.languages.registerCompletionItemProvider('*', {
      provideCompletionItems: async (model: editor.ITextModel, position: IPosition) => {
        const word = model.getWordUntilPosition(position);
        const range = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn
        };
        const suggestions: any[] = [];
        const lang = model.getLanguageId();

        // 1. 内置代码片段
        const snippets = SnippetStore.getByScope(lang);
        for (const s of snippets) {
          suggestions.push({
            label: s.prefix,
            kind: monaco.languages.CompletionItemKind.Snippet,
            detail: s.name,
            documentation: s.description,
            insertText: s.body,
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            sortText: '0_' + s.prefix,
            range
          });
        }

        // 2. LSP 补全（含补全记忆排序）
        const items = await getCompletions(resolvedFilePath, position.lineNumber - 1, position.column - 1, workspacePath);
        if (items.length > 0) {
          suggestions.push(...items.map(item => {
            const label = item.label;
            const memoryScore = completionMemory.getScore(label, lang);
            const baseSort = item.sort_text || label;
            // 记忆分数越高，排序越靠前（0-99 补全记忆提升，100+ LSP 原始排序）
            const boostPrefix = memoryScore > 0 ? String(99 - Math.min(99, memoryScore)).padStart(2, '0') : '99';
            return {
              label,
              kind: mapLspKind(item.kind),
              detail: item.detail,
              documentation: item.documentation ? {
                value: item.documentation,
                isTrusted: true
              } : undefined,
              insertText: item.insert_text || label,
              sortText: `1_${boostPrefix}_${baseSort}`,
              range,
              // 补全记忆：选中时记录
              command: memoryScore > 0 ? undefined : {
                id: 'nexterm.recordCompletion',
                title: t("yuan-code.EditorPanel.k10"),
                arguments: [label, lang]
              }
            };
          }));
        }
        return {
          suggestions
        };
      },
      resolveCompletionItem: async (item: any) => {
        // 补全记忆：当用户选中补全项时记录
        if (item.command?.id === 'nexterm.recordCompletion') {
          const [label, lang] = item.command.arguments || [];
          if (label && lang) {
            completionMemory.record(label, lang);
          }
        }
        return item;
      }
    });

    // LSP 悬停提供器（Markdown 渲染）
    const hoverProvider = monaco.languages.registerHoverProvider('*', {
      provideHover: async (_model: editor.ITextModel, position: IPosition) => {
        const result = await getHover(resolvedFilePath, position.lineNumber - 1, position.column - 1, workspacePath);
        if (!result) return null;
        return {
          contents: [{
            value: result.contents,
            isTrusted: true,
            supportHtml: true
          }],
          range: result.range ? {
            startLineNumber: result.range.start.line + 1,
            startColumn: result.range.start.character + 1,
            endLineNumber: result.range.end.line + 1,
            endColumn: result.range.end.character + 1
          } : undefined
        };
      }
    });

    // LSP 跳转定义提供器
    const definitionProvider = monaco.languages.registerDefinitionProvider('*', {
      provideDefinition: async (_model: editor.ITextModel, position: IPosition) => {
        const locations = await getDefinition(resolvedFilePath, position.lineNumber - 1, position.column - 1, workspacePath);
        return locations.map(loc => ({
          uri: monaco.Uri.parse(loc.uri),
          range: {
            startLineNumber: loc.range.start.line + 1,
            startColumn: loc.range.start.character + 1,
            endLineNumber: loc.range.end.line + 1,
            endColumn: loc.range.end.character + 1
          }
        }));
      }
    });

    // Quick Fix 代码操作提供器（灯泡图标）
    const codeActionProvider = monaco.languages.registerCodeActionProvider('*', {
      provideCodeActions: async (model: editor.ITextModel, _range: any, context: any) => {
        const actions: any[] = [];
        const lang = model.getLanguageId();
        for (const marker of context.markers || []) {
          const msg = (marker.message || '').toLowerCase();
          const line = marker.startLineNumber;

          // 未使用变量/导入修复
          if (msg.includes('unused') || msg.includes('not used') || msg.includes('never used')) {
            if (msg.includes('import') || msg.includes('unused import')) {
              actions.push({
                title: t("yuan-code.EditorPanel.k11"),
                kind: 'quickfix',
                isPreferred: true,
                edit: {
                  edits: [{
                    resource: model.uri,
                    textEdit: {
                      range: {
                        startLineNumber: line,
                        startColumn: 1,
                        endLineNumber: line + 1,
                        endColumn: 1
                      },
                      text: ''
                    },
                    versionId: model.getVersionId()
                  }]
                }
              });
            }
          }

          // 缺失分号修复
          if (msg.includes('missing semicolon') || msg.includes("expected ';'") || msg.includes('; expected')) {
            const lineContent = model.getLineContent(line);
            const trimmedLen = lineContent.trimEnd().length;
            actions.push({
              title: t("yuan-code.EditorPanel.k12"),
              kind: 'quickfix',
              edit: {
                edits: [{
                  resource: model.uri,
                  textEdit: {
                    range: {
                      startLineNumber: line,
                      startColumn: (trimmedLen || 1) + 1,
                      endLineNumber: line,
                      endColumn: (trimmedLen || 1) + 1
                    },
                    text: ';'
                  },
                  versionId: model.getVersionId()
                }]
              }
            });
          }

          // 缩进修复
          if (msg.includes('indent') || msg.includes('indentation') || msg.includes('expected indented')) {
            const lineContent = model.getLineContent(line);
            const indent = lineContent.match(/^(\s*)/)?.[1] || '';
            const prefix = lang === 'python' ? indent + '    ' : indent + '  ';
            actions.push({
              title: t("yuan-code.EditorPanel.k13"),
              kind: 'quickfix',
              edit: {
                edits: [{
                  resource: model.uri,
                  textEdit: {
                    range: {
                      startLineNumber: line,
                      startColumn: 1,
                      endLineNumber: line,
                      endColumn: lineContent.length + 1
                    },
                    text: prefix + lineContent.trimStart()
                  },
                  versionId: model.getVersionId()
                }]
              }
            });
          }
        }
        return {
          actions,
          dispose: () => {}
        };
      }
    });

    // Inlay Hints 提供器（类型推断提示）
    const inlayHintsProvider = monaco.languages.registerInlayHintsProvider('*', {
      provideInlayHints: async (model: editor.ITextModel, _range: any) => {
        const hints: any[] = [];
        const lang = model.getLanguageId();
        const lineCount = model.getLineCount();

        // 支持的语言
        const supported = ['typescript', 'javascript', 'python', 'rust', 'go', 'java'];
        if (!supported.includes(lang)) return {
          hints,
          dispose: () => {}
        };
        for (let i = 1; i <= Math.min(lineCount, 1000); i++) {
          const line = model.getLineContent(i);
          const trimmed = line.trim();

          // TypeScript/JavaScript: 变量声明类型推断
          if ((lang === 'typescript' || lang === 'javascript') && trimmed.match(/^(const|let|var)\s+\w+\s*=\s*.+/)) {
            const varMatch = trimmed.match(/^(const|let|var)\s+(\w+)\s*=\s*(.+)/);
            if (varMatch) {
              const value = varMatch[3].trim();
              let typeHint = '';
              if (/^\d+$/.test(value)) typeHint = ': number';else if (/^['"`]/.test(value)) typeHint = ': string';else if (/^(true|false)$/.test(value)) typeHint = ': boolean';else if (/^\[/.test(value)) typeHint = ': []';else if (/^\{/.test(value)) typeHint = ': {}';else if (/^new\s+/.test(value)) typeHint = ': ' + value.replace(/^new\s+/, '');else if (/^[A-Z]/.test(value) && value.match(/^[A-Z]\w+\./)) typeHint = ': ' + value.split('.')[0];
              if (typeHint) {
                const col = line.indexOf('=') + 1;
                hints.push({
                  kind: monaco.languages.InlayHintKind.Type,
                  position: {
                    lineNumber: i,
                    column: col
                  },
                  label: typeHint,
                  paddingLeft: true
                });
              }
            }
          }

          // Python: 函数参数类型提示
          if (lang === 'python' && trimmed.match(/^def\s+\w+\s*\(/)) {
            const paramMatch = trimmed.match(/def\s+\w+\(([^)]*)\)/);
            if (paramMatch && paramMatch[1].trim()) {
              const params = paramMatch[1].split(',').map((p: string) => p.trim());
              for (const param of params) {
                const nameMatch = param.match(/^(\w+)/);
                if (nameMatch) {
                  const paramName = nameMatch[1];
                  // 简单类型推断
                  let typeHint = '';
                  if (paramName.startsWith('is_') || paramName.startsWith('has_') || paramName.startsWith('can_')) typeHint = ': bool';else if (paramName.endsWith('_id') || paramName.endsWith('_num') || paramName.endsWith('_count')) typeHint = ': int';else if (paramName.endsWith('_name') || paramName.endsWith('_path') || paramName.endsWith('_str')) typeHint = ': str';else if (paramName.endsWith('_list') || paramName.endsWith('s') && paramName.length > 3) typeHint = ': list';
                  if (typeHint) {
                    const col = line.indexOf(paramName) + paramName.length + 1;
                    hints.push({
                      kind: monaco.languages.InlayHintKind.Parameter,
                      position: {
                        lineNumber: i,
                        column: col
                      },
                      label: typeHint,
                      paddingLeft: true
                    });
                  }
                }
              }
            }
          }
        }
        return {
          hints,
          dispose: () => {}
        };
      }
    });

    // 颜色装饰器提供器（hex/rgb/rgba/hsl 色块预览）
    const colorProvider = monaco.languages.registerColorProvider('*', {
      provideColorPresentations: (_model: any, colorInfo: any) => {
        const {
          red,
          green,
          blue,
          alpha
        } = colorInfo.color;
        const r = Math.round(red * 255);
        const g = Math.round(green * 255);
        const b = Math.round(blue * 255);
        const a = alpha ?? 1;
        const presentations = [{
          label: `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`.toUpperCase()
        }, {
          label: `rgb(${r}, ${g}, ${b})`
        }];
        if (a < 1) {
          presentations.push({
            label: `rgba(${r}, ${g}, ${b}, ${a})`
          });
        }
        return presentations;
      },
      provideDocumentColors: async (model: any) => {
        const colors: any[] = [];
        const fullText = model.getValue();
        const lang = model.getLanguageId();

        // 支持的语言
        const supported = ['css', 'scss', 'less', 'html', 'javascript', 'typescript', 'typescriptreact', 'javascriptreact', 'python', 'rust', 'json'];
        if (!supported.includes(lang)) return colors;

        // 匹配 hex 颜色: #RGB, #RRGGBB, #RRGGBBAA
        const hexRegex = /#([0-9a-fA-F]{3,8})\b/g;
        let match: RegExpExecArray | null;
        while ((match = hexRegex.exec(fullText)) !== null) {
          const hex = match[1];
          let r = 0,
            g = 0,
            b = 0;
          let alpha = 1;
          if (hex.length === 3) {
            r = parseInt(hex[0] + hex[0], 16) / 255;
            g = parseInt(hex[1] + hex[1], 16) / 255;
            b = parseInt(hex[2] + hex[2], 16) / 255;
          } else if (hex.length === 6) {
            r = parseInt(hex.substring(0, 2), 16) / 255;
            g = parseInt(hex.substring(2, 4), 16) / 255;
            b = parseInt(hex.substring(4, 6), 16) / 255;
          } else if (hex.length === 8) {
            r = parseInt(hex.substring(0, 2), 16) / 255;
            g = parseInt(hex.substring(2, 4), 16) / 255;
            b = parseInt(hex.substring(4, 6), 16) / 255;
            alpha = parseInt(hex.substring(6, 8), 16) / 255;
          }
          const pos = model.getPositionAt(match.index);
          colors.push({
            color: {
              red: r,
              green: g,
              blue: b,
              alpha
            },
            range: {
              startLineNumber: pos.lineNumber,
              startColumn: pos.column,
              endLineNumber: pos.lineNumber,
              endColumn: pos.column + match[0].length
            }
          });
        }

        // 匹配 rgb/rgba: rgb(255, 0, 128) / rgba(255, 0, 128, 0.5)
        const rgbRegex = /rgba?\s*\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*(?:,\s*([\d.]+))?\s*\)/g;
        while ((match = rgbRegex.exec(fullText)) !== null) {
          const r = Math.min(255, parseInt(match[1])) / 255;
          const g = Math.min(255, parseInt(match[2])) / 255;
          const b = Math.min(255, parseInt(match[3])) / 255;
          const alpha = match[4] ? parseFloat(match[4]) : 1;
          const pos = model.getPositionAt(match.index);
          colors.push({
            color: {
              red: r,
              green: g,
              blue: b,
              alpha
            },
            range: {
              startLineNumber: pos.lineNumber,
              startColumn: pos.column,
              endLineNumber: pos.lineNumber,
              endColumn: pos.column + match[0].length
            }
          });
        }

        // 匹配 hsl/hsla: hsl(240, 100%, 50%) / hsla(240, 100%, 50%, 0.5)
        const hslRegex = /hsla?\s*\(\s*(\d{1,3})\s*,\s*(\d{1,3})%\s*,\s*(\d{1,3})%\s*(?:,\s*([\d.]+))?\s*\)/g;
        while ((match = hslRegex.exec(fullText)) !== null) {
          const h = parseInt(match[1]) / 360;
          const s = parseInt(match[2]) / 100;
          const l = parseInt(match[3]) / 100;
          const alpha = match[4] ? parseFloat(match[4]) : 1;

          // HSL to RGB
          let r = 0,
            g = 0,
            b = 0;
          if (s === 0) {
            r = g = b = l;
          } else {
            const hue2rgb = (p: number, q: number, t: number) => {
              if (t < 0) t += 1;
              if (t > 1) t -= 1;
              if (t < 1 / 6) return p + (q - p) * 6 * t;
              if (t < 1 / 2) return q;
              if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
              return p;
            };
            const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
            const p = 2 * l - q;
            r = hue2rgb(p, q, h + 1 / 3);
            g = hue2rgb(p, q, h);
            b = hue2rgb(p, q, h - 1 / 3);
          }
          const pos = model.getPositionAt(match.index);
          colors.push({
            color: {
              red: r,
              green: g,
              blue: b,
              alpha
            },
            range: {
              startLineNumber: pos.lineNumber,
              startColumn: pos.column,
              endLineNumber: pos.lineNumber,
              endColumn: pos.column + match[0].length
            }
          });
        }
        return colors;
      }
    });

    // 格式化文档提供器
    const formatProvider = monaco.languages.registerDocumentFormattingEditProvider('*', {
      async provideDocumentFormattingEdits(model: editor.ITextModel) {
        const lang = model.getLanguageId();
        const content = model.getValue();
        const filePath = resolvedFilePath;
        try {
          const res = await ipc.invoke<{
            changed: boolean;
            formatted: string;
          }>('yuan_format_code', {
            request: {
              file_path: filePath,
              language: lang,
              content
            }
          });
          if (res.data?.changed) {
            return [{
              range: model.getFullModelRange(),
              text: res.data.formatted
            }];
          }
        } catch {
          // 格式化失败，静默返回
        }
        return [];
      }
    });

    // 格式化快捷键: Shift+Alt+F
    editor.addAction({
      id: 'format-document',
      label: t("yuan-code.EditorPanel.k14"),
      keybindings: [monaco.KeyMod.Shift | monaco.KeyMod.Alt | monaco.KeyCode.KeyF],
      run: () => editor.getAction('editor.action.formatDocument')?.run()
    });

    // 智能选区: Shift+Alt+Right 扩展选区
    editor.addAction({
      id: 'smart-select-expand',
      label: t("yuan-code.EditorPanel.k15"),
      keybindings: [monaco.KeyMod.Shift | monaco.KeyMod.Alt | monaco.KeyCode.RightArrow],
      run: () => editor.getAction('editor.action.smartSelect.expand')?.run()
    });

    // 智能选区: Shift+Alt+Left 收缩选区
    editor.addAction({
      id: 'smart-select-shrink',
      label: t("yuan-code.EditorPanel.k16"),
      keybindings: [monaco.KeyMod.Shift | monaco.KeyMod.Alt | monaco.KeyCode.LeftArrow],
      run: () => editor.getAction('editor.action.smartSelect.shrink')?.run()
    });

    // Peek 定义: Alt+F12
    editor.addAction({
      id: 'peek-definition',
      label: t("yuan-code.EditorPanel.k17"),
      keybindings: [monaco.KeyMod.Alt | monaco.KeyCode.F12],
      run: () => editor.getAction('editor.action.peekDefinition')?.run()
    });

    // Peek 引用: Shift+F12
    editor.addAction({
      id: 'peek-references',
      label: t("yuan-code.EditorPanel.k18"),
      keybindings: [monaco.KeyCode.F12],
      run: () => editor.getAction('editor.action.peekReferences')?.run()
    });

    // Quick Fix: Ctrl+.
    editor.addAction({
      id: 'quick-fix',
      label: t("yuan-code.EditorPanel.k19"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Period],
      run: () => editor.getAction('editor.action.quickFix')?.run()
    });

    // 行内补全提供器 (Ghost Text)
    const inlineProvider = monaco.languages.registerInlineCompletionsProvider('*', {
      provideInlineCompletions: async (model: editor.ITextModel, position: IPosition, _context: any, _token: any) => {
        // A5 Phase 3 Task 2: 离线时跳过云端 AI 补全，回退到 Monaco 本地补全
        if (!isOnlineRef.current) return { items: [] };
        try {
          const lineCount = model.getLineCount();
          const lineContent = model.getLineContent(position.lineNumber);
          const beforeCursor = lineContent.substring(0, position.column - 1);
          const afterCursor = lineContent.substring(position.column - 1);

          // 获取上下文（当前行前后各 5 行）
          const contextBeforeLines: string[] = [];
          for (let i = Math.max(1, position.lineNumber - 5); i < position.lineNumber; i++) {
            contextBeforeLines.push(model.getLineContent(i));
          }
          contextBeforeLines.push(beforeCursor);
          const contextAfterLines: string[] = [afterCursor];
          for (let i = position.lineNumber + 1; i <= Math.min(lineCount, position.lineNumber + 5); i++) {
            contextAfterLines.push(model.getLineContent(i));
          }
          const res = await ipc.invoke<{
            items: Array<{
              insert_text: string;
              range: {
                start_line: number;
                start_character: number;
                end_line: number;
                end_character: number;
              };
              filter_text?: string;
              sort_text?: string;
            }>;
            source: string;
          }>('yuan_inline_complete', {
            request: {
              file_path: resolvedFilePath,
              language: model.getLanguageId(),
              position: {
                line: position.lineNumber - 1,
                character: position.column - 1
              },
              context_before: contextBeforeLines.join('\n'),
              context_after: contextAfterLines.join('\n'),
              // D1.1：传入统一模型管理中选中的模型 ID
              model_id: yuanModelId ?? undefined
            }
          });
          if (res.code === 0 && res.data && res.data.items.length > 0) {
            return {
              items: res.data.items.map(item => ({
                insertText: item.insert_text,
                range: new monaco.Range((item.range.start_line ?? position.lineNumber - 1) + 1, (item.range.start_character ?? position.column - 1) + 1, (item.range.end_line ?? position.lineNumber - 1) + 1, (item.range.end_character ?? position.column - 1) + 1),
                filterText: item.filter_text,
                command: {
                  id: 'acceptInlineCompletion',
                  title: t("yuan-code.EditorPanel.k20")
                }
              }))
            };
          }
        } catch {
          // 静默失败
        }
        return {
          items: []
        };
      },
      freeInlineCompletions: () => {},
      handleItemDidShow: () => {},
      groupId: 'nexterm-inline'
    });

    // 清理函数
    editor.onDidDispose(() => {
      completionProvider.dispose();
      hoverProvider.dispose();
      definitionProvider.dispose();
      formatProvider.dispose();
      inlineProvider.dispose();
      codeActionProvider.dispose();
      inlayHintsProvider.dispose();
      colorProvider.dispose();
    });

    // 强制覆盖缩进线颜色（Monaco 虚拟滚动会重建 DOM）
    const forceIndentColor = () => {
      const container = editor.getContainerDomNode?.() || document.querySelector('.monaco-editor');
      if (!container) return;
      const selectors = ['.indent-guide', '.cigrd', '.bracket-pair-guide', '.bracket-pair-guide-active', '.indent-guide-active', '.core-guide-indent', '[class*="indent"]', '[class*="guide"]', '[class*="core-guide"]', '.vertical', '.horizontal-top', '.horizontal-bottom'];
      for (const sel of selectors) {
        container.querySelectorAll<HTMLElement>(sel).forEach((el: HTMLElement) => {
          const isActive = el.className && (el.className as string).includes('active');
          el.style.setProperty('box-shadow', isActive ? '1px 0 0 0 #707070 inset' : '1px 0 0 0 #404040 inset', 'important');
          el.style.setProperty('background', 'transparent', 'important');
        });
      }
    };

    // 初始加载多次覆盖
    requestAnimationFrame(() => forceIndentColor());
    setTimeout(() => forceIndentColor(), 200);
    setTimeout(() => forceIndentColor(), 1000);

    // 滚动事件：Monaco 虚拟滚动会重建 DOM，滚动后必须重新覆盖
    let scrollTimer: ReturnType<typeof setTimeout> | null = null;
    editor.onDidScrollChange(() => {
      if (scrollTimer) clearTimeout(scrollTimer);
      scrollTimer = setTimeout(() => forceIndentColor(), 50); // 防抖 50ms
    });
    editor.focus();
  };

  // Inline edit handlers
  const handleInlineAccept = useCallback((modifiedCode: string) => {
    const editor = editorRef.current;
    const selection = inlineEditSelectionRef.current;
    if (editor && selection) {
      editor.executeEdits('ai-inline-edit', [{
        range: selection,
        text: modifiedCode
      }]);
      // Update content
      const newContent = editor.getValue();
      onContentChange(newContent);
    }
    setShowInlineEdit(false);
    inlineEditSelectionRef.current = null;
  }, [onContentChange]);
  const handleInlineReject = useCallback(() => {
    setShowInlineEdit(false);
    inlineEditSelectionRef.current = null;
  }, []);
  const handleInlineClose = useCallback(() => {
    setShowInlineEdit(false);
    inlineEditSelectionRef.current = null;
  }, []);
  const handleChange = useCallback((value: string | undefined) => {
    if (value !== undefined) {
      onContentChange(value);
    }
  }, [onContentChange]);
  if (openTabs.length === 0) {
    return <div className={styles.editorArea}>
        <div className={styles.placeholder}>
          {/* T2.12 XSS 审查：此处 SVG 为静态字符串常量，非用户输入，理论安全 */}
          <div className={styles.placeholderLogo} dangerouslySetInnerHTML={{
          __html: '<svg width="48" height="48" viewBox="0 0 48 48" fill="none"><path d="M4 10h40v28H4z" stroke="rgba(0,240,255,0.2)" strokeWidth="1.5" rx="3"/><path d="M14 18h20M14 24h14M14 30h8" stroke="rgba(0,240,255,0.25)" strokeWidth="1.5" strokeLinecap="round"/></svg>'
        }} />
          <div className={styles.placeholderTitle}>Yuan Code</div>
          <div className={styles.placeholderHint}>{t("yuan-code.EditorPanel.k21")}</div>
          <div className={styles.placeholderShortcuts}>
            <span className={styles.shortcutItem}><kbd>Ctrl</kbd>+<kbd>S</kbd> {t("common.save")}</span>
            <span className={styles.shortcutItem}><kbd>Ctrl</kbd>+<kbd>Enter</kbd> {t("yuan-code.EditorPanel.k22")}</span>
            <span className={styles.shortcutItem}><kbd>Ctrl</kbd>+<kbd>G</kbd> {t("yuan-code.EditorPanel.k23")}</span>
          </div>
        </div>
      </div>;
  }
  return <div className={styles.editorArea}>
      <div className={styles.tabBar}>
        {openTabs.map((tab, idx) => <div key={tab.path} className={`${styles.tab} ${idx === activeTabIndex ? styles.tabActive : ''} ${tab.is_dirty ? styles.tabDirty : ''}`} onClick={() => onTabChange(idx)}>
            <span>{tab.name}</span>
            <button className={styles.tabClose} onClick={e => {
          e.stopPropagation();
          onTabClose(idx);
        }}>
              ✕
            </button>
          </div>)}
      </div>
      <div className={styles.editorBody}>
        <Breadcrumb activeFile={activeTab?.path ?? null} onPathClick={_path => {
        // 可以在这里实现路径导航
      }} />
        {showFind && editorRef.current && <FindReplace editor={editorRef.current} onClose={() => setShowFind(false)} />}
        {showInlineEdit && activeTab && editorRef.current && monacoRef.current && <InlineEdit editor={editorRef.current} selectedCode={inlineEditSelectedCode} language={activeTab.language} filePath={resolvedFilePath} onAccept={handleInlineAccept} onReject={handleInlineReject} onClose={handleInlineClose} modelId={yuanModelId} />}
        {activeTab ? <Editor key={activeTab.path} language={getMonacoLanguage(activeTab.language)} value={activeTab.content} onChange={handleChange} beforeMount={handleBeforeMount} onMount={handleEditorMount} theme="nexterm-dark" loading={<div className={styles.placeholder}>
                <div className={styles.placeholderHint}>{t("yuan-code.EditorPanel.k24")}</div>
              </div>} options={{
        fontSize: editorSettings.fontSize,
        fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
        fontLigatures: true,
        minimap: {
          enabled: editorSettings.showMinimap,
          scale: 1,
          showSlider: 'mouseover'
        },
        lineNumbers: 'on',
        renderLineHighlight: 'line',
        scrollBeyondLastLine: false,
        wordWrap: editorSettings.wordWrap,
        tabSize: editorSettings.tabSize,
        insertSpaces: true,
        bracketPairColorization: {
          enabled: true
        },
        autoClosingBrackets: 'always',
        autoClosingQuotes: 'always',
        folding: true,
        foldingStrategy: 'indentation',
        suggest: {
          showWords: true,
          showSnippets: true
        },
        padding: {
          top: 8
        },
        smoothScrolling: true,
        cursorBlinking: 'smooth',
        cursorSmoothCaretAnimation: 'on',
        roundedSelection: true,
        overviewRulerBorder: false,
        hideCursorInOverviewRuler: true,
        // 选择高亮：选中单词时高亮所有匹配
        occurrencesHighlight: 'singleFile',
        // 颜色装饰器：颜色值旁显示色块预览
        colorDecorators: true,
        // 空白/缩进指南
        renderWhitespace: 'boundary',
        guides: {
          indentation: true,
          bracketPairs: true,
          bracketPairsHorizontal: 'active'
        },
        stickyScroll: {
          enabled: true
        },
        // 多光标编辑
        multiCursorModifier: 'alt',
        columnSelection: true,
        multiCursorMergeOverlapping: true,
        multiCursorPaste: 'spread',
        multiCursorLimit: 10000,
        // 查找
        find: {
          addExtraSpaceOnTop: false,
          autoFindInSelection: 'never',
          seedSearchStringFromSelection: 'always'
        }
      }} /> : <div className={styles.placeholder}>
            <div className={styles.placeholderHint}>{t("yuan-code.EditorPanel.k25")}</div>
          </div>}
      </div>
    </div>;
}