import { t } from "i18next";
import { useState, useCallback, useEffect } from 'react';
import { ipc, yuanCode, yuanCompact } from '@/lib/ipc';
import type { EditorSettings } from '../YuanCode';
import type { AgentsMdFile } from '@/lib/ipc';
import { GoalManager } from './GoalManager';
import styles from '../YuanCode.module.css';
interface McpServer {
  id: string;
  name: string;
  url: string;
  transport: 'sse' | 'stdio' | 'streamable';
  enabled: boolean;
  description: string;
}
interface BackendMcpServer {
  id: string;
  name: string;
  command: string;
  args: string[];
  env: Record<string, string> | null;
  enabled: boolean;
}
function mapBackendMcpServer(b: BackendMcpServer): McpServer {
  const isStdio = !b.command.startsWith('http');
  return {
    id: b.id,
    name: b.name,
    url: isStdio ? `stdio://${b.command}` : b.command,
    transport: isStdio ? 'stdio' : 'sse',
    enabled: b.enabled,
    description: b.args.length > 0 ? `Args: ${b.args.join(' ')}` : b.command
  };
}
interface SecurityConfig {
  algorithm: string;
  keyStore: string;
  autoLockMinutes: number;
  showSensitive: boolean;
}
interface CompressionConfig {
  strategy: 'auto' | 'manual' | 'off';
  autoThreshold: number;
  keepRecentTurns: number;
  summaryStyle: 'concise' | 'detailed';
  compactStrategy: string;
  currentTokens: number;
  maxTokens: number;
}
interface TokenBudget {
  maxPerCall: number;
  sessionLimit: number;
  warningThreshold: number;
}
interface ModelConfig {
  orchestrator: string;
  subAgent: string;
  embedding: string;
}
interface ToolDefinition {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
  examples: Array<{
    description: string;
    arguments: Record<string, unknown>;
  }>;
  category: string;
  tags: string[];
}
interface ToolInfo {
  definition: ToolDefinition;
  permission: {
    requires_approval: boolean;
    approval_level: string;
    sandbox_required: boolean;
    max_retries: number;
    timeout_ms: number;
  };
}
const CATEGORY_LABELS: Record<string, string> = {
  file: t("yuan-code.SettingsPanel.k1"),
  code: t("profile.ActivityTimeline.k1"),
  search: t("common.search"),
  plan: t("yuan-code.AgentDialog.k2"),
  execute: t("yuan-code.SettingsPanel.k2"),
  agent: 'Agent',
  knowledge: t("components.intelligence.ActivityPanel.k1"),
  sandbox: t("yuan-code.SettingsPanel.k3"),
  mcp: 'MCP',
  other: t("game3d.components.UI.BuildingDetail.k8")
};
const DEFAULT_MCP_SERVERS: McpServer[] = [{
  id: 'mcp_001',
  name: t("yuan-code.SettingsPanel.k4"),
  url: 'http://localhost:8899/sse',
  transport: 'sse',
  enabled: true,
  description: t("yuan-code.SettingsPanel.k5")
}, {
  id: 'mcp_002',
  name: t("yuan-code.SettingsPanel.k6"),
  url: 'stdio://filesystem',
  transport: 'stdio',
  enabled: false,
  description: t("yuan-code.SettingsPanel.k7")
}, {
  id: 'mcp_003',
  name: 'GitHub MCP',
  url: 'https://api.github.com/mcp',
  transport: 'streamable',
  enabled: false,
  description: t("yuan-code.SettingsPanel.k8")
}];
const EMBEDDING_OPTIONS = [{
  value: 'text-embedding-3-small',
  label: 'OpenAI text-embedding-3-small'
}, {
  value: 'text-embedding-3-large',
  label: 'OpenAI text-embedding-3-large'
}, {
  value: 'bge-m3',
  label: 'BGE-M3'
}, {
  value: 'e5-mistral-7b',
  label: 'E5 Mistral 7B'
}];

// 统一模型管理返回的模型项（与 get_ai_models 命令返回的 AiModelResponse 对应）
interface UnifiedAiModel {
  id: number;
  name: string;
  provider: string;
  model_name: string;
  display_name: string;
  model_type: string; // 'local' | 'api'
  multimodal: boolean;
  has_api_key: boolean;
}

