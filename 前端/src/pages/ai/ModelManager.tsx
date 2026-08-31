import { t } from "i18next";
import { useEffect, useState } from 'react';
import type { AIModel, NewModelForm } from './types';
import { API_FORMAT_OPTIONS, PROVIDER_OPTIONS } from './constants';
import { getProviderLabel } from './utils';
import { ai } from '@/lib/ipc';
import { time } from '@/lib/utils';
import { tauriStorage } from '@/lib/tauriStorage';
import styles from '../AI.module.css';

// spec 20260808 M1.②: 后台检测间隔持久化存储键
// 值枚举: off | 5m | 10m | 15m | 30m（字符串，易扩展）
const HEALTH_CHECK_INTERVAL_KEY = 'ai.model_health_check_interval';
// 默认间隔（分钟），0 表示关闭
const DEFAULT_CHECK_INTERVAL = 15;
// 将存储值字符串解析为分钟数（0 表示关闭）
function parseIntervalValue(val: string | null): number {
  if (!val) return DEFAULT_CHECK_INTERVAL;
  if (val === 'off') return 0;
  const m = /^(\d+)m$/.exec(val);
  if (m) return Number(m[1]);
  return DEFAULT_CHECK_INTERVAL;
}
// 将分钟数序列化为存储值字符串
function serializeIntervalValue(minutes: number): string {
  return minutes <= 0 ? 'off' : `${minutes}m`;
}

// spec ai-chat-enhancement Phase 1 Task 5.2: 健康状态徽章渲染
// 5 种状态：online（绿点+延迟）/ offline（红点+离线）/ checking（黄点+检测中+旋转）/ error（红点+错误+tooltip）/ unknown（灰点+未检测）
// isChecking 用于"立即检测"按钮的乐观 UI（点击后立即显示 checking，无需等待后端刷新）
function renderHealthBadge(model: AIModel, isChecking: boolean) {
  const effective = isChecking || model.status === 'checking' ? 'checking' : model.status;
  let dotClass = styles.statusUnknown;
  let label = '未检测';
  let tooltip: string | undefined;
  if (effective === 'online') {
    dotClass = styles.statusOnline;
    // spec 20260808 M1.③: 方案 B — 带单位显示，hover 提示含义（更低更好）
    label = model.latency_ms != null ? `${model.latency_ms} ms` : '在线';
    tooltip = '健康检测往返延迟（更低更好）';
  } else if (effective === 'offline') {
    dotClass = styles.statusOffline;
    label = '离线';
  } else if (effective === 'checking') {
    dotClass = styles.statusChecking;
    label = '检测中...';
  } else if (effective === 'error') {
    dotClass = styles.statusError;
    label = '错误';
    tooltip = '连续 3 次检测失败';
  }
  return <span className={styles.healthBadge} title={tooltip}>
      <span className={`${styles.statusDot} ${dotClass}`}></span>
      <span>{label}</span>
    </span>;
}

