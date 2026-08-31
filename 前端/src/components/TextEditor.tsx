import { t } from "i18next";
import { useState, useRef, useEffect, useCallback, useMemo } from 'react';
import Editor, { type OnMount } from '@monaco-editor/react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { ipc } from '@/lib/ipc';
interface TextEditorProps {
  filePath: string;
  fileExt: string;
  initialContent: string;
  formatType: string;
  fileName: string;
  onSave: () => void;
  onClose: () => void;
  showStatus: (type: 'success' | 'error', text: string) => void;
  wikiEntries?: string[];
  onWikiLinkClick?: (entryName: string) => void;
}
const languageMap: Record<string, string> = {
  md: 'markdown',
  txt: 'plaintext',
  json: 'json',
  xml: 'xml',
  yaml: 'yaml',
  yml: 'yaml',
  toml: 'toml',
  csv: 'plaintext',
  html: 'html',
  htm: 'html',
  css: 'css',
  js: 'javascript',
  ts: 'typescript',
  jsx: 'javascript',
  tsx: 'typescript',
  py: 'python',
  rs: 'rust',
  go: 'go',
  java: 'java',
  c: 'c',
  cpp: 'cpp',
  h: 'c',
  sh: 'shell',
  bash: 'shell',
  sql: 'sql',
  r: 'r',
  lua: 'lua',
  php: 'php',
  rb: 'ruby',
  swift: 'swift',
  scala: 'scala',
  docx: 'xml',
  pptx: 'xml',
  log: 'plaintext',
  ini: 'ini',
  cfg: 'plaintext',
  conf: 'plaintext'
};
type MdViewMode = 'edit' | 'split' | 'preview';
interface ToolbarAction {
  label: string;
  title: string;
  prefix: string;
  suffix: string;
  multiline?: boolean;
}
const toolbarActions: ToolbarAction[] = [{
  label: 'B',
  title: t("components.TextEditor.k1"),
  prefix: '**',
  suffix: '**'
}, {
  label: 'I',
  title: t("components.TextEditor.k2"),
  prefix: '*',
  suffix: '*'
}, {
  label: 'H2',
  title: t("components.TextEditor.k3"),
  prefix: '\n## ',
  suffix: ''
}, {
  label: 'H3',
  title: t("components.TextEditor.k4"),
  prefix: '\n### ',
  suffix: ''
}, {
  label: '•',
  title: t("components.TextEditor.k5"),
  prefix: '\n- ',
  suffix: ''
}, {
  label: '1.',
  title: t("components.TextEditor.k6"),
  prefix: '\n1. ',
  suffix: ''
}, {
  label: '<>',
  title: t("components.TextEditor.k7"),
  prefix: '\n```\n',
  suffix: '\n```\n',
  multiline: true
}, {
  label: '"',
  title: t("components.TextEditor.k8"),
  prefix: '\n> ',
  suffix: ''
}, {
  label: '🔗',
  title: t("components.TextEditor.k9"),
  prefix: '[',
  suffix: '](url)'
}, {
  label: '—',
  title: t("components.TextEditor.k10"),
  prefix: '\n---\n',
  suffix: ''
}];
export default function TextEditor({
  filePath,
  fileExt,
  initialContent,
  formatType,
  fileName,
  onSave,
  onClose,
  showStatus,
  wikiEntries,
  onWikiLinkClick
}: TextEditorProps) {
  const [content, setContent] = useState(initialContent);
  const [isSaving, setIsSaving] = useState(false);
  const [unsaved, setUnsaved] = useState(false);
  const [mdViewMode, setMdViewMode] = useState<MdViewMode>('split');
  const contentRef = useRef(content);
  const editorRef = useRef<any>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    contentRef.current = content;
  }, [content]);
  const language = languageMap[fileExt] || 'plaintext';
  const isMarkdown = fileExt === 'md';
  const handleEditorMount: OnMount = editor => {
    editorRef.current = editor;
    editor.addAction({
      id: 'save-file',
      label: t("components.TextEditor.k11"),
      keybindings: [2048 | 49],
      run: () => handleSave()
    });

    // Register wiki-link completion provider
    if (wikiEntries && wikiEntries.length > 0) {
      const monaco = (window as any).monaco;
      if (monaco) {
        monaco.languages.registerCompletionItemProvider('markdown', {
          provideCompletionItems: (model: any, position: any) => {
            const lineContent = model.getLineContent(position.lineNumber);
            const textBeforeCursor = lineContent.substring(0, position.column - 1);
            const bracketMatch = textBeforeCursor.match(/\[\[([^\]]*)$/);
            if (!bracketMatch) return {
              suggestions: []
            };
            const filter = bracketMatch[1].toLowerCase();
            const suggestions = wikiEntries.filter(name => name.toLowerCase().includes(filter)).slice(0, 20).map((name, i) => ({
              label: name,
              kind: monaco.languages.CompletionItemKind.Reference,
              insertText: name + ']]',
              range: {
                startLineNumber: position.lineNumber,
                endLineNumber: position.lineNumber,
                startColumn: position.column - bracketMatch[1].length,
                endColumn: position.column
              },
              sortText: String(i).padStart(4, '0'),
              filterText: name
            }));
            return {
              suggestions
            };
          },
          triggerCharacters: ['[']
        });
      }
    }
  };
  const insertAtCursor = useCallback((prefix: string, suffix: string, multiline?: boolean) => {
    const editor = editorRef.current;
    if (!editor) return;
    const selection = editor.getSelection();
    const selectedText = editor.getModel()?.getValueInRange(selection) || '';
    const newText = multiline && selectedText ? prefix + selectedText + suffix : prefix + selectedText + suffix;
    editor.executeEdits('toolbar', [{
      range: selection,
      text: newText
    }]);
    if (!multiline || !selectedText) {
      const pos = selection.getStartPosition();
      const newLine = prefix.length;
      const newCol = pos.column + newLine;
      editor.setPosition({
        lineNumber: pos.lineNumber,
        column: newCol
      });
    }
    editor.focus();
    setUnsaved(true);
  }, []);
  const handleSave = useCallback(async () => {
    setIsSaving(true);
    try {
      const res = await ipc.invoke('fileedit_write', {
        path: filePath,
        content: contentRef.current,
        ext: fileExt
      });
      if (res.code === 0) {
        setUnsaved(false);
        showStatus('success', t("components.TableEditor.k1"));
        onSave();
      } else {
        showStatus('error', res.message || t("errors.saveFailed"));
      }
    } catch (e: any) {
      showStatus('error', e?.toString() || t("errors.saveFailed"));
    } finally {
      setIsSaving(false);
    }
  }, [filePath, fileExt, onSave, showStatus]);
  const handleAutoSave = useCallback(async () => {
    if (!unsaved) return;
    try {
      await ipc.invoke('fileedit_write', {
        path: filePath,
        content: contentRef.current,
        ext: fileExt
      });
    } catch {/* silent */}
  }, [filePath, fileExt, unsaved]);
  useEffect(() => {
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => handleAutoSave(), 3000);
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    };
  }, [content]);
  const handleClose = () => {
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    onClose();
  };
  const headerStyles = useMemo(() => ({
    display: 'flex' as const,
    alignItems: 'center' as const,
    justifyContent: 'space-between' as const,
    padding: '8px 16px',
    borderBottom: '1px solid rgba(255,255,255,0.08)',
    background: '#0d0d14',
    flexShrink: 0
  }), []);
  const toolbarStyles = useMemo(() => ({
    display: 'flex' as const,
    alignItems: 'center' as const,
    gap: '2px',
    padding: '4px 12px',
    borderBottom: '1px solid rgba(255,255,255,0.06)',
    background: '#0a0a10',
    flexShrink: 0
  }), []);
  const tbBtn = (active?: boolean) => ({
    padding: '3px 8px',
    borderRadius: '3px',
    border: 'none',
    background: active ? 'rgba(0,240,255,0.12)' : 'transparent',
    color: active ? '#00F0FF' : '#888',
    fontSize: '12px',
    fontFamily: 'inherit',
    cursor: 'pointer',
    transition: 'all 0.1s'
  }) as const;
  const editorDiv = useMemo(() => ({
    flex: 1,
    minHeight: 0,
    overflow: 'hidden'
  }), []);
  const previewDiv = useMemo(() => ({
    flex: 1,
    minHeight: 0,
    overflowY: 'auto' as const,
    padding: '16px 20px',
    background: '#0d0d14',
    color: '#d0d0d0',
    fontSize: '14px',
    lineHeight: '1.8',
    fontFamily: "'Microsoft YaHei', 'PingFang SC', sans-serif"
  }), []);

  // Custom component to render wiki-links [[...]] in Markdown preview
  const wikiLinkRegex = /\[\[([^\]]+)\]\]/g;
  const renderMarkdownContent = (text: string) => {
    return text.split(wikiLinkRegex).map((part, i) => {
      if (i % 2 === 1) {
        return <span key={i} style={{
          color: '#0F0',
          textDecoration: 'underline',
          textUnderlineOffset: '3px',
          cursor: onWikiLinkClick ? 'pointer' : 'default',
          fontWeight: 500
        }} onClick={() => onWikiLinkClick?.(part)} title={t("components.TextEditor.k12", {
          part: part
        })}>[[{part}]]</span>;
      }
      return <ReactMarkdown key={i} remarkPlugins={[remarkGfm]}>{part}</ReactMarkdown>;
    });
  };
  return <div style={{
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    background: '#0a0a10'
  }}>
      <div style={headerStyles}>
        <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: '12px'
      }}>
          <span style={{
          color: '#888',
          fontSize: '13px'
        }}>
            {t("components.TextEditor.k13")} <strong style={{
            color: '#ccc'
          }}>{fileName}</strong>
          </span>
          <span style={{
          color: '#555',
          fontSize: '12px'
        }}>
            .{fileExt} {formatType === 'xml' ? t("components.TextEditor.k14") : ''}
          </span>
          {unsaved && <span style={{
          color: '#f0a030',
          fontSize: '12px',
          background: 'rgba(240,160,48,0.1)',
          padding: '2px 8px',
          borderRadius: '4px'
        }}>{t("components.AudioEditor.k5")}</span>}
        </div>
        <div style={{
        display: 'flex',
        gap: '8px',
        alignItems: 'center'
      }}>
          {isMarkdown && <div style={{
          display: 'flex',
          gap: '2px',
          border: '1px solid rgba(255,255,255,0.08)',
          borderRadius: '4px',
          overflow: 'hidden',
          marginRight: '8px'
        }}>
              {(['edit', 'split', 'preview'] as const).map(mode => <button key={mode} onClick={() => setMdViewMode(mode)} style={{
            padding: '4px 10px',
            border: 'none',
            background: mdViewMode === mode ? 'rgba(0,240,255,0.12)' : 'transparent',
            color: mdViewMode === mode ? '#00F0FF' : '#666',
            fontSize: '11px',
            cursor: 'pointer',
            fontFamily: 'inherit'
          }}>
                  {mode === 'edit' ? t("components.TextEditor.k15") : mode === 'split' ? t("components.TextEditor.k16") : t("components.TextEditor.k17")}
                </button>)}
            </div>}
          <button onClick={handleSave} disabled={isSaving} style={{
          padding: '6px 16px',
          borderRadius: '4px',
          border: 'none',
          background: isSaving ? '#555' : '#B026FF',
          color: '#fff',
          fontSize: '13px',
          cursor: isSaving ? 'not-allowed' : 'pointer'
        }}>
            {isSaving ? t("components.AudioEditor.k6") : t("components.TableEditor.k10")}
          </button>
          <button onClick={handleClose} style={{
          padding: '6px 16px',
          borderRadius: '4px',
          border: '1px solid rgba(255,255,255,0.12)',
          background: 'transparent',
          color: '#aaa',
          fontSize: '13px',
          cursor: 'pointer'
        }}>
            {t("components.FloatingXin.k28")}
          </button>
        </div>
      </div>

      {isMarkdown && mdViewMode !== 'preview' && <div style={toolbarStyles}>
          {toolbarActions.map(action => <button key={action.label} title={action.title} onClick={() => insertAtCursor(action.prefix, action.suffix, action.multiline)} style={tbBtn()} onMouseEnter={e => {
        e.currentTarget.style.color = '#00F0FF';
        e.currentTarget.style.background = 'rgba(0,240,255,0.08)';
      }} onMouseLeave={e => {
        e.currentTarget.style.color = '#888';
        e.currentTarget.style.background = 'transparent';
      }}>
              {action.label}
            </button>)}
          <span style={{
        color: '#444',
        fontSize: '10px',
        marginLeft: 'auto'
      }}>{t("components.TextEditor.k18")}</span>
        </div>}

      {isMarkdown && mdViewMode === 'split' ? <div style={{
      flex: 1,
      display: 'flex',
      minHeight: 0
    }}>
          <div style={{
        flex: 1,
        minWidth: 0,
        borderRight: '1px solid rgba(255,255,255,0.06)'
      }}>
            <Editor height="100%" language={language} value={content} onChange={v => {
          setContent(v || '');
          setUnsaved(true);
        }} onMount={handleEditorMount} theme="vs-dark" options={{
          fontSize: 14,
          fontFamily: "'Cascadia Code', 'Fira Code', Consolas, Monaco, monospace",
          minimap: {
            enabled: false
          },
          lineNumbers: 'on',
          wordWrap: 'on',
          tabSize: 2,
          scrollBeyondLastLine: false,
          automaticLayout: true
        }} />
          </div>
          <div style={previewDiv}>
            {renderMarkdownContent(content)}
          </div>
        </div> : isMarkdown && mdViewMode === 'preview' ? <div style={previewDiv}>
          {renderMarkdownContent(content)}
        </div> : <div style={editorDiv}>
          <Editor height="100%" language={language} value={content} onChange={v => {
        setContent(v || '');
        setUnsaved(true);
      }} onMount={handleEditorMount} theme="vs-dark" options={{
        fontSize: 14,
        fontFamily: "'Cascadia Code', 'Fira Code', Consolas, Monaco, monospace",
        minimap: {
          enabled: true
        },
        lineNumbers: 'on',
        wordWrap: 'on',
        tabSize: 2,
        scrollBeyondLastLine: false,
        automaticLayout: true
      }} />
        </div>}
    </div>;
}