export default function SettingsPanel({
  editorSettings,
  onEditorSettingsChange,
  onOrchestratorModelIdChange
}: {
  editorSettings: EditorSettings;
  onEditorSettingsChange: (settings: EditorSettings) => void;
  /** 编排模型（用于代码补全/分析）变更时通知父组件，传入统一模型 ID */
  onOrchestratorModelIdChange?: (modelId: number | null) => void;
}) {
  const [security, setSecurity] = useState<SecurityConfig>({
    algorithm: 'AES-256-GCM',
    keyStore: t("yuan-code.SettingsPanel.k9"),
    autoLockMinutes: 15,
    showSensitive: false
  });
  const [mcpServers, setMcpServers] = useState<McpServer[]>(DEFAULT_MCP_SERVERS);
  const [compression, setCompression] = useState<CompressionConfig>({
    strategy: 'auto',
    autoThreshold: 80,
    keepRecentTurns: 5,
    summaryStyle: 'concise',
    compactStrategy: 'tiered',
    currentTokens: 12000,
    maxTokens: 128000
  });
  const [tokenBudget, setTokenBudget] = useState<TokenBudget>({
    maxPerCall: 4096,
    sessionLimit: 128000,
    warningThreshold: 80
  });
  const [models, setModels] = useState<ModelConfig>({
    orchestrator: '',
    subAgent: '',
    embedding: 'text-embedding-3-small'
  });
  // D1.1：从统一模型管理拉取模型列表（替换原硬编码 MODEL_OPTIONS）
  const [aiModels, setAiModels] = useState<UnifiedAiModel[]>([]);
  const [aiModelsLoading, setAiModelsLoading] = useState(false);
  const [showAddMcp, setShowAddMcp] = useState(false);
  const [newMcp, setNewMcp] = useState<{
    name: string;
    url: string;
    transport: 'sse' | 'stdio' | 'streamable';
    description: string;
  }>({
    name: '',
    url: '',
    transport: 'sse',
    description: ''
  });
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState<{
    type: 'ok' | 'err';
    text: string;
  } | null>(null);

  // 工具管理
  const [tools, setTools] = useState<ToolInfo[]>([]);
  const [toolsLoading, setToolsLoading] = useState(false);
  const [toolSearchQuery, setToolSearchQuery] = useState('');
  const [expandedTool, setExpandedTool] = useState<string | null>(null);

  // 目标管理
  const [showGoalPanel, setShowGoalPanel] = useState(false);

  // AGENTS.md 发现
  const [agentsMdFiles, setAgentsMdFiles] = useState<AgentsMdFile[]>([]);
  const [agentsMdLoading, setAgentsMdLoading] = useState(false);
  const [agentsMdMaxBytes, setAgentsMdMaxBytes] = useState(40000);
  const [agentsMdResult, setAgentsMdResult] = useState<{
    totalBytes: number;
    truncated: boolean;
  } | null>(null);

  // 从后端加载工具列表
  useEffect(() => {
    const loadTools = async () => {
      setToolsLoading(true);
      try {
        const res = await ipc.invoke<ToolInfo[]>('yuan_tools_list');
        if (res.code === 0 && res.data) {
          setTools(res.data);
        }
      } catch {
        // 静默处理
      } finally {
        setToolsLoading(false);
      }
    };
    loadTools();
  }, []);

  // 从后端加载 MCP 服务器列表
  useEffect(() => {
    const loadMcp = async () => {
      try {
        const res = await ipc.invoke<BackendMcpServer[]>('yuan_mcp_list_servers');
        if (res.code === 0 && res.data && res.data.length > 0) {
          setMcpServers(res.data.map(mapBackendMcpServer));
        }
      } catch {
        // 后端不可用，使用默认数据
      }
    };
    loadMcp();
  }, []);

  // D1.1：从统一模型管理加载所有 AI 模型（替代硬编码 MODEL_OPTIONS）
  useEffect(() => {
    const loadAiModels = async () => {
      setAiModelsLoading(true);
      try {
        const res = await ipc.invoke<UnifiedAiModel[]>('get_ai_models');
        if (res.code === 0 && res.data) {
          setAiModels(res.data);
          // 默认选第一个模型作为 orchestrator（如果当前为空）
          if (!models.orchestrator && res.data.length > 0) {
            const firstId = String(res.data[0].id);
            setModels(prev => ({ ...prev, orchestrator: firstId }));
            onOrchestratorModelIdChange?.(res.data[0].id);
          } else if (models.orchestrator) {
            // 已有选择，同步通知父组件
            const parsedId = Number(models.orchestrator);
            if (!Number.isNaN(parsedId)) {
              onOrchestratorModelIdChange?.(parsedId);
            }
          }
        }
      } catch {
        // 静默处理（用户可能未配置模型）
      } finally {
        setAiModelsLoading(false);
      }
    };
    loadAiModels();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  const toggleMcp = useCallback((id: string) => {
    setMcpServers(prev => prev.map(s => s.id === id ? {
      ...s,
      enabled: !s.enabled
    } : s));
  }, []);
  const removeMcp = useCallback((id: string) => {
    setMcpServers(prev => prev.filter(s => s.id !== id));
  }, []);
  const addMcp = useCallback(() => {
    if (!newMcp.name.trim() || !newMcp.url.trim()) return;
    const server: McpServer = {
      id: `mcp_${Date.now()}`,
      name: newMcp.name.trim(),
      url: newMcp.url.trim(),
      transport: newMcp.transport,
      enabled: false,
      description: newMcp.description.trim()
    };
    setMcpServers(prev => [...prev, server]);
    setNewMcp({
      name: '',
      url: '',
      transport: 'sse',
      description: ''
    });
    setShowAddMcp(false);
  }, [newMcp]);
  const handleSave = useCallback(async () => {
    setSaving(true);
    setSaveMsg(null);
    try {
      await ipc.invoke('yuan_settings_save', {
        security,
        mcpServers,
        compression,
        tokenBudget,
        models
      });
      setSaveMsg({
        type: 'ok',
        text: t("yuan-code.SettingsPanel.k10")
      });
    } catch {
      setSaveMsg({
        type: 'err',
        text: t("errors.saveFailed")
      });
    } finally {
      setSaving(false);
      setTimeout(() => setSaveMsg(null), 2500);
    }
  }, [security, mcpServers, compression, tokenBudget, models]);
  return <div className={styles.panelContainer}>
      <div className={styles.panelHeader}>
        <span className={styles.panelTitle}>{t("yuan-code.SettingsPanel.k11")}</span>
        <div style={{
        display: 'flex',
        gap: 8,
        alignItems: 'center'
      }}>
          {saveMsg && <span style={{
          fontFamily: 'var(--nt-font-mono)',
          fontSize: 11,
          color: saveMsg.type === 'ok' ? '#00F0FF' : '#FF006E'
        }}>
              {saveMsg.text}
            </span>}
          <button className={styles.btnPrimary} onClick={handleSave} disabled={saving} style={{
          padding: '4px 16px',
          fontSize: 12
        }}>
            {saving ? t("components.AudioEditor.k6") : t("components.AudioEditor.k7")}
          </button>
        </div>
      </div>

      <div className={styles.panelBody}>

        {/* 安全加密 */}
        <div className={styles.settingsSection}>
          <h3 className={styles.settingsSectionTitle}>{t("yuan-code.SettingsPanel.k12")}</h3>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k13")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k14")}</span>
            </div>
            <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8
          }}>
              <select className={styles.formSelect} value={security.algorithm} onChange={e => setSecurity(prev => ({
              ...prev,
              algorithm: e.target.value
            }))}>
                <option value="AES-256-GCM">AES-256-GCM</option>
                <option value="AES-256-CBC">AES-256-CBC</option>
                <option value="ChaCha20-Poly1305">ChaCha20-Poly1305</option>
              </select>
              <span className={styles.badgeRunning}>{t("yuan-code.AgentPanel.k38")}</span>
            </div>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k15")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k16")}</span>
            </div>
            <span className={styles.badgeIdle}>{t("yuan-code.SettingsPanel.k17")}</span>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k18")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k19")}</span>
            </div>
            <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6
          }}>
              <input type="number" className={styles.formInput} style={{
              width: 70
            }} min={1} max={120} value={security.autoLockMinutes} onChange={e => setSecurity(prev => ({
              ...prev,
              autoLockMinutes: Number(e.target.value) || 15
            }))} />
              <span style={{
              color: 'var(--nt-text-muted)',
              fontSize: 11
            }}>{t("home.TodoPanel.k15")}</span>
            </div>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k20")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k21")}</span>
            </div>
            <div className={`${styles.toggleSwitch} ${security.showSensitive ? styles.toggleSwitchOn : ''}`} onClick={() => setSecurity(prev => ({
            ...prev,
            showSensitive: !prev.showSensitive
          }))}>
              <div className={styles.toggleKnob} />
            </div>
          </div>
        </div>

        <div className={styles.divider} />

        {/* MCP 服务器 */}
        <div className={styles.settingsSection}>
          <h3 className={styles.settingsSectionTitle}>{t("yuan-code.SettingsPanel.k22")}</h3>

          {mcpServers.map(server => <div key={server.id} className={styles.settingRow}>
              <div style={{
            flex: 1
          }}>
                <div style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              marginBottom: 2
            }}>
                  <span className={styles.settingLabel}>{server.name}</span>
                  <span style={{
                fontFamily: 'var(--nt-font-mono)',
                fontSize: 9,
                color: 'var(--nt-secondary)',
                padding: '1px 4px',
                border: '1px solid rgba(176,38,255,0.2)',
                borderRadius: 2
              }}>
                    {server.transport.toUpperCase()}
                  </span>
                </div>
                <span className={styles.settingDesc}>{server.description}</span>
                <div style={{
              marginTop: 2
            }}>
                  <code style={{
                fontFamily: 'var(--nt-font-mono)',
                fontSize: 10,
                color: 'var(--nt-text-muted)'
              }}>
                    {server.url}
                  </code>
                </div>
              </div>
              <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 10
          }}>
                <div className={`${styles.toggleSwitch} ${server.enabled ? styles.toggleSwitchOn : ''}`} onClick={() => toggleMcp(server.id)}>
                  <div className={styles.toggleKnob} />
                </div>
                <button onClick={() => removeMcp(server.id)} style={{
              background: 'none',
              border: 'none',
              color: 'var(--nt-text-muted)',
              cursor: 'pointer',
              fontSize: 14,
              padding: 0,
              lineHeight: 1
            }} title={t("common.remove")}>
                  ✕
                </button>
              </div>
            </div>)}

          {showAddMcp ? <div style={{
          marginTop: 10,
          padding: 12,
          border: '1px solid rgba(0,240,255,0.15)',
          borderRadius: 2,
          background: 'rgba(0,240,255,0.02)'
        }}>
              <div style={{
            display: 'grid',
            gridTemplateColumns: '1fr 1fr',
            gap: 8
          }}>
                <div className={styles.formGroup}>
                  <label className={styles.formLabel}>{t("game3d.components.UI.BuildingDetail.k13")}</label>
                  <input className={styles.formInput} placeholder={t("yuan-code.SettingsPanel.k23")} value={newMcp.name} onChange={e => setNewMcp(prev => ({
                ...prev,
                name: e.target.value
              }))} style={{
                padding: '4px 8px',
                fontSize: 11
              }} />
                </div>
                <div className={styles.formGroup}>
                  <label className={styles.formLabel}>{t("yuan-code.SettingsPanel.k24")}</label>
                  <select className={styles.formSelect} value={newMcp.transport} onChange={e => setNewMcp(prev => ({
                ...prev,
                transport: e.target.value as McpServer['transport']
              }))} style={{
                padding: '4px 8px',
                fontSize: 11,
                height: 28
              }}>
                    <option value="sse">SSE</option>
                    <option value="stdio">STDIO</option>
                    <option value="streamable">Streamable HTTP</option>
                  </select>
                </div>
              </div>
              <div className={styles.formGroup} style={{
            marginTop: 8
          }}>
                <label className={styles.formLabel}>URL</label>
                <input className={styles.formInput} placeholder="http://localhost:8899/sse" value={newMcp.url} onChange={e => setNewMcp(prev => ({
              ...prev,
              url: e.target.value
            }))} style={{
              padding: '4px 8px',
              fontSize: 11
            }} />
              </div>
              <div className={styles.formGroup} style={{
            marginTop: 8
          }}>
                <label className={styles.formLabel}>{t("common.description")}</label>
                <input className={styles.formInput} placeholder={t("yuan-code.SettingsPanel.k25")} value={newMcp.description} onChange={e => setNewMcp(prev => ({
              ...prev,
              description: e.target.value
            }))} style={{
              padding: '4px 8px',
              fontSize: 11
            }} />
              </div>
              <div style={{
            display: 'flex',
            gap: 6,
            marginTop: 10,
            justifyContent: 'flex-end'
          }}>
                <button className={styles.btnDanger} onClick={() => {
              setShowAddMcp(false);
              setNewMcp({
                name: '',
                url: '',
                transport: 'sse',
                description: ''
              });
            }} style={{
              padding: '3px 10px',
              fontSize: 11
            }}>
                  {t("common.cancel")}
                </button>
                <button className={styles.btnPrimary} onClick={addMcp} disabled={!newMcp.name.trim() || !newMcp.url.trim()} style={{
              padding: '3px 10px',
              fontSize: 11
            }}>
                  {t("common.add")}
                </button>
              </div>
            </div> : <button className={styles.btnPurple} onClick={() => setShowAddMcp(true)} style={{
          padding: '4px 12px',
          fontSize: 11,
          marginTop: 10
        }}>
              {t("yuan-code.SettingsPanel.k26")}
            </button>}
        </div>

        <div className={styles.divider} />

        {/* AGENTS.md 发现 */}
        <div className={styles.settingsSection}>
          <h3 className={styles.settingsSectionTitle}> {t("yuan-code.SettingsPanel.k27")}</h3>

          <div style={{
          marginBottom: 12
        }}>
            <span className={styles.settingDesc}>
              {t("yuan-code.SettingsPanel.k28")}
            </span>
          </div>

          <div style={{
          display: 'flex',
          gap: 8,
          marginBottom: 12
        }}>
            <button className={styles.btnPurple} onClick={async () => {
            setAgentsMdLoading(true);
            try {
              const files = await yuanCode.discoverAgentsMd();
              setAgentsMdFiles(files.data || []);
              // 获取字节预算
              const maxBytesRes = await yuanCode.getMaxBytes().catch(() => null);
              const maxBytes = (maxBytesRes as any)?.data || 40000;
              setAgentsMdMaxBytes(maxBytes);

              // 组装指令并计算总字节数
              const filesData = files.data || [];
              if (filesData.length > 0) {
                const result = await yuanCode.assembleInstructions(filesData, undefined, maxBytes);
                const resultData = (result as any)?.data;
                if (resultData) {
                  setAgentsMdResult({
                    totalBytes: resultData.total_bytes,
                    truncated: resultData.truncated
                  });
                }
              }
            } catch {
              setAgentsMdFiles([]);
            } finally {
              setAgentsMdLoading(false);
            }
          }} style={{
            padding: '4px 12px',
            fontSize: 11
          }}>
              {agentsMdLoading ? t("yuan-code.SettingsPanel.k29") : t("yuan-code.SettingsPanel.k30")}
            </button>
            {agentsMdFiles.length > 0 && <button className={styles.btnPurple} onClick={() => {
            setAgentsMdFiles([]);
            setAgentsMdResult(null);
          }} style={{
            padding: '4px 12px',
            fontSize: 11,
            opacity: 0.6
          }}>
                {t("Search.k14")}
              </button>}
          </div>

          {agentsMdFiles.length > 0 && <>
              <div className={styles.formGroup}>
                <label className={styles.formLabel}>{t("yuan-code.SettingsPanel.k31")}</label>
                <div style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8
            }}>
                  <input type="number" className={styles.formInput} style={{
                width: 100,
                padding: '2px 6px',
                fontSize: 11
              }} min={1000} max={200000} step={1000} value={agentsMdMaxBytes} onChange={async e => {
                const val = Number(e.target.value) || 40000;
                setAgentsMdMaxBytes(val);
                await yuanCode.setMaxBytes(val);
                if (agentsMdFiles.length > 0) {
                  const result = await yuanCode.assembleInstructions(agentsMdFiles, undefined, val);
                  const resultData = (result as any)?.data;
                  if (resultData) {
                    setAgentsMdResult({
                      totalBytes: resultData.total_bytes,
                      truncated: resultData.truncated
                    });
                  }
                }
              }} />
                  <span className={styles.settingDesc}>bytes</span>
                </div>
              </div>

              {agentsMdResult && <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 12,
            padding: '6px 10px',
            borderRadius: 6,
            background: 'rgba(0,240,255,0.06)',
            border: '1px solid rgba(0,240,255,0.12)',
            marginBottom: 10
          }}>
                  <span style={{
              fontSize: 11,
              fontFamily: 'var(--nt-font-mono)',
              color: 'var(--nt-text-secondary)'
            }}>
                    {t("yuan-code.SettingsPanel.k32")} {agentsMdResult.totalBytes.toLocaleString()}
                  </span>
                  <span style={{
              fontSize: 11,
              fontFamily: 'var(--nt-font-mono)',
              color: 'var(--nt-text-secondary)'
            }}>
                    {t("yuan-code.SettingsPanel.k33")} {agentsMdMaxBytes.toLocaleString()}
                  </span>
                  {agentsMdResult.truncated && <span style={{
              fontSize: 10,
              color: '#FFB86C',
              fontFamily: 'var(--nt-font-mono)'
            }}>
                      {t("yuan-code.SettingsPanel.k34")}
                    </span>}
                  {/* 进度条 */}
                  <div style={{
              flex: 1,
              height: 4,
              borderRadius: 2,
              background: 'rgba(255,255,255,0.08)',
              overflow: 'hidden'
            }}>
                    <div style={{
                height: '100%',
                width: `${Math.min(100, agentsMdResult.totalBytes / agentsMdMaxBytes * 100)}%`,
                background: agentsMdResult.totalBytes > agentsMdMaxBytes ? '#FFB86C' : agentsMdResult.totalBytes > agentsMdMaxBytes * 0.8 ? '#FFD700' : '#00F0FF',
                borderRadius: 2,
                transition: 'width 0.3s'
              }} />
                  </div>
                </div>}

              <div style={{
            maxHeight: 200,
            overflowY: 'auto',
            borderRadius: 6,
            border: '1px solid rgba(0,240,255,0.08)'
          }}>
                {agentsMdFiles.map((file, i) => <div key={i} style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '6px 10px',
              borderBottom: i < agentsMdFiles.length - 1 ? '1px solid rgba(0,240,255,0.06)' : 'none',
              fontSize: 11
            }}>
                    <span style={{
                color: '#00F0FF',
                fontFamily: 'var(--nt-font-mono)',
                fontSize: 10,
                minWidth: 20
              }}>
                      L{file.depth}
                    </span>
                    <span style={{
                color: 'var(--nt-text-primary)',
                fontFamily: 'var(--nt-font-mono)',
                fontSize: 10,
                flex: 1
              }}>
                      {file.path}
                    </span>
                    <span style={{
                color: 'var(--nt-text-secondary)',
                fontFamily: 'var(--nt-font-mono)',
                fontSize: 10
              }}>
                      {file.content.length.toLocaleString()} bytes
                    </span>
                    <span style={{
                color: 'var(--nt-text-secondary)',
                fontSize: 9,
                fontFamily: 'var(--nt-font-mono)'
              }}>
                      {file.scope_root}
                    </span>
                  </div>)}
              </div>
            </>}

          {agentsMdFiles.length === 0 && !agentsMdLoading && <div className={styles.settingRow}>
              <span className={styles.settingDesc}>
                {t("yuan-code.SettingsPanel.k35")}
              </span>
            </div>}
        </div>

        <div className={styles.divider} />

        {/* 上下文压缩 */}
        <div className={styles.settingsSection}>
          <h3 className={styles.settingsSectionTitle}>{t("yuan-code.SettingsPanel.k36")}</h3>
          <div className={styles.settingRow}>
            <span className={styles.settingDesc}>
              {t("yuan-code.SettingsPanel.k37")}
            </span>
          </div>

          <div className={styles.compactionStrategy}>
            <div className={styles.card} style={{
            borderColor: compression.strategy === 'auto' ? 'rgba(0,240,255,0.35)' : undefined
          }}>
              <div className={styles.cardHeader}>
                <span className={styles.cardTitle}>{t("yuan-code.SettingsPanel.k38")}</span>
                <span className={compression.strategy === 'auto' ? styles.badgeRunning : styles.badgeIdle} onClick={async () => {
                setCompression(prev => ({
                  ...prev,
                  strategy: 'auto'
                }));
                try {
                  await yuanCompact.updateConfig({
                    trigger: 'auto'
                  });
                } catch {/* 后端不可用时仅本地生效 */}
              }} style={{
                cursor: 'pointer'
              }}>
                  {compression.strategy === 'auto' ? t("game3d.KnowledgeMapping.k27") : t("common.enable")}
                </span>
              </div>
              <div className={styles.cardBody}>
                <p style={{
                color: 'var(--nt-text-secondary)',
                fontSize: 12,
                margin: 0
              }}>
                  {t("yuan-code.SettingsPanel.k39")}
                </p>
                {compression.strategy === 'auto' && <>
                    <div className={styles.formGroup} style={{
                  marginTop: 10
                }}>
                      <label className={styles.formLabel}>{t("Xin.k61")}</label>
                      <select className={styles.formSelect} value={compression.autoThreshold} onChange={e => {
                    const val = Number(e.target.value);
                    setCompression(prev => ({
                      ...prev,
                      autoThreshold: val
                    }));
                    yuanCompact.updateConfig({
                      auto_threshold: val
                    }).catch(() => {});
                  }}>
                        <option value="60">60%</option>
                        <option value="70">70%</option>
                        <option value="80">80%</option>
                        <option value="90">90%</option>
                      </select>
                    </div>
                    <div className={styles.formGroup} style={{
                  marginTop: 8
                }}>
                      <label className={styles.formLabel}>{t("yuan-code.SettingsPanel.k40")}</label>
                      <select className={styles.formSelect} value={compression.compactStrategy} onChange={e => {
                    setCompression(prev => ({
                      ...prev,
                      compactStrategy: e.target.value
                    }));
                    yuanCompact.updateConfig({
                      strategy: e.target.value
                    }).catch(() => {});
                  }}>
                        <option value="tiered">{t("yuan-code.SettingsPanel.k41")}</option>
                        <option value="sliding_window">{t("yuan-code.SettingsPanel.k42")}</option>
                        <option value="summary">{t("yuan-code.SettingsPanel.k43")}</option>
                        <option value="hybrid">{t("yuan-code.SettingsPanel.k44")}</option>
                      </select>
                    </div>
                    <div className={styles.formGroup} style={{
                  marginTop: 8
                }}>
                      <label className={styles.formLabel}>{t("yuan-code.SettingsPanel.k45")}</label>
                      <input type="number" className={styles.formInput} style={{
                    width: 80,
                    padding: '2px 6px',
                    fontSize: 11
                  }} min={1} max={20} value={compression.keepRecentTurns} onChange={e => {
                    const val = Number(e.target.value) || 5;
                    setCompression(prev => ({
                      ...prev,
                      keepRecentTurns: val
                    }));
                    yuanCompact.updateConfig({
                      keep_recent_turns: val
                    }).catch(() => {});
                  }} />
                    </div>
                  </>}
              </div>
            </div>

            <div className={styles.card} style={{
            borderColor: compression.strategy === 'manual' ? 'rgba(0,240,255,0.35)' : undefined
          }}>
              <div className={styles.cardHeader}>
                <span className={styles.cardTitle}>{t("yuan-code.SettingsPanel.k46")}</span>
                <span className={compression.strategy === 'manual' ? styles.badgeRunning : styles.badgeIdle} onClick={async () => {
                setCompression(prev => ({
                  ...prev,
                  strategy: 'manual'
                }));
                try {
                  await yuanCompact.updateConfig({
                    trigger: 'manual'
                  });
                } catch {/* 后端不可用时仅本地生效 */}
              }} style={{
                cursor: 'pointer'
              }}>
                  {compression.strategy === 'manual' ? t("game3d.KnowledgeMapping.k27") : t("common.enable")}
                </span>
              </div>
              <div className={styles.cardBody}>
                <p style={{
                color: 'var(--nt-text-secondary)',
                fontSize: 12,
                margin: 0
              }}>
                  {t("yuan-code.SettingsPanel.k47")}
                </p>
                {compression.strategy === 'manual' && <div className={styles.formGroup} style={{
                marginTop: 10
              }}>
                    <label className={styles.formLabel}>{t("yuan-code.SettingsPanel.k48")}</label>
                    <select className={styles.formSelect} value={compression.summaryStyle} onChange={e => {
                  setCompression(prev => ({
                    ...prev,
                    summaryStyle: e.target.value as 'concise' | 'detailed'
                  }));
                  yuanCompact.updateConfig({
                    summary_style: e.target.value
                  }).catch(() => {});
                }}>
                      <option value="concise">{t("yuan-code.SettingsPanel.k49")}</option>
                      <option value="detailed">{t("yuan-code.SettingsPanel.k50")}</option>
                    </select>
                  </div>}
              </div>
            </div>
          </div>

          {/* 预算可视化 */}
          <div className={styles.formGroup} style={{
          marginTop: 12
        }}>
            <label className={styles.formLabel}>{t("yuan-code.SettingsPanel.k51")}</label>
            <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '6px 10px',
            borderRadius: 6,
            background: 'rgba(0,240,255,0.04)',
            border: '1px solid rgba(0,240,255,0.08)'
          }}>
              <div style={{
              flex: 1,
              height: 6,
              borderRadius: 3,
              background: 'rgba(255,255,255,0.06)',
              overflow: 'hidden'
            }}>
                <div style={{
                height: '100%',
                width: `${Math.min(100, compression.currentTokens / compression.maxTokens * 100)}%`,
                background: compression.currentTokens > compression.maxTokens ? '#FF6B6B' : compression.currentTokens > compression.maxTokens * 0.8 ? '#FFB86C' : '#00F0FF',
                borderRadius: 3,
                transition: 'width 0.5s'
              }} />
              </div>
              <span style={{
              fontSize: 10,
              fontFamily: 'var(--nt-font-mono)',
              color: 'var(--nt-text-secondary)',
              whiteSpace: 'nowrap'
            }}>
                {compression.currentTokens}/{compression.maxTokens} tokens
              </span>
            </div>
          </div>
        </div>

        <div className={styles.divider} />

        {/* Token 预算 */}
        <div className={styles.settingsSection}>
          <h3 className={styles.settingsSectionTitle}>{t("yuan-code.SettingsPanel.k52")}</h3>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k53")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k54")}</span>
            </div>
            <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6
          }}>
              <input type="number" className={styles.formInput} style={{
              width: 110
            }} min={512} max={131072} step={512} value={tokenBudget.maxPerCall} onChange={e => setTokenBudget(prev => ({
              ...prev,
              maxPerCall: Number(e.target.value) || 4096
            }))} />
              <span style={{
              color: 'var(--nt-text-muted)',
              fontSize: 11
            }}>tokens</span>
            </div>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k55")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k56")}</span>
            </div>
            <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6
          }}>
              <input type="number" className={styles.formInput} style={{
              width: 110
            }} min={16000} max={500000} step={8000} value={tokenBudget.sessionLimit} onChange={e => setTokenBudget(prev => ({
              ...prev,
              sessionLimit: Number(e.target.value) || 128000
            }))} />
              <span style={{
              color: 'var(--nt-text-muted)',
              fontSize: 11
            }}>tokens</span>
            </div>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k57")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k58")}</span>
            </div>
            <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6
          }}>
              <input type="number" className={styles.formInput} style={{
              width: 80
            }} min={50} max={95} value={tokenBudget.warningThreshold} onChange={e => setTokenBudget(prev => ({
              ...prev,
              warningThreshold: Number(e.target.value) || 80
            }))} />
              <span style={{
              color: 'var(--nt-text-muted)',
              fontSize: 11
            }}>%</span>
            </div>
          </div>
        </div>

        <div className={styles.divider} />

        {/* 模型选择 */}
        <div className={styles.settingsSection}>
          <h3 className={styles.settingsSectionTitle}>{t("yuan-code.SettingsPanel.k59")}</h3>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k60")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k61")}</span>
            </div>
            <select className={styles.formSelect} value={models.orchestrator} onChange={e => {
              const v = e.target.value;
              setModels(prev => ({ ...prev, orchestrator: v }));
              const parsedId = Number(v);
              if (!Number.isNaN(parsedId)) {
                onOrchestratorModelIdChange?.(parsedId);
              } else {
                onOrchestratorModelIdChange?.(null);
              }
            }}>
              {aiModelsLoading && <option value="">{t("components.AudioEditor.k6")}...</option>}
              {!aiModelsLoading && aiModels.length === 0 && <option value="">{t("yuan-code.SettingsPanel.k9")}</option>}
              {aiModels.map(m => <option key={m.id} value={String(m.id)}>{m.display_name} ({m.provider}){m.model_type === 'local' ? ' [本地]' : ''}</option>)}
            </select>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k62")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k63")}</span>
            </div>
            <select className={styles.formSelect} value={models.subAgent} onChange={e => setModels(prev => ({
            ...prev,
            subAgent: e.target.value
          }))}>
              {aiModels.map(m => <option key={m.id} value={String(m.id)}>{m.display_name} ({m.provider})</option>)}
            </select>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k64")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k65")}</span>
            </div>
            <select className={styles.formSelect} value={models.embedding} onChange={e => setModels(prev => ({
            ...prev,
            embedding: e.target.value
          }))}>
              {EMBEDDING_OPTIONS.map(m => <option key={m.value} value={m.value}>{m.label}</option>)}
            </select>
          </div>
        </div>

        <div className={styles.divider} />

        {/* 编辑器设置 */}
        <div className={styles.settingsSection}>
          <h3 className={styles.settingsSectionTitle}>{t("yuan-code.SettingsPanel.k66")}</h3>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k67")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k68")}</span>
            </div>
            <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6
          }}>
              <input type="range" min={12} max={24} value={editorSettings.fontSize} onChange={e => onEditorSettingsChange({
              ...editorSettings,
              fontSize: Number(e.target.value)
            })} style={{
              width: 100
            }} />
              <span style={{
              fontFamily: 'var(--nt-font-mono)',
              fontSize: 12,
              color: 'var(--nt-primary)',
              minWidth: 24
            }}>
                {editorSettings.fontSize}px
              </span>
            </div>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k69")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k70")}</span>
            </div>
            <select className={styles.formSelect} value={editorSettings.tabSize} onChange={e => onEditorSettingsChange({
            ...editorSettings,
            tabSize: Number(e.target.value)
          })}>
              <option value={2}>{t("yuan-code.SettingsPanel.k71")}</option>
              <option value={4}>{t("yuan-code.SettingsPanel.k72")}</option>
              <option value={8}>{t("yuan-code.SettingsPanel.k73")}</option>
            </select>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k74")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k75")}</span>
            </div>
            <select className={styles.formSelect} value={editorSettings.wordWrap} onChange={e => onEditorSettingsChange({
            ...editorSettings,
            wordWrap: e.target.value as EditorSettings['wordWrap']
          })}>
              <option value="off">{t("common.close")}</option>
              <option value="on">{t("IconDemo.k7")}</option>
              <option value="wordWrapColumn">{t("yuan-code.SettingsPanel.k76")}</option>
            </select>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k77")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k78")}</span>
            </div>
            <div className={`${styles.toggleSwitch} ${editorSettings.showMinimap ? styles.toggleSwitchOn : ''}`} onClick={() => onEditorSettingsChange({
            ...editorSettings,
            showMinimap: !editorSettings.showMinimap
          })}>
              <div className={styles.toggleKnob} />
            </div>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("yuan-code.SettingsPanel.k79")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k80")}</span>
            </div>
            <div className={`${styles.toggleSwitch} ${editorSettings.autoSave ? styles.toggleSwitchOn : ''}`} onClick={() => onEditorSettingsChange({
            ...editorSettings,
            autoSave: !editorSettings.autoSave
          })}>
              <div className={styles.toggleKnob} />
            </div>
          </div>

          <div className={styles.settingRow}>
            <div>
              <span className={styles.settingLabel}>{t("CommandManual.k116")}</span>
              <span className={styles.settingDesc}>{t("yuan-code.SettingsPanel.k81")}</span>
            </div>
            <div className={`${styles.toggleSwitch} ${editorSettings.formatOnSave ? styles.toggleSwitchOn : ''}`} onClick={() => onEditorSettingsChange({
            ...editorSettings,
            formatOnSave: !editorSettings.formatOnSave
          })}>
              <div className={styles.toggleKnob} />
            </div>
          </div>
        </div>

        <div className={styles.divider} />

        {/* 目标管理 */}
        <div className={styles.settingsSection}>
          <h3 className={styles.settingsSectionTitle}>{t("yuan-code.GoalManager.k8")}</h3>
          <div className={styles.settingRow}>
            <span className={styles.settingDesc}>
              {t("yuan-code.SettingsPanel.k82")}
            </span>
          </div>
          <button className={styles.btnPurple} onClick={() => setShowGoalPanel(true)} style={{
          padding: '6px 16px',
          fontSize: 12,
          marginTop: 8
        }}>
            {t("yuan-code.SettingsPanel.k83")}
          </button>
        </div>

        <div className={styles.divider} />

        {/* 工具管理 */}
        <div className={styles.settingsSection}>
          <h3 className={styles.settingsSectionTitle}>{t("yuan-code.SettingsPanel.k84")}</h3>

          <div style={{
          marginBottom: 10
        }}>
            <input className={styles.formInput} placeholder={t("yuan-code.SettingsPanel.k85")} value={toolSearchQuery} onChange={e => setToolSearchQuery(e.target.value)} style={{
            padding: '4px 10px',
            fontSize: 11,
            width: '100%'
          }} />
          </div>

          {toolsLoading ? <div style={{
          fontSize: 12,
          color: 'var(--nt-text-muted)',
          textAlign: 'center',
          padding: 16
        }}>
              {t("yuan-code.SettingsPanel.k86")}
            </div> : tools.length === 0 ? <div style={{
          fontSize: 12,
          color: 'var(--nt-text-muted)',
          textAlign: 'center',
          padding: 16
        }}>
              {t("yuan-code.SettingsPanel.k87")}
            </div> : <div style={{
          maxHeight: 300,
          overflowY: 'auto'
        }}>
              {tools.filter(t => {
            if (!toolSearchQuery.trim()) return true;
            const q = toolSearchQuery.toLowerCase();
            return t.definition.name.toLowerCase().includes(q) || t.definition.description.toLowerCase().includes(q) || t.definition.tags.some(tag => tag.toLowerCase().includes(q));
          }).map(tool => <div key={tool.definition.name} style={{
            padding: '8px 10px',
            marginBottom: 6,
            border: '1px solid rgba(0,240,255,0.1)',
            borderRadius: 4,
            background: 'rgba(0,240,255,0.02)',
            cursor: 'pointer'
          }} onClick={() => setExpandedTool(expandedTool === tool.definition.name ? null : tool.definition.name)}>
                    <div style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between'
            }}>
                      <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                flex: 1
              }}>
                        <span style={{
                  fontFamily: 'var(--nt-font-mono)',
                  fontSize: 12,
                  color: 'var(--nt-primary)'
                }}>
                          {tool.definition.name}
                        </span>
                        <span style={{
                  fontSize: 9,
                  padding: '1px 5px',
                  borderRadius: 3,
                  background: 'rgba(0,240,255,0.1)',
                  color: 'var(--nt-primary)'
                }}>
                          {CATEGORY_LABELS[tool.definition.category] || tool.definition.category}
                        </span>
                        {tool.permission.requires_approval && <span style={{
                  fontSize: 9,
                  padding: '1px 5px',
                  borderRadius: 3,
                  background: 'rgba(255,180,0,0.12)',
                  color: '#FFB400'
                }}>
                            {t("yuan-code.SettingsPanel.k88")}
                          </span>}
                        {tool.permission.sandbox_required && <span style={{
                  fontSize: 9,
                  padding: '1px 5px',
                  borderRadius: 3,
                  background: 'rgba(176,38,255,0.12)',
                  color: '#B026FF'
                }}>
                            {t("yuan-code.SettingsPanel.k3")}
                          </span>}
                      </div>
                      <span style={{
                fontSize: 10,
                color: 'var(--nt-text-muted)',
                transform: expandedTool === tool.definition.name ? 'rotate(90deg)' : ''
              }}>
                        ▸
                      </span>
                    </div>
                    <div style={{
              fontSize: 11,
              color: 'var(--nt-text-secondary)',
              marginTop: 4
            }}>
                      {tool.definition.description}
                    </div>
                    {tool.definition.tags.length > 0 && <div style={{
              display: 'flex',
              gap: 4,
              marginTop: 4,
              flexWrap: 'wrap'
            }}>
                        {tool.definition.tags.map(tag => <span key={tag} style={{
                fontSize: 9,
                padding: '1px 5px',
                borderRadius: 2,
                background: 'rgba(0,240,255,0.05)',
                color: 'var(--nt-text-muted)'
              }}>
                            {tag}
                          </span>)}
                      </div>}

                    {expandedTool === tool.definition.name && <div style={{
              marginTop: 8,
              paddingTop: 8,
              borderTop: '1px solid rgba(0,240,255,0.08)'
            }}>
                        <div style={{
                fontSize: 10,
                color: 'var(--nt-text-muted)',
                marginBottom: 6
              }}>
                          {t("yuan-code.SettingsPanel.k89")}
                        </div>
                        <div style={{
                display: 'grid',
                gridTemplateColumns: '1fr 1fr',
                gap: 4,
                fontSize: 10
              }}>
                          <div>
                            <span style={{
                    color: 'var(--nt-text-muted)'
                  }}>{t("yuan-code.SettingsPanel.k90")} </span>
                            <span style={{
                    color: 'var(--nt-text-primary)'
                  }}>{tool.permission.approval_level}</span>
                          </div>
                          <div>
                            <span style={{
                    color: 'var(--nt-text-muted)'
                  }}>{t("yuan-code.SettingsPanel.k91")} </span>
                            <span style={{
                    color: 'var(--nt-text-primary)'
                  }}>{tool.permission.max_retries}</span>
                          </div>
                          <div>
                            <span style={{
                    color: 'var(--nt-text-muted)'
                  }}>{t("yuan-code.SettingsPanel.k92")} </span>
                            <span style={{
                    color: 'var(--nt-text-primary)'
                  }}>{tool.permission.timeout_ms}ms</span>
                          </div>
                          <div>
                            <span style={{
                    color: 'var(--nt-text-muted)'
                  }}>{t("yuan-code.SettingsPanel.k93")} </span>
                            <span style={{
                    color: 'var(--nt-text-primary)'
                  }}>
                              {tool.permission.requires_approval ? t("common.yes") : t("common.no")}
                            </span>
                          </div>
                          <div>
                            <span style={{
                    color: 'var(--nt-text-muted)'
                  }}>{t("yuan-code.SettingsPanel.k94")} </span>
                            <span style={{
                    color: 'var(--nt-text-primary)'
                  }}>
                              {tool.permission.sandbox_required ? t("common.yes") : t("common.no")}
                            </span>
                          </div>
                        </div>
                      </div>}
                  </div>)}
            </div>}
        </div>

        {/* 底部留白 */}
        <div style={{
        height: 40
      }} />
      </div>

      {/* 目标管理弹窗 */}
      {showGoalPanel && <GoalManager onClose={() => setShowGoalPanel(false)} />}
    </div>;
}