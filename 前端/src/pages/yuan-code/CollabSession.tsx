/**
 * Yuan Code v3.2 Task 3.4.4 — CollabSession 协作会话组件
 *
 * 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.4（Phase 6 多模型与协作）
 *
 * 功能：
 * 1. 创建协作会话（指定名称 + 工作区根路径）
 * 2. 列出所有活跃会话
 * 3. 加入/退出会话
 * 4. 查看参与者列表 + 在线状态 + 实时光标
 * 5. 关闭会话（仅创建者）
 *
 * 与底层智能的边界：协作会话是非 AI 功能，不调用云端 API，不依赖底层智能。
 */
import { t } from 'i18next';
import { useState, useCallback, useEffect } from 'react';
import { yuanCode } from '@/lib/ipc';
import type {
  CollabSessionInfo,
  CollabSessionSummaryInfo,
  CreateCollabSessionRequest,
} from '@/lib/ipc/yuan-code';
import styles from '../YuanCode.module.css';

interface CollabSessionProps {
  defaultExpanded?: boolean;
  /** 当前工作区根路径（用于默认填充创建表单） */
  workspaceRoot?: string;
}

export default function CollabSession({
  defaultExpanded = false,
  workspaceRoot = '',
}: CollabSessionProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);
  const [sessions, setSessions] = useState<CollabSessionSummaryInfo[]>([]);
  const [selectedSession, setSelectedSession] = useState<CollabSessionInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [createForm, setCreateForm] = useState<CreateCollabSessionRequest>({
    name: '',
    workspace_root: workspaceRoot,
  });
  const [creating, setCreating] = useState(false);

  const loadSessions = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await yuanCode.collabSessionList();
      if (res.code === 0 && res.data) {
        setSessions(res.data);
      } else if (res.code !== 0) {
        setError(res.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (expanded) loadSessions();
  }, [expanded, loadSessions]);

  const handleCreate = useCallback(async () => {
    if (!createForm.name.trim() || !createForm.workspace_root.trim()) {
      setError(t('common.invalidInput'));
      return;
    }
    setCreating(true);
    setError(null);
    try {
      const res = await yuanCode.collabSessionCreate(createForm.name.trim(), createForm.workspace_root.trim());
      if (res.code === 0 && res.data) {
        setShowCreateForm(false);
        setCreateForm({ name: '', workspace_root: workspaceRoot });
        await loadSessions();
        setSelectedSession(res.data);
      } else {
        setError(res.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setCreating(false);
    }
  }, [createForm, loadSessions, workspaceRoot]);

  const handleJoin = useCallback(async (sessionId: string) => {
    setError(null);
    try {
      const res = await yuanCode.collabSessionJoin(sessionId);
      if (res.code === 0 && res.data) {
        setSelectedSession(res.data);
        await loadSessions();
      } else {
        setError(res.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [loadSessions]);

  const handleLeave = useCallback(async (sessionId: string) => {
    setError(null);
    try {
      const res = await yuanCode.collabSessionLeave(sessionId);
      if (res.code === 0) {
        setSelectedSession(null);
        await loadSessions();
      } else {
        setError(res.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [loadSessions]);

  const handleClose = useCallback(async (sessionId: string) => {
    if (!confirm(t('common.remove') + '?')) return;
    setError(null);
    try {
      const res = await yuanCode.collabSessionClose(sessionId);
      if (res.code === 0) {
        setSelectedSession(null);
        await loadSessions();
      } else {
        setError(res.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [loadSessions]);

  const handleRefreshSession = useCallback(async (sessionId: string) => {
    try {
      const res = await yuanCode.collabSessionGet(sessionId);
      if (res.code === 0 && res.data) {
        setSelectedSession(res.data);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);

  return (
    <div className={styles.settingsSection}>
      <div
        style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', cursor: 'pointer' }}
        onClick={() => setExpanded(!expanded)}
      >
        <h3 className={styles.settingsSectionTitle}>
          {t('yuan-code.CollabSession.k1') || 'Collaboration Sessions'}
        </h3>
        <span style={{ fontSize: 11, color: 'var(--nt-text-muted)' }}>
          {expanded ? '▾' : '▸'}
        </span>
      </div>

      <div className={styles.settingRow}>
        <span className={styles.settingDesc}>
          {t('yuan-code.CollabSession.k2') ||
            '多人实时协作会话（创建/加入/退出 + 在线状态 + 实时光标）'}
        </span>
      </div>

      {expanded && (
        <>
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

          {/* 创建按钮 */}
          {!showCreateForm ? (
            <button
              className={styles.btnPurple}
              onClick={() => setShowCreateForm(true)}
              style={{ padding: '4px 12px', fontSize: 11, marginTop: 6 }}
            >
              + {t('yuan-code.CollabSession.k3') || '新建会话'}
            </button>
          ) : (
            <div style={{
              marginTop: 10,
              padding: 12,
              border: '1px solid rgba(0,240,255,0.15)',
              borderRadius: 4,
              background: 'rgba(0,240,255,0.02)',
            }}>
              <div className={styles.formGroup}>
                <label className={styles.formLabel}>{t('yuan-code.CollabSession.k4') || '会话名称'}</label>
                <input
                  className={styles.formInput}
                  placeholder="Refactor login module"
                  value={createForm.name}
                  onChange={(e) => setCreateForm((prev) => ({ ...prev, name: e.target.value }))}
                  style={{ padding: '4px 8px', fontSize: 11 }}
                />
              </div>
              <div className={styles.formGroup} style={{ marginTop: 8 }}>
                <label className={styles.formLabel}>{t('yuan-code.CollabSession.k5') || '工作区根路径'}</label>
                <input
                  className={styles.formInput}
                  placeholder="/workspace/project"
                  value={createForm.workspace_root}
                  onChange={(e) => setCreateForm((prev) => ({ ...prev, workspace_root: e.target.value }))}
                  style={{ padding: '4px 8px', fontSize: 11, fontFamily: 'var(--nt-font-mono)' }}
                />
              </div>
              <div style={{ display: 'flex', gap: 6, marginTop: 10, justifyContent: 'flex-end' }}>
                <button
                  className={styles.btnDanger}
                  onClick={() => {
                    setShowCreateForm(false);
                    setCreateForm({ name: '', workspace_root: workspaceRoot });
                  }}
                  style={{ padding: '3px 10px', fontSize: 11 }}
                >
                  {t('common.cancel')}
                </button>
                <button
                  className={styles.btnPrimary}
                  onClick={handleCreate}
                  disabled={creating || !createForm.name.trim() || !createForm.workspace_root.trim()}
                  style={{ padding: '3px 10px', fontSize: 11 }}
                >
                  {creating ? t('components.AudioEditor.k6') : t('common.save')}
                </button>
              </div>
            </div>
          )}

          {/* 会话列表 */}
          {loading ? (
            <div style={{ padding: 16, textAlign: 'center', color: 'var(--nt-text-muted)', fontSize: 12 }}>
              {t('components.AudioEditor.k6')}...
            </div>
          ) : sessions.length === 0 ? (
            <div style={{ padding: 16, textAlign: 'center', color: 'var(--nt-text-muted)', fontSize: 12 }}>
              {t('yuan-code.CollabSession.k6') || '暂无活跃会话'}
            </div>
          ) : (
            <div style={{ maxHeight: 240, overflowY: 'auto', marginTop: 10 }}>
              {sessions.map((s) => (
                <div
                  key={s.session_id}
                  style={{
                    padding: '8px 10px',
                    marginBottom: 6,
                    border: `1px solid ${selectedSession?.session_id === s.session_id ? 'rgba(0,240,255,0.35)' : 'rgba(0,240,255,0.1)'}`,
                    borderRadius: 4,
                    background: selectedSession?.session_id === s.session_id ? 'rgba(0,240,255,0.04)' : 'rgba(0,240,255,0.02)',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                    <div style={{ flex: 1 }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                        <span style={{
                          fontFamily: 'var(--nt-font-mono)',
                          fontSize: 12,
                          color: 'var(--nt-primary)',
                        }}>
                          {s.name}
                        </span>
                        <span style={{
                          fontSize: 9,
                          padding: '1px 5px',
                          borderRadius: 3,
                          background: 'rgba(0,255,136,0.1)',
                          color: '#00FF88',
                        }}>
                          {s.participant_count} {t('yuan-code.CollabSession.k7') || '在线'}
                        </span>
                      </div>
                      <div style={{ marginTop: 4 }}>
                        <code style={{
                          fontFamily: 'var(--nt-font-mono)',
                          fontSize: 10,
                          color: 'var(--nt-text-muted)',
                        }}>
                          {s.workspace_root}
                        </code>
                      </div>
                    </div>
                    <div style={{ display: 'flex', gap: 6 }}>
                      <button
                        onClick={() => handleJoin(s.session_id)}
                        style={{
                          padding: '2px 8px',
                          fontSize: 10,
                          background: 'none',
                          border: '1px solid rgba(0,240,255,0.2)',
                          borderRadius: 3,
                          color: 'var(--nt-primary)',
                          cursor: 'pointer',
                        }}
                      >
                        {t('yuan-code.CollabSession.k8') || '加入'}
                      </button>
                      <button
                        onClick={() => handleClose(s.session_id)}
                        title={t('common.remove')}
                        style={{
                          background: 'none',
                          border: 'none',
                          color: 'var(--nt-text-muted)',
                          cursor: 'pointer',
                          fontSize: 14,
                          padding: 0,
                          lineHeight: 1,
                        }}
                      >
                        ✕
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* 当前会话详情 */}
          {selectedSession && (
            <div style={{
              marginTop: 12,
              padding: 12,
              border: '1px solid rgba(0,240,255,0.15)',
              borderRadius: 4,
              background: 'rgba(0,240,255,0.02)',
            }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
                <span style={{ fontSize: 12, color: 'var(--nt-primary)', fontWeight: 600 }}>
                  {selectedSession.name}
                </span>
                <div style={{ display: 'flex', gap: 6 }}>
                  <button
                    onClick={() => handleRefreshSession(selectedSession.session_id)}
                    style={{
                      padding: '2px 8px',
                      fontSize: 10,
                      background: 'none',
                      border: '1px solid rgba(0,240,255,0.2)',
                      borderRadius: 3,
                      color: 'var(--nt-primary)',
                      cursor: 'pointer',
                    }}
                  >
                    ↻
                  </button>
                  <button
                    onClick={() => handleLeave(selectedSession.session_id)}
                    style={{
                      padding: '2px 8px',
                      fontSize: 10,
                      background: 'none',
                      border: '1px solid rgba(255,0,110,0.2)',
                      borderRadius: 3,
                      color: '#FF006E',
                      cursor: 'pointer',
                    }}
                  >
                    {t('yuan-code.CollabSession.k9') || '退出'}
                  </button>
                </div>
              </div>

              <div style={{ fontSize: 10, color: 'var(--nt-text-muted)', marginBottom: 6 }}>
                <code style={{ fontFamily: 'var(--nt-font-mono)' }}>
                  {selectedSession.workspace_root}
                </code>
              </div>

              {/* 参与者列表 */}
              <div style={{ fontSize: 11, marginTop: 8 }}>
                <div style={{ marginBottom: 4, color: 'var(--nt-text-muted)' }}>
                  {t('yuan-code.CollabSession.k7') || '在线'} ({selectedSession.participants.length})
                </div>
                {selectedSession.participants.map((p) => (
                  <div
                    key={p.user_id}
                    style={{
                      padding: '4px 8px',
                      marginBottom: 4,
                      borderRadius: 3,
                      background: 'rgba(0,240,255,0.04)',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                      <span style={{
                        width: 6,
                        height: 6,
                        borderRadius: '50%',
                        background: '#00FF88',
                        display: 'inline-block',
                      }} />
                      <span style={{ color: 'var(--nt-text-primary)' }}>{p.display_name}</span>
                      {p.user_id === selectedSession.created_by && (
                        <span style={{
                          fontSize: 9,
                          padding: '1px 5px',
                          borderRadius: 3,
                          background: 'rgba(255,180,0,0.12)',
                          color: '#FFB400',
                        }}>
                          {t('yuan-code.CollabSession.k10') || '创建者'}
                        </span>
                      )}
                    </div>
                    {p.cursor && (
                      <code style={{
                        fontFamily: 'var(--nt-font-mono)',
                        fontSize: 9,
                        color: 'var(--nt-text-muted)',
                      }}>
                        {p.cursor.file_path}:{p.cursor.start_line}:{p.cursor.start_column}
                      </code>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
