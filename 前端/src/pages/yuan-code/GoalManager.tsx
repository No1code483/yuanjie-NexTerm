import { t } from "i18next";
import { useState, useCallback } from 'react';
import { yuanGoal } from '@/lib/ipc';
import styles from '../YuanCode.module.css';
interface GoalSnapshot {
  goal_uuid: string;
  session_id: string;
  title: string;
  description: string;
  status: string;
  priority: number;
  progress_pct: number;
  token_budget: number | null;
  tokens_used: number;
  token_pct: number;
  created_at: number;
  updated_at: number;
}
interface ContinuationData {
  goal_uuid: string;
  title: string;
  status: string;
  progress_pct: number;
  tokens_used: number;
  token_budget: number | null;
  continuation_prompt: string;
  checkpoints_count: number;
}
const STATUS_LABELS: Record<string, string> = {
  pending: t("components.intelligence.SuggestionsPanel.k13"),
  in_progress: t("yuan-code.AgentPanel.k2"),
  paused: t("yuan-code.GoalManager.k1"),
  completed: t("home.TodoPanel.k2"),
  aborted: t("yuan-code.GoalManager.k2")
};
const STATUS_COLORS: Record<string, string> = {
  pending: '#888',
  in_progress: '#00F0FF',
  paused: '#FFB86C',
  completed: '#50FA7B',
  aborted: '#FF6B6B'
};
interface Props {
  onClose: () => void;
}
export function GoalManager({
  onClose
}: Props) {
  const [goals, setGoals] = useState<GoalSnapshot[]>([]);
  const [loading, setLoading] = useState(false);
  const [sessionId, setSessionId] = useState('');
  const [showCreate, setShowCreate] = useState(false);
  const [newGoal, setNewGoal] = useState({
    title: '',
    description: '',
    token_budget: 100000,
    priority: 0
  });
  const [continuation, setContinuation] = useState<ContinuationData | null>(null);
  const [selectedGoalUuid, setSelectedGoalUuid] = useState<string | null>(null);
  const [msg, setMsg] = useState<{
    type: 'ok' | 'err';
    text: string;
  } | null>(null);
  const loadGoals = useCallback(async () => {
    if (!sessionId.trim()) return;
    setLoading(true);
    try {
      const res = await yuanGoal.list(sessionId);
      setGoals(res.data || []);
    } catch {
      setGoals([]);
    } finally {
      setLoading(false);
    }
  }, [sessionId]);
  const showMsg = (type: 'ok' | 'err', text: string) => {
    setMsg({
      type,
      text
    });
    setTimeout(() => setMsg(null), 3000);
  };
  const handleCreate = async () => {
    if (!newGoal.title.trim() || !sessionId.trim()) return;
    try {
      await yuanGoal.create({
        session_id: sessionId,
        title: newGoal.title,
        description: newGoal.description,
        total_token_budget: newGoal.token_budget,
        priority: newGoal.priority
      });
      showMsg('ok', t("lib.ipcMock.k173"));
      setShowCreate(false);
      setNewGoal({
        title: '',
        description: '',
        token_budget: 100000,
        priority: 0
      });
      loadGoals();
    } catch (e: any) {
      showMsg('err', t("yuan-code.GoalManager.k3", {
        arg0: e?.message || e
      }));
    }
  };
  const handleAction = async (goalUuid: string, action: string) => {
    try {
      const goal = goals.find(g => g.goal_uuid === goalUuid);
      if (!goal) return;
      const goalId = (goal as any).id;
      if (!goalId) return;
      switch (action) {
        case 'start':
          await yuanGoal.start(goalId);
          break;
        case 'pause':
          await yuanGoal.pause(goalId);
          break;
        case 'complete':
          await yuanGoal.complete(goalId);
          break;
        case 'abort':
          await yuanGoal.abort(goalId, t("yuan-code.GoalManager.k4"));
          break;
        case 'continue':
          const data = await yuanGoal.buildContinuation(goalId);
          setContinuation(data.data || null);
          setSelectedGoalUuid(goalUuid);
          return;
      }
      showMsg('ok', t("yuan-code.GoalManager.k5", {
        action: action
      }));
      loadGoals();
    } catch (e: any) {
      showMsg('err', t("yuan-code.GoalManager.k6", {
        arg0: e?.message || e
      }));
    }
  };
  const handleDelete = async (goalUuid: string) => {
    const goal = goals.find(g => g.goal_uuid === goalUuid);
    if (!goal) return;
    const goalId = (goal as any).id;
    if (!goalId) return;
    try {
      await yuanGoal.delete(goalId);
      showMsg('ok', t("lib.ipcMock.k175"));
      loadGoals();
    } catch (e: any) {
      showMsg('err', t("yuan-code.GoalManager.k7", {
        arg0: e?.message || e
      }));
    }
  };
  return <div className={styles.modal}>
      <div className={styles.modalContent} style={{
      maxWidth: 720,
      maxHeight: '85vh'
    }}>
        <div className={styles.modalHeader}>
          <h2 className={styles.modalTitle}>{t("yuan-code.GoalManager.k8")}</h2>
          <button onClick={onClose} style={{
          background: 'none',
          border: 'none',
          color: 'var(--nt-text-secondary)',
          cursor: 'pointer',
          fontSize: 18,
          padding: '0 4px'
        }}>
            ✕
          </button>
        </div>

        <div className={styles.modalBody} style={{
        minHeight: 300,
        maxHeight: '65vh',
        overflowY: 'auto'
      }}>
          {/* 会话ID */}
          <div className={styles.formGroup}>
            <label className={styles.formLabel}>{t("yuan-code.GoalManager.k9")}</label>
            <div style={{
            display: 'flex',
            gap: 8
          }}>
              <input className={styles.formInput} value={sessionId} onChange={e => setSessionId(e.target.value)} placeholder={t("yuan-code.GoalManager.k10")} style={{
              flex: 1,
              padding: '4px 10px',
              fontSize: 11
            }} />
              <button className={styles.btnPurple} onClick={loadGoals} disabled={loading || !sessionId.trim()} style={{
              padding: '4px 12px',
              fontSize: 11
            }}>
                {loading ? t("common.loading") : t("yuan-code.GoalManager.k11")}
              </button>
            </div>
          </div>

          {msg && <div style={{
          padding: '6px 10px',
          borderRadius: 4,
          marginBottom: 10,
          fontSize: 11,
          background: msg.type === 'ok' ? 'rgba(80,250,123,0.1)' : 'rgba(255,107,107,0.1)',
          border: `1px solid ${msg.type === 'ok' ? 'rgba(80,250,123,0.3)' : 'rgba(255,107,107,0.3)'}`,
          color: msg.type === 'ok' ? '#50FA7B' : '#FF6B6B'
        }}>
              {msg.text}
            </div>}

          {/* 创建目标 */}
          {showCreate && <div style={{
          padding: 12,
          borderRadius: 6,
          marginBottom: 12,
          border: '1px solid rgba(0,240,255,0.15)',
          background: 'rgba(0,240,255,0.03)'
        }}>
              <div style={{
            fontSize: 13,
            fontWeight: 600,
            color: 'var(--nt-primary)',
            marginBottom: 10
          }}>
                {t("yuan-code.GoalManager.k12")}
              </div>
              <div className={styles.formGroup}>
                <label className={styles.formLabel}>{t("yuan-code.GoalManager.k13")}</label>
                <input className={styles.formInput} value={newGoal.title} onChange={e => setNewGoal(prev => ({
              ...prev,
              title: e.target.value
            }))} placeholder={t("yuan-code.GoalManager.k14")} style={{
              padding: '4px 8px',
              fontSize: 11
            }} />
              </div>
              <div className={styles.formGroup}>
                <label className={styles.formLabel}>{t("common.description")}</label>
                <textarea className={styles.formInput} value={newGoal.description} onChange={e => setNewGoal(prev => ({
              ...prev,
              description: e.target.value
            }))} placeholder={t("yuan-code.GoalManager.k15")} rows={2} style={{
              padding: '4px 8px',
              fontSize: 11,
              resize: 'vertical'
            }} />
              </div>
              <div className={styles.formGroup}>
                <label className={styles.formLabel}>{t("components.GroupChatOrchestrationPanel.k14")}</label>
                <input type="number" className={styles.formInput} value={newGoal.token_budget} onChange={e => setNewGoal(prev => ({
              ...prev,
              token_budget: Number(e.target.value) || 0
            }))} style={{
              width: 140,
              padding: '4px 8px',
              fontSize: 11
            }} />
              </div>
              <div className={styles.formGroup}>
                <label className={styles.formLabel}>{t("yuan-code.GoalManager.k16")}</label>
                <input type="number" className={styles.formInput} value={newGoal.priority} onChange={e => setNewGoal(prev => ({
              ...prev,
              priority: Number(e.target.value) || 0
            }))} min={0} max={10} style={{
              width: 80,
              padding: '4px 8px',
              fontSize: 11
            }} />
              </div>
              <div style={{
            display: 'flex',
            gap: 8,
            marginTop: 10
          }}>
                <button className={styles.btnPurple} onClick={handleCreate} style={{
              padding: '4px 12px',
              fontSize: 11
            }}>
                  {t("common.created")}
                </button>
                <button onClick={() => setShowCreate(false)} style={{
              padding: '4px 12px',
              fontSize: 11,
              background: 'transparent',
              border: '1px solid rgba(255,255,255,0.1)',
              color: 'var(--nt-text-secondary)',
              borderRadius: 4,
              cursor: 'pointer'
            }}>
                  {t("common.cancel")}
                </button>
              </div>
            </div>}

          {!showCreate && <button className={styles.btnPurple} onClick={() => setShowCreate(true)} disabled={!sessionId.trim()} style={{
          padding: '4px 12px',
          fontSize: 11,
          marginBottom: 12
        }}>
              {t("yuan-code.GoalManager.k17")}
            </button>}

          {/* 续接提示 */}
          {continuation && <div style={{
          padding: 12,
          borderRadius: 6,
          marginBottom: 12,
          border: '1px solid rgba(0,240,255,0.2)',
          background: 'rgba(0,240,255,0.05)'
        }}>
              <div style={{
            fontSize: 13,
            fontWeight: 600,
            color: 'var(--nt-primary)',
            marginBottom: 8
          }}>
                {t("yuan-code.GoalManager.k18")} {continuation.title}
              </div>
              <div style={{
            fontSize: 10,
            color: 'var(--nt-text-secondary)',
            fontFamily: 'var(--nt-font-mono)',
            marginBottom: 6
          }}>
                {t("yuan-code.GoalManager.k19")} {STATUS_LABELS[continuation.status] || continuation.status}
                {' | '}{t("yuan-code.GoalManager.k20")} {continuation.progress_pct}%
                {' | '}Token: {continuation.tokens_used}{continuation.token_budget ? `/${continuation.token_budget}` : ''}
                {' | '}{t("yuan-code.GoalManager.k21")} {continuation.checkpoints_count}
              </div>
              <div style={{
            fontSize: 10,
            color: 'var(--nt-text-secondary)',
            background: 'rgba(0,0,0,0.2)',
            padding: '6px 8px',
            borderRadius: 4,
            maxHeight: 100,
            overflowY: 'auto',
            whiteSpace: 'pre-wrap',
            fontFamily: 'var(--nt-font-mono)'
          }}>
                {continuation.continuation_prompt}
              </div>
              <button onClick={() => setContinuation(null)} style={{
            marginTop: 8,
            padding: '2px 10px',
            fontSize: 10,
            background: 'transparent',
            border: '1px solid rgba(255,255,255,0.1)',
            color: 'var(--nt-text-secondary)',
            borderRadius: 4,
            cursor: 'pointer'
          }}>
                {t("common.close")}
              </button>
            </div>}

          {/* 目标列表 */}
          {goals.length === 0 && !loading && sessionId.trim() && <div style={{
          textAlign: 'center',
          padding: 20,
          color: 'var(--nt-text-muted)',
          fontSize: 12
        }}>
              {t("yuan-code.GoalManager.k22")}
            </div>}

          {goals.length > 0 && <div style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 8
        }}>
              {goals.map(goal => <div key={goal.goal_uuid} style={{
            padding: 10,
            borderRadius: 6,
            border: `1px solid ${STATUS_COLORS[goal.status] || '#444'}33`,
            background: selectedGoalUuid === goal.goal_uuid ? 'rgba(0,240,255,0.05)' : 'rgba(0,240,255,0.02)',
            cursor: 'pointer'
          }} onClick={() => setSelectedGoalUuid(selectedGoalUuid === goal.goal_uuid ? null : goal.goal_uuid)}>
                  <div style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              marginBottom: 4
            }}>
                    <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8
              }}>
                      <span style={{
                  display: 'inline-block',
                  width: 8,
                  height: 8,
                  borderRadius: '50%',
                  background: STATUS_COLORS[goal.status] || '#888'
                }} />
                      <span style={{
                  fontSize: 13,
                  fontWeight: 600,
                  color: 'var(--nt-text-primary)'
                }}>
                        {goal.title}
                      </span>
                    </div>
                    <span style={{
                fontSize: 10,
                padding: '1px 6px',
                borderRadius: 3,
                background: `${STATUS_COLORS[goal.status] || '#888'}22`,
                color: STATUS_COLORS[goal.status] || '#888',
                fontFamily: 'var(--nt-font-mono)'
              }}>
                      {STATUS_LABELS[goal.status] || goal.status}
                    </span>
                  </div>

                  {/* 进度条 */}
                  <div style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              marginBottom: 4
            }}>
                    <div style={{
                flex: 1,
                height: 4,
                borderRadius: 2,
                background: 'rgba(255,255,255,0.06)',
                overflow: 'hidden'
              }}>
                      <div style={{
                  height: '100%',
                  width: `${goal.progress_pct}%`,
                  background: STATUS_COLORS[goal.status] || '#00F0FF',
                  borderRadius: 2,
                  transition: 'width 0.3s'
                }} />
                    </div>
                    <span style={{
                fontSize: 10,
                fontFamily: 'var(--nt-font-mono)',
                color: 'var(--nt-text-muted)',
                minWidth: 30
              }}>
                      {goal.progress_pct}%
                    </span>
                  </div>

                  {/* Token 进度条 */}
                  {goal.token_budget && goal.token_budget > 0 && <div style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              marginBottom: 4
            }}>
                      <div style={{
                flex: 1,
                height: 3,
                borderRadius: 2,
                background: 'rgba(255,255,255,0.04)',
                overflow: 'hidden'
              }}>
                        <div style={{
                  height: '100%',
                  width: `${Math.min(100, goal.token_pct)}%`,
                  background: goal.token_pct > 100 ? '#FF6B6B' : goal.token_pct > 80 ? '#FFB86C' : '#FFD700',
                  borderRadius: 2,
                  transition: 'width 0.3s'
                }} />
                      </div>
                      <span style={{
                fontSize: 9,
                fontFamily: 'var(--nt-font-mono)',
                color: 'var(--nt-text-muted)',
                minWidth: 60
              }}>
                        {goal.tokens_used.toLocaleString()}/{goal.token_budget.toLocaleString()} tokens
                      </span>
                    </div>}

                  {goal.description && <div style={{
              fontSize: 10,
              color: 'var(--nt-text-secondary)',
              marginTop: 2
            }}>
                      {goal.description}
                    </div>}

                  {/* 操作按钮 */}
                  {selectedGoalUuid === goal.goal_uuid && <div style={{
              display: 'flex',
              gap: 6,
              marginTop: 8,
              flexWrap: 'wrap'
            }}>
                      {goal.status === 'pending' && <ActionBtn label={t("common.start")} color="#00F0FF" onClick={() => handleAction(goal.goal_uuid, 'start')} />}
                      {goal.status === 'in_progress' && <>
                          <ActionBtn label={t("common.pause")} color="#FFB86C" onClick={() => handleAction(goal.goal_uuid, 'pause')} />
                          <ActionBtn label={t("common.finish")} color="#50FA7B" onClick={() => handleAction(goal.goal_uuid, 'complete')} />
                          <ActionBtn label={t("yuan-code.GoalManager.k23")} color="#FF6B6B" onClick={() => handleAction(goal.goal_uuid, 'abort')} />
                        </>}
                      {goal.status === 'paused' && <>
                          <ActionBtn label={t("common.resume")} color="#00F0FF" onClick={() => handleAction(goal.goal_uuid, 'start')} />
                          <ActionBtn label={t("yuan-code.GoalManager.k23")} color="#FF6B6B" onClick={() => handleAction(goal.goal_uuid, 'abort')} />
                        </>}
                      {(goal.status === 'paused' || goal.status === 'aborted') && <ActionBtn label={t("yuan-code.GoalManager.k24")} color="#B026FF" onClick={() => handleAction(goal.goal_uuid, 'continue')} />}
                      <ActionBtn label={t("common.delete")} color="#888" onClick={() => handleDelete(goal.goal_uuid)} />
                    </div>}
                </div>)}
            </div>}
        </div>
      </div>
    </div>;
}
function ActionBtn({
  label,
  color,
  onClick
}: {
  label: string;
  color: string;
  onClick: () => void;
}) {
  return <button onClick={e => {
    e.stopPropagation();
    onClick();
  }} style={{
    padding: '2px 10px',
    fontSize: 10,
    borderRadius: 3,
    background: `${color}18`,
    border: `1px solid ${color}33`,
    color,
    cursor: 'pointer',
    fontFamily: 'var(--nt-font-mono)'
  }}>
      {label}
    </button>;
}