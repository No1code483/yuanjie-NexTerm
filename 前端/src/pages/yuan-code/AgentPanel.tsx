import { t } from "i18next";
import { useState, useCallback, useEffect, useRef } from 'react';
import { ipc, ai } from '@/lib/ipc';
import { useNetworkStatus } from '@/hooks/useNetworkStatus';
import type { UseEngineEventsReturn } from './useEngineEvents';
import styles from '../YuanCode.module.css';
type AgentStatus = 'running' | 'idle' | 'error' | 'stopped';
interface AgentInfo {
  id: string;
  name: string;
  role: string;
  description: string;
  model: string;
  status: AgentStatus;
  tokenUsed: number;
  tokenLimit: number;
  iterationCount: number;
  maxIterations: number;
  temperature: number;
}
interface AgentTemplate {
  name: string;
  display_name: string;
  description: string;
  system_prompt: string;
  recommended_model: string | null;
  temperature: number | null;
  tools: string[];
  permission_level: number;
  max_iterations: number;
  token_budget: number;
  icon: string | null;
  tags: string[];
}
interface GoalNode {
  id: string;
  text: string;
  status: 'pending' | 'in_progress' | 'completed' | 'blocked';
  assignedTo: string | null;
  subGoals: GoalNode[];
}
interface PlanStep {
  id: string;
  title: string;
  description: string;
  status: string;
  dependencies: string[];
  assigned_agent: string | null;
  result: string | null;
  created_at: number;
  updated_at: number;
}
interface PlanInfo {
  id: string;
  title: string;
  description: string;
  steps: PlanStep[];
  status: string;
  created_at: number;
  updated_at: number;
  session_id: string;
}
const PLAN_STATUS_LABELS: Record<string, string> = {
  Pending: t("yuan-code.AgentPanel.k1"),
  InProgress: t("yuan-code.AgentPanel.k2"),
  Completed: t("home.TodoPanel.k2"),
  Failed: t("common.failed"),
  Skipped: t("yuan-code.AgentPanel.k3")
};
const PLAN_STATUS_ICONS: Record<string, string> = {
  Pending: '⏳',
  InProgress: '🔄',
  Completed: '✅',
  Failed: '❌',
  Skipped: '⏭️'
};

