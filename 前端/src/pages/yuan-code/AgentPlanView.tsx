/**
 * Yuan Code v3.1 Task 3.1.3 — AgentPlanView 组件
 *
 * 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.1.3
 *       + 项目核心设计意图 §三（Agent AI 调用走云端 API）
 *
 * 功能：
 * 1. 列出 7 种 Agent 类型（yuan_v3_agent_types）
 * 2. 创建 Agent 实例（yuan_v3_agent_create）
 * 3. 生成执行计划（yuan_v3_agent_plan）— 调用云端 API
 * 4. 展示 Plan + Steps + Status + Safety Check
 * 5. 触发执行（yuan_v3_agent_execute）后通过 onExecuted 回调通知 AgentReview
 *
 * 强制约束：所有 AI 调用走 cloud_api_router（云端 API），后端已强制校验。
 */
import { t } from "i18next";
import { useState, useCallback, useEffect } from 'react';
import { yuanCode } from '@/lib/ipc';
import type {
  AgentV3TypeInfo,
  AgentV3Created,
  AgentV3PlanResponse,
  AgentV3Result,
  AgentV3Plan,
  AgentV3SafetyCheck,
} from '@/lib/ipc/yuan-code';
import styles from '../YuanCode.module.css';

interface AgentPlanViewProps {
  /** Agent 执行完成后回调，传递 result 给 AgentReview 组件 */
  onExecuted?: (agentId: string, result: AgentV3Result) => void;
  /** 当前工作区路径（review 阶段需要） */
  workspacePath?: string;
}

type Phase = 'idle' | 'creating' | 'planning' | 'executing' | 'done';

const RISK_COLORS: Record<string, string> = {
  safe: '#00FF88',
  warning: '#FFB86C',
  dangerous: '#FF006E',
};

const STEP_STATUS_ICONS: Record<string, string> = {
  pending: '⏳',
  in_progress: '🔄',
  completed: '✅',
  failed: '❌',
  skipped: '⏭️',
};

