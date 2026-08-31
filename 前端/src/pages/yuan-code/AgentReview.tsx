/**
 * Yuan Code v3.1 Task 3.1.6 — AgentReview 组件
 *
 * 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.1.6
 *       + 项目核心设计意图 §八（敏感操作必须经用户确认）
 *
 * 功能：
 * 1. 接收 AgentPlanView 传来的 AgentV3Result（diff 列表）
 * 2. 按文件展示 diff（unified_diff 着色 + added/removed 行数统计）
 * 3. 单文件勾选（用于部分批准）
 * 4. 三种决策：Approve（全部应用）/ Reject（全部丢弃）/ ApprovePartial（仅应用勾选）
 * 5. 调用 yuan_v3_agent_review 将 diff 应用到工作区文件系统
 *
 * 强制约束：v3.1 阶段 diff 应用由后端完成（apply_diff_to_workspace），前端仅负责 UI。
 */
import { t } from "i18next";
import { useState, useCallback, useMemo } from 'react';
import { yuanCode } from '@/lib/ipc';
import type {
  AgentV3Result,
  AgentV3ReviewResponse,
  AgentV3FileDiff,
} from '@/lib/ipc/yuan-code';
import styles from '../YuanCode.module.css';

interface AgentReviewProps {
  /** Agent ID（来自 AgentPlanView） */
  agentId: string;
  /** Agent 执行结果（含 diffs） */
  result: AgentV3Result | null;
  /** 工作区路径（应用到文件系统时需要） */
  workspacePath?: string;
  /** Review 完成后回调（应用成功 / 拒绝） */
  onReviewed?: (response: AgentV3ReviewResponse) => void;
  /** 重置（清空当前 result） */
  onReset?: () => void;
}

type Decision = 'approve' | 'reject' | 'approve_partial';