// 默认 Agent 角色模板（模型从 AI会话板块获取，不再硬编码）
const DEFAULT_AGENT_ROLES: Omit<AgentInfo, 'model'>[] = [{
  id: 'orchestrator',
  name: 'Orchestrator',
  role: t("yuan-code.AgentPanel.k4"),
  description: t("yuan-code.AgentPanel.k5"),
  status: 'running',
  tokenUsed: 12500,
  tokenLimit: 128000,
  iterationCount: 8,
  maxIterations: 50,
  temperature: 0.3
}, {
  id: 'coder',
  name: 'Coder Agent',
  role: t("yuan-code.AgentPanel.k6"),
  description: t("yuan-code.AgentPanel.k7"),
  status: 'idle',
  tokenUsed: 0,
  tokenLimit: 64000,
  iterationCount: 0,
  maxIterations: 30,
  temperature: 0.5
}, {
  id: 'reviewer',
  name: 'Reviewer Agent',
  role: t("yuan-code.AgentPanel.k8"),
  description: t("yuan-code.AgentPanel.k9"),
  status: 'stopped',
  tokenUsed: 0,
  tokenLimit: 32000,
  iterationCount: 0,
  maxIterations: 15,
  temperature: 0.1
}, {
  id: 'test',
  name: 'Test Agent',
  role: t("lib.xinChatEngine.k8"),
  description: t("yuan-code.AgentPanel.k10"),
  status: 'stopped',
  tokenUsed: 0,
  tokenLimit: 32000,
  iterationCount: 0,
  maxIterations: 20,
  temperature: 0.4
}, {
  id: 'deploy',
  name: 'Deploy Agent',
  role: t("yuan-code.AgentPanel.k11"),
  description: t("yuan-code.AgentPanel.k12"),
  status: 'stopped',
  tokenUsed: 0,
  tokenLimit: 16000,
  iterationCount: 0,
  maxIterations: 10,
  temperature: 0.2
}];
const INITIAL_GOALS: GoalNode[] = [{
  id: 'g1',
  text: t("yuan-code.AgentPanel.k13"),
  status: 'in_progress',
  assignedTo: 'orchestrator',
  subGoals: [{
    id: 'g1.1',
    text: t("yuan-code.AgentPanel.k14"),
    status: 'completed',
    assignedTo: 'coder',
    subGoals: []
  }, {
    id: 'g1.2',
    text: t("yuan-code.AgentPanel.k15"),
    status: 'completed',
    assignedTo: 'coder',
    subGoals: []
  }, {
    id: 'g1.3',
    text: t("yuan-code.AgentPanel.k16"),
    status: 'in_progress',
    assignedTo: 'coder',
    subGoals: []
  }, {
    id: 'g1.4',
    text: t("yuan-code.AgentPanel.k17"),
    status: 'pending',
    assignedTo: 'reviewer',
    subGoals: []
  }]
}];
const STATUS_BADGE: Record<AgentStatus, string> = {
  running: styles.badgeRunning,
  idle: styles.badgeIdle,
  error: styles.badgeError,
  stopped: styles.badgeWarning
};
const STATUS_LABEL: Record<AgentStatus, string> = {
  running: t("Linux.k46"),
  idle: t("yuan-code.AgentPanel.k18"),
  error: t("yuan-code.AgentPanel.k19"),
  stopped: t("yuan-code.AgentPanel.k20")
};
function formatTokens(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
  return String(n);
}
export default function AgentPanel({
  engineEvents,
  sessionId
}: {
  engineEvents: UseEngineEventsReturn;
  sessionId: string | null;
}) {
  // AI会话板块的模型和Agent列表
  const [aiModels, setAiModels] = useState<Array<{
    id: number;
    name: string;
    provider: string;
    model_name: string;
  }>>([]);
  const [aiLoaded, setAiLoaded] = useState(false);
  const aiModelsRef = useRef<typeof aiModels>(aiModels);
  useEffect(() => {
    aiModelsRef.current = aiModels;
  }, [aiModels]);

  // 从 AI会话模型构建默认 Agent 列表
  const buildDefaultAgents = (models: typeof aiModels): AgentInfo[] => {
    const defaultModelName = models.length > 0 ? models[0].name : t("Xin.k28");
    return DEFAULT_AGENT_ROLES.map((role, idx) => ({
      ...role,
      model: models.length > 0 ? models[Math.min(idx, models.length - 1)].name : defaultModelName
    }));
  };
  const [agents, setAgents] = useState<AgentInfo[]>(buildDefaultAgents([]));
  const [templates, setTemplates] = useState<AgentTemplate[]>([]);
  const [templateLoaded, setTemplateLoaded] = useState(false);
  const [goals] = useState<GoalNode[]>(INITIAL_GOALS);
  const [expandedAgent, setExpandedAgent] = useState<string | null>(null);
  const [deploying, setDeploying] = useState(false);
  const [showTemplatePicker, setShowTemplatePicker] = useState(false);
  const [backendAvailable, setBackendAvailable] = useState(false);

  // Plan 计划
  const [plans, setPlans] = useState<PlanInfo[]>([]);
  const [expandedPlan, setExpandedPlan] = useState<string | null>(null);

  // 引擎对话输入
  const [chatInput, setChatInput] = useState('');
  const [sending, setSending] = useState(false);
  const chatEndRef = useRef<HTMLDivElement | null>(null);

  // A5 Phase 3 Task 2: 云端 AI 离线降级
  // 设计依据：.trae/rules/项目核心设计意图.md §三（Yuan Code 走云端 API，离线不切本地 ollama）
  // 离线时禁用 Agent 部署/引擎对话（避免触发云端 API 请求），
  // 显示「AI 离线，本地编辑可用」提示。本地 Monaco 编辑、文件读写不受影响。
  const { isOnline } = useNetworkStatus();

  // 加载 AI会话板块的模型列表
  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const modelRes = await ai.getModels();
        if (!cancelled) {
          if (modelRes?.data) {
            setAiModels(modelRes.data);
          }
          setAiLoaded(true);
        }
      } catch {
        if (!cancelled) setAiLoaded(true);
      }
    };
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  // 当 AI会话模型加载完成后，更新默认 Agent 的模型名
  useEffect(() => {
    if (aiLoaded && aiModels.length > 0 && agents.every(a => a.model === t("Xin.k28"))) {
      setAgents(buildDefaultAgents(aiModels));
    }
  }, [aiLoaded, aiModels.length]);

  // 加载 Agent 模板
  useEffect(() => {
    let cancelled = false;
    const loadTemplates = async () => {
      try {
        const res = await ipc.invoke<AgentTemplate[]>('yuan_agent_list_templates');
        if (res.code === 0 && res.data && !cancelled) {
          setTemplates(res.data);
          setBackendAvailable(true);
        }
      } catch {
        // 后端不可用，使用默认数据
      }
      if (!cancelled) setTemplateLoaded(true);
    };
    loadTemplates();
    return () => {
      cancelled = true;
    };
  }, []);

  // 加载后端 Agent 列表
  useEffect(() => {
    if (!backendAvailable) return;
    let cancelled = false;
    const loadAgents = async () => {
      try {
        const res = await ipc.invoke<Array<{
          agent_id: string;
          role_name: string;
          status: string;
          turns_executed: number;
          tokens_used: number;
        }>>('yuan_agent_list_all');
        if (res.code === 0 && res.data && !cancelled) {
          setAgents(res.data.map(a => ({
            id: a.agent_id,
            name: a.role_name,
            role: a.role_name,
            description: '',
            model: aiModelsRef.current[0]?.name || t("Xin.k28"),
            status: mapBackendStatus(a.status),
            tokenUsed: a.tokens_used,
            tokenLimit: 100_000,
            iterationCount: a.turns_executed,
            maxIterations: 50,
            temperature: 0.5
          })));
        }
      } catch {
        // 保持现有数据
      }
    };
    loadAgents();
    return () => {
      cancelled = true;
    };
  }, [backendAvailable]);

  // 加载 Plan 列表
  useEffect(() => {
    if (!backendAvailable) return;
    let cancelled = false;
    const loadPlans = async () => {
      try {
        const res = await ipc.invoke<PlanInfo[]>('yuan_plan_list_all');
        if (res.code === 0 && res.data && !cancelled) {
          setPlans(res.data);
        }
      } catch {
        // 静默处理
      }
    };
    loadPlans();
    return () => {
      cancelled = true;
    };
  }, [backendAvailable]);

  // 自动滚动到底部
  useEffect(() => {
    chatEndRef.current?.scrollIntoView({
      behavior: 'smooth'
    });
  }, [engineEvents.currentStream?.content, engineEvents.turns]);

  // 发送引擎消息
  const handleSendMessage = useCallback(async () => {
    const input = chatInput.trim();
    if (!input || !sessionId || sending) return;
    // A5 Phase 3 Task 2: 离线时直接返回，不调用云端 AI
    if (!isOnline) return;
    setChatInput('');
    setSending(true);
    try {
      // 开始回合
      const res = await ipc.invoke<{
        turn_id: string;
        turn_number: number;
        status: string;
      }>('engine_start_turn', {
        sessionId,
        userInput: input
      });
      if (res.code === 0 && res.data) {
        engineEvents.addTurn(res.data.turn_id, res.data.turn_number, input);
      }

      // 模拟 AI 响应（实际应由后端 AI 模型生成）
      // 这里通过事件总线发送流式响应
      const response = t("yuan-code.AgentPanel.k21", {
        input: input
      });
      await ipc.invoke('engine_complete_turn', {
        sessionId,
        response,
        inputTokens: input.length,
        outputTokens: response.length
      });
      engineEvents.updateTurnResponse(res.data?.turn_id || '', response, 'completed');
    } catch (e) {
      console.error('[Agent] 发送消息失败:', e);
    } finally {
      setSending(false);
    }
  }, [chatInput, sessionId, sending, engineEvents, isOnline]);
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  }, [handleSendMessage]);
  const toggleAgent = useCallback((id: string) => {
    setAgents(prev => prev.map(a => {
      if (a.id !== id) return a;
      if (a.status === 'stopped') return {
        ...a,
        status: 'idle' as AgentStatus
      };
      if (a.status === 'idle') return {
        ...a,
        status: 'stopped' as AgentStatus
      };
      return a;
    }));
  }, []);
  const toggleExpand = useCallback((id: string) => {
    setExpandedAgent(prev => prev === id ? null : id);
  }, []);

  // 从模板创建 Agent
  const spawnFromTemplate = useCallback(async (template: AgentTemplate) => {
    setShowTemplatePicker(false);
    setDeploying(true);
    const newId = `agent_${template.name}_${Date.now()}`;
    const newAgent: AgentInfo = {
      id: newId,
      name: template.display_name,
      role: template.name,
      description: template.description,
      model: template.recommended_model || (aiModelsRef.current.length > 0 ? aiModelsRef.current[0].name : t("Xin.k28")),
      status: 'idle',
      tokenUsed: 0,
      tokenLimit: template.token_budget,
      iterationCount: 0,
      maxIterations: template.max_iterations,
      temperature: template.temperature || 0.5
    };
    setAgents(prev => [...prev, newAgent]);
    try {
      await ipc.invoke('yuan_agent_spawn', {
        request: {
          session_id: 'default',
          role: {
            name: template.name,
            description: template.description,
            system_prompt: template.system_prompt,
            model: template.recommended_model,
            temperature: template.temperature,
            max_tokens: null,
            tools: template.tools,
            token_budget: template.token_budget,
            max_turns: template.max_iterations,
            permission_level: template.permission_level
          },
          parent_id: null
        }
      });
      setAgents(prev => prev.map(a => a.id === newId ? {
        ...a,
        status: 'running' as AgentStatus
      } : a));
    } catch {
      // 后端不可用，保持 idle 状态
    }
    setDeploying(false);
  }, []);
  const handleDeploy = useCallback(async () => {
    // A5 Phase 3 Task 2: 离线时直接返回，不调用云端 AI
    if (!isOnline) return;
    setDeploying(true);
    try {
      const res = await ipc.invoke<{
        success: boolean;
        message: string;
      }>('yuan_agent_spawn', {
        agents: agents.filter(a => a.status !== 'stopped').map(a => ({
          id: a.id,
          model: a.model,
          temperature: a.temperature,
          max_iterations: a.maxIterations
        }))
      });
      if (res.code === 0 && res.data?.success) {
        setAgents(prev => prev.map(a => a.status === 'idle' ? {
          ...a,
          status: 'running' as AgentStatus
        } : a));
      }
    } catch {
      // 后端未就绪，直接切换状态
      setAgents(prev => prev.map(a => a.status === 'idle' ? {
        ...a,
        status: 'running' as AgentStatus
      } : a));
    } finally {
      setDeploying(false);
    }
  }, [agents, isOnline]);
  const handleStopAgent = useCallback(async (agentId: string) => {
    try {
      await ipc.invoke('yuan_agent_abort', {
        agent_id: agentId,
        reason: t("yuan-code.AgentPanel.k22")
      });
    } catch {/* 忽略 */}
    setAgents(prev => prev.map(a => a.id === agentId ? {
      ...a,
      status: 'stopped' as AgentStatus
    } : a));
  }, []);
  const totalTokens = agents.reduce((sum, a) => sum + a.tokenUsed, 0);
  const totalLimit = agents.reduce((sum, a) => sum + a.tokenLimit, 0);
  const activeAgentCount = agents.filter(a => a.status !== 'stopped').length;
  const countGoals = (nodes: GoalNode[]): {
    total: number;
    done: number;
    inProgress: number;
  } => {
    let total = 0,
      done = 0,
      inProgress = 0;
    const walk = (list: GoalNode[]) => {
      for (const g of list) {
        total++;
        if (g.status === 'completed') done++;
        if (g.status === 'in_progress') inProgress++;
        if (g.subGoals.length > 0) walk(g.subGoals);
      }
    };
    walk(nodes);
    return {
      total,
      done,
      inProgress
    };
  };
  const goalStats = countGoals(goals);
  const goalPercent = goalStats.total > 0 ? Math.round(goalStats.done / goalStats.total * 100) : 0;
  const renderGoalTree = (nodes: GoalNode[], depth: number = 0) => nodes.map(g => <div key={g.id}>
        <div className={styles.agentGoal} style={{
      paddingLeft: 12 + depth * 16,
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      borderLeft: depth > 0 ? '1px solid rgba(0,240,255,0.06)' : 'none'
    }}>
          <span style={{
        fontSize: 10,
        width: 14
      }}>
            {g.status === 'completed' ? '✔' : g.status === 'in_progress' ? '⏳' : g.status === 'blocked' ? '🚫' : '○'}
          </span>
          <span style={{
        color: g.status === 'completed' ? 'var(--nt-text-muted)' : g.status === 'in_progress' ? 'var(--nt-primary)' : 'var(--nt-text-secondary)',
        textDecoration: g.status === 'completed' ? 'line-through' : 'none'
      }}>
            {g.text}
          </span>
          {g.assignedTo && <span className={styles.agentRole}>{g.assignedTo}</span>}
        </div>
        {g.subGoals.length > 0 && renderGoalTree(g.subGoals, depth + 1)}
      </div>);
  return <div className={styles.panelContainer}>
      <div className={styles.panelHeader}>
        <span className={styles.panelTitle}>{t("yuan-code.AgentPanel.k23")}</span>
        <div style={{
        display: 'flex',
        gap: 8,
        alignItems: 'center'
      }}>
          <span className={styles.panelBadge}>
            {activeAgentCount}/{agents.length} {t("yuan-code.AgentPanel.k24")}
          </span>
          <button
            className={styles.btnPrimary}
            onClick={handleDeploy}
            disabled={deploying || !isOnline}
            title={!isOnline ? t('yuan-code.AgentPanel.offlineHint', { defaultValue: 'AI 离线，本地编辑可用。请连接网络后部署 Agent。' }) : undefined}
            style={{
          padding: '4px 12px',
          fontSize: 12
        }}>
            {deploying ? t("yuan-code.AgentPanel.k25") : t("yuan-code.AgentPanel.k26")}
          </button>
        </div>
      </div>

      {/* A5 Phase 3 Task 2: 离线降级提示横幅 */}
      {!isOnline && <div style={{
        padding: '6px 12px',
        margin: '8px 12px 0',
        border: '1px solid rgba(255, 179, 0, 0.4)',
        borderRadius: 4,
        background: 'rgba(255, 179, 0, 0.08)',
        fontSize: 11,
        color: '#FFB300',
        fontFamily: 'var(--nt-font-mono)',
        display: 'flex',
        alignItems: 'center',
        gap: 6
      }}>
          <span>📴</span>
          <span>{t('yuan-code.AgentPanel.offlineHint', { defaultValue: 'AI 离线，本地编辑可用。请连接网络后部署 Agent。' })}</span>
        </div>}

      <div className={styles.panelBody}>
        {/* Goal 跟踪 */}
        <div className={styles.goalPanel}>
          <div className={styles.goalTitle}>{t("yuan-code.AgentPanel.k27")}</div>
          <div style={{
          display: 'flex',
          gap: 24,
          marginBottom: 8
        }}>
            <div className={styles.goalStats}><span>{t("yuan-code.AgentPanel.k28")}</span><span style={{
              color: 'var(--nt-primary)'
            }}>{goalStats.total}</span></div>
            <div className={styles.goalStats}><span>{t("home.TodoPanel.k2")}</span><span style={{
              color: '#00F0FF'
            }}>{goalStats.done}</span></div>
            <div className={styles.goalStats}><span>{t("yuan-code.AgentPanel.k2")}</span><span style={{
              color: 'var(--nt-secondary)'
            }}>{goalStats.inProgress}</span></div>
            <div className={styles.goalStats}><span>{t("game.components.RealmCard.k23")}</span><span style={{
              color: 'var(--nt-warning)'
            }}>{goalPercent}%</span></div>
          </div>
          <div className={styles.progressBar} style={{
          marginBottom: 0
        }}>
            <div className={`${styles.progressFill} ${styles.progressPurple}`} style={{
            width: `${goalPercent}%`
          }} />
          </div>
          <div style={{
          marginTop: 12,
          display: 'flex',
          flexDirection: 'column',
          gap: 2
        }}>
            {renderGoalTree(goals)}
          </div>
        </div>

        {/* Token 预算总览 */}
        <div className={styles.tokenBudget}>
          <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          marginBottom: 4
        }}>
            <span className={styles.tokenLabel}>{t("yuan-code.AgentPanel.k29")}</span>
            <span className={styles.tokenValue}>{formatTokens(totalTokens)} / {formatTokens(totalLimit)}</span>
          </div>
          <div className={styles.progressBar}>
            <div className={`${styles.progressFill} ${totalTokens / totalLimit > 0.8 ? styles.progressGold : styles.progressCyan}`} style={{
            width: `${Math.min(totalTokens / totalLimit * 100, 100)}%`
          }} />
          </div>
        </div>

        {/* Agent 列表 */}
        <div style={{
        marginTop: 16
      }}>
          <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 10
        }}>
            <span style={{
            fontFamily: 'var(--nt-font-mono)',
            fontSize: 12,
            color: 'var(--nt-primary)'
          }}>
              {t("yuan-code.AgentPanel.k30")}
            </span>
            {templateLoaded && templates.length > 0 && <button className={styles.btnPrimary} onClick={() => setShowTemplatePicker(prev => !prev)} style={{
            padding: '2px 10px',
            fontSize: 11
          }}>
                {showTemplatePicker ? t("common.collapse") : t("yuan-code.AgentPanel.k31")}
              </button>}
          </div>

          {/* 模板选择器 */}
          {showTemplatePicker && <div style={{
          marginBottom: 12,
          padding: 10,
          background: 'rgba(0,240,255,0.03)',
          border: '1px solid rgba(0,240,255,0.1)',
          borderRadius: 6,
          maxHeight: 200,
          overflowY: 'auto'
        }}>
              <div style={{
            fontSize: 11,
            color: 'var(--nt-text-muted)',
            marginBottom: 8
          }}>{t("yuan-code.AgentPanel.k32")}</div>
              {templates.map(tmpl => <div key={tmpl.name} onClick={() => spawnFromTemplate(tmpl)} style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '6px 8px',
            borderRadius: 4,
            cursor: 'pointer',
            marginBottom: 4,
            background: 'rgba(0,240,255,0.02)'
          }} onMouseEnter={e => e.currentTarget.style.background = 'rgba(0,240,255,0.08)'} onMouseLeave={e => e.currentTarget.style.background = 'rgba(0,240,255,0.02)'}>
                  <span>{tmpl.icon || '🤖'}</span>
                  <div style={{
              flex: 1
            }}>
                    <div style={{
                fontSize: 12,
                color: 'var(--nt-text-primary)'
              }}>{tmpl.display_name}</div>
                    <div style={{
                fontSize: 10,
                color: 'var(--nt-text-muted)'
              }}>{tmpl.description}</div>
                  </div>
                  <div style={{
              display: 'flex',
              gap: 4
            }}>
                    {tmpl.tags.slice(0, 2).map(tag => <span key={tag} style={{
                fontSize: 9,
                padding: '1px 4px',
                borderRadius: 3,
                background: 'rgba(0,240,255,0.1)',
                color: 'var(--nt-primary)'
              }}>{tag}</span>)}
                  </div>
                </div>)}
            </div>}

          <div className={styles.agentList}>
            {agents.map(agent => <div key={agent.id} className={styles.agentCard} style={{
            borderColor: agent.status === 'running' ? 'rgba(0,240,255,0.3)' : agent.status === 'error' ? 'rgba(255,0,110,0.25)' : undefined
          }}>
                <div className={styles.agentCardHeader} onClick={() => toggleExpand(agent.id)} style={{
              cursor: 'pointer'
            }}>
                  <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8
              }}>
                    <span className={styles.agentName}>{agent.name}</span>
                    <span className={styles.agentRole}>{agent.role}</span>
                  </div>
                  <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8
              }}>
                    <span className={STATUS_BADGE[agent.status]}>{STATUS_LABEL[agent.status]}</span>
                    <span style={{
                  fontSize: 10,
                  color: 'var(--nt-text-muted)',
                  transform: expandedAgent === agent.id ? 'rotate(90deg)' : ''
                }}>▸</span>
                  </div>
                </div>

                <div className={styles.agentGoal}>{agent.description}</div>

                <div className={styles.agentMeta}>
                  <span>{agent.model}</span>
                  <span>{t("yuan-code.AgentPanel.k33")} {agent.iterationCount}/{agent.maxIterations}</span>
                  <span>Token {formatTokens(agent.tokenUsed)}/{formatTokens(agent.tokenLimit)}</span>
                </div>

                {expandedAgent === agent.id && <div style={{
              marginTop: 10,
              paddingTop: 10,
              borderTop: '1px solid rgba(0,240,255,0.08)'
            }}>
                    <div style={{
                display: 'grid',
                gridTemplateColumns: '1fr 1fr',
                gap: 8
              }}>
                      <div className={styles.formGroup}>
                        <label className={styles.formLabel}>{t("yuan-code.AgentPanel.k34")}</label>
                        <select className={styles.formSelect} value={agent.model} onChange={e => {
                    setAgents(prev => prev.map(a => a.id === agent.id ? {
                      ...a,
                      model: e.target.value
                    } : a));
                  }}>
                          {aiModels.length > 0 ? aiModels.map(m => <option key={m.id} value={m.name}>{m.name} ({m.provider})</option>) : <option value={agent.model}>{agent.model || t("Xin.k28")}</option>}
                        </select>
                      </div>
                      <div className={styles.formGroup}>
                        <label className={styles.formLabel}>{t("yuan-code.AgentPanel.k35")}</label>
                        <input className={styles.formInput} type="number" min={0} max={1} step={0.1} defaultValue={agent.temperature} />
                      </div>
                      <div className={styles.formGroup}>
                        <label className={styles.formLabel}>{t("yuan-code.AgentPanel.k36")}</label>
                        <input className={styles.formInput} type="number" min={1} max={200} defaultValue={agent.maxIterations} />
                      </div>
                      <div className={styles.formGroup}>
                        <label className={styles.formLabel}>{t("yuan-code.AgentPanel.k37")}</label>
                        <input className={styles.formInput} type="number" min={1000} step={1000} defaultValue={agent.tokenLimit} />
                      </div>
                    </div>

                    <div className={styles.agentActions}>
                      {agent.status !== 'running' && agent.status !== 'error' && <>
                          <div className={`${styles.toggleSwitch} ${agent.status !== 'stopped' ? styles.toggleSwitchOn : ''}`} onClick={() => toggleAgent(agent.id)} style={{
                    marginRight: 8
                  }}>
                            <div className={styles.toggleKnob} />
                          </div>
                          <span style={{
                    fontFamily: 'var(--nt-font-mono)',
                    fontSize: 10,
                    color: 'var(--nt-text-muted)'
                  }}>
                            {agent.status === 'stopped' ? t("yuan-code.AgentPanel.k20") : t("yuan-code.AgentPanel.k38")}
                          </span>
                        </>}
                      {agent.status === 'running' && <button className={styles.btnDanger} style={{
                  padding: '2px 10px',
                  fontSize: 11
                }} onClick={() => handleStopAgent(agent.id)}>{t("common.stop")}</button>}
                    </div>
                  </div>}
              </div>)}
          </div>
        </div>

        {/* Plan 计划展示 */}
        {plans.length > 0 && <div style={{
        marginTop: 16
      }}>
            <div style={{
          fontFamily: 'var(--nt-font-mono)',
          fontSize: 12,
          color: 'var(--nt-primary)',
          marginBottom: 8
        }}>
              {t("yuan-code.AgentPanel.k39")}{plans.length})
            </div>
            <div style={{
          maxHeight: 250,
          overflowY: 'auto'
        }}>
              {plans.map(plan => <div key={plan.id} style={{
            marginBottom: 8,
            padding: '8px 10px',
            border: '1px solid rgba(0,240,255,0.1)',
            borderRadius: 4,
            background: 'rgba(0,240,255,0.02)'
          }}>
                  <div style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              cursor: 'pointer'
            }} onClick={() => setExpandedPlan(expandedPlan === plan.id ? null : plan.id)}>
                    <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: 6,
                flex: 1
              }}>
                      <span style={{
                  fontSize: 12,
                  color: 'var(--nt-text-primary)'
                }}>{plan.title}</span>
                      <span style={{
                  fontSize: 9,
                  padding: '1px 5px',
                  borderRadius: 3,
                  background: plan.status === 'Completed' ? 'rgba(0,255,135,0.12)' : plan.status === 'Failed' ? 'rgba(255,0,110,0.12)' : 'rgba(0,240,255,0.12)',
                  color: plan.status === 'Completed' ? '#00FF87' : plan.status === 'Failed' ? '#FF006E' : 'var(--nt-primary)'
                }}>
                        {PLAN_STATUS_LABELS[plan.status] || plan.status}
                      </span>
                    </div>
                    <span style={{
                fontSize: 10,
                color: 'var(--nt-text-muted)',
                transform: expandedPlan === plan.id ? 'rotate(90deg)' : ''
              }}>▸</span>
                  </div>
                  <div style={{
              fontSize: 10,
              color: 'var(--nt-text-muted)',
              marginTop: 2
            }}>
                    {plan.steps.filter(s => s.status === 'Completed').length}/{plan.steps.length} {t("yuan-code.AgentPanel.k40")}
                  </div>

                  {expandedPlan === plan.id && <div style={{
              marginTop: 8,
              paddingTop: 8,
              borderTop: '1px solid rgba(0,240,255,0.08)'
            }}>
                      {plan.steps.map((step, idx) => <div key={step.id} style={{
                display: 'flex',
                alignItems: 'flex-start',
                gap: 6,
                padding: '4px 0',
                fontSize: 11,
                borderBottom: idx < plan.steps.length - 1 ? '1px solid rgba(0,240,255,0.04)' : 'none'
              }}>
                          <span style={{
                  fontSize: 10,
                  minWidth: 18
                }}>
                            {PLAN_STATUS_ICONS[step.status] || '⏳'}
                          </span>
                          <div style={{
                  flex: 1
                }}>
                            <div style={{
                    color: 'var(--nt-text-primary)'
                  }}>{step.title}</div>
                            {step.result && <div style={{
                    color: 'var(--nt-text-muted)',
                    fontSize: 10,
                    marginTop: 1
                  }}>
                                {step.result}
                              </div>}
                          </div>
                          {step.assigned_agent && <span style={{
                  fontSize: 9,
                  padding: '1px 4px',
                  borderRadius: 2,
                  background: 'rgba(176,38,255,0.08)',
                  color: '#B026FF',
                  whiteSpace: 'nowrap'
                }}>
                              {step.assigned_agent}
                            </span>}
                        </div>)}
                    </div>}
                </div>)}
            </div>
          </div>}

        {/* 引擎对话面板 */}
        {sessionId && <div style={{
        marginTop: 16
      }}>
            <div style={{
          fontFamily: 'var(--nt-font-mono)',
          fontSize: 12,
          color: 'var(--nt-primary)',
          marginBottom: 8,
          display: 'flex',
          alignItems: 'center',
          gap: 6
        }}>
              <span>{t("yuan-code.AgentPanel.k41")}</span>
              {engineEvents.isStreaming && <span style={{
            fontSize: 10,
            color: 'var(--nt-secondary)',
            animation: 'pulse 1s infinite'
          }}>{t("yuan-code.AgentPanel.k42")}</span>}
              <span style={{
            fontSize: 10,
            color: 'var(--nt-text-muted)',
            marginLeft: 'auto'
          }}>
                {t("Xin.k29")} {sessionId.slice(0, 8)}...
              </span>
            </div>

            {/* 对话历史 */}
            <div style={{
          maxHeight: 200,
          overflowY: 'auto',
          marginBottom: 8,
          padding: '8px',
          borderRadius: 6,
          background: 'rgba(0,240,255,0.03)',
          border: '1px solid rgba(0,240,255,0.1)'
        }}>
              {engineEvents.turns.length === 0 && !engineEvents.isStreaming && <div style={{
            fontSize: 11,
            color: 'var(--nt-text-muted)',
            textAlign: 'center',
            padding: '16px 0'
          }}>
                  {t("yuan-code.AgentPanel.k43")}
                </div>}
              {engineEvents.turns.map((turn, idx: number) => <div key={turn.id} style={{
            marginBottom: 8
          }}>
                  {/* 用户输入 */}
                  <div style={{
              display: 'flex',
              gap: 6,
              marginBottom: 4,
              alignItems: 'flex-start'
            }}>
                    <span style={{
                fontSize: 10,
                color: 'var(--nt-secondary)',
                minWidth: 32
              }}>👤</span>
                    <div style={{
                fontSize: 11,
                color: 'var(--nt-text-primary)',
                background: 'rgba(0,240,255,0.06)',
                padding: '4px 8px',
                borderRadius: 4,
                flex: 1
              }}>
                      {turn.userInput}
                    </div>
                  </div>
                  {/* AI 响应 */}
                  {(turn.response || idx === engineEvents.turns.length - 1 && engineEvents.currentStream) && <div style={{
              display: 'flex',
              gap: 6,
              alignItems: 'flex-start'
            }}>
                      <span style={{
                fontSize: 10,
                color: 'var(--nt-primary)',
                minWidth: 32
              }}>🤖</span>
                      <div style={{
                fontSize: 11,
                color: 'var(--nt-text-primary)',
                background: 'rgba(255,0,110,0.04)',
                padding: '4px 8px',
                borderRadius: 4,
                flex: 1
              }}>
                        {idx === engineEvents.turns.length - 1 && engineEvents.currentStream?.isStreaming ? engineEvents.currentStream.content : turn.response || ''}
                        {idx === engineEvents.turns.length - 1 && engineEvents.currentStream?.isStreaming && <span className={styles.cursorBlink}>▌</span>}
                      </div>
                    </div>}
                  {turn.status === 'error' && <div style={{
              fontSize: 10,
              color: 'var(--nt-error)',
              marginLeft: 38
            }}>{t("yuan-code.AgentPanel.k44")}</div>}
                </div>)}
              <div ref={chatEndRef} />
            </div>

            {/* 输入框 */}
            <div style={{
          display: 'flex',
          gap: 8
        }}>
              <textarea
                value={chatInput}
                onChange={e => setChatInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={isOnline ? t("yuan-code.AgentPanel.k45") : t('yuan-code.AgentPanel.offlineHint', { defaultValue: 'AI 离线，本地编辑可用。请连接网络后部署 Agent。' })}
                disabled={sending || !isOnline}
                style={{
            flex: 1,
            minHeight: 36,
            maxHeight: 80,
            resize: 'vertical',
            background: 'rgba(0,0,0,0.3)',
            border: '1px solid rgba(0,240,255,0.15)',
            borderRadius: 4,
            padding: '6px 10px',
            fontSize: 12,
            color: 'var(--nt-text-primary)',
            fontFamily: 'var(--nt-font-mono)',
            outline: 'none'
          }} />
              <button
                className={styles.btnPrimary}
                onClick={handleSendMessage}
                disabled={sending || !isOnline || !chatInput.trim()}
                title={!isOnline ? t('yuan-code.AgentPanel.offlineHint', { defaultValue: 'AI 离线，本地编辑可用。请连接网络后部署 Agent。' }) : undefined}
                style={{
            padding: '4px 14px',
            fontSize: 12,
            whiteSpace: 'nowrap'
          }}>
                {sending ? '...' : t("components.FloatingBall.k64")}
              </button>
            </div>
          </div>}

        {!sessionId && <div style={{
        marginTop: 16,
        padding: 12,
        textAlign: 'center'
      }}>
            <span style={{
          fontSize: 11,
          color: 'var(--nt-text-muted)'
        }}>
              {t("yuan-code.AgentPanel.k46")}
            </span>
          </div>}
      </div>
    </div>;
}

/** 将后端状态映射到前端状态 */
function mapBackendStatus(status: string): AgentStatus {
  switch (status) {
    case 'running':
    case 'thinking':
    case 'executing':
      return 'running';
    case 'idle':
    case 'pending_init':
      return 'idle';
    case 'errored':
    case 'error':
      return 'error';
    case 'completed':
    case 'completed_empty':
    case 'shutdown':
    case 'interrupted':
      return 'stopped';
    default:
      return 'idle';
  }
}