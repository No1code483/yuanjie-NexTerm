/**
 * Yuan Code v3.2 Task 3.4.1 — ModelRoutingConfig 组件
 *
 * 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.1（Phase 6 多模型与协作）
 *       + 项目核心设计意图 §三（Yuan Code 编程 AI 必须走云端 API 模型）
 *
 * 功能：
 * 1. 列出支持的 task_type（编程/分析/审查/文档/通用）
 * 2. 为每个 task_type 配置云端 provider + model_name
 * 3. 显示当前路由解析结果（rule | default）
 * 4. 启用/禁用/删除规则
 *
 * 强制约束：provider 选项仅显示云端 API 白名单（openai/anthropic/deepseek/...），
 *          本地底层智能模型不会出现在选项中（后端 model_routing_service 双重守卫强制）。
 */
import { t } from 'i18next';
import { useState, useCallback, useEffect } from 'react';
import { yuanCode } from '@/lib/ipc';
import type {
  ModelRoutingRule,
  UpsertRoutingRuleRequest,
  TaskTypeInfo,
  ResolvedRouteInfo,
} from '@/lib/ipc/yuan-code';
import styles from '../YuanCode.module.css';

interface ModelRoutingConfigProps {
  defaultExpanded?: boolean;
}

interface EditForm {
  task_type: string;
  provider: string;
  model_name: string;
  temperature: string;
  max_tokens: string;
  is_enabled: boolean;
}

const EMPTY_FORM: EditForm = {
  task_type: 'programming',
  provider: 'openai',
  model_name: 'gpt-4o',
  temperature: '0.2',
  max_tokens: '',
  is_enabled: true,
};

