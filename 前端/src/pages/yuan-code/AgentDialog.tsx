import { t } from "i18next";
import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import { ipc } from '@/lib/ipc';
import { useNetworkStatus } from '@/hooks/useNetworkStatus';
import type { FileNode, OpenTab } from '../YuanCode';
import styles from './AgentDialog.module.css';

// ===== Types =====

interface ContextRef {
  path: string;
  name: string;
  isDir: boolean;
}
interface Checkpoint {
  id: string;
  timestamp: number;
  description: string;
  files: {
    path: string;
    content: string;
  }[];
}
interface LintIssue {
  file: string;
  line?: number;
  column?: number;
  severity: string;
  message: string;
}
interface DiffFile {
  path: string;
  original: string;
  modified: string;
  hunks: {
    oldStart: number;
    oldCount: number;
    newStart: number;
    newCount: number;
    lines: {
      kind: string;
      content: string;
      oldLine?: number;
      newLine?: number;
    }[];
  }[];
}
type FlowStepStatus = 'pending' | 'active' | 'done' | 'error';
interface FlowStep {
  id: string;
  title: string;
  status: FlowStepStatus;
  content: string;
}
interface AgentDialogProps {
  isOpen: boolean;
  onClose: () => void;
  fileTree: FileNode[];
  openTabs: OpenTab[];
  workspacePath: string;
  onUpdateFileContent?: (path: string, content: string) => void;
}

// ===== Helpers =====

function flattenTree(nodes: FileNode[], basePath = ''): {
  name: string;
  path: string;
  isDir: boolean;
}[] {
  const result: {
    name: string;
    path: string;
    isDir: boolean;
  }[] = [];
  for (const n of nodes) {
    const fullPath = basePath ? `${basePath}/${n.name}` : n.name;
    result.push({
      name: n.name,
      path: fullPath,
      isDir: n.is_dir
    });
    if (n.children && n.children.length > 0) {
      result.push(...flattenTree(n.children, fullPath));
    }
  }
  return result;
}
function formatTime(ts: number): string {
  const d = new Date(ts);
  const h = String(d.getHours()).padStart(2, '0');
  const m = String(d.getMinutes()).padStart(2, '0');
  const s = String(d.getSeconds()).padStart(2, '0');
  return `${h}:${m}:${s}`;
}
function computeSimpleDiff(original: string, modified: string): DiffFile['hunks'] {
  const origLines = original.split('\n');
  const modLines = modified.split('\n');
  const maxLen = Math.max(origLines.length, modLines.length);
  const lines: DiffFile['hunks'][0]['lines'] = [];
  for (let i = 0; i < maxLen; i++) {
    const oLine = origLines[i];
    const mLine = modLines[i];
    if (oLine === undefined && mLine !== undefined) {
      lines.push({
        kind: 'added',
        content: mLine,
        newLine: i + 1
      });
    } else if (mLine === undefined && oLine !== undefined) {
      lines.push({
        kind: 'removed',
        content: oLine,
        oldLine: i + 1
      });
    } else if (oLine !== mLine) {
      if (oLine) lines.push({
        kind: 'removed',
        content: oLine,
        oldLine: i + 1
      });
      if (mLine) lines.push({
        kind: 'added',
        content: mLine,
        newLine: i + 1
      });
    } else {
      lines.push({
        kind: 'unchanged',
        content: oLine
      });
    }
  }
  return [{
    oldStart: 1,
    oldCount: origLines.length,
    newStart: 1,
    newCount: modLines.length,
    lines
  }];
}

// ===== Component =====

