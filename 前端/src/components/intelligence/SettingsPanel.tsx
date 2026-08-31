import { t } from "i18next";
import { useState, useEffect, useCallback } from 'react';
import { ipc, intelligence, ai } from '@/lib/ipc';
import styles from './Intelligence.module.css';
interface ToggleSetting {
  key: string;
  name: string;
  description: string;
  enabled: boolean;
}
const DEFAULT_TOGGLES: ToggleSetting[] = [{
  key: 'master_switch',
  name: t("components.intelligence.SettingsPanel.k1"),
  description: t("components.intelligence.SettingsPanel.k2"),
  enabled: true
}, {
  key: 'auto_suggest',
  name: t("components.intelligence.ActivityPanel.k17"),
  description: t("components.intelligence.SettingsPanel.k3"),
  enabled: true
}, {
  key: 'context_collection',
  name: t("components.intelligence.SettingsPanel.k4"),
  description: t("components.intelligence.SettingsPanel.k5"),
  enabled: true
}, {
  key: 'activity_tracking',
  name: t("components.intelligence.SettingsPanel.k6"),
  description: t("components.intelligence.SettingsPanel.k7"),
  enabled: true
}, {
  key: 'focus_detection',
  name: t("components.intelligence.SettingsPanel.k8"),
  description: t("components.intelligence.SettingsPanel.k9"),
  enabled: false
}, {
  key: 'smart_complete',
  name: t("components.intelligence.SettingsPanel.k10"),
  description: t("components.intelligence.SettingsPanel.k11"),
  enabled: true
}, {
  key: 'auto_classify',
  name: t("components.intelligence.SettingsPanel.k12"),
  description: t("components.intelligence.SettingsPanel.k13"),
  enabled: true
}, {
  key: 'news_summary',
  name: t("components.intelligence.SettingsPanel.k14"),
  description: t("components.intelligence.SettingsPanel.k15"),
  enabled: false
}, {
  key: 'quote_check',
  name: t("components.intelligence.SettingsPanel.k16"),
  description: t("components.intelligence.SettingsPanel.k17"),
  enabled: false
}, {
  key: 'command_suggest',
  name: t("components.intelligence.SettingsPanel.k18"),
  description: t("components.intelligence.SettingsPanel.k19"),
  enabled: true
}, {
  key: 'game_recommend',
  name: t("components.intelligence.SettingsPanel.k20"),
  description: t("components.intelligence.SettingsPanel.k21"),
  enabled: false
}, {
  key: 'search_enhance',
  name: t("components.intelligence.SettingsPanel.k22"),
  description: t("components.intelligence.SettingsPanel.k23"),
  enabled: true
}, {
  key: 'timer_smart',
  name: t("components.intelligence.SettingsPanel.k24"),
  description: t("components.intelligence.SettingsPanel.k25"),
  enabled: true
}, {
  key: 'floating_ball',
  name: t("components.intelligence.SettingsPanel.k26"),
  description: t("components.intelligence.SettingsPanel.k27"),
  enabled: false
}];
const LLM_MODELS = [{
  key: 'ollama',
  label: t("components.intelligence.SettingsPanel.k28"),
  hint: 'http://localhost:11434'
}, {
  key: 'openai',
  label: 'OpenAI API',
  hint: 'https://api.openai.com/v1'
}, {
  key: 'custom',
  label: t("components.intelligence.SettingsPanel.k29"),
  hint: 'http://localhost:8000'
}];
const SETTINGS_KEY = 'intelligence_frontend_settings';
interface FrontendIntelligenceSettings {
  toggles: Record<string, boolean>;
  llmProvider: string;
  llmEndpoint: string;
  llmModel: string;
  llmApiKey: string;
}
function buildFrontendSettings(toggles: ToggleSetting[], llmProvider: string, llmEndpoint: string, llmModel: string, llmApiKey: string): FrontendIntelligenceSettings {
  const togglesMap: Record<string, boolean> = {};
  toggles.forEach(t => {
    togglesMap[t.key] = t.enabled;
  });
  return {
    toggles: togglesMap,
    llmProvider,
    llmEndpoint,
    llmModel,
    llmApiKey
  };
}
export default function SettingsPanel() {
  const [toggles, setToggles] = useState<ToggleSetting[]>(DEFAULT_TOGGLES);
  const [llmProvider, setLlmProvider] = useState('ollama');
  const [llmEndpoint, setLlmEndpoint] = useState('http://localhost:11434');
  const [llmModel, setLlmModel] = useState('');
  const [llmApiKey, setLlmApiKey] = useState('');
  const [feedback, setFeedback] = useState<{
    type: 'success' | 'error';
    message: string;
  } | null>(null);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    success: boolean;
    latency_ms: number;
    message: string;
  } | null>(null);
  const handleProviderChange = (provider: string) => {
    setLlmProvider(provider);
    setTestResult(null);
    const model = LLM_MODELS.find(m => m.key === provider);
    if (model?.hint) setLlmEndpoint(model.hint);
  };
  const handleTestConnection = async () => {
    setTesting(true);
    setTestResult(null);
    setFeedback(null);
    try {
      const res = await intelligence.testConnection(llmProvider, llmEndpoint, llmProvider !== 'ollama' ? llmApiKey : undefined, llmModel || undefined);
      if (res?.data) {
        setTestResult(res.data);
      } else {
        setFeedback({
          type: 'error',
          message: res?.message || t("components.intelligence.SettingsPanel.k30")
        });
      }
    } catch (e: any) {
      setFeedback({
        type: 'error',
        message: t("components.intelligence.SettingsPanel.k31", {
          e: e
        })
      });
    }
    setTesting(false);
  };

  // AI会话板块的模型列表（声明要求：模型来源统一）
  const [aiModels, setAiModels] = useState<Array<{
    id: number;
    name: string;
    provider: string;
    model_name: string;
  }>>([]);
  const loadAiModels = useCallback(async () => {
    try {
      const res = await ai.getModels();
      if (res?.data && Array.isArray(res.data)) {
        setAiModels(res.data);
        // 如果还没选模型且有可用模型，自动选第一个
        if (!llmModel && res.data.length > 0) {
          setLlmModel(res.data[0].name);
        }
      }
    } catch {
      // 静默失败，不回退用户输入
    }
  }, []);
  useEffect(() => {
    loadAiModels();
  }, [loadAiModels]);
  const load = useCallback(async () => {
    try {
      // 优先从 V4 专用设置接口加载，再回退到 system_configs
      const v4Res = await intelligence.getSettings();
      const res = v4Res?.data;
      if (res) {
        setLlmEndpoint(res.llm_endpoint || 'http://localhost:11434');
        if (res.llm_model) setLlmModel(res.llm_model);
        if (res.llm_provider) setLlmProvider(res.llm_provider);
        if (res.llm_api_key) setLlmApiKey(res.llm_api_key);
      }

      // 从 system_configs 加载前端专属开关（粒度更细）
      const configRes = await ipc.invoke<any>('get_system_config', {
        key: SETTINGS_KEY
      });
      if (configRes?.code === 0 && configRes?.data?.config_value) {
        const parsed: FrontendIntelligenceSettings = JSON.parse(configRes.data.config_value);
        if (parsed.toggles) {
          setToggles(prev => prev.map(t => ({
            ...t,
            enabled: parsed.toggles[t.key] ?? t.enabled
          })));
        }
        // LLM配置优先从 system_configs 覆盖
        if (parsed.llmProvider) setLlmProvider(parsed.llmProvider);
        if (parsed.llmEndpoint) setLlmEndpoint(parsed.llmEndpoint);
        if (parsed.llmModel) setLlmModel(parsed.llmModel);
        if (parsed.llmApiKey) setLlmApiKey(parsed.llmApiKey);
      }
    } catch (e) {
      console.error('加载设置失败:', e);
    }
  }, []);
  useEffect(() => {
    load();
  }, [load]);
  const handleSave = useCallback(async () => {
    setFeedback(null);
    try {
      // 保存到 V4 专用设置
      await intelligence.saveSettings({
        llm_provider: llmProvider,
        llm_endpoint: llmEndpoint,
        llm_model: llmModel,
        llm_api_key: llmApiKey
      });

      // 保存前端专属开关到 system_configs
      const settings = buildFrontendSettings(toggles, llmProvider, llmEndpoint, llmModel, llmApiKey);
      const res = await ipc.invoke<any>('set_system_config', {
        key: SETTINGS_KEY,
        value: JSON.stringify(settings)
      });
      if (res?.code === 0) {
        setFeedback({
          type: 'success',
          message: t("components.intelligence.SettingsPanel.k32")
        });
      } else {
        throw new Error(res?.message || t("errors.unknown"));
      }
    } catch (e) {
      setFeedback({
        type: 'error',
        message: t("components.intelligence.SettingsPanel.k33", {
          e: e
        })
      });
    }
  }, [toggles, llmProvider, llmEndpoint, llmModel, llmApiKey]);
  const handleReset = useCallback(async () => {
    setFeedback(null);
    try {
      await intelligence.resetSettings();
      const defaults = buildFrontendSettings(DEFAULT_TOGGLES, 'ollama', 'http://localhost:11434', '', '');
      const res = await ipc.invoke<any>('set_system_config', {
        key: SETTINGS_KEY,
        value: JSON.stringify(defaults)
      });
      if (res?.code === 0) {
        setToggles(DEFAULT_TOGGLES);
        setLlmProvider('ollama');
        setLlmEndpoint('http://localhost:11434');
        setLlmModel('');
        setLlmApiKey('');
        setFeedback({
          type: 'success',
          message: t("components.intelligence.SettingsPanel.k34")
        });
      } else {
        throw new Error(res?.message || t("errors.unknown"));
      }
    } catch (e) {
      setFeedback({
        type: 'error',
        message: t("components.intelligence.SettingsPanel.k35", {
          e: e
        })
      });
    }
  }, []);
  const toggleSetting = (key: string) => {
    setToggles(prev => prev.map(t => t.key === key ? {
      ...t,
      enabled: !t.enabled
    } : t));
  };
  return <div className={styles.panel}>
      {/* Function Toggles */}
      <div className={styles.settingsSection}>
        <div className={styles.settingsSectionLabel}>{t("components.intelligence.SettingsPanel.k36")}</div>
        <div className={styles.settingsList}>
          {toggles.map(toggle => <div key={toggle.key} className={styles.settingItem}>
              <div className={styles.settingInfo}>
                <div className={styles.settingName}>{toggle.name}</div>
                <div className={styles.settingDesc}>{toggle.description}</div>
              </div>
              <button className={`${styles.settingToggle} ${toggle.enabled ? styles.settingToggleOn : ''}`} onClick={() => toggleSetting(toggle.key)}>
                <span className={styles.settingToggleKnob} />
              </button>
            </div>)}
        </div>
      </div>

      {/* LLM Configuration */}
      <div className={styles.settingsSection}>
        <div className={styles.settingsSectionLabel}>{t("components.intelligence.SettingsPanel.k37")}</div>
        <div className={styles.sectionCard}>
          <div className={styles.llmForm}>
            <div className={styles.llmField}>
              <label className={styles.llmLabel}>{t("components.intelligence.SettingsPanel.k38")}</label>
              <select className={styles.llmInput} value={llmProvider} onChange={e => handleProviderChange(e.target.value)}>
                {LLM_MODELS.map(m => <option key={m.key} value={m.key}>{m.label}</option>)}
              </select>
            </div>
            <div className={styles.llmField}>
              <label className={styles.llmLabel}>{t("components.intelligence.SettingsPanel.k39")}</label>
              <input className={styles.llmInput} value={llmEndpoint} onChange={e => {
              setLlmEndpoint(e.target.value);
              setTestResult(null);
            }} placeholder={LLM_MODELS.find(m => m.key === llmProvider)?.hint || 'http://localhost:11434'} />
            </div>
            <div className={styles.llmField}>
              <label className={styles.llmLabel}>{t("components.intelligence.SettingsPanel.k40")}</label>
              <div style={{
              display: 'flex',
              gap: 8,
              alignItems: 'center'
            }}>
                {aiModels.length > 0 ? <select className={styles.llmInput} value={llmModel} onChange={e => setLlmModel(e.target.value)} style={{
                flex: 1
              }}>
                    {aiModels.map(m => <option key={m.id} value={m.name}>{m.name} ({m.provider})</option>)}
                  </select> : <input className={styles.llmInput} value={llmModel} onChange={e => setLlmModel(e.target.value)} placeholder="qwen2.5:7b" style={{
                flex: 1
              }} />}
                <button onClick={handleTestConnection} disabled={testing || !llmEndpoint.trim()} style={{
                padding: '6px 14px',
                fontSize: 12,
                borderRadius: 4,
                whiteSpace: 'nowrap',
                border: '1px solid rgba(0,240,255,0.3)',
                background: testing ? 'rgba(0,240,255,0.15)' : 'rgba(0,240,255,0.08)',
                color: '#00F0FF',
                cursor: testing ? 'wait' : 'pointer'
              }}>
                  {testing ? t("components.intelligence.SettingsPanel.k41") : t("components.intelligence.SettingsPanel.k42")}
                </button>
              </div>
            </div>
            {llmProvider !== 'ollama' && <div className={styles.llmField}>
                <label className={styles.llmLabel}>API Key</label>
                <input className={styles.llmInput} type="password" value={llmApiKey} onChange={e => {
              setLlmApiKey(e.target.value);
              setTestResult(null);
            }} placeholder="sk-..." />
              </div>}
            {/* 连接测试结果 */}
            {testResult && <div style={{
            marginTop: 8,
            padding: '8px 12px',
            borderRadius: 6,
            fontSize: 12,
            border: `1px solid ${testResult.success ? 'rgba(0,240,255,0.3)' : 'rgba(255,80,80,0.3)'}`,
            background: testResult.success ? 'rgba(0,240,255,0.06)' : 'rgba(255,80,80,0.06)'
          }}>
                <span style={{
              color: testResult.success ? '#00F0FF' : '#FF5050',
              fontWeight: 600
            }}>
                  {testResult.success ? '✓' : '✗'}
                </span>
                <span style={{
              color: 'rgba(255,255,255,0.7)',
              marginLeft: 8
            }}>{testResult.message}</span>
                {testResult.latency_ms > 0 && <span style={{
              color: 'rgba(255,255,255,0.35)',
              marginLeft: 12
            }}>
                    {testResult.latency_ms < 1000 ? t("components.intelligence.SettingsPanel.k43", {
                latency_ms: testResult.latency_ms
              }) : t("components.intelligence.SettingsPanel.k44", {
                arg0: (testResult.latency_ms / 1000).toFixed(1)
              })}
                  </span>}
              </div>}
          </div>
        </div>
      </div>

      {/* Actions */}
      <div className={styles.settingsActions}>
        <button className={styles.saveBtn} onClick={handleSave}>{t("components.intelligence.SettingsPanel.k45")}</button>
        <button className={styles.resetBtn} onClick={handleReset}>{t("components.intelligence.SettingsPanel.k46")}</button>
      </div>

      {/* Feedback */}
      {feedback && <div className={`${styles.feedback} ${feedback.type === 'success' ? styles.feedbackSuccess : styles.feedbackError}`}>
          {feedback.message}
        </div>}
    </div>;
}