export default function ModelRoutingConfig({
  defaultExpanded = true,
}: ModelRoutingConfigProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);
  const [rules, setRules] = useState<ModelRoutingRule[]>([]);
  const [taskTypes, setTaskTypes] = useState<TaskTypeInfo[]>([]);
  const [providers, setProviders] = useState<{ provider: string; display_name: string; available_models: string[] }[]>([]);
  const [resolved, setResolved] = useState<Record<string, ResolvedRouteInfo>>({});
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState<EditForm>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);

  const loadRules = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await yuanCode.modelRoutingList();
      if (res.code === 0 && res.data) {
        setRules(res.data);
        // 同时加载每个 task_type 的解析结果
        const resolveMap: Record<string, ResolvedRouteInfo> = {};
        const allTaskTypes = ['programming', 'analysis', 'review', 'documentation', 'general'];
        await Promise.all(
          allTaskTypes.map(async (tt) => {
            const r = await yuanCode.modelRoutingResolve(tt);
            if (r.code === 0 && r.data) {
              resolveMap[tt] = r.data;
            }
          })
        );
        setResolved(resolveMap);
      } else if (res.code !== 0) {
        setError(res.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const loadMeta = useCallback(async () => {
    try {
      const [typesRes, providersRes] = await Promise.all([
        yuanCode.modelRoutingTaskTypes(),
        yuanCode.cloudApiProviders(),
      ]);
      if (typesRes.code === 0 && typesRes.data) setTaskTypes(typesRes.data);
      if (providersRes.code === 0 && providersRes.data) {
        setProviders(
          providersRes.data.map((p) => ({
            provider: p.provider,
            display_name: p.display_name,
            available_models: p.available_models,
          }))
        );
      }
    } catch {
      // 静默处理
    }
  }, []);

  useEffect(() => {
    loadMeta();
  }, [loadMeta]);

  useEffect(() => {
    if (expanded) loadRules();
  }, [expanded, loadRules]);

  const handleUpsert = useCallback(async () => {
    if (!form.task_type || !form.provider || !form.model_name.trim()) {
      setError(t('common.invalidInput'));
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const req: UpsertRoutingRuleRequest = {
        task_type: form.task_type,
        provider: form.provider,
        model_name: form.model_name.trim(),
        temperature: form.temperature.trim() ? Number(form.temperature) : undefined,
        max_tokens: form.max_tokens.trim() ? Number(form.max_tokens) : undefined,
        is_enabled: form.is_enabled,
      };
      const res = await yuanCode.modelRoutingUpsert(req);
      if (res.code === 0) {
        setShowForm(false);
        setForm(EMPTY_FORM);
        await loadRules();
      } else {
        setError(res.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  }, [form, loadRules]);

  const handleToggle = useCallback(async (id: number, isEnabled: boolean) => {
    try {
      await yuanCode.modelRoutingSetEnabled(id, !isEnabled);
      await loadRules();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [loadRules]);

  const handleDelete = useCallback(async (id: number) => {
    if (!confirm(t('common.remove') + '?')) return;
    try {
      await yuanCode.modelRoutingDelete(id);
      await loadRules();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [loadRules]);

  const handleEditRule = useCallback((rule: ModelRoutingRule) => {
    setForm({
      task_type: rule.task_type,
      provider: rule.provider,
      model_name: rule.model_name,
      temperature: rule.temperature != null ? String(rule.temperature) : '',
      max_tokens: rule.max_tokens != null ? String(rule.max_tokens) : '',
      is_enabled: rule.is_enabled,
    });
    setShowForm(true);
  }, []);

  const onProviderChange = useCallback((provider: string) => {
    const providerInfo = providers.find((p) => p.provider === provider);
    setForm((prev) => ({
      ...prev,
      provider,
      model_name: providerInfo?.available_models[0] || '',
    }));
  }, [providers]);

  return (
    <div className={styles.settingsSection}>
      <div
        style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', cursor: 'pointer' }}
        onClick={() => setExpanded(!expanded)}
      >
        <h3 className={styles.settingsSectionTitle}>
          {t('yuan-code.ModelRoutingConfig.k1') || 'Model Routing (Task → Cloud API)'}
        </h3>
        <span style={{ fontSize: 11, color: 'var(--nt-text-muted)' }}>
          {expanded ? '▾' : '▸'}
        </span>
      </div>

      <div className={styles.settingRow}>
        <span className={styles.settingDesc}>
          {t('yuan-code.ModelRoutingConfig.k2') ||
            '按任务类型（编程/分析/审查/文档）路由到不同云端 API 模型；编程任务强制走云端 API（项目核心设计意图 §三）'}
        </span>
      </div>

      {expanded && (
        <>
          {/* 强制云端约束提示 */}
          <div style={{
            padding: '6px 10px',
            marginBottom: 10,
            background: 'rgba(255,180,0,0.06)',
            border: '1px solid rgba(255,180,0,0.18)',
            borderRadius: 4,
            fontSize: 11,
            color: '#FFB86C',
          }}>
            ⚠ {t('yuan-code.ModelRoutingConfig.k3') ||
              '编程任务路由规则只能选云端 API provider，本地底层智能模型（如 ollama）被后端双重守卫拒绝'}
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

          {/* 当前解析结果（按 task_type 展示） */}
          {loading ? (
            <div style={{ padding: 16, textAlign: 'center', color: 'var(--nt-text-muted)', fontSize: 12 }}>
              {t('components.AudioEditor.k6')}...
            </div>
          ) : (
            <div style={{ maxHeight: 320, overflowY: 'auto' }}>
              {taskTypes.map((tt) => {
                const rule = rules.find((r) => r.task_type === tt.task_type);
                const r = resolved[tt.task_type];
                return (
                  <div
                    key={tt.task_type}
                    style={{
                      padding: '8px 10px',
                      marginBottom: 6,
                      border: '1px solid rgba(0,240,255,0.1)',
                      borderRadius: 4,
                      background: 'rgba(0,240,255,0.02)',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                      <div style={{ flex: 1 }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                          <span style={{
                            fontFamily: 'var(--nt-font-mono)',
                            fontSize: 12,
                            color: 'var(--nt-primary)',
                            fontWeight: 600,
                          }}>
                            {tt.display_name}
                          </span>
                          {rule ? (
                            <span style={{
                              fontSize: 9,
                              padding: '1px 5px',
                              borderRadius: 3,
                              background: 'rgba(0,255,136,0.1)',
                              color: '#00FF88',
                            }}>
                              {t('yuan-code.ModelRoutingConfig.k4') || '已配置'}
                            </span>
                          ) : (
                            <span style={{
                              fontSize: 9,
                              padding: '1px 5px',
                              borderRadius: 3,
                              background: 'rgba(255,180,0,0.12)',
                              color: '#FFB400',
                            }}>
                              {t('yuan-code.ModelRoutingConfig.k5') || '默认'}
                            </span>
                          )}
                          {rule && !rule.is_enabled && (
                            <span style={{
                              fontSize: 9,
                              padding: '1px 5px',
                              borderRadius: 3,
                              background: 'rgba(255,0,110,0.12)',
                              color: '#FF006E',
                            }}>
                              {t('yuan-code.ModelRoutingConfig.k6') || '已禁用'}
                            </span>
                          )}
                        </div>
                        <div style={{
                          marginTop: 4,
                          fontSize: 10,
                          color: 'var(--nt-text-muted)',
                        }}>
                          {tt.description}
                        </div>
                        {r && (
                          <div style={{ marginTop: 4 }}>
                            <code style={{
                              fontFamily: 'var(--nt-font-mono)',
                              fontSize: 10,
                              color: r.source === 'rule' ? 'var(--nt-primary)' : 'var(--nt-text-muted)',
                            }}>
                              {r.provider} / {r.model_name}
                              {r.temperature != null && ` · T=${r.temperature}`}
                              {r.max_tokens != null && ` · max=${r.max_tokens}`}
                            </code>
                          </div>
                        )}
                      </div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                        {rule && (
                          <>
                            <div
                              className={`${styles.toggleSwitch} ${rule.is_enabled ? styles.toggleSwitchOn : ''}`}
                              onClick={() => handleToggle(rule.id, rule.is_enabled)}
                              title={rule.is_enabled ? t('common.disable') : t('common.enable')}
                            >
                              <div className={styles.toggleKnob} />
                            </div>
                            <button
                              onClick={() => handleEditRule(rule)}
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
                              {t('common.edit')}
                            </button>
                            <button
                              onClick={() => handleDelete(rule.id)}
                              style={{
                                background: 'none',
                                border: 'none',
                                color: 'var(--nt-text-muted)',
                                cursor: 'pointer',
                                fontSize: 14,
                                padding: 0,
                                lineHeight: 1,
                              }}
                              title={t('common.remove')}
                            >
                              ✕
                            </button>
                          </>
                        )}
                        {!rule && (
                          <button
                            onClick={() => {
                              setForm({
                                ...EMPTY_FORM,
                                task_type: tt.task_type,
                                provider: tt.default_provider,
                                model_name: tt.default_model,
                                temperature: String(tt.default_temperature),
                              });
                              setShowForm(true);
                            }}
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
                            + {t('common.configure') || 'Configure'}
                          </button>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {/* 新增/编辑表单 */}
          {showForm && (
            <div style={{
              marginTop: 10,
              padding: 12,
              border: '1px solid rgba(0,240,255,0.15)',
              borderRadius: 4,
              background: 'rgba(0,240,255,0.02)',
            }}>
              <div className={styles.formGroup}>
                <label className={styles.formLabel}>{t('yuan-code.ModelRoutingConfig.k7') || 'Task Type'}</label>
                <select
                  className={styles.formSelect}
                  value={form.task_type}
                  onChange={(e) => setForm((prev) => ({ ...prev, task_type: e.target.value }))}
                  style={{ padding: '4px 8px', fontSize: 11, height: 28 }}
                >
                  {taskTypes.map((tt) => (
                    <option key={tt.task_type} value={tt.task_type}>
                      {tt.display_name}
                    </option>
                  ))}
                </select>
              </div>

              <div className={styles.formGroup} style={{ marginTop: 8 }}>
                <label className={styles.formLabel}>
                  {t('yuan-code.ModelRoutingConfig.k8') || 'Cloud API Provider'}
                </label>
                <select
                  className={styles.formSelect}
                  value={form.provider}
                  onChange={(e) => onProviderChange(e.target.value)}
                  style={{ padding: '4px 8px', fontSize: 11, height: 28 }}
                >
                  {providers.map((p) => (
                    <option key={p.provider} value={p.provider}>
                      {p.display_name}
                    </option>
                  ))}
                </select>
              </div>

              <div className={styles.formGroup} style={{ marginTop: 8 }}>
                <label className={styles.formLabel}>{t('yuan-code.ModelRoutingConfig.k9') || 'Model Name'}</label>
                <select
                  className={styles.formSelect}
                  value={form.model_name}
                  onChange={(e) => setForm((prev) => ({ ...prev, model_name: e.target.value }))}
                  style={{ padding: '4px 8px', fontSize: 11, height: 28 }}
                >
                  {providers.find((p) => p.provider === form.provider)?.available_models.map((m) => (
                    <option key={m} value={m}>{m}</option>
                  )) || <option value={form.model_name}>{form.model_name}</option>}
                </select>
              </div>

              <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
                <div className={styles.formGroup} style={{ flex: 1 }}>
                  <label className={styles.formLabel}>{t('yuan-code.ModelRoutingConfig.k10') || 'Temperature'}</label>
                  <input
                    type="number"
                    step="0.1"
                    min="0"
                    max="2"
                    className={styles.formInput}
                    value={form.temperature}
                    onChange={(e) => setForm((prev) => ({ ...prev, temperature: e.target.value }))}
                    style={{ padding: '4px 8px', fontSize: 11 }}
                  />
                </div>
                <div className={styles.formGroup} style={{ flex: 1 }}>
                  <label className={styles.formLabel}>{t('yuan-code.ModelRoutingConfig.k11') || 'Max Tokens'}</label>
                  <input
                    type="number"
                    min="1"
                    className={styles.formInput}
                    value={form.max_tokens}
                    onChange={(e) => setForm((prev) => ({ ...prev, max_tokens: e.target.value }))}
                    style={{ padding: '4px 8px', fontSize: 11 }}
                  />
                </div>
              </div>

              <div className={styles.settingRow} style={{ marginTop: 8 }}>
                <span className={styles.settingLabel}>{t('yuan-code.ModelRoutingConfig.k12') || '启用'}</span>
                <div
                  className={`${styles.toggleSwitch} ${form.is_enabled ? styles.toggleSwitchOn : ''}`}
                  onClick={() => setForm((prev) => ({ ...prev, is_enabled: !prev.is_enabled }))}
                >
                  <div className={styles.toggleKnob} />
                </div>
              </div>

              <div style={{ display: 'flex', gap: 6, marginTop: 10, justifyContent: 'flex-end' }}>
                <button
                  className={styles.btnDanger}
                  onClick={() => {
                    setShowForm(false);
                    setForm(EMPTY_FORM);
                  }}
                  style={{ padding: '3px 10px', fontSize: 11 }}
                >
                  {t('common.cancel')}
                </button>
                <button
                  className={styles.btnPrimary}
                  onClick={handleUpsert}
                  disabled={saving || !form.model_name.trim()}
                  style={{ padding: '3px 10px', fontSize: 11 }}
                >
                  {saving ? t('components.AudioEditor.k6') : t('common.save')}
                </button>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