export default function AgentDialog({
  isOpen,
  onClose,
  fileTree,
  openTabs,
  workspacePath,
  onUpdateFileContent
}: AgentDialogProps) {
  // Input state
  const [prompt, setPrompt] = useState('');
  const [contextRefs, setContextRefs] = useState<ContextRef[]>([]);
  const [showAutocomplete, setShowAutocomplete] = useState(false);
  const [autocompleteFilter, setAutocompleteFilter] = useState('');
  const [autocompleteIndex, setAutocompleteIndex] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Execution state
  const [executing, setExecuting] = useState(false);
  const [flowSteps, setFlowSteps] = useState<FlowStep[]>([]);
  const [diffs, setDiffs] = useState<DiffFile[]>([]);
  const [expandedDiff, setExpandedDiff] = useState<Set<string>>(new Set());
  const [diffAccepted, setDiffAccepted] = useState<Set<string>>(new Set());
  const [diffRejected, setDiffRejected] = useState<Set<string>>(new Set());

  // Checkpoints
  const [checkpoints, setCheckpoints] = useState<Checkpoint[]>([]);
  const [expandedCheckpoints, setExpandedCheckpoints] = useState(false);

  // Lint
  const [lintIssues, setLintIssues] = useState<LintIssue[]>([]);
  const [expandedLint, setExpandedLint] = useState(false);
  const [autoFixing, setAutoFixing] = useState(false);
  const [autoFixDiff, setAutoFixDiff] = useState<DiffFile | null>(null);

  // Error
  const [error, setError] = useState('');

  // A5 Phase 3 Task 2: 云端 AI 离线降级
  // 设计依据：.trae/rules/项目核心设计意图.md §三（Yuan Code 走云端 API，离线不切本地 ollama）
  // 离线时禁用 Execute 按钮 + 显示「AI 离线，本地编辑可用」提示，
  // 不进入 agent 执行流程（避免触发云端 API 请求）。
  const { isOnline } = useNetworkStatus();

  // Flattened file tree for autocomplete
  const flatFiles = useMemo(() => flattenTree(fileTree), [fileTree]);

  // Filtered autocomplete items
  const autocompleteItems = useMemo(() => {
    if (!autocompleteFilter) return flatFiles.slice(0, 20);
    const lower = autocompleteFilter.toLowerCase();
    return flatFiles.filter(f => f.path.toLowerCase().includes(lower) || f.name.toLowerCase().includes(lower)).slice(0, 20);
  }, [flatFiles, autocompleteFilter]);

  // Reset on open
  useEffect(() => {
    if (isOpen) {
      setPrompt('');
      setContextRefs([]);
      setShowAutocomplete(false);
      setAutocompleteFilter('');
      setAutocompleteIndex(0);
      setExecuting(false);
      setFlowSteps([]);
      setDiffs([]);
      setExpandedDiff(new Set());
      setDiffAccepted(new Set());
      setDiffRejected(new Set());
      setCheckpoints([]);
      setExpandedCheckpoints(false);
      setLintIssues([]);
      setExpandedLint(false);
      setAutoFixing(false);
      setAutoFixDiff(null);
      setError('');
      setTimeout(() => textareaRef.current?.focus(), 100);
    }
  }, [isOpen]);

  // Close on Escape
  useEffect(() => {
    if (!isOpen) return;
    const handle = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !executing) onClose();
    };
    window.addEventListener('keydown', handle);
    return () => window.removeEventListener('keydown', handle);
  }, [isOpen, onClose, executing]);

  // ===== @file Autocomplete =====
  const handlePromptChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const value = e.target.value;
    setPrompt(value);

    // Check for @ trigger
    const cursorPos = e.target.selectionStart;
    const textBeforeCursor = value.slice(0, cursorPos);
    const atMatch = textBeforeCursor.match(/@([^\s@]*)$/);
    if (atMatch) {
      setShowAutocomplete(true);
      setAutocompleteFilter(atMatch[1]);
      setAutocompleteIndex(0);
    } else {
      setShowAutocomplete(false);
      setAutocompleteFilter('');
    }
  }, []);
  const handleSelectAutocomplete = useCallback((item: {
    name: string;
    path: string;
    isDir: boolean;
  }) => {
    // Replace the @... text in the prompt
    const textarea = textareaRef.current;
    if (!textarea) return;
    const cursorPos = textarea.selectionStart;
    const textBefore = prompt.slice(0, cursorPos);
    const textAfter = prompt.slice(cursorPos);
    const atMatch = textBefore.match(/@([^\s@]*)$/);
    if (atMatch) {
      const newBefore = textBefore.slice(0, atMatch.index) + ' ';
      setPrompt(newBefore + textAfter);
    }

    // Add to context refs
    setContextRefs(prev => {
      if (prev.some(r => r.path === item.path)) return prev;
      return [...prev, {
        path: item.path,
        name: item.name,
        isDir: item.isDir
      }];
    });
    setShowAutocomplete(false);
    setAutocompleteFilter('');
    textarea.focus();
  }, [prompt]);
  const handleRemoveContextRef = useCallback((path: string) => {
    setContextRefs(prev => prev.filter(r => r.path !== path));
  }, []);
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (showAutocomplete) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setAutocompleteIndex(i => Math.min(i + 1, autocompleteItems.length - 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setAutocompleteIndex(i => Math.max(i - 1, 0));
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (autocompleteItems[autocompleteIndex]) {
          handleSelectAutocomplete(autocompleteItems[autocompleteIndex]);
        }
      } else if (e.key === 'Escape') {
        setShowAutocomplete(false);
      }
    } else if (e.key === 'Enter' && !e.shiftKey && !executing) {
      e.preventDefault();
      handleExecute();
    }
  }, [showAutocomplete, autocompleteItems, autocompleteIndex, handleSelectAutocomplete, executing]);

  // ===== Read file content =====
  const readFileContent = useCallback(async (relativePath: string): Promise<string> => {
    // Check if file is open in tabs
    const openTab = openTabs.find(t => t.path === relativePath);
    if (openTab) return openTab.content;

    // Read from disk
    try {
      const normalized = relativePath.replace(/\\/g, '/');
      const fullPath = workspacePath ? `${workspacePath.replace(/\\/g, '/')}/${normalized}` : normalized;
      const res = await ipc.invoke<{
        content: string;
      }>('yuan_read_file', {
        request: {
          path: fullPath
        }
      });
      if (res.code === 0 && res.data) {
        return res.data.content || '';
      }
    } catch {
      // ignore
    }
    return '';
  }, [openTabs, workspacePath]);

  // ===== Typing effect =====
  const typewriteText = useCallback(async (stepId: string, text: string, setSteps: React.Dispatch<React.SetStateAction<FlowStep[]>>, speed = 15) => {
    for (let i = 0; i <= text.length; i++) {
      setSteps(prev => prev.map(s => s.id === stepId ? {
        ...s,
        content: text.slice(0, i)
      } : s));
      await new Promise(r => setTimeout(r, speed));
    }
  }, []);

  // ===== Add flow step =====
  const addFlowStep = useCallback((id: string, title: string, status: FlowStepStatus = 'pending') => {
    setFlowSteps(prev => [...prev, {
      id,
      title,
      status,
      content: ''
    }]);
  }, []);
  const updateFlowStep = useCallback((id: string, updates: Partial<FlowStep>) => {
    setFlowSteps(prev => prev.map(s => s.id === id ? {
      ...s,
      ...updates
    } : s));
  }, []);

  // ===== Save checkpoint =====
  const saveCheckpoint = useCallback(async (description: string, filePaths: string[]) => {
    const files: {
      path: string;
      content: string;
    }[] = [];
    for (const fp of filePaths) {
      const content = await readFileContent(fp);
      files.push({
        path: fp,
        content
      });
    }
    const cp: Checkpoint = {
      id: `cp_${Date.now()}`,
      timestamp: Date.now(),
      description,
      files
    };
    setCheckpoints(prev => [cp, ...prev]);
    return cp;
  }, [readFileContent]);

  // ===== Rollback checkpoint =====
  const handleRollback = useCallback((cp: Checkpoint) => {
    if (!onUpdateFileContent) return;
    for (const f of cp.files) {
      onUpdateFileContent(f.path, f.content);
    }
    // Remove checkpoints after this one
    setCheckpoints(prev => {
      const idx = prev.findIndex(c => c.id === cp.id);
      if (idx < 0) return prev;
      return prev.slice(0, idx + 1);
    });
  }, [onUpdateFileContent]);

  // ===== Main execution =====
  const handleExecute = useCallback(async () => {
    const finalPrompt = prompt.trim();
    if (!finalPrompt && contextRefs.length === 0) return;
    if (executing) return;
    // A5 Phase 3 Task 2: 离线时直接返回，不调用云端 AI
    if (!isOnline) {
      setError(t('yuan-code.AgentDialog.offlineHint', { defaultValue: 'AI 离线，本地编辑可用。请连接网络后使用 Agent 执行。' }));
      return;
    }
    setExecuting(true);
    setError('');
    setFlowSteps([]);
    setDiffs([]);
    setExpandedDiff(new Set());
    setDiffAccepted(new Set());
    setDiffRejected(new Set());
    setLintIssues([]);
    setExpandedLint(false);
    setAutoFixDiff(null);

    // Step 1: 分析中...
    addFlowStep('analyze', t("components.intelligence.DashboardPanel.k82"), 'active');
    await new Promise(r => setTimeout(r, 600));
    try {
      // Try backend agent execution
      const contextPaths = contextRefs.filter(r => !r.isDir).map(r => r.path);
      const res = await ipc.invoke<{
        plan: string;
        result: string;
        modified_files: {
          path: string;
          original: string;
          modified: string;
        }[];
        terminal_commands: string[];
      }>('yuan_agent_execute', {
        prompt: finalPrompt,
        context_files: contextPaths,
        workspace_path: workspacePath
      });
      if (res.code === 0 && res.data) {
        const data = res.data;

        // Step 1 -> done
        updateFlowStep('analyze', {
          status: 'done',
          content: t("yuan-code.AgentDialog.k1")
        });

        // Step 2: 计划
        addFlowStep('plan', t("yuan-code.AgentDialog.k2"), 'active');
        await typewriteText('plan', data.plan || t("yuan-code.AgentDialog.k3"), setFlowSteps, 12);
        updateFlowStep('plan', {
          status: 'done'
        });

        // Save checkpoint before modifications
        if (data.modified_files && data.modified_files.length > 0) {
          await saveCheckpoint(t("yuan-code.AgentDialog.k4", {
            arg0: data.plan ? data.plan.slice(0, 60) : t("yuan-code.AgentDialog.k5")
          }), data.modified_files.map(f => f.path));
        }

        // Step 3: 执行中...
        addFlowStep('execute', t("Xin.k159"), 'active');
        await new Promise(r => setTimeout(r, 400));

        // Apply modified files
        const diffFiles: DiffFile[] = [];
        for (const mf of data.modified_files || []) {
          if (onUpdateFileContent) {
            onUpdateFileContent(mf.path, mf.modified);
          }
          diffFiles.push({
            path: mf.path,
            original: mf.original,
            modified: mf.modified,
            hunks: computeSimpleDiff(mf.original, mf.modified)
          });
        }
        updateFlowStep('execute', {
          status: 'done',
          content: t("yuan-code.AgentDialog.k6", {
            arg0: data.modified_files?.length || 0
          })
        });

        // Step 4: 结果
        addFlowStep('result', t("common.result"), 'active');
        await typewriteText('result', data.result || t("yuan-code.AgentDialog.k7"), setFlowSteps, 12);
        updateFlowStep('result', {
          status: 'done'
        });

        // Step 5: Diff 预览
        if (diffFiles.length > 0) {
          addFlowStep('diff', t("yuan-code.AgentDialog.k8"), 'done');
          setDiffs(diffFiles);
          setExpandedDiff(new Set(diffFiles.map(d => d.path)));
        }

        // Run lint
        if (diffFiles.length > 0) {
          runLintCheck(diffFiles.map(d => d.path));
        }
      } else {
        // Fallback to simulated execution
        await simulateExecution();
      }
    } catch {
      // Backend not available, simulate
      await simulateExecution();
    }

    // eslint-disable-next-line react-hooks/exhaustive-deps
    async function simulateExecution() {
      updateFlowStep('analyze', {
        status: 'done',
        content: t("yuan-code.AgentDialog.k9")
      });

      // Step 2: 计划
      addFlowStep('plan', t("yuan-code.AgentDialog.k2"), 'active');
      const planText = t("yuan-code.AgentDialog.k10", {
        finalPrompt: finalPrompt
      });
      await typewriteText('plan', planText, setFlowSteps, 10);
      updateFlowStep('plan', {
        status: 'done'
      });

      // Save checkpoint
      const filePaths = contextRefs.filter(r => !r.isDir).map(r => r.path);
      if (filePaths.length > 0) {
        await saveCheckpoint(t("yuan-code.AgentDialog.k4", {
          arg0: finalPrompt.slice(0, 60)
        }), filePaths);
      }

      // Step 3: 执行中...
      addFlowStep('execute', t("Xin.k159"), 'active');
      await new Promise(r => setTimeout(r, 500));
      updateFlowStep('execute', {
        status: 'done',
        content: t("yuan-code.AgentDialog.k11")
      });

      // Step 4: 结果
      addFlowStep('result', t("common.result"), 'active');
      const resultText = t("yuan-code.AgentDialog.k12", {
        finalPrompt: finalPrompt,
        arg0: contextRefs.map(r => r.path).join(', ') || t("common.none")
      });
      await typewriteText('result', resultText, setFlowSteps, 10);
      updateFlowStep('result', {
        status: 'done'
      });
    }
    setExecuting(false);
  }, [prompt, contextRefs, executing, workspacePath, openTabs, addFlowStep, updateFlowStep, typewriteText, saveCheckpoint, readFileContent, onUpdateFileContent, isOnline]);

  // ===== Lint check =====
  const runLintCheck = useCallback(async (filePaths: string[]) => {
    const issues: LintIssue[] = [];
    for (const fp of filePaths) {
      try {
        const content = await readFileContent(fp);
        const res = await ipc.invoke<{
          issues: {
            line?: number;
            column?: number;
            severity: string;
            message: string;
          }[];
        }>('yuan_analyze', {
          request: {
            code: content,
            language: fp.split('.').pop() || 'text',
            analysis_type: 'lint'
          }
        });
        if (res.code === 0 && res.data?.issues) {
          for (const issue of res.data.issues) {
            issues.push({
              ...issue,
              file: fp
            });
          }
        }
      } catch {
        // ignore
      }
    }
    setLintIssues(issues);
    if (issues.length > 0) {
      setExpandedLint(true);
    }
  }, [readFileContent]);

  // ===== Auto-fix lint =====
  const handleAutoFix = useCallback(async () => {
    if (lintIssues.length === 0 || autoFixing) return;
    setAutoFixing(true);

    // Simulate AI fix - in real implementation would call AI
    await new Promise(r => setTimeout(r, 800));

    // For now, create a simple diff showing the lint issues
    const fixContent = lintIssues.map(i => `[${i.severity.toUpperCase()}] ${i.file}:${i.line || '?'} - ${i.message}`).join('\n');
    setAutoFixDiff({
      path: 'lint-fixes',
      original: '',
      modified: fixContent,
      hunks: [{
        oldStart: 0,
        oldCount: 0,
        newStart: 1,
        newCount: lintIssues.length,
        lines: lintIssues.map(i => ({
          kind: 'added' as const,
          content: `[${i.severity.toUpperCase()}] ${i.file}:${i.line || '?'} - ${i.message}`,
          newLine: lintIssues.indexOf(i) + 1
        }))
      }]
    });
    setAutoFixing(false);
  }, [lintIssues, autoFixing]);
  const handleAcceptAutoFix = useCallback(() => {
    setLintIssues([]);
    setAutoFixDiff(null);
  }, []);
  const handleRejectAutoFix = useCallback(() => {
    setAutoFixDiff(null);
  }, []);

  // ===== Diff accept/reject =====
  const handleAcceptDiff = useCallback((path: string) => {
    setDiffAccepted(prev => new Set(prev).add(path));
    setDiffRejected(prev => {
      const next = new Set(prev);
      next.delete(path);
      return next;
    });
  }, []);
  const handleRejectDiff = useCallback((path: string) => {
    // Rollback to original
    const diff = diffs.find(d => d.path === path);
    if (diff && onUpdateFileContent) {
      onUpdateFileContent(diff.path, diff.original);
    }
    setDiffRejected(prev => new Set(prev).add(path));
    setDiffAccepted(prev => {
      const next = new Set(prev);
      next.delete(path);
      return next;
    });
  }, [diffs, onUpdateFileContent]);
  const toggleDiff = useCallback((path: string) => {
    setExpandedDiff(prev => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);else next.add(path);
      return next;
    });
  }, []);
  if (!isOpen) return null;
  return <div className={styles.overlay} onClick={e => {
    if (e.target === e.currentTarget && !executing) onClose();
  }}>
      <div className={styles.dialog}>
        {/* Header */}
        <div className={styles.header}>
          <div className={styles.headerLeft}>
            <div className={styles.headerDot} />
            <span className={styles.headerTitle}>AGENT_MODE</span>
            <span className={styles.headerShortcut}>Ctrl+.</span>
          </div>
          <button className={styles.closeBtn} onClick={onClose} disabled={executing}>X</button>
        </div>

        {/* Body */}
        <div className={styles.body}>
          {/* Input Area */}
          <div className={styles.inputArea}>
            <div className={styles.inputWrapper}>
              {/* Context chips */}
              {contextRefs.length > 0 && <div className={styles.contextChips}>
                  {contextRefs.map(ref => <span key={ref.path} className={styles.contextChip}>
                      {ref.isDir ? '📁' : '📄'} {ref.name}
                      <button className={styles.contextChipRemove} onClick={() => handleRemoveContextRef(ref.path)}>×</button>
                    </span>)}
                </div>}

              <textarea ref={textareaRef} className={styles.promptInput} value={prompt} onChange={handlePromptChange} onKeyDown={handleKeyDown} placeholder={t("yuan-code.AgentDialog.k13")} disabled={executing} rows={3} />

              {/* Autocomplete dropdown */}
              {showAutocomplete && autocompleteItems.length > 0 && <div className={styles.autocomplete}>
                  {autocompleteItems.map((item, idx) => <div key={item.path} className={`${styles.autocompleteItem} ${idx === autocompleteIndex ? styles.autocompleteItemActive : ''}`} onClick={() => handleSelectAutocomplete(item)} onMouseEnter={() => setAutocompleteIndex(idx)}>
                      <span className={styles.autocompleteItemIcon}>
                        {item.isDir ? '📁' : '📄'}
                      </span>
                      <span>{item.name}</span>
                      <span className={styles.autocompleteItemPath}>{item.path}</span>
                    </div>)}
                </div>}
            </div>

            <div className={styles.inputActions}>
              <span className={styles.inputHint}>
                {t("yuan-code.AgentDialog.k14")}
              </span>
              <button
                className={styles.sendBtn}
                onClick={handleExecute}
                disabled={executing || !isOnline || (!prompt.trim() && contextRefs.length === 0)}
                title={!isOnline ? t('yuan-code.AgentDialog.offlineHint', { defaultValue: 'AI 离线，本地编辑可用。请连接网络后使用 Agent 执行。' }) : undefined}
              >
                {executing ? t("Xin.k159") : 'Execute'}
              </button>
            </div>
          </div>

          {/* A5 Phase 3 Task 2: 离线降级提示（在输入区下方显示，区别于普通 error） */}
          {!isOnline && <div className={styles.offlineHint}>
              {t('yuan-code.AgentDialog.offlineHint', { defaultValue: 'AI 离线，本地编辑可用。请连接网络后使用 Agent 执行。' })}
            </div>}

          {/* Error */}
          {error && <div className={styles.errorBlock}>
              <span className={styles.errorText}>{error}</span>
            </div>}

          {/* Execution Flow Steps */}
          {flowSteps.length > 0 && <div className={styles.flowSteps}>
              {flowSteps.map(step => <div key={step.id} className={`${styles.flowStep} ${step.status === 'active' ? styles.flowStepActive : step.status === 'done' ? styles.flowStepDone : step.status === 'error' ? styles.flowStepError : ''}`}>
                  <div className={styles.flowStepIcon}>
                    {step.status === 'active' && <span className={styles.spinner} />}
                    {step.status === 'done' && '✓'}
                    {step.status === 'error' && '✗'}
                    {step.status === 'pending' && '○'}
                  </div>
                  <div className={styles.flowStepContent}>
                    <div className={styles.flowStepTitle}>{step.title}</div>
                    {step.content && <div className={styles.flowStepText}>{step.content}</div>}
                  </div>
                </div>)}
            </div>}

          {/* Diff Preview */}
          {diffs.length > 0 && <div className={styles.diffSection}>
              <div className={styles.diffSectionHeader} onClick={() => {
            if (diffs.length === 1) toggleDiff(diffs[0].path);
          }}>
                <span className={styles.diffSectionTitle}>{t("yuan-code.AgentDialog.k15")}{diffs.length})</span>
                <span className={styles.diffSectionToggle}>
                  {diffs.length === 1 ? expandedDiff.has(diffs[0].path) ? '▲' : '▼' : ''}
                </span>
              </div>
              {diffs.map(diff => <div key={diff.path}>
                  <div className={styles.diffSectionHeader} onClick={() => toggleDiff(diff.path)} style={{
              borderBottom: expandedDiff.has(diff.path) ? undefined : 'none'
            }}>
                    <span className={styles.diffSectionTitle} style={{
                fontSize: 11
              }}>
                      {diff.path}
                    </span>
                    <span className={styles.diffSectionToggle}>
                      {expandedDiff.has(diff.path) ? '▲' : '▼'}
                    </span>
                  </div>
                  {expandedDiff.has(diff.path) && <>
                      <div className={styles.diffContent}>
                        {diff.hunks[0]?.lines.map((line, i) => <div key={i} className={`${styles.diffLine} ${line.kind === 'added' ? styles.diffAdded : line.kind === 'removed' ? styles.diffRemoved : styles.diffUnchanged}`}>
                            {line.kind === 'added' ? '+ ' : line.kind === 'removed' ? '- ' : '  '}
                            {line.content}
                          </div>)}
                      </div>
                      {!diffAccepted.has(diff.path) && !diffRejected.has(diff.path) && <div className={styles.diffActions}>
                          <button className={styles.acceptBtn} onClick={() => handleAcceptDiff(diff.path)}>
                            {t("yuan-code.AgentDialog.k16")}
                          </button>
                          <button className={styles.rejectBtn} onClick={() => handleRejectDiff(diff.path)}>
                            {t("yuan-code.AgentDialog.k17")}
                          </button>
                        </div>}
                      {diffAccepted.has(diff.path) && <div className={styles.diffActions}>
                          <span style={{
                  fontFamily: 'Courier New, monospace',
                  fontSize: 11,
                  color: '#00FF00'
                }}>{t("yuan-code.AgentDialog.k18")}</span>
                        </div>}
                      {diffRejected.has(diff.path) && <div className={styles.diffActions}>
                          <span style={{
                  fontFamily: 'Courier New, monospace',
                  fontSize: 11,
                  color: '#FF0000'
                }}>{t("yuan-code.AgentDialog.k19")}</span>
                        </div>}
                    </>}
                </div>)}
            </div>}

          {/* Checkpoints */}
          {checkpoints.length > 0 && <div className={styles.checkpointsSection}>
              <div className={styles.checkpointsHeader} onClick={() => setExpandedCheckpoints(!expandedCheckpoints)}>
                <span className={styles.checkpointsTitle}>{t("yuan-code.AgentDialog.k20")}</span>
                <span className={styles.checkpointCount}>
                  {checkpoints.length} · {expandedCheckpoints ? '▲' : '▼'}
                </span>
              </div>
              {expandedCheckpoints && <div className={styles.checkpointsList}>
                  {checkpoints.map(cp => <div key={cp.id} className={styles.checkpointItem}>
                      <div className={styles.checkpointInfo}>
                        <div className={styles.checkpointTime}>{formatTime(cp.timestamp)}</div>
                        <div className={styles.checkpointDesc}>{cp.description}</div>
                        <div className={styles.checkpointFiles}>
                          {cp.files.map(f => f.path).join(', ')}
                        </div>
                      </div>
                      <button className={styles.rollbackBtn} onClick={() => handleRollback(cp)}>
                        {t("yuan-code.AgentDialog.k21")}
                      </button>
                    </div>)}
                </div>}
            </div>}

          {/* Lint Issues */}
          {lintIssues.length > 0 && <div className={styles.lintSection}>
              <div className={styles.lintHeader} onClick={() => setExpandedLint(!expandedLint)}>
                <span className={styles.lintTitle}>{t("yuan-code.AgentDialog.k22")}</span>
                <span className={styles.lintCount}>
                  {lintIssues.length} {t("yuan-code.AgentDialog.k23")} {expandedLint ? '▲' : '▼'}
                </span>
              </div>
              {expandedLint && <>
                  <div className={styles.lintList}>
                    {lintIssues.map((issue, idx) => <div key={idx} className={styles.lintItem}>
                        <span className={styles.lintSeverity}>{issue.severity}</span>
                        <span className={styles.lintMessage}>{issue.message}</span>
                        <span className={styles.lintLocation}>
                          {issue.file}:{issue.line || '?'}
                        </span>
                      </div>)}
                  </div>
                  <button className={styles.autoFixBtn} onClick={handleAutoFix} disabled={autoFixing}>
                    {autoFixing ? t("yuan-code.AgentDialog.k24") : t("yuan-code.AgentDialog.k25")}
                  </button>
                </>}

              {/* Auto-fix diff */}
              {autoFixDiff && <div className={styles.diffSection} style={{
            margin: '0 12px 12px'
          }}>
                  <div className={styles.diffSectionHeader}>
                    <span className={styles.diffSectionTitle}>{t("yuan-code.AgentDialog.k26")}</span>
                  </div>
                  <div className={styles.diffContent}>
                    {autoFixDiff.hunks[0]?.lines.map((line, i) => <div key={i} className={`${styles.diffLine} ${styles.diffAdded}`}>
                        + {line.content}
                      </div>)}
                  </div>
                  <div className={styles.diffActions}>
                    <button className={styles.acceptBtn} onClick={handleAcceptAutoFix}>
                      {t("yuan-code.AgentDialog.k27")}
                    </button>
                    <button className={styles.rejectBtn} onClick={handleRejectAutoFix}>
                      {t("yuan-code.AgentDialog.k17")}
                    </button>
                  </div>
                </div>}
            </div>}

          {/* Empty state */}
          {flowSteps.length === 0 && !error && <div className={styles.placeholder}>
              <div className={styles.placeholderIcon}>⌨</div>
              <div className={styles.placeholderText}>
                {t("yuan-code.AgentDialog.k28")}
              </div>
              <div className={styles.placeholderHint}>
                {t("yuan-code.AgentDialog.k29")}
              </div>
            </div>}
        </div>
      </div>
    </div>;
}