export default function AgentPlanView({ onExecuted, workspacePath }: AgentPlanViewProps) {
  const [agentTypes, setAgentTypes] = useState<AgentV3TypeInfo[]>([]);
  const [selectedType, setSelectedType] = useState<string>('coding');
  const [taskPrompt, setTaskPrompt] = useState('');
  const [phase, setPhase] = useState<Phase>('idle');
  const [agent, setAgent] = useState<AgentV3Created | null>(null);
  const [planResponse, setPlanResponse] = useState<AgentV3PlanResponse | null>(null);
  const [executeResult, setExecuteResult] = useState<AgentV3Result | null>(null);
  const [error, setError] = useState<string | null>(null);

  // 加载 7 种 Agent 类型
  useEffect(() => {
    const loadTypes = async () => {
      try {
        const res = await yuanCode.agentV3Types();
        if (res.code === 0 && res.data) {
          setAgentTypes(res.data);
          if (res.data.length > 0 && selectedType === 'coding') {
            setSelectedType(res.data[0].type_name);
          }
        }
      } catch {
        // 静默处理
      }
    };
    loadTypes();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleCreate = useCallback(async () => {
    if (!taskPrompt.trim()) {
      setError(t('yuan-code.AgentPlanView.k1'));
      return;
    }
    setPhase('creating');
    setError(null);
    setAgent(null);
    setPlanResponse(null);
    setExecuteResult(null);
    try {
      const res = await yuanCode.agentV3Create(selectedType, taskPrompt.trim());
      if (res.code === 0 && res.data) {
        setAgent(res.data);
        setPhase('idle');
      } else {
        setError(res.message);
        setPhase('idle');
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPhase('idle');
    }
  }, [selectedType, taskPrompt]);

  const handlePlan = useCallback(async () => {
    if (!agent) return;
    setPhase('planning');
    setError(null);
    try {
      const res = await yuanCode.agentV3Plan(agent.agent_id, taskPrompt.trim());
      if (res.code === 0 && res.data) {
        setPlanResponse(res.data);
      } else {
        setError(res.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setPhase('idle');
    }
  }, [agent, taskPrompt]);

  const handleExecute = useCallback(async () => {
    if (!agent) return;
    setPhase('executing');
    setError(null);
    try {
      const res = await yuanCode.agentV3Execute(agent.agent_id);
      if (res.code === 0 && res.data) {
        setExecuteResult(res.data);
        setPhase('done');
        onExecuted?.(agent.agent_id, res.data);
      } else {
        setError(res.message);
        setPhase('idle');
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setPhase('idle');
    }
  }, [agent, onExecuted]);

  const handleReset = useCallback(() => {
    setAgent(null);
    setPlanResponse(null);
    setExecuteResult(null);
    setError(null);
    setPhase('idle');
  }, []);

  const plan = planResponse?.plan;
  const safetyCheck = planResponse?.safety_check;
  const canExecute = !!agent && !!plan && phase === 'idle';
  const isBusy = phase === 'creating' || phase === 'planning' || phase === 'executing';

  return (
    <div className={styles.settingsSection}>
      <h3 className={styles.settingsSectionTitle}>
        {t('yuan-code.AgentPlanView.k2')}
      </h3>

      <div className={styles.settingRow}>
        <span className={styles.settingDesc}>
          {t('yuan-code.AgentPlanView.k3')}
        </span>
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

      {/* Agent 类型选择（7 种） */}
      <div className={styles.formGroup}>
        <label className={styles.formLabel}>{t('yuan-code.AgentPlanView.k4')}</label>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 4, marginTop: 4 }}>
          {agentTypes.length === 0 ? (
            <span style={{ fontSize: 11, color: 'var(--nt-text-muted)' }}>
              {t('components.AudioEditor.k6')}...
            </span>
          ) : agentTypes.map(at => (
            <button
              key={at.type_name}
              onClick={() => setSelectedType(at.type_name)}
              style={{
                padding: '6px 4px',
                fontSize: 10,
                border: `1px solid ${selectedType === at.type_name ? 'rgba(0,240,255,0.4)' : 'rgba(0,240,255,0.1)'}`,
                borderRadius: 3,
                background: selectedType === at.type_name ? 'rgba(0,240,255,0.08)' : 'rgba(0,240,255,0.02)',
                color: selectedType === at.type_name ? 'var(--nt-primary)' : 'var(--nt-text-secondary)',
                cursor: 'pointer',
                textAlign: 'center',
              }}
              title={at.description}
            >
              {at.display_name}
            </button>
          ))}
        </div>
        {agentTypes.find(at => at.type_name === selectedType) && (
          <div style={{ marginTop: 4, fontSize: 10, color: 'var(--nt-text-muted)' }}>
            {agentTypes.find(at => at.type_name === selectedType)!.description}
          </div>
        )}
      </div>

      {/* 任务输入 */}
      <div className={styles.formGroup} style={{ marginTop: 10 }}>
        <label className={styles.formLabel}>{t('yuan-code.AgentPlanView.k5')}</label>
        <textarea
          className={styles.formInput}
          placeholder={t('yuan-code.AgentPlanView.k6')}
          value={taskPrompt}
          onChange={e => setTaskPrompt(e.target.value)}
          rows={3}
          style={{
            padding: '6px 8px',
            fontSize: 11,
            width: '100%',
            resize: 'vertical',
            fontFamily: 'var(--nt-font-mono)',
          }}
        />
      </div>

      {/* Agent 状态信息 */}
      {agent && (
        <div style={{
          marginTop: 10,
          padding: '6px 10px',
          borderRadius: 4,
          background: 'rgba(0,240,255,0.04)',
          border: '1px solid rgba(0,240,255,0.12)',
          fontSize: 11,
        }}>
          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <span style={{ color: 'var(--nt-text-muted)' }}>{t('yuan-code.AgentPlanView.k7')}:</span>
            <code style={{ color: 'var(--nt-primary)', fontFamily: 'var(--nt-font-mono)' }}>
              {agent.agent_id}
            </code>
            <span style={{
              fontSize: 9,
              padding: '1px 5px',
              borderRadius: 3,
              background: 'rgba(0,240,255,0.1)',
              color: 'var(--nt-primary)',
            }}>
              {agent.agent_type}
            </span>
            <span style={{
              fontSize: 9,
              padding: '1px 5px',
              borderRadius: 3,
              background: phase === 'done' ? 'rgba(0,255,136,0.1)' : 'rgba(255,180,0,0.12)',
              color: phase === 'done' ? '#00FF88' : '#FFB86C',
            }}>
              {phase === 'done' ? t('common.complete') : (agent.phase || phase)}
            </span>
          </div>
        </div>
      )}

      {/* 计划展示 */}
      {plan && <PlanDisplay plan={plan} safetyCheck={safetyCheck} />}

      {/* 执行结果摘要（详情交给 AgentReview） */}
      {executeResult && (
        <div style={{
          marginTop: 10,
          padding: '8px 10px',
          borderRadius: 4,
          background: 'rgba(0,255,136,0.04)',
          border: '1px solid rgba(0,255,136,0.18)',
          fontSize: 11,
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
            <span style={{ color: '#00FF88' }}>✅ {t('yuan-code.AgentPlanView.k8')}</span>
            <span style={{
              fontSize: 9,
              padding: '1px 5px',
              borderRadius: 3,
              background: 'rgba(0,240,255,0.1)',
              color: 'var(--nt-primary)',
              fontFamily: 'var(--nt-font-mono)',
            }}>
              {executeResult.provider} / {executeResult.model_used}
            </span>
            {executeResult.tokens_used && (
              <span style={{
                fontSize: 9,
                color: 'var(--nt-text-muted)',
                fontFamily: 'var(--nt-font-mono)',
              }}>
                {executeResult.tokens_used} tokens
              </span>
            )}
          </div>
          <div style={{ color: 'var(--nt-text-secondary)', fontSize: 11 }}>
            {executeResult.summary}
          </div>
          <div style={{ marginTop: 4, fontSize: 10, color: 'var(--nt-text-muted)' }}>
            {t('yuan-code.AgentPlanView.k9')}: {executeResult.diffs.length} ·
            {' '}{t('yuan-code.AgentPlanView.k10')}: {executeResult.execution_log.length}
          </div>
        </div>
      )}

      {/* 操作按钮 */}
      <div style={{ display: 'flex', gap: 6, marginTop: 10, flexWrap: 'wrap' }}>
        {!agent && (
          <button
            className={styles.btnPrimary}
            onClick={handleCreate}
            disabled={isBusy || !taskPrompt.trim()}
            style={{ padding: '4px 12px', fontSize: 11 }}
          >
            {phase === 'creating' ? t('components.AudioEditor.k6') : t('yuan-code.AgentPlanView.k11')}
          </button>
        )}
        {agent && !plan && (
          <button
            className={styles.btnPurple}
            onClick={handlePlan}
            disabled={isBusy}
            style={{ padding: '4px 12px', fontSize: 11 }}
          >
            {phase === 'planning' ? t('components.AudioEditor.k6') : t('yuan-code.AgentPlanView.k12')}
          </button>
        )}
        {canExecute && (
          <button
            className={styles.btnPrimary}
            onClick={handleExecute}
            disabled={isBusy}
            style={{ padding: '4px 12px', fontSize: 11 }}
          >
            {t('yuan-code.AgentPlanView.k13')}
          </button>
        )}
        {agent && (
          <button
            className={styles.btnDanger}
            onClick={handleReset}
            disabled={isBusy}
            style={{ padding: '4px 12px', fontSize: 11 }}
          >
            {t('common.reset')}
          </button>
        )}
      </div>

      {workspacePath && (
        <div style={{ marginTop: 6, fontSize: 10, color: 'var(--nt-text-muted)', fontFamily: 'var(--nt-font-mono)' }}>
          {t('yuan-code.AgentPlanView.k14')}: {workspacePath}
        </div>
      )}
    </div>
  );
}

/** Plan + Steps + Safety Check 可视化子组件 */
function PlanDisplay({
  plan,
  safetyCheck,
}: {
  plan: AgentV3Plan;
  safetyCheck?: AgentV3SafetyCheck;
}) {
  return (
    <div style={{
      marginTop: 10,
      padding: 10,
      border: '1px solid rgba(0,240,255,0.12)',
      borderRadius: 4,
      background: 'rgba(0,240,255,0.02)',
    }}>
      {/* 计划标题 */}
      <div style={{ marginBottom: 6 }}>
        <div style={{ fontSize: 12, color: 'var(--nt-primary)', fontWeight: 500 }}>
          {plan.title}
        </div>
        <div style={{ fontSize: 11, color: 'var(--nt-text-secondary)', marginTop: 2 }}>
          {plan.summary}
        </div>
        <div style={{ display: 'flex', gap: 8, marginTop: 4, fontSize: 10, color: 'var(--nt-text-muted)' }}>
          <span>📊 {plan.steps.length}/{plan.estimated_steps} {t('yuan-code.AgentPlanView.k15')}</span>
          {plan.estimated_duration_sec && (
            <span>⏱ ~{plan.estimated_duration_sec}s</span>
          )}
          <span style={{ fontFamily: 'var(--nt-font-mono)' }}>{plan.agent_type}</span>
        </div>
      </div>

      {/* 安全检查结果 */}
      {safetyCheck && (
        <div style={{
          padding: '4px 8px',
          marginBottom: 8,
          borderRadius: 3,
          background: safetyCheck.passed
            ? 'rgba(0,255,136,0.06)'
            : 'rgba(255,0,110,0.06)',
          border: `1px solid ${safetyCheck.passed ? 'rgba(0,255,136,0.2)' : 'rgba(255,0,110,0.2)'}`,
          fontSize: 10,
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span style={{ color: RISK_COLORS[safetyCheck.risk_level] || '#FF006E' }}>
              {safetyCheck.passed ? '✅' : '⚠️'} {t('yuan-code.AgentPlanView.k16')}:
            </span>
            <span style={{
              padding: '1px 5px',
              borderRadius: 3,
              background: `${RISK_COLORS[safetyCheck.risk_level]}22`,
              color: RISK_COLORS[safetyCheck.risk_level],
              fontFamily: 'var(--nt-font-mono)',
            }}>
              {safetyCheck.risk_level}
            </span>
            {safetyCheck.blocked_steps.length > 0 && (
              <span style={{ color: '#FF006E' }}>
                {t('yuan-code.AgentPlanView.k17')}: {safetyCheck.blocked_steps.join(', ')}
              </span>
            )}
          </div>
          {safetyCheck.reasons.length > 0 && (
            <ul style={{ margin: '4px 0 0 16px', padding: 0, color: 'var(--nt-text-muted)' }}>
              {safetyCheck.reasons.map((r, i) => (
                <li key={i}>{r}</li>
              ))}
            </ul>
          )}
        </div>
      )}

      {/* 步骤列表 */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
        {plan.steps.map(step => (
          <div
            key={step.step_id}
            style={{
              padding: '6px 8px',
              border: '1px solid rgba(0,240,255,0.08)',
              borderRadius: 3,
              background: 'rgba(0,240,255,0.02)',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <span style={{ fontSize: 12 }}>
                {STEP_STATUS_ICONS[step.status] || '⏳'}
              </span>
              <span style={{ fontSize: 11, color: 'var(--nt-text-primary)', flex: 1 }}>
                {step.step_id + 1}. {step.title}
              </span>
              {step.requires_confirmation && (
                <span style={{
                  fontSize: 9,
                  padding: '1px 5px',
                  borderRadius: 3,
                  background: 'rgba(255,180,0,0.12)',
                  color: '#FFB400',
                }}>
                  {t('yuan-code.AgentPlanView.k18')}
                </span>
              )}
            </div>
            <div style={{ fontSize: 10, color: 'var(--nt-text-muted)', marginTop: 2, marginLeft: 20 }}>
              {step.description}
            </div>
            {step.target_files.length > 0 && (
              <div style={{ marginTop: 4, marginLeft: 20, display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                {step.target_files.map(f => (
                  <code key={f} style={{
                    fontSize: 9,
                    padding: '1px 4px',
                    borderRadius: 2,
                    background: 'rgba(0,240,255,0.06)',
                    color: 'var(--nt-text-muted)',
                  }}>
                    {f}
                  </code>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
