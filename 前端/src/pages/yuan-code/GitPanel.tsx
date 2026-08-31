import { t } from "i18next";
import { useState, useEffect, useCallback } from 'react';
import { ipc } from '@/lib/ipc';
import styles from './GitPanel.module.css';
interface GitFileStatus {
  path: string;
  status: string;
  staged: boolean;
}
interface GitStatusResult {
  branch: string;
  files: GitFileStatus[];
}
interface GitBranchesResult {
  branches: string[];
  current: string;
}
interface GitCommit {
  hash: string;
  message: string;
  author: string;
  timestamp: number;
}
interface GitPanelProps {
  workspacePath: string;
}
export default function GitPanel({
  workspacePath
}: GitPanelProps) {
  const [status, setStatus] = useState<GitStatusResult | null>(null);
  const [branches, setBranches] = useState<GitBranchesResult | null>(null);
  const [commits, setCommits] = useState<GitCommit[]>([]);
  const [commitMessage, setCommitMessage] = useState('');
  const [expandedDiff, setExpandedDiff] = useState<string | null>(null);
  const [diffContent, setDiffContent] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [logExpanded, setLogExpanded] = useState(false);
  const refreshStatus = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true);
    setError('');
    try {
      const res = await ipc.invoke<GitStatusResult>('git_status', {
        workspace_path: workspacePath
      });
      if (res.code === 0 && res.data) {
        setStatus(res.data);
      } else {
        setError(res.message || t("yuan-code.GitPanel.k1"));
        setStatus(null);
      }
    } catch {
      setError(t("yuan-code.GitPanel.k2"));
      setStatus(null);
    }
    setLoading(false);
  }, [workspacePath]);
  const refreshBranches = useCallback(async () => {
    if (!workspacePath) return;
    try {
      const res = await ipc.invoke<GitBranchesResult>('git_branches', {
        workspace_path: workspacePath
      });
      if (res.code === 0 && res.data) {
        setBranches(res.data);
      }
    } catch {
      // ignore
    }
  }, [workspacePath]);
  const refreshLog = useCallback(async () => {
    if (!workspacePath) return;
    try {
      const res = await ipc.invoke<GitCommit[]>('git_log', {
        workspace_path: workspacePath,
        count: 20
      });
      if (res.code === 0 && res.data) {
        setCommits(res.data);
      }
    } catch {
      // ignore
    }
  }, [workspacePath]);
  const refreshAll = useCallback(() => {
    refreshStatus();
    refreshBranches();
    refreshLog();
  }, [refreshStatus, refreshBranches, refreshLog]);
  useEffect(() => {
    refreshAll();
  }, [workspacePath]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleStageFile = async (filePath: string) => {
    await ipc.invoke('git_stage_file', {
      workspace_path: workspacePath,
      file_path: filePath
    });
    refreshAll();
  };
  const handleUnstageFile = async (filePath: string) => {
    await ipc.invoke('git_unstage_file', {
      workspace_path: workspacePath,
      file_path: filePath
    });
    refreshAll();
  };
  const handleStageAll = async () => {
    await ipc.invoke('git_stage_all', {
      workspace_path: workspacePath
    });
    refreshAll();
  };
  const handleCommit = async () => {
    if (!commitMessage.trim()) return;
    setError('');
    const res = await ipc.invoke<string>('git_commit', {
      workspace_path: workspacePath,
      message: commitMessage.trim()
    });
    if (res.code === 0) {
      setCommitMessage('');
      refreshAll();
    } else {
      setError(res.message || t("yuan-code.GitPanel.k3"));
    }
  };
  const handlePush = async () => {
    if (!branches) return;
    setError('');
    const res = await ipc.invoke('git_push', {
      workspace_path: workspacePath,
      remote: 'origin',
      branch: branches.current
    });
    if (res.code !== 0) {
      setError(res.message || t("yuan-code.GitPanel.k4"));
    }
  };
  const handlePull = async () => {
    if (!branches) return;
    setError('');
    const res = await ipc.invoke('git_pull', {
      workspace_path: workspacePath,
      remote: 'origin',
      branch: branches.current
    });
    if (res.code !== 0) {
      setError(res.message || t("yuan-code.GitPanel.k5"));
    } else {
      refreshAll();
    }
  };
  const handleCheckout = async (branch: string) => {
    setError('');
    const res = await ipc.invoke('git_checkout', {
      workspace_path: workspacePath,
      branch
    });
    if (res.code === 0) {
      refreshAll();
    } else {
      setError(res.message || t("yuan-code.GitPanel.k6"));
    }
  };
  const handleInit = async () => {
    setError('');
    const res = await ipc.invoke('git_init', {
      workspace_path: workspacePath
    });
    if (res.code === 0) {
      refreshAll();
    } else {
      setError(res.message || t("yuan-code.GitPanel.k7"));
    }
  };
  const handleToggleDiff = async (filePath: string) => {
    if (expandedDiff === filePath) {
      setExpandedDiff(null);
      setDiffContent('');
      return;
    }
    setExpandedDiff(filePath);
    try {
      const res = await ipc.invoke<string>('git_diff_file', {
        workspace_path: workspacePath,
        file_path: filePath
      });
      if (res.code === 0 && res.data) {
        setDiffContent(res.data);
      } else {
        setDiffContent('');
      }
    } catch {
      setDiffContent('');
    }
  };
  const stagedFiles = status?.files.filter(f => f.staged) || [];
  const unstagedFiles = status?.files.filter(f => !f.staged) || [];
  const statusColor = (s: string) => {
    switch (s) {
      case 'M':
        return '#FFD700';
      case 'A':
        return '#00FF00';
      case 'D':
        return '#FF0000';
      case 'R':
        return '#B026FF';
      default:
        return '#00FF00';
    }
  };

  // Not a git repo
  if (!status && !loading && !error) {
    return <div className={styles.container}>
        <div className={styles.emptyState}>
          <div className={styles.emptyIcon}>git</div>
          <div className={styles.emptyText}>{t("yuan-code.GitPanel.k8")}</div>
          <button className={styles.btn} onClick={handleInit}>
            $ git init
          </button>
        </div>
      </div>;
  }
  return <div className={styles.container}>
      {/* Header */}
      <div className={styles.header}>
        <div className={styles.headerLeft}>
          <span className={styles.branchIcon}>⎇</span>
          <span className={styles.branchName}>
            {branches?.current || status?.branch || '...'}
          </span>
          {branches && branches.branches.length > 1 && <select className={styles.branchSelect} value={branches.current} onChange={e => handleCheckout(e.target.value)}>
              {branches.branches.map(b => <option key={b} value={b}>{b === branches.current ? `* ${b}` : b}</option>)}
            </select>}
        </div>
        <div className={styles.headerRight}>
          <button className={styles.btn} onClick={handlePull} title="Pull">↓ Pull</button>
          <button className={styles.btn} onClick={handlePush} title="Push">↑ Push</button>
          <button className={styles.btn} onClick={refreshAll} title="Refresh">↻</button>
        </div>
      </div>

      {error && <div className={styles.errorBar}>{error}</div>}

      {/* Changes */}
      <div className={styles.changesSection}>
        {/* Staged Changes */}
        {stagedFiles.length > 0 && <div className={styles.changeGroup}>
            <div className={styles.changeGroupTitle}>
              <span className={styles.dot} style={{
            background: '#00FF00'
          }} />
              Staged Changes ({stagedFiles.length})
            </div>
            {stagedFiles.map(f => <div key={`staged-${f.path}`} className={styles.fileItem}>
                <span className={styles.statusBadge} style={{
            color: statusColor(f.status),
            borderColor: statusColor(f.status)
          }}>
                  {f.status}
                </span>
                <span className={styles.filePath} onClick={() => handleToggleDiff(f.path)}>
                  {f.path}
                </span>
                <button className={styles.btnSmall} onClick={() => handleUnstageFile(f.path)}>
                  Unstage
                </button>
              </div>)}
          </div>}

        {/* Unstaged Changes */}
        {unstagedFiles.length > 0 && <div className={styles.changeGroup}>
            <div className={styles.changeGroupTitle}>
              <span className={styles.dot} style={{
            background: '#FFD700'
          }} />
              Changes ({unstagedFiles.length})
            </div>
            {unstagedFiles.map(f => <div key={`unstaged-${f.path}`} className={styles.fileItem}>
                <span className={styles.statusBadge} style={{
            color: statusColor(f.status),
            borderColor: statusColor(f.status)
          }}>
                  {f.status}
                </span>
                <span className={styles.filePath} onClick={() => handleToggleDiff(f.path)}>
                  {f.path}
                </span>
                <button className={styles.btnSmall} onClick={() => handleStageFile(f.path)}>
                  Stage
                </button>
              </div>)}
          </div>}

        {stagedFiles.length === 0 && unstagedFiles.length === 0 && <div className={styles.cleanMessage}>working tree clean</div>}
      </div>

      {/* Diff Preview */}
      {expandedDiff && <div className={styles.diffSection}>
          <div className={styles.diffHeader}>
            <span>diff -- {expandedDiff}</span>
            <button className={styles.btnSmall} onClick={() => {
          setExpandedDiff(null);
          setDiffContent('');
        }}>
              ×
            </button>
          </div>
          <pre className={styles.diffContent}>
            {diffContent ? diffContent.split('\n').map((line, i) => {
          let className = styles.diffLine;
          if (line.startsWith('+')) className = `${styles.diffLine} ${styles.diffAdded}`;else if (line.startsWith('-')) className = `${styles.diffLine} ${styles.diffRemoved}`;else if (line.startsWith('@@')) className = `${styles.diffLine} ${styles.diffHunk}`;
          return <span key={i} className={className}>
                  {line}{'\n'}
                </span>;
        }) : <span className={styles.diffLoading}>Loading diff...</span>}
          </pre>
        </div>}

      {/* Commit */}
      <div className={styles.commitSection}>
        <textarea className={styles.commitInput} placeholder="Commit message..." value={commitMessage} onChange={e => setCommitMessage(e.target.value)} onKeyDown={e => {
        if (e.key === 'Enter' && e.ctrlKey) {
          handleCommit();
        }
      }} rows={3} />
        <div className={styles.commitActions}>
          <button className={styles.btn} onClick={handleStageAll}>Stage All</button>
          <button className={styles.btnPrimary} onClick={handleCommit} disabled={!commitMessage.trim() || stagedFiles.length === 0}>
            Commit
          </button>
        </div>
      </div>

      {/* Log */}
      <div className={styles.logSection}>
        <div className={styles.logHeader} onClick={() => setLogExpanded(!logExpanded)}>
          <span className={styles.logToggle}>{logExpanded ? '▼' : '▶'}</span>
          <span>Recent Commits ({commits.length})</span>
        </div>
        {logExpanded && <div className={styles.logList}>
            {commits.map(c => <div key={c.hash} className={styles.logItem}>
                <span className={styles.logHash}>{c.hash.substring(0, 7)}</span>
                <span className={styles.logMsg}>{c.message.split('\n')[0]}</span>
                <span className={styles.logAuthor}>{c.author}</span>
                <span className={styles.logTime}>
                  {new Date(c.timestamp * 1000).toLocaleDateString()}
                </span>
              </div>)}
            {commits.length === 0 && <div className={styles.logEmpty}>No commits yet</div>}
          </div>}
      </div>
    </div>;
}