export default function AgentReview({
  agentId,
  result,
  workspacePath = '',
  onReviewed,
  onReset,
}: AgentReviewProps) {
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set());
  const [expandedFile, setExpandedFile] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [reviewResponse, setReviewResponse] = useState<AgentV3ReviewResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [feedback, setFeedback] = useState('');

  // 初始化选中所有文件
  const diffs = result?.diffs || [];
  const allSelected = useMemo(
    () => diffs.length > 0 && selectedFiles.size === diffs.length,
    [diffs, selectedFiles]
  );

  const toggleFile = useCallback((path: string) => {
    setSelectedFiles(prev => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }, []);

  const toggleAll = useCallback(() => {
    if (allSelected) {
      setSelectedFiles(new Set());
    } else {
      setSelectedFiles(new Set(diffs.map(d => d.path)));
    }
  }, [allSelected, diffs]);

  const handleReview = useCallback(async (decision: Decision) => {
    if (!result) return;
    setSubmitting(true);
    setError(null);
    try {
      const response = await yuanCode.agentV3Review({
        agent_id: agentId,
        decision,
        selected_files: decision === 'approve_partial' ? Array.from(selectedFiles) : undefined,
        feedback: feedback.trim() || undefined,
        workspace_path: workspacePath,
      });
      if (response.code === 0 && response.data) {
        setReviewResponse(response.data);
        onReviewed?.(response.data);
      } else {
        setError(response.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  }, [result, agentId, selectedFiles, feedback, workspacePath, onReviewed]);

  const handleReset = useCallback(() => {
    setSelectedFiles(new Set());
    setExpandedFile(null);
    setReviewResponse(null);
    setError(null);
    setFeedback('');
    onReset?.();
  }, [onReset]);

  // 空状态
  if (!result) {
    return (
      <div className={styles.settingsSection}>
        <h3 className={styles.settingsSectionTitle}>
          {t('yuan-code.AgentReview.k1')}
        </h3>
        <div className={styles.settingRow}>
          <span className={styles.settingDesc}>
            {t('yuan-code.AgentReview.k2')}
          </span>
        </div>
      </div>
    );
  }

  // Review 完成后的结果展示
  if (reviewResponse) {
    return (
      <div className={styles.settingsSection}>
        <h3 className={styles.settingsSectionTitle}>
          {t('yuan-code.AgentReview.k3')}
        </h3>

        <div style={{
          padding: '8px 10px',
          marginBottom: 10,
          borderRadius: 4,
          background: reviewResponse.errors.length > 0
            ? 'rgba(255,180,0,0.06)'
            : 'rgba(0,255,136,0.04)',
          border: `1px solid ${reviewResponse.errors.length > 0 ? 'rgba(255,180,0,0.2)' : 'rgba(0,255,136,0.18)'}`,
          fontSize: 11,
        }}>
          <div style={{ marginBottom: 4 }}>
            <span style={{ color: 'var(--nt-primary)', fontFamily: 'var(--nt-font-mono)' }}>
              {reviewResponse.decision}
            </span>
            {reviewResponse.applied_files.length > 0 && (
              <span style={{ marginLeft: 8, color: '#00FF88' }}>
                ✅ {t('yuan-code.AgentReview.k4')}: {reviewResponse.applied_files.length}
              </span>
            )}
            {reviewResponse.skipped_files.length > 0 && (
              <span style={{ marginLeft: 8, color: 'var(--nt-text-muted)' }}>
                ⏭️ {t('yuan-code.AgentReview.k5')}: {reviewResponse.skipped_files.length}
              </span>
            )}
            {reviewResponse.errors.length > 0 && (
              <span style={{ marginLeft: 8, color: '#FF006E' }}>
                ❌ {t('yuan-code.AgentReview.k6')}: {reviewResponse.errors.length}
              </span>
            )}
          </div>

          {reviewResponse.applied_files.length > 0 && (
            <div style={{ marginTop: 4 }}>
              <div style={{ fontSize: 10, color: 'var(--nt-text-muted)' }}>{t('yuan-code.AgentReview.k7')}:</div>
              {reviewResponse.applied_files.map(f => (
                <code key={f} style={{
                  display: 'block',
                  fontSize: 10,
                  color: '#00FF88',
                  fontFamily: 'var(--nt-font-mono)',
                  padding: '1px 0',
                }}>
                  ✓ {f}
                </code>
              ))}
            </div>
          )}

          {reviewResponse.errors.length > 0 && (
            <div style={{ marginTop: 6 }}>
              <div style={{ fontSize: 10, color: 'var(--nt-text-muted)' }}>{t('yuan-code.AgentReview.k6')}:</div>
              {reviewResponse.errors.map((e, i) => (
                <div key={i} style={{ fontSize: 10, color: '#FF006E', fontFamily: 'var(--nt-font-mono)', padding: '1px 0' }}>
                  ✗ {e}
                </div>
              ))}
            </div>
          )}
        </div>

        <button
          className={styles.btnPrimary}
          onClick={handleReset}
          style={{ padding: '4px 12px', fontSize: 11 }}
        >
          {t('common.reset')}
        </button>
      </div>
    );
  }

  return (
    <div className={styles.settingsSection}>
      <h3 className={styles.settingsSectionTitle}>
        {t('yuan-code.AgentReview.k1')}
      </h3>

      <div className={styles.settingRow}>
        <span className={styles.settingDesc}>
          {t('yuan-code.AgentReview.k8', { count: diffs.length })}
        </span>
      </div>

      {/* 强制约束提示 */}
      <div style={{
        padding: '6px 10px',
        marginBottom: 10,
        background: 'rgba(255,180,0,0.06)',
        border: '1px solid rgba(255,180,0,0.18)',
        borderRadius: 4,
        fontSize: 11,
        color: '#FFB86C',
      }}>
        ⚠ {t('yuan-code.AgentReview.k9')}
      </div>

      {error && (
        <div style={{
          padding: '6px 10px',
          marginBottom: 10,
          background: 'rgba(255,0,110,0.06)',
          border: '1px solid rgba(255,0,110,0.2)',
          borderRadius: 4,
          fontSize: 11,
          color: '#FF006E',
        }}>
          {error}
        </div>
      )}

      {/* 全选/反选 */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
        <input
          type="checkbox"
          checked={allSelected}
          onChange={toggleAll}
          style={{ cursor: 'pointer' }}
        />
        <span style={{ fontSize: 11, color: 'var(--nt-text-secondary)' }}>
          {t('yuan-code.AgentReview.k10')} ({selectedFiles.size}/{diffs.length})
        </span>
      </div>

      {/* 文件 diff 列表 */}
      <div style={{ maxHeight: 400, overflowY: 'auto' }}>
        {diffs.map(diff => (
          <FileDiffItem
            key={diff.path}
            diff={diff}
            selected={selectedFiles.has(diff.path)}
            expanded={expandedFile === diff.path}
            onToggleSelect={() => toggleFile(diff.path)}
            onToggleExpand={() => setExpandedFile(expandedFile === diff.path ? null : diff.path)}
          />
        ))}
      </div>

      {/* 反馈输入（可选） */}
      <div className={styles.formGroup} style={{ marginTop: 10 }}>
        <label className={styles.formLabel}>{t('yuan-code.AgentReview.k11')}</label>
        <textarea
          className={styles.formInput}
          placeholder={t('yuan-code.AgentReview.k12')}
          value={feedback}
          onChange={e => setFeedback(e.target.value)}
          rows={2}
          style={{
            padding: '4px 8px',
            fontSize: 11,
            width: '100%',
            resize: 'vertical',
          }}
        />
      </div>

      {/* 决策按钮 */}
      <div style={{ display: 'flex', gap: 6, marginTop: 10, flexWrap: 'wrap' }}>
        <button
          className={styles.btnPrimary}
          onClick={() => handleReview('approve')}
          disabled={submitting}
          style={{
            padding: '5px 14px',
            fontSize: 11,
            background: 'rgba(0,255,136,0.12)',
            borderColor: 'rgba(0,255,136,0.4)',
            color: '#00FF88',
          }}
        >
          ✅ {t('yuan-code.AgentReview.k13')}
        </button>
        <button
          className={styles.btnPurple}
          onClick={() => handleReview('approve_partial')}
          disabled={submitting || selectedFiles.size === 0}
          style={{ padding: '5px 14px', fontSize: 11 }}
        >
          ◐ {t('yuan-code.AgentReview.k14')} ({selectedFiles.size})
        </button>
        <button
          className={styles.btnDanger}
          onClick={() => handleReview('reject')}
          disabled={submitting}
          style={{ padding: '5px 14px', fontSize: 11 }}
        >
          ❌ {t('yuan-code.AgentReview.k15')}
        </button>
        <button
          onClick={handleReset}
          disabled={submitting}
          style={{
            padding: '5px 10px',
            fontSize: 11,
            background: 'none',
            border: '1px solid rgba(255,255,255,0.1)',
            borderRadius: 3,
            color: 'var(--nt-text-muted)',
            cursor: 'pointer',
          }}
        >
          {t('common.reset')}
        </button>
      </div>

      {submitting && (
        <div style={{ marginTop: 8, fontSize: 10, color: 'var(--nt-text-muted)' }}>
          {t('components.AudioEditor.k6')}...
        </div>
      )}
    </div>
  );
}

/** 单文件 diff 展示子组件 */
function FileDiffItem({
  diff,
  selected,
  expanded,
  onToggleSelect,
  onToggleExpand,
}: {
  diff: AgentV3FileDiff;
  selected: boolean;
  expanded: boolean;
  onToggleSelect: () => void;
  onToggleExpand: () => void;
}) {
  return (
    <div style={{
      marginBottom: 6,
      border: '1px solid rgba(0,240,255,0.1)',
      borderRadius: 3,
      background: 'rgba(0,240,255,0.02)',
      overflow: 'hidden',
    }}>
      <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '6px 8px',
      }}>
        <input
          type="checkbox"
          checked={selected}
          onChange={onToggleSelect}
          style={{ cursor: 'pointer' }}
          onClick={(e) => e.stopPropagation()}
        />
        <div
          style={{ flex: 1, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 6 }}
          onClick={onToggleExpand}
        >
          <span style={{
            fontSize: 10,
            color: expanded ? 'var(--nt-primary)' : 'var(--nt-text-muted)',
            transform: expanded ? 'rotate(90deg)' : '',
            transition: 'transform 0.15s',
          }}>
            ▸
          </span>
          <code style={{
            fontSize: 11,
            color: 'var(--nt-text-primary)',
            fontFamily: 'var(--nt-font-mono)',
            flex: 1,
          }}>
              {diff.path}
            </code>
            {diff.is_new_file && (
              <span style={{
                fontSize: 9,
                padding: '1px 5px',
                borderRadius: 3,
                background: 'rgba(0,255,136,0.12)',
                color: '#00FF88',
              }}>
                {t('yuan-code.AgentReview.k16')}
              </span>
            )}
        </div>
        <div style={{ display: 'flex', gap: 6, fontSize: 10, fontFamily: 'var(--nt-font-mono)' }}>
          {diff.added_lines > 0 && (
            <span style={{ color: '#00FF88' }}>+{diff.added_lines}</span>
          )}
          {diff.removed_lines > 0 && (
            <span style={{ color: '#FF006E' }}>-{diff.removed_lines}</span>
          )}
        </div>
      </div>

      {expanded && (
        <div style={{
          padding: '6px 8px',
          borderTop: '1px solid rgba(0,240,255,0.08)',
          background: 'rgba(0,0,0,0.18)',
          maxHeight: 280,
          overflowY: 'auto',
        }}>
          {diff.unified_diff ? (
            <pre style={{
              margin: 0,
              fontSize: 10,
              fontFamily: 'var(--nt-font-mono)',
              lineHeight: 1.5,
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-word',
            }}>
              {diff.unified_diff.split('\n').map((line, i) => {
                let color = 'var(--nt-text-secondary)';
                let bg = 'transparent';
                if (line.startsWith('+') && !line.startsWith('+++')) {
                  color = '#00FF88';
                  bg = 'rgba(0,255,136,0.06)';
                } else if (line.startsWith('-') && !line.startsWith('---')) {
                  color = '#FF006E';
                  bg = 'rgba(255,0,110,0.06)';
                } else if (line.startsWith('@@')) {
                  color = 'var(--nt-primary)';
                  bg = 'rgba(0,240,255,0.04)';
                } else if (line.startsWith('diff ') || line.startsWith('index ')) {
                  color = 'var(--nt-text-muted)';
                }
                return (
                  <div key={i} style={{ color, background: bg, padding: '0 4px' }}>
                    {line || ' '}
                  </div>
                );
              })}
            </pre>
          ) : (
            <div style={{ fontSize: 10, color: 'var(--nt-text-muted)', fontStyle: 'italic' }}>
              {t('yuan-code.AgentReview.k17')}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
