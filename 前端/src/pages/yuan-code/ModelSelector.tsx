/**
 * Yuan Code v3.1 Task 3.5.2 — ModelSelector 组件
 *
 * 规范：功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.5.2
 *       + 项目核心设计意图 §三（编程 AI 必须走云端 API 模型）
 *
 * 功能：
 * 1. 列出已配置的云端 API Key（cloud_api_list）
 * 2. 新增/更新 API Key（cloud_api_upsert，AES-GCM 加密落盘）
 * 3. 启用/禁用/删除 API Key（cloud_api_set_enabled / cloud_api_delete）
 * 4. 测试连通性（cloud_api_test_connection）
 * 5. 列出支持的云端 provider 白名单（cloud_api_providers）
 *
 * 强制约束：仅允许云端 provider（OpenAI/Claude/GPT-4o/DeepSeek/Moonshot/Zhipu/Qwen/Azure/custom），
 *          本地底层智能模型不会出现在选项中（由后端 CLOUD_API_PROVIDERS 白名单强制）。
 */
import { t } from "i18next";
import { useState, useCallback, useEffect } from 'react';
import { yuanCode } from '@/lib/ipc';
import type {
  CloudApiKeyInfo,
  CloudProviderInfo,
  CloudConnectionTestResult,
  UpsertCloudApiKeyRequest,
} from '@/lib/ipc/yuan-code';
import styles from '../YuanCode.module.css';

interface ModelSelectorProps {
  /** 选中 provider 后通知父组件（用于路由层选模型） */
  onProviderSelected?: (provider: string, model: string) => void;
  /** 默认展开/收起 */
  defaultExpanded?: boolean;
}

interface UpsertForm {
  provider: string;
  display_name: string;
  api_key_plain: string;
  api_url: string;
  is_enabled: boolean;
}

const EMPTY_FORM: UpsertForm = {
  provider: 'openai',
  display_name: '',
  api_key_plain: '',
  api_url: '',
  is_enabled: true,
};

