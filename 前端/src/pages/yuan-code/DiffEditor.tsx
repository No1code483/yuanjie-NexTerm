import { t } from "i18next";
import { useCallback, useRef, useState } from 'react';
import { DiffEditor as MonacoDiffEditor } from '@monaco-editor/react';
import type { OnMount } from '@monaco-editor/react';
import { ipc } from '@/lib/ipc';
import styles from './YuanCode.module.css';
interface DiffEditorProps {
  originalPath: string;
  originalContent: string;
  modifiedPath: string;
  modifiedContent: string;
  language: string;
  onClose: () => void;
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
const NEXTERM_DIFF_THEME = {
  base: 'vs-dark' as const,
  inherit: true,
  rules: [{
    token: 'comment',
    foreground: '6A6A8A',
    fontStyle: 'italic'
  }, {
    token: 'keyword',
    foreground: 'BD93F9',
    fontStyle: 'bold'
  }, {
    token: 'string',
    foreground: '50FA7B'
  }, {
    token: 'number',
    foreground: 'FF79C6'
  }, {
    token: 'type',
    foreground: '00F0FF'
  }, {
    token: 'function',
    foreground: '8BE9FD'
  }, {
    token: 'variable',
    foreground: 'B8B8D0'
  }, {
    token: 'delimiter',
    foreground: 'B026FF'
  }, {
    token: 'operator',
    foreground: 'FF79C6'
  }, {
    token: 'tag',
    foreground: 'FF5555'
  }, {
    token: 'attribute',
    foreground: 'F1FA8C'
  }, {
    token: 'regexp',
    foreground: 'FFB86C'
  }],
  colors: {
    'editor.background': '#0A0014',
    'editor.foreground': '#B8B8D0',
    'editor.lineHighlightBackground': '#1A0A2E',
    'editorCursor.foreground': '#B026FF',
    'editorLineNumber.foreground': '#4A4A6A',
    'editorLineNumber.activeForeground': '#B8B8D0',
    'diffEditor.insertedTextBackground': 'rgba(80, 250, 123, 0.12)',
    'diffEditor.removedTextBackground': 'rgba(255, 85, 85, 0.12)',
    'diffEditor.insertedLineBackground': 'rgba(80, 250, 123, 0.06)',
    'diffEditor.removedLineBackground': 'rgba(255, 85, 85, 0.06)',
    'diffEditor.diagonalFill': 'rgba(128, 128, 128, 0.2)',
    'diffEditor.border': 'rgba(0, 240, 255, 0.15)',
    'editorOverviewRuler.insertedForeground': 'rgba(80, 250, 123, 0.6)',
    'editorOverviewRuler.removedForeground': 'rgba(255, 85, 85, 0.6)',
    'editorOverviewRuler.modifiedForeground': 'rgba(0, 240, 255, 0.6)',
    'editorOverviewRuler.border': 'rgba(0, 240, 255, 0.1)',
    'scrollbarSlider.background': 'rgba(0, 240, 255, 0.2)',
    'scrollbarSlider.hoverBackground': 'rgba(0, 240, 255, 0.4)',
    'scrollbarSlider.activeBackground': 'rgba(0, 240, 255, 0.5)'
  }
};
function getMonacoLanguage(language: string): string {
  return LANGUAGE_MAP[language] || 'plaintext';
}
export default function DiffEditor({
  originalPath,
  originalContent,
  modifiedPath,
  modifiedContent,
  language,
  onClose
}: DiffEditorProps) {
  const diffEditorRef = useRef<any>(null);
  const [viewMode, setViewMode] = useState<'side-by-side' | 'inline'>('side-by-side');

  // Apply Patch 状态
  const [applying, setApplying] = useState(false);
  const [applyResult, setApplyResult] = useState<{
    ok: boolean;
    msg: string;
  } | null>(null);
  const handleApplyPatch = useCallback(async () => {
    setApplying(true);
    setApplyResult(null);
    try {
      const res = await ipc.invoke<{
        file_path: string;
        applied: boolean;
        changes: Array<{
          line_start: number;
          line_end: number;
          change_type: string;
          content: string;
        }>;
        original_content: string | null;
        new_content: string | null;
      }>('yuan_apply_patch', {
        file_path: modifiedPath,
        patch_content: modifiedContent,
        description: `Apply diff: ${originalPath.split('/').pop()} → ${modifiedPath.split('/').pop()}`,
        create_if_not_exists: false
      });
      if (res.code === 0 && res.data) {
        setApplyResult({
          ok: true,
          msg: t("yuan-code.DiffEditor.k1", {
            length: res.data.changes.length
          })
        });
      } else {
        setApplyResult({
          ok: false,
          msg: res.message || t("yuan-code.DiffEditor.k2")
        });
      }
    } catch (e: any) {
      setApplyResult({
        ok: false,
        msg: e?.message || t("yuan-code.DiffEditor.k2")
      });
    } finally {
      setApplying(false);
      setTimeout(() => setApplyResult(null), 3000);
    }
  }, [modifiedPath, modifiedContent, originalPath]);
  const handleMount: OnMount = (editor, monaco) => {
    diffEditorRef.current = editor;
    monaco.editor.defineTheme('nexterm-diff-dark', NEXTERM_DIFF_THEME);
    monaco.editor.setTheme('nexterm-diff-dark');
    editor.addAction({
      id: 'close-diff',
      label: t("yuan-code.DiffEditor.k3"),
      keybindings: [monaco.KeyCode.Escape],
      run: () => onClose()
    });
    editor.addAction({
      id: 'toggle-diff-view',
      label: t("yuan-code.DiffEditor.k4"),
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyD],
      run: () => {
        const next = viewMode === 'side-by-side' ? 'inline' : 'side-by-side';
        setViewMode(next);
      }
    });

    // 导航: 下一个更改
    editor.addAction({
      id: 'next-change',
      label: t("yuan-code.DiffEditor.k5"),
      keybindings: [monaco.KeyCode.F7],
      run: () => editor.getAction('editor.action.diffReview.next')?.run()
    });

    // 导航: 上一个更改
    editor.addAction({
      id: 'prev-change',
      label: t("yuan-code.DiffEditor.k6"),
      keybindings: [monaco.KeyMod.Shift | monaco.KeyCode.F7],
      run: () => editor.getAction('editor.action.diffReview.prev')?.run()
    });
  };
  const handleToggleView = useCallback(() => {
    setViewMode(prev => prev === 'side-by-side' ? 'inline' : 'side-by-side');
  }, []);
  return <div className={styles.diffContainer}>
      <div className={styles.diffHeader}>
        <div className={styles.diffHeaderLeft}>
          <span className={styles.diffLabel}>{t("yuan-code.DiffEditor.k7")}</span>
          <span className={styles.diffPath}>
            <span className={styles.diffPathOriginal} title={originalPath}>
              {t("yuan-code.DiffEditor.k8")} {originalPath.split('/').pop()}
            </span>
            <span className={styles.diffArrow}>→</span>
            <span className={styles.diffPathModified} title={modifiedPath}>
              {t("yuan-code.DiffEditor.k9")} {modifiedPath.split('/').pop()}
            </span>
          </span>
        </div>
        <div className={styles.diffHeaderRight}>
          <button className={`${styles.diffBtn} ${viewMode === 'inline' ? styles.diffBtnActive : ''}`} onClick={handleToggleView} title={t("yuan-code.DiffEditor.k10")}>
            {viewMode === 'side-by-side' ? '◫' : '⊟'}
          </button>
          <button className={styles.diffBtn} onClick={handleApplyPatch} disabled={applying} title={t("yuan-code.DiffEditor.k11")} style={{
          borderColor: 'rgba(0,255,135,0.3)',
          color: '#00FF87'
        }}>
            {applying ? '⏳' : '✓'} {t("common.apply")}
          </button>
          <button className={styles.diffBtn} onClick={onClose} title={t("components.NexTermTerminal.k5")}>
            ✕
          </button>
        </div>
      </div>
      <div className={styles.diffBody}>
        <MonacoDiffEditor original={originalContent} modified={modifiedContent} language={getMonacoLanguage(language)} onMount={handleMount} theme="nexterm-diff-dark" loading={<div className={styles.diffLoading}>
              <div className={styles.diffLoadingText}>{t("yuan-code.DiffEditor.k12")}</div>
            </div>} options={{
        renderSideBySide: viewMode === 'side-by-side',
        originalEditable: false,
        fontSize: 13,
        fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace",
        fontLigatures: true,
        minimap: {
          enabled: false
        },
        lineNumbers: 'on',
        renderLineHighlight: 'line',
        scrollBeyondLastLine: false,
        wordWrap: 'on',
        tabSize: 4,
        smoothScrolling: true,
        cursorBlinking: 'smooth',
        readOnly: false,
        padding: {
          top: 4
        },
        folding: true,
        guides: {
          indentation: true,
          bracketPairs: true
        },
        bracketPairColorization: {
          enabled: true
        },
        // 概览标尺：差异区域标记
        overviewRulerBorder: true,
        hideCursorInOverviewRuler: false,
        renderOverviewRuler: true,
        // Diff 导航
        diffAlgorithm: 'advanced',
        showFoldingControls: 'always',
        useInlineViewWhenSpaceIsLimited: false
      }} />
      </div>
      <div className={styles.diffFooter}>
        <span className={styles.diffFooterStat}>
          <span className={styles.diffAdded}>{t("yuan-code.DiffEditor.k13")}</span>
          <span className={styles.diffRemoved}>{t("yuan-code.DiffEditor.k14")}</span>
        </span>
        {applyResult && <span style={{
        fontSize: 11,
        fontFamily: 'var(--nt-font-mono)',
        color: applyResult.ok ? '#00FF87' : '#FF006E'
      }}>
            {applyResult.msg}
          </span>}
        <span className={styles.diffFooterHint}>
          {t("yuan-code.DiffEditor.k15")}
        </span>
      </div>
    </div>;
}