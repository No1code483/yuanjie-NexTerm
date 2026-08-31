import { t } from "i18next";
import { useState, useRef, useEffect, useCallback } from 'react';
import { ipc } from '@/lib/ipc';
import { useNetworkStatus } from '@/hooks/useNetworkStatus';
import styles from './InlineEdit.module.css';
interface InlineEditProps {
  editor: any;
  selectedCode: string;
  language: string;
  filePath: string;
  onAccept: (modifiedCode: string) => void;
  onReject: () => void;
  onClose: () => void;
  /** D1.1：Yuan Code 编排模型 ID（来自统一模型管理） */
  modelId?: number | null;
}
type Phase = 'input' | 'loading' | 'diff';
interface DiffLine {
  type: 'add' | 'remove' | 'equal';
  content: string;
  oldLine?: number;
  newLine?: number;
}
export default function InlineEdit({
  editor,
  selectedCode,
  language,
  filePath,
  onAccept,
  onReject,
  onClose,
  modelId
}: InlineEditProps) {
  const [phase, setPhase] = useState<Phase>('input');
  const [instruction, setInstruction] = useState('');
  const [modifiedCode, setModifiedCode] = useState('');
  const [error, setError] = useState('');
  const [diffLines, setDiffLines] = useState<DiffLine[]>([]);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // A5 Phase 3 Task 2: 云端 AI 离线降级
  // 设计依据：.trae/rules/项目核心设计意图.md §三（Yuan Code 走云端 API，离线不切本地 ollama）
  // 离线时禁用内联编辑提交 + 显示「AI 离线，本地编辑可用」提示，
  // 不进入 yuan_inline_edit IPC 流程（避免触发云端 API 请求）。
  const { isOnline } = useNetworkStatus();

  // Auto-focus textarea
  useEffect(() => {
    if (phase === 'input' && textareaRef.current) {
      textareaRef.current.focus();
    }
  }, [phase]);

  // Compute position near the selection
  useEffect(() => {
    if (!editor || !containerRef.current) return;
    const selection = editor.getSelection();
    if (!selection) return;

    // Get the position of the end of selection
    const endPos = selection.getEndPosition();
    const coords = editor.getScrolledVisiblePosition(endPos);
    if (!coords) return;
    const editorDom = editor.getDomNode();
    if (!editorDom) return;
    const editorRect = editorDom.getBoundingClientRect();
    const container = containerRef.current;

    // Position below the selection end, with some padding
    const top = coords.top + 20;
    const left = Math.max(16, coords.left);
    container.style.top = `${top}px`;
    container.style.left = `${left}px`;
    container.style.maxWidth = `${editorRect.width - left - 16}px`;
  }, [editor, phase]);

  // Compute simple line-based diff
  const computeDiff = useCallback((original: string, modified: string): DiffLine[] => {
    const origLines = original.split('\n');
    const modLines = modified.split('\n');

    // Simple LCS-based diff
    const n = origLines.length;
    const m = modLines.length;
    const dp: number[][] = Array.from({
      length: n + 1
    }, () => Array(m + 1).fill(0));
    for (let i = 1; i <= n; i++) {
      for (let j = 1; j <= m; j++) {
        if (origLines[i - 1] === modLines[j - 1]) {
          dp[i][j] = dp[i - 1][j - 1] + 1;
        } else {
          dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
        }
      }
    }

    // Backtrack
    let i = n,
      j = m;
    const temp: DiffLine[] = [];
    while (i > 0 || j > 0) {
      if (i > 0 && j > 0 && origLines[i - 1] === modLines[j - 1]) {
        temp.push({
          type: 'equal',
          content: origLines[i - 1],
          oldLine: i,
          newLine: j
        });
        i--;
        j--;
      } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
        temp.push({
          type: 'add',
          content: modLines[j - 1],
          newLine: j
        });
        j--;
      } else {
        temp.push({
          type: 'remove',
          content: origLines[i - 1],
          oldLine: i
        });
        i--;
      }
    }
    return temp.reverse();
  }, []);
  const handleSubmit = useCallback(async () => {
    if (!instruction.trim()) return;
    // A5 Phase 3 Task 2: 离线时直接返回，不调用云端 AI
    if (!isOnline) {
      setError(t('yuan-code.InlineEdit.offlineHint', { defaultValue: 'AI 离线，本地编辑可用。请连接网络后使用 AI 内联编辑。' }));
      return;
    }
    setPhase('loading');
    setError('');
    try {
      const res = await ipc.invoke<{
        modified_code: string;
      }>('yuan_inline_edit', {
        request: {
          selected_code: selectedCode,
          instruction: instruction.trim(),
          language,
          file_path: filePath,
          // D1.1：传入统一模型管理中选中的模型 ID
          model_id: modelId ?? undefined
        }
      });
      if (res.code === 0 && res.data) {
        const mod = res.data.modified_code;
        setModifiedCode(mod);
        const diff = computeDiff(selectedCode, mod);
        setDiffLines(diff);
        setPhase('diff');
      } else {
        setError(res.message || t("yuan-code.InlineEdit.k1"));
        setPhase('input');
      }
    } catch (e: any) {
      setError(e?.message || t("yuan-code.InlineEdit.k2"));
      setPhase('input');
    }
  }, [instruction, selectedCode, language, filePath, computeDiff, modelId, isOnline]);
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }, [handleSubmit, onClose]);

  // Close on click outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    // Delay to avoid immediate close on the same click
    const timer = setTimeout(() => {
      document.addEventListener('mousedown', handleClickOutside);
    }, 100);
    return () => {
      clearTimeout(timer);
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [onClose]);
  return <div className={styles.overlay}>
      <div className={styles.container} ref={containerRef}>
        {/* Header */}
        <div className={styles.header}>
          <span className={styles.headerTitle}>{t("yuan-code.EditorPanel.k9")}</span>
          <span className={styles.headerShortcut}>Ctrl+K</span>
          <button className={styles.closeBtn} onClick={onClose}>✕</button>
        </div>

        {/* Input Phase */}
        {phase === 'input' && <div className={styles.body}>
            {/* Selected code preview */}
            <div className={styles.codePreview}>
              <div className={styles.codePreviewLabel}>{t("yuan-code.InlineEdit.k3")}</div>
              <pre className={styles.codePreviewContent}>
                <code>{selectedCode}</code>
              </pre>
            </div>

            {/* Instruction input */}
            <div className={styles.inputGroup}>
              <label className={styles.inputLabel}>{t("yuan-code.InlineEdit.k4")}</label>
              <textarea
                ref={textareaRef}
                className={styles.textarea}
                value={instruction}
                onChange={e => setInstruction(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={isOnline ? t("yuan-code.InlineEdit.k5") : t('yuan-code.InlineEdit.offlineHint', { defaultValue: 'AI 离线，本地编辑可用。请连接网络后使用 AI 内联编辑。' })}
                rows={3}
                disabled={!isOnline}
              />
              <div className={styles.inputHint}>
                <kbd>Enter</kbd> {t("yuan-code.InlineEdit.k6")} <kbd>Esc</kbd> {t("yuan-code.InlineEdit.k7")} <kbd>Shift+Enter</kbd> {t("ai.ChatPanel.k23")}
              </div>
            </div>

            {/* A5 Phase 3 Task 2: 离线降级提示（独立块，区别于普通 error） */}
            {!isOnline && <div className={styles.offlineHint}>
                {t('yuan-code.InlineEdit.offlineHint', { defaultValue: 'AI 离线，本地编辑可用。请连接网络后使用 AI 内联编辑。' })}
              </div>}

            {error && <div className={styles.error}>{error}</div>}
          </div>}

        {/* Loading Phase */}
        {phase === 'loading' && <div className={styles.body}>
            <div className={styles.loading}>
              <span className={styles.spinner} />
              <span className={styles.loadingText}>{t("yuan-code.InlineEdit.k8")}</span>
            </div>
          </div>}

        {/* Diff Phase */}
        {phase === 'diff' && <div className={styles.body}>
            <div className={styles.diffHeader}>
              <span className={styles.diffStat}>
                <span className={styles.diffAdded}>+{diffLines.filter(l => l.type === 'add').length}</span>
                {' '}
                <span className={styles.diffRemoved}>-{diffLines.filter(l => l.type === 'remove').length}</span>
              </span>
            </div>

            <div className={styles.diffView}>
              {diffLines.map((line, idx) => <div key={idx} className={`${styles.diffLine} ${line.type === 'add' ? styles.diffLineAdd : line.type === 'remove' ? styles.diffLineRemove : styles.diffLineEqual}`}>
                  <span className={styles.diffLinePrefix}>
                    {line.type === 'add' ? '+' : line.type === 'remove' ? '-' : ' '}
                  </span>
                  <span className={styles.diffLineContent}>{line.content}</span>
                </div>)}
            </div>

            <div className={styles.diffActions}>
              <button className={styles.acceptBtn} onClick={() => onAccept(modifiedCode)}>
                {t("yuan-code.InlineEdit.k9")}
              </button>
              <button className={styles.rejectBtn} onClick={onReject}>
                {t("yuan-code.InlineEdit.k10")}
              </button>
            </div>
          </div>}
      </div>
    </div>;
}