export default function ModelSelector({
  onProviderSelected,
  defaultExpanded = true,
}: ModelSelectorProps) {
  const [expanded, setExpanded] = useState(defaultExpanded);
  const [keys, setKeys] = useState<CloudApiKeyInfo[]>([]);
  const [providers, setProviders] = useState<CloudProviderInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState<UpsertForm>(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [testingId, setTestingId] = useState<number | null>(null);
  const [testResults, setTestResults] = useState<Record<number, CloudConnectionTestResult | null>>({});
  const [selectedKey, setSelectedKey] = useState<number | null>(null);

  const loadKeys = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await yuanCode.cloudApiList();
      if (res.code === 0 && res.data) {
        setKeys(res.data);
        // 自动选中第一个已启用的 provider
        const firstEnabled = res.data.find(k => k.is_enabled && k.has_api_key);
        if (firstEnabled && selectedKey === null) {
          setSelectedKey(firstEnabled.id);
          const providerInfo = providers.find(p => p.provider === firstEnabled.provider);
          if (providerInfo && onProviderSelected) {
            onProviderSelected(firstEnabled.provider, providerInfo.default_model || '');
          }
        }
      } else if (res.code !== 0) {
        setError(res.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [providers, selectedKey, onProviderSelected]);

  const loadProviders = useCallback(async () => {
    try {
      const res = await yuanCode.cloudApiProviders();
      if (res.code === 0 && res.data) {
        setProviders(res.data);
      }
    } catch {
      // 静默处理
    }
  }, []);

  useEffect(() => {
    loadProviders();
  }, [loadProviders]);

  useEffect(() => {
    if (expanded) {
      loadKeys();
    }
  }, [expanded, loadKeys]);

  const handleUpsert = useCallback(async () => {
    if (!form.provider || !form.api_key_plain.trim()) {
      setError(t('common.invalidInput'));
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const req: UpsertCloudApiKeyRequest = {
        provider: form.provider,
        display_name: form.display_name.trim() || undefined,
        api_key_plain: form.api_key_plain.trim(),
        api_url: form.api_url.trim() || undefined,
        is_enabled: form.is_enabled,
      };
      const res = await yuanCode.cloudApiUpsert(req);
      if (res.code === 0) {
        setShowForm(false);
        setForm(EMPTY_FORM);
        await loadKeys();
      } else {
        setError(res.message);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  }, [form, loadKeys]);

  const handleToggle = useCallback(async (id: number, isEnabled: boolean) => {
    try {
      await yuanCode.cloudApiSetEnabled(id, !isEnabled);
      await loadKeys();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [loadKeys]);

  const handleDelete = useCallback(async (id: number) => {
    if (!confirm(t('yuan-code.ModelSelector.k1'))) return;
    try {
      await yuanCode.cloudApiDelete(id);
      if (selectedKey === id) setSelectedKey(null);
      await loadKeys();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [loadKeys, selectedKey]);

  const handleTest = useCallback(async (keyInfo: CloudApiKeyInfo) => {
    setTestingId(keyInfo.id);
    setError(null);
    try {
      const providerInfo = providers.find(p => p.provider === keyInfo.provider);
      const modelName = providerInfo?.default_model || 'gpt-4o-mini';
      const res = await yuanCode.cloudApiTestConnection(keyInfo.provider, modelName);
      if (res.code === 0 && res.data) {
        setTestResults(prev => ({ ...prev, [keyInfo.id]: res.data! }));
      } else {
        setTestResults(prev => ({
          ...prev,
          [keyInfo.id]: {
            success: false,
            provider: keyInfo.provider,
            model_used: modelName,
            response_snippet: '',
            latency_ms: 0,
            error: res.message,
          },
        }));
      }
    } catch (e) {
      const errMsg = e instanceof Error ? e.message : String(e);
      setTestResults(prev => ({
        ...prev,
        [keyInfo.id]: {
          success: false,
          provider: keyInfo.provider,
          model_used: '',
          response_snippet: '',
          latency_ms: 0,
          error: errMsg,
        },
      }));
    } finally {
      setTestingId(null);
    }
  }, [providers]);

  const handleSelectKey = useCallback((keyInfo: CloudApiKeyInfo) => {
    setSelectedKey(keyInfo.id);
    const providerInfo = providers.find(p => p.provider === keyInfo.provider);
    if (onProviderSelected) {
      onProviderSelected(keyInfo.provider, providerInfo?.default_model || '');
    }
  }, [providers, onProviderSelected]);

  const selectedKeyInfo = keys.find(k => k.id === selectedKey);

  return (
    <div className={styles.settingsSection}>
      <div
        style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', cursor: 'pointer' }}
        onClick={() => setExpanded(!expanded)}
      >
        <h3 className={styles.settingsSectionTitle}>
          {t('yuan-code.ModelSelector.k2')}
        </h3>
        <span style={{ fontSize: 11, color: 'var(--nt-text-muted)' }}>
          {expanded ? '▾' : '▸'}
        </span>
      </div>

      <div className={styles.settingRow}>
        <span className={styles.settingDesc}>
          {t('yuan-code.ModelSelector.k3')}
        </span>
      </div>

      {expanded && (
        <>
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
            ⚠ {t('yuan-code.ModelSelector.k4')}
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

          {loading ? (
            <div style={{ padding: 16, textAlign: 'center', color: 'var(--nt-text-muted)', fontSize: 12 }}>
              {t('components.AudioEditor.k6')}...
            </div>
          ) : keys.length === 0 ? (
            <div style={{ padding: 16, textAlign: 'center', color: 'var(--nt-text-muted)', fontSize: 12 }}>
              {t('yuan-code.ModelSelector.k5')}
            </div>
          ) : (
            <div style={{ maxHeight: 280, overflowY: 'auto' }}>
              {keys.map(k => {
                const providerInfo = providers.find(p => p.provider === k.provider);
                return (
                  <div
                    key={k.id}
                    style={{
                      padding: '8px 10px',
                      marginBottom: 6,
                      border: `1px solid ${selectedKey === k.id ? 'rgba(0,240,255,0.35)' : 'rgba(0,240,255,0.1)'}`,
                      borderRadius: 4,
                      background: selectedKey === k.id ? 'rgba(0,240,255,0.04)' : 'rgba(0,240,255,0.02)',
                      cursor: 'pointer',
                    }}
                    onClick={() => handleSelectKey(k)}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1 }}>
                        <span style={{
                          fontFamily: 'var(--nt-font-mono)',
                          fontSize: 12,
                          color: 'var(--nt-primary)',
                        }}>
                          {k.display_name || k.provider}
                        </span>
                        <span style={{
                          fontSize: 9,
                          padding: '1px 5px',
                          borderRadius: 3,
                          background: 'rgba(0,240,255,0.1)',
                          color: 'var(--nt-primary)',
                        }}>
                          {k.provider}
                        </span>
                        {k.has_api_key ? (
                          <span style={{
                            fontSize: 9,
                            padding: '1px 5px',
                            borderRadius: 3,
                            background: 'rgba(0,255,136,0.1)',
                            color: '#00FF88',
                          }}>
                            {t('yuan-code.ModelSelector.k6')}
                          </span>
                        ) : (
                          <span style={{
                            fontSize: 9,
                            padding: '1px 5px',
                            borderRadius: 3,
                            background: 'rgba(255,180,0,0.12)',
                            color: '#FFB400',
                          }}>
                            {t('yuan-code.ModelSelector.k7')}
                          </span>
                        )}
                        {!k.is_enabled && (
                          <span style={{
                            fontSize: 9,
                            padding: '1px 5px',
                            borderRadius: 3,
                            background: 'rgba(255,0,110,0.12)',
                            color: '#FF006E',
                          }}>
                            {t('yuan-code.ModelSelector.k8')}
                          </span>
                        )}
                      </div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }} onClick={(e) => e.stopPropagation()}>
                        <div
                          className={`${styles.toggleSwitch} ${k.is_enabled ? styles.toggleSwitchOn : ''}`}
                          onClick={() => handleToggle(k.id, k.is_enabled)}
                          title={k.is_enabled ? t('common.disable') : t('common.enable')}
                        >
                          <div className={styles.toggleKnob} />
                        </div>
                        <button
                          onClick={() => handleTest(k)}
                          disabled={testingId === k.id}
                          style={{
                            padding: '2px 8px',
                            fontSize: 10,
                            background: 'none',
                            border: '1px solid rgba(0,240,255,0.2)',
                            borderRadius: 3,
                            color: 'var(--nt-primary)',
                            cursor: testingId === k.id ? 'wait' : 'pointer',
                          }}
                        >
                          {testingId === k.id ? t('components.AudioEditor.k6') : t('yuan-code.ModelSelector.k9')}
                        </button>
                        <button
                          onClick={() => handleDelete(k.id)}
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
                      </div>
                    </div>

                    {k.api_url && (
                      <div style={{ marginTop: 4 }}>
                        <code style={{
                          fontFamily: 'var(--nt-font-mono)',
                          fontSize: 10,
                          color: 'var(--nt-text-muted)',
                        }}>
                          {k.api_url}
                        </code>
                      </div>
                    )}

                    {providerInfo && providerInfo.available_models.length > 0 && selectedKey === k.id && (
                      <div style={{ marginTop: 6, display: 'flex', gap: 4, flexWrap: 'wrap' }}>
                        {providerInfo.available_models.map(m => (
                          <span key={m} style={{
                            fontSize: 9,
                            padding: '1px 5px',
                            borderRadius: 2,
                            background: 'rgba(0,240,255,0.05)',
                            color: 'var(--nt-text-muted)',
                            fontFamily: 'var(--nt-font-mono)',
                          }}>
                            {m}
                          </span>
                        ))}
                      </div>
                    )}

                    {testResults[k.id] && (
                      <div style={{
                        marginTop: 6,
                        padding: '4px 8px',
                        borderRadius: 3,
                        background: testResults[k.id]!.success
                          ? 'rgba(0,255,136,0.06)'
                          : 'rgba(255,0,110,0.06)',
                        border: `1px solid ${testResults[k.id]!.success ? 'rgba(0,255,136,0.2)' : 'rgba(255,0,110,0.2)'}`,
                        fontSize: 10,
                      }}>
                        {testResults[k.id]!.success ? (
                          <>
                            ✅ {testResults[k.id]!.model_used} · {testResults[k.id]!.latency_ms}ms
                            {testResults[k.id]!.response_snippet && (
                              <div style={{ marginTop: 2, color: 'var(--nt-text-muted)', fontFamily: 'var(--nt-font-mono)' }}>
                                "{testResults[k.id]!.response_snippet.slice(0, 60)}"
                              </div>
                            )}
                          </>
                        ) : (
                          <>❌ {testResults[k.id]!.error || t('errors.unknown')}</>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          )}

          {/* 新增/更新表单 */}
          {showForm ? (
            <div style={{
              marginTop: 10,
              padding: 12,
              border: '1px solid rgba(0,240,255,0.15)',
              borderRadius: 4,
              background: 'rgba(0,240,255,0.02)',
            }}>
              <div className={styles.formGroup}>
                <label className={styles.formLabel}>{t('yuan-code.ModelSelector.k10')}</label>
                <select
                  className={styles.formSelect}
                  value={form.provider}
                  onChange={e => {
                    const provider = e.target.value;
                    const providerInfo = providers.find(p => p.provider === provider);
                    setForm(prev => ({
                      ...prev,
                      provider,
                      api_url: providerInfo?.default_api_url || '',
                    }));
                  }}
                  style={{ padding: '4px 8px', fontSize: 11, height: 28 }}
                >
                  {providers.map(p => (
                    <option key={p.provider} value={p.provider}>
                      {p.display_name}
                    </option>
                  ))}
                </select>
              </div>

              <div className={styles.formGroup} style={{ marginTop: 8 }}>
                <label className={styles.formLabel}>{t('yuan-code.ModelSelector.k11')}</label>
                <input
                  className={styles.formInput}
                  placeholder={t('yuan-code.ModelSelector.k12')}
                  value={form.display_name}
                  onChange={e => setForm(prev => ({ ...prev, display_name: e.target.value }))}
                  style={{ padding: '4px 8px', fontSize: 11 }}
                />
              </div>

              <div className={styles.formGroup} style={{ marginTop: 8 }}>
                <label className={styles.formLabel}>{t('yuan-code.ModelSelector.k13')}</label>
                <input
                  type="password"
                  className={styles.formInput}
                  placeholder="sk-..."
                  value={form.api_key_plain}
                  onChange={e => setForm(prev => ({ ...prev, api_key_plain: e.target.value }))}
                  style={{ padding: '4px 8px', fontSize: 11, fontFamily: 'var(--nt-font-mono)' }}
                />
              </div>

              <div className={styles.formGroup} style={{ marginTop: 8 }}>
                <label className={styles.formLabel}>API URL</label>
                <input
                  className={styles.formInput}
                  placeholder="https://api.openai.com/v1"
                  value={form.api_url}
                  onChange={e => setForm(prev => ({ ...prev, api_url: e.target.value }))}
                  style={{ padding: '4px 8px', fontSize: 11, fontFamily: 'var(--nt-font-mono)' }}
                />
              </div>

              <div className={styles.settingRow} style={{ marginTop: 8 }}>
                <span className={styles.settingLabel}>{t('yuan-code.ModelSelector.k14')}</span>
                <div
                  className={`${styles.toggleSwitch} ${form.is_enabled ? styles.toggleSwitchOn : ''}`}
                  onClick={() => setForm(prev => ({ ...prev, is_enabled: !prev.is_enabled }))}
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
                  disabled={saving || !form.api_key_plain.trim()}
                  style={{ padding: '3px 10px', fontSize: 11 }}
                >
                  {saving ? t('components.AudioEditor.k6') : t('common.save')}
                </button>
              </div>
            </div>
          ) : (
            <button
              className={styles.btnPurple}
              onClick={() => setShowForm(true)}
              style={{ padding: '4px 12px', fontSize: 11, marginTop: 10 }}
            >
              + {t('yuan-code.ModelSelector.k15')}
            </button>
          )}

          {selectedKeyInfo && (
            <div style={{
              marginTop: 10,
              padding: '6px 10px',
              borderRadius: 4,
              background: 'rgba(0,240,255,0.04)',
              border: '1px solid rgba(0,240,255,0.12)',
              fontSize: 11,
            }}>
              <span style={{ color: 'var(--nt-text-muted)' }}>{t('yuan-code.ModelSelector.k16')}: </span>
              <span style={{ color: 'var(--nt-primary)', fontFamily: 'var(--nt-font-mono)' }}>
                {selectedKeyInfo.provider}
              </span>
              {(() => {
                const pi = providers.find(p => p.provider === selectedKeyInfo.provider);
                return pi?.default_model ? (
                  <span style={{ color: 'var(--nt-text-muted)', fontFamily: 'var(--nt-font-mono)' }}>
                    {' / '}{pi.default_model}
                  </span>
                ) : null;
              })()}
            </div>
          )}
        </>
      )}
    </div>
  );
}