interface Props {
  models: AIModel[];
  loadingModels: boolean;
  selectedModel: AIModel | null;
  onSelectModel: (model: AIModel | null) => void;
  onDeleteModel: (id: number) => void;
  errorMsg: string;
  showError: (msg: string) => void;
  showSuccess: (msg: string) => void;
  onModelsChanged: () => void;
}
export default function ModelManager({
  models,
  loadingModels,
  selectedModel,
  onSelectModel,
  errorMsg,
  showError,
  showSuccess,
  onModelsChanged
}: Props) {
  const [showModal, setShowModal] = useState(false);
  const [editingModelId, setEditingModelId] = useState<number | null>(null);
  const [modelForm, setModelForm] = useState<NewModelForm>({
    name: '',
    provider: 'openai',
    api_url: 'https://api.openai.com/v1',
    api_key: '',
    api_format: 'openai-chat',
    model_name: 'gpt-4o-mini',
    display_name: '',
    is_local: false,
    multimodal: false,
    system_prompt: '',
    context_window: 8192,
    temperature: 0.7
  });
  const [modalTab, setModalTab] = useState<'provider' | 'custom'>('provider');
  const [fullUrlMode, setFullUrlMode] = useState(false);
  const [advancedExpanded, setAdvancedExpanded] = useState(false);
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  // spec ai-chat-enhancement Phase 1 Task 5.3/5.4/5.5: 健康检测 UI 状态
  // spec 20260808 M1.②: 从持久化存储读取初始间隔值（修复切页丢失 Bug）
  const [checkInterval, setCheckInterval] = useState<number>(DEFAULT_CHECK_INTERVAL);
  // spec 20260808 M1.①: 是否已关闭后台自动检测（用于状态区提示）
  const [autoCheckOff, setAutoCheckOff] = useState(false);
  // spec 20260808 M1.②: 组件挂载时从 Tauri Storage 读取持久化的间隔值
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const stored = await tauriStorage.getItem(HEALTH_CHECK_INTERVAL_KEY);
      if (cancelled) return;
      const parsed = parseIntervalValue(stored);
      setCheckInterval(parsed);
      setAutoCheckOff(parsed <= 0);
    })();
    return () => { cancelled = true; };
  }, []);
  const [checkingIds, setCheckingIds] = useState<Set<number>>(new Set());
  const resetForm = () => {
    setModelForm({
      name: '',
      provider: 'openai',
      api_url: 'https://api.openai.com/v1',
      api_key: '',
      api_format: 'openai-chat',
      model_name: 'gpt-4o-mini',
      display_name: '',
      is_local: false,
      multimodal: false,
      system_prompt: '',
      context_window: 8192,
      temperature: 0.7
    });
    setEditingModelId(null);
    setModalTab('provider');
    setFullUrlMode(false);
    setAdvancedExpanded(false);
  };
  const handleProviderChange = async (provider: string) => {
    const option = PROVIDER_OPTIONS.find(o => o.value === provider);
    let apiFormat = 'openai-chat';
    if (provider === 'anthropic') apiFormat = 'anthropic-messages';else if (provider === 'ollama') apiFormat = 'ollama';
    setModelForm(prev => ({
      ...prev,
      provider,
      api_url: option?.defaultUrl || '',
      model_name: option?.defaultModel || '',
      api_format: apiFormat,
      is_local: provider === 'ollama'
    }));
    try {
      const res = await (ai as any).getProviderInfo(provider);
      setAvailableModels(res.data?.available_models || []);
    } catch {
      setAvailableModels([]);
    }
  };
  const handleSubmit = async () => {
    const autoName = modelForm.display_name.trim() || modelForm.model_name.trim() || `${PROVIDER_OPTIONS.find(o => o.value === modelForm.provider)?.label || 'Custom'}-${Date.now()}`;
    if (!modelForm.is_local && !modelForm.model_name.trim()) {
      showError(t("ai.ModelManager.k1"));
      return;
    }
    try {
      if (editingModelId) {
        await ai.updateModel({
          id: editingModelId,
          name: autoName,
          provider: modelForm.provider,
          api_url: modelForm.api_url.trim() || null,
          api_key: modelForm.api_key.trim() || null,
          api_format: modelForm.api_format || null,
          model_name: modelForm.model_name.trim() || null,
          display_name: modelForm.display_name.trim() || null,
          multimodal: modelForm.multimodal,
          system_prompt: modelForm.system_prompt.trim() || null,
          context_window: modelForm.context_window > 0 ? modelForm.context_window : null,
          temperature: modelForm.temperature >= 0 ? modelForm.temperature : null
        });
        showSuccess(t("lib.ipcMock.k44"));
      } else {
        await ai.createModel({
          name: autoName,
          provider: modelForm.provider,
          api_url: modelForm.api_url.trim() || null,
          api_key: modelForm.api_key.trim() || null,
          api_format: modelForm.api_format || null,
          model_name: modelForm.model_name.trim() || null,
          display_name: modelForm.display_name.trim() || null,
          is_local: modelForm.is_local,
          multimodal: modelForm.multimodal,
          system_prompt: modelForm.system_prompt.trim() || null,
          context_window: modelForm.context_window > 0 ? modelForm.context_window : null,
          temperature: modelForm.temperature >= 0 ? modelForm.temperature : null
        });
        showSuccess(t("ai.ModelManager.k2"));
      }
      setShowModal(false);
      resetForm();
      onModelsChanged();
    } catch (e) {
      showError(t("ai.ModelManager.k3", {
        arg0: editingModelId ? t("common.updated") : t("common.add"),
        e: e
      }));
    }
  };
  // spec ai-chat-enhancement Phase 1 Task 5.3: 单个模型立即检测
  // 检测期间乐观显示 checking 状态，完成后由 Task 6 事件监听更新（或这里 fallback 调用 onModelsChanged）
  const handleCheckModel = async (model: AIModel, e: React.MouseEvent) => {
    e.stopPropagation();
    setCheckingIds(prev => {
      const next = new Set(prev);
      next.add(model.id);
      return next;
    });
    try {
      await ai.checkModelHealth(model.id);
      onModelsChanged();
    } catch (err) {
      showError(`检测失败: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setCheckingIds(prev => {
        const next = new Set(prev);
        next.delete(model.id);
        return next;
      });
    }
  };
  // spec ai-chat-enhancement Phase 1 Task 5.4: 全部模型批量检测
  const handleCheckAll = async () => {
    if (models.length === 0) return;
    setCheckingIds(new Set(models.map(m => m.id)));
    try {
      await ai.checkAllModelsHealth();
      onModelsChanged();
    } catch (err) {
      showError(`批量检测失败: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setCheckingIds(new Set());
    }
  };
  // spec ai-chat-enhancement Phase 1 Task 5.5: 设置后台检测间隔（分钟）
  // spec 20260808 M1.②: 先写持久化存储，成功后再更新内存态（修复切页丢失 Bug）
  // spec 20260808 M1.①: 支持"关闭"（minutes=0），关闭后取消后台自动检测
  const handleIntervalChange = async (minutes: number) => {
    try {
      // 先持久化到 Tauri Storage
      await tauriStorage.setItem(HEALTH_CHECK_INTERVAL_KEY, serializeIntervalValue(minutes));
      // 持久化成功后再更新内存态
      setCheckInterval(minutes);
      setAutoCheckOff(minutes <= 0);
      // 通知后端更新轮询间隔（minutes=0 表示关闭）
      await ai.setHealthCheckInterval(minutes);
      if (minutes <= 0) {
        showSuccess('已关闭后台自动检测，如需检测请在模型管理页点击【手动检测】');
      } else {
        showSuccess(`检测间隔已设为 ${minutes} 分钟`);
      }
    } catch (err) {
      showError(`设置间隔失败: ${err instanceof Error ? err.message : String(err)}`);
    }
  };
  const isCustomTab = modalTab === 'custom';
  const apiUrlHint = fullUrlMode ? t("ai.ModelManager.k4") : t("ai.ModelManager.k5");
  return <>
      <div className={styles.sidebarList}>
        <div className={styles.healthToolbar}>
          <button className={styles.checkAllButton} onClick={handleCheckAll} disabled={checkingIds.size > 0 || models.length === 0}>
            ⟳ 全部检测
          </button>
          <select className={styles.intervalSelect} value={checkInterval} onChange={e => handleIntervalChange(Number(e.target.value))} title="后台检测间隔">
            <option value={0}>关闭</option>
            <option value={5}>5 分钟</option>
            <option value={10}>10 分钟</option>
            <option value={15}>15 分钟</option>
            <option value={30}>30 分钟</option>
          </select>
        </div>
        {autoCheckOff && <div className={styles.autoCheckOffHint}>
            后台自动检测已关闭，可点击【手动检测】按钮触发
          </div>}
        <button className={styles.addBtn} onClick={() => {
        setEditingModelId(null);
        resetForm();
        setShowModal(true);
      }}>
          {t("ai.ModelManager.k6")}
        </button>
        {loadingModels ? <div className={styles.loadingHint}>{t("common.loading")}</div> : models.length === 0 ? <div className={styles.emptyHint}>{t("ai.ModelManager.k7")}</div> : models.map(model => <div key={model.id} className={`${styles.sidebarItem} ${selectedModel?.id === model.id ? styles.sidebarItemActive : ''}`} onClick={() => onSelectModel(model)}>
              <div className={styles.sidebarItemHeader}>
                <span className={styles.sidebarItemName}>{model.name}</span>
                <span className={`${styles.providerTag} ${styles[`provider${model.provider}` as keyof typeof styles] || ''}`}>
                  {getProviderLabel(model.provider)}
                </span>
                <button className={styles.checkButton} onClick={e => handleCheckModel(model, e)} title="立即检测" disabled={checkingIds.has(model.id)}>⟳</button>
              </div>
              <div className={styles.sidebarItemMeta}>
                <span>{model.model_type === 'local' ? t("ai.ConversationModal.k11") : 'API'}</span>
                {renderHealthBadge(model, checkingIds.has(model.id))}
              </div>
            </div>)}
      </div>

      {showModal && <div className={styles.modalOverlay} onClick={() => {
      setShowModal(false);
      resetForm();
    }}>
          <div className={`${styles.modal} ${styles.modelConfigModal}`} onClick={e => e.stopPropagation()}>
            <div className={styles.modalHeader}>
              <h3>{editingModelId ? t("ai.ModelManager.k8") : t("ai.ModelManager.k9")}</h3>
              <button className={styles.modalClose} onClick={() => {
            setShowModal(false);
            resetForm();
          }}>✕</button>
            </div>

            <div className={styles.modalTabs}>
              <button className={`${styles.tabBtn} ${modalTab === 'provider' ? styles.tabBtnActive : ''}`} onClick={() => setModalTab('provider')}>{t("ai.ModelManager.k10")}</button>
              <button className={`${styles.tabBtn} ${modalTab === 'custom' ? styles.tabBtnActive : ''}`} onClick={() => setModalTab('custom')}>{t("ai.ModelManager.k11")}</button>
            </div>

            <div className={styles.modalBody}>
              {!isCustomTab && <div className={styles.formGroupC}>
                  <label className={styles.formLabel}>{t("ai.ModelManager.k12")}</label>
                  <select className={styles.formInput} value={modelForm.provider} onChange={e => handleProviderChange(e.target.value)}>
                    {PROVIDER_OPTIONS.map(opt => <option key={opt.value} value={opt.value}>{opt.label}</option>)}
                  </select>
                </div>}

              <div className={styles.formGroupC}>
                <label className={styles.formLabel}><span className={styles.requiredMark}>*</span> {t("ai.ModelManager.k13")}</label>
                <select className={styles.formInput} value={modelForm.api_format} onChange={e => setModelForm({
              ...modelForm,
              api_format: e.target.value
            })}>
                  {API_FORMAT_OPTIONS.map(opt => <option key={opt.value} value={opt.value}>{opt.label}</option>)}
                </select>
              </div>

              <div className={styles.formGroupC}>
                <label className={styles.formLabel}><span className={styles.requiredMark}>*</span> {t("ai.ModelManager.k14")}</label>
                <div className={styles.urlRow}>
                  <input type="text" className={`${styles.formInput} ${styles.monoInput} ${styles.urlInput}`} placeholder="e.g. https://api.openai.com/v1" value={modelForm.api_url} onChange={e => setModelForm({
                ...modelForm,
                api_url: e.target.value
              })} />
                  <div className={styles.toggleRow}>
                    <span className={styles.toggleLabel}>{t("ai.ModelManager.k15")}</span>
                    <button className={`${styles.toggleSwitch} ${fullUrlMode ? styles.toggleOn : ''}`} onClick={() => setFullUrlMode(!fullUrlMode)} type="button">
                      <span className={styles.toggleKnob} />
                    </button>
                  </div>
                </div>
                <div className={styles.infoBox}>
                  <span className={styles.infoIcon}>ⓘ</span>
                  <span className={styles.infoText}>{apiUrlHint}</span>
                </div>
              </div>

              <div className={styles.formGroupC}>
                <label className={styles.formLabel}><span className={styles.requiredMark}>*</span> {t("ai.ModelManager.k16")}</label>
                <div className={styles.urlRow}>
                  <div style={{
                flex: 1
              }}>
                    {availableModels.length > 0 && !isCustomTab && !modelForm.is_local ? <select className={`${styles.formInput} ${styles.monoInput}`} value={modelForm.model_name} onChange={e => setModelForm({
                  ...modelForm,
                  model_name: e.target.value
                })}>
                        <option value="">{t("ai.ModelManager.k17")}</option>
                        {availableModels.map(m => <option key={m} value={m}>{m}</option>)}
                        <option value="__custom__">{t("ai.ModelManager.k18")}</option>
                      </select> : <input type="text" className={`${styles.formInput} ${styles.monoInput}`} placeholder={t("ai.ModelManager.k19")} value={modelForm.model_name === '__custom__' ? '' : modelForm.model_name} onChange={e => setModelForm({
                  ...modelForm,
                  model_name: e.target.value
                })} />}
                  </div>
                  <div className={styles.toggleRow}>
                    <span className={styles.toggleLabel}>{t("ai.ModelManager.k20")}</span>
                    <button className={`${styles.toggleSwitch} ${modelForm.multimodal ? styles.toggleOn : ''}`} onClick={() => setModelForm({
                  ...modelForm,
                  multimodal: !modelForm.multimodal
                })} type="button">
                      <span className={styles.toggleKnob} />
                    </button>
                  </div>
                </div>
              </div>

              <div className={styles.formGroupC}>
                <label className={styles.formLabel}><span className={styles.requiredMark}>*</span> {t("ai.ModelManager.k21")}</label>
                <input type="password" className={`${styles.formInput} ${styles.monoInput}`} placeholder={editingModelId ? t("ai.ModelManager.k22") : t("ai.ModelManager.k23")} value={modelForm.api_key} onChange={e => setModelForm({
              ...modelForm,
              api_key: e.target.value
            })} />
              </div>

              <div className={styles.advancedSection}>
                <button className={styles.advancedHeader} onClick={() => setAdvancedExpanded(!advancedExpanded)} type="button">
                  <span className={styles.advancedArrow}>{advancedExpanded ? '▾' : '▸'}</span>
                  <span>{t("ai.ModelManager.k24")}</span>
                </button>
                {advancedExpanded && <div className={styles.advancedBody}>
                    <p className={styles.advancedDesc}>{t("ai.ModelManager.k25")}</p>
                    <div className={styles.formGroupC}>
                      <label className={styles.formLabel}>{t("ai.ModelManager.k26")}</label>
                      <input type="text" className={styles.formInput} placeholder={t("ai.ModelManager.k27")} value={modelForm.display_name} onChange={e => setModelForm({
                  ...modelForm,
                  display_name: e.target.value
                })} />
                    </div>
                    <div className={styles.formGroupC}>
                      <label className={styles.formLabel}>{t("ai.AgentManager.k15")}</label>
                      <textarea className={`${styles.formInput} ${styles.textareaInput}`} placeholder={t("ai.ModelManager.k28")} value={modelForm.system_prompt} onChange={e => setModelForm({
                  ...modelForm,
                  system_prompt: e.target.value
                })} rows={3} />
                    </div>
                    <div className={styles.row2col}>
                      <div className={styles.formGroupC}>
                        <label className={styles.formLabel}>{t("ai.ModelManager.k29")}</label>
                        <input type="number" className={styles.formInput} min={1024} max={1000000} step={1024} value={modelForm.context_window} onChange={e => setModelForm({
                    ...modelForm,
                    context_window: Number(e.target.value)
                  })} />
                      </div>
                      <div className={styles.formGroupC}>
                        <label className={styles.formLabel}>Temperature</label>
                        <input type="number" className={styles.formInput} min={0} max={2} step={0.1} value={modelForm.temperature} onChange={e => setModelForm({
                    ...modelForm,
                    temperature: Number(e.target.value)
                  })} />
                      </div>
                    </div>
                  </div>}
              </div>

              {errorMsg && <div className={styles.errorMsg}>{errorMsg}</div>}
            </div>

            <div className={styles.modalFooter}>
              <button className={styles.cancelBtn} onClick={() => {
            setShowModal(false);
            resetForm();
          }}>{t("common.cancel")}</button>
              <button className={styles.primaryBtn} onClick={handleSubmit}>
                {editingModelId ? t("ai.AgentManager.k20") : t("ai.ModelManager.k9")}
              </button>
            </div>
          </div>
        </div>}
    </>;
}
export function ModelDetail({
  model,
  onEdit,
  onDelete
}: {
  model: AIModel;
  onEdit: () => void;
  onDelete: () => void;
}) {
  return <div className={styles.detailPanel}>
      <div className={styles.detailHeader}>
        <h3 className={styles.detailTitle}>{model.name}</h3>
        <div className={styles.detailActions}>
          <button className={styles.editBtn} onClick={onEdit}>{t("common.edit")}</button>
          <button className={styles.deleteBtn} onClick={onDelete}>{t("common.delete")}</button>
        </div>
      </div>
      <div className={styles.detailGrid}>
        <div className={styles.detailItem}>
          <span className={styles.detailLabel}>{t("ai.ModelManager.k12")}</span>
          <span className={styles.detailValue}>{getProviderLabel(model.provider)}</span>
        </div>
        <div className={styles.detailItem}>
          <span className={styles.detailLabel}>{t("ai.ModelManager.k30")}</span>
          <span className={styles.detailValue}>{model.model_type === 'local' ? t("ai.ModelManager.k31") : t("ai.ModelManager.k32")}</span>
        </div>
        <div className={styles.detailItem}>
          <span className={styles.detailLabel}>{t("components.intelligence.SettingsPanel.k39")}</span>
          <span className={`${styles.detailValue} ${styles.mono}`}>{model.api_endpoint || t("ai.ModelManager.k33")}</span>
        </div>
        <div className={styles.detailItem}>
          <span className={styles.detailLabel}>{t("ai.ModelManager.k34")}</span>
          <span className={`${styles.detailValue} ${styles.mono}`}>{model.model_name || t("ai.ModelManager.k35")}</span>
        </div>
        <div className={styles.detailItem}>
          <span className={styles.detailLabel}>API Key</span>
          <span className={styles.detailValue}>
            {model.has_api_key ? <span className={styles.hasKey}>{t("ai.ModelManager.k36")}</span> : <span className={styles.noKey}>{t("ai.ModelManager.k37")}</span>}
          </span>
        </div>
        <div className={styles.detailItem}>
          <span className={styles.detailLabel}>{t("common.status")}</span>
          <span className={styles.detailValue}>
            {renderHealthBadge(model, false)}
          </span>
        </div>
        <div className={styles.detailItem}>
          <span className={styles.detailLabel}>{t("ai.ModelManager.k40")}</span>
          <span className={styles.detailValue}>{time.formatUtcToLocal(model.created_at)}</span>
        </div>
        <div className={styles.detailItem}>
          <span className={styles.detailLabel}>{t("ai.ModelManager.k41")}</span>
          <span className={styles.detailValue}>{time.formatUtcToLocal(model.updated_at)}</span>
        </div>
      </div>
    </div>;
}