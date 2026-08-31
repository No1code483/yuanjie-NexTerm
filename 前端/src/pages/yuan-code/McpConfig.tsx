// D1 v3.1 Task 3.2.10: MCP 配置 UI（启用/禁用/参数配置）
// 列出 8 个内置 MCP 服务器 + 外部 MCP 服务器，支持参数配置与启用/禁用
import { t } from "i18next";
import { useState, useCallback, useEffect } from 'react';
import { yuanMcp, BUILTIN_MCP_SERVERS } from '@/lib/ipc';
import type { McpServerRecord } from '@/lib/ipc';
import styles from '../YuanCode.module.css';

type Tab = 'builtin' | 'external';

export default function McpConfig() {
  const [tab, setTab] = useState<Tab>('builtin');
  const [builtinServers, setBuiltinServers] = useState<string[]>([]);
  const [builtinTools, setBuiltinTools] = useState<Record<string, number>>({});
  // 每个内置服务器的配置表单值（按 server name 索引）
  const [configValues, setConfigValues] = useState<Record<string, Record<string, string>>>({});
  // 已配置标记（内存态，反映是否调用过 configureBuiltin）
  const [configured, setConfigured] = useState<Record<string, boolean>>({});
  const [saving, setSaving] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [externalServers, setExternalServers] = useState<McpServerRecord[]>([]);
  const [statuses, setStatuses] = useState<Record<string, boolean>>({});

  const loadBuiltin = useCallback(async () => {
    try {
      const [namesRes, toolsRes] = await Promise.all([
        yuanMcp.listBuiltinServers(),
        yuanMcp.listBuiltinTools()
      ]);
      if (namesRes.code === 0 && namesRes.data) {
        setBuiltinServers(namesRes.data);
      }
      if (toolsRes.code === 0 && toolsRes.data) {
        const counts: Record<string, number> = {};
        for (const t of toolsRes.data) {
          counts[t.server_id] = (counts[t.server_id] || 0) + 1;
        }
        setBuiltinTools(counts);
      }
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const loadExternal = useCallback(async () => {
    try {
      const [recordsRes, stsRes] = await Promise.all([
        yuanMcp.listPersistedServers(),
        yuanMcp.listServers()
      ]);
      if (recordsRes.code === 0 && recordsRes.data) {
        setExternalServers(recordsRes.data);
      }
      const connected: Record<string, boolean> = {};
      if (stsRes.code === 0 && stsRes.data) {
        for (const s of stsRes.data) {
          connected[s.id] = s.connected;
        }
      }
      setStatuses(connected);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    loadBuiltin();
    loadExternal();
  }, [loadBuiltin, loadExternal]);

  const handleConfigChange = (serverName: string, key: string, value: string) => {
    setConfigValues(prev => ({
      ...prev,
      [serverName]: { ...(prev[serverName] || {}), [key]: value }
    }));
  };

  const handleSaveBuiltin = useCallback(async (serverName: string) => {
    setSaving(serverName);
    setError(null);
    try {
      const values = configValues[serverName] || {};
      await yuanMcp.configureBuiltin(serverName, values);
      setConfigured(prev => ({ ...prev, [serverName]: true }));
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(null);
    }
  }, [configValues]);

  const handleToggleExternal = useCallback(async (record: McpServerRecord, enabled: boolean) => {
    setError(null);
    try {
      await yuanMcp.setServerEnabled(record.id, enabled);
      setExternalServers(prev =>
        prev.map(r => (r.id === record.id ? { ...r, enabled } : r))
      );
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const handleDeleteExternal = useCallback(async (id: string) => {
    setError(null);
    try {
      await yuanMcp.deleteServerConfig(id);
      setExternalServers(prev => prev.filter(r => r.id !== id));
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const handleConnectExternal = useCallback(async (id: string) => {
    setError(null);
    try {
      await yuanMcp.connectServer(id);
      await loadExternal();
    } catch (e) {
      setError(String(e));
    }
  }, [loadExternal]);

  const handleDisconnectExternal = useCallback(async (id: string) => {
    setError(null);
    try {
      await yuanMcp.disconnectServer(id);
      await loadExternal();
    } catch (e) {
      setError(String(e));
    }
  }, [loadExternal]);

  return (
    <div className={styles.panelContainer}>
      <div className={styles.panelHeader}>
        <span className={styles.panelTitle}>{t('yuanCode.mcpConfig.title', 'MCP 服务器配置')}</span>
        <span className={styles.panelBadge}>
          {tab === 'builtin' ? builtinServers.length : externalServers.length}
        </span>
      </div>
      <div className={styles.skillScopeTabs}>
        <button
          className={`${styles.skillScopeTab} ${tab === 'builtin' ? styles.topTabActive : ''}`}
          onClick={() => setTab('builtin')}
        >
          {t('yuanCode.mcpConfig.builtin', '内置服务器')}
        </button>
        <button
          className={`${styles.skillScopeTab} ${tab === 'external' ? styles.topTabActive : ''}`}
          onClick={() => setTab('external')}
        >
          {t('yuanCode.mcpConfig.external', '外部服务器')}
        </button>
      </div>
      <div className={styles.panelBody}>
        {error && (
          <div style={{ color: 'var(--color-danger, #e5484d)', marginBottom: 8, fontSize: 13 }}>
            {error}
          </div>
        )}
        {tab === 'builtin' ? (
          <div className={styles.cardGrid}>
            {BUILTIN_MCP_SERVERS.map(info => {
              const isConfigured = configured[info.name];
              const toolCount = builtinTools[info.name] || 0;
              return (
                <div className={styles.skillCard} key={info.name}>
                  <div className={styles.skillName}>{info.name}</div>
                  <div className={styles.skillDesc}>{info.description}</div>
                  <div className={styles.skillMeta}>
                    <span className={styles.skillTag}>{toolCount} 工具</span>
                    {isConfigured && (
                      <span className={styles.skillTag} style={{ color: 'var(--color-success, #30a46c)' }}>
                        {t('yuanCode.mcpConfig.configured', '已配置')}
                      </span>
                    )}
                  </div>
                  <div style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 6 }}>
                    {info.config_fields.map(field => (
                      <div className={styles.formGroup} key={field.key}>
                        <label className={styles.formLabel}>
                          {field.label}
                          {field.required && <span style={{ color: 'var(--color-danger, #e5484d)' }}> *</span>}
                        </label>
                        <input
                          className={styles.formInput}
                          type={field.type === 'password' ? 'password' : 'text'}
                          placeholder={field.placeholder}
                          value={configValues[info.name]?.[field.key] || ''}
                          onChange={e => handleConfigChange(info.name, field.key, e.target.value)}
                        />
                      </div>
                    ))}
                  </div>
                  <button
                    className={styles.btnPrimary}
                    style={{ marginTop: 8, width: '100%' }}
                    disabled={saving === info.name}
                    onClick={() => handleSaveBuiltin(info.name)}
                  >
                    {saving === info.name
                      ? t('yuanCode.mcpConfig.saving', '保存中...')
                      : t('yuanCode.mcpConfig.save', '保存配置')}
                  </button>
                </div>
              );
            })}
          </div>
        ) : (
          <div className={styles.cardGrid}>
            {externalServers.length === 0 && (
              <div style={{ color: 'var(--color-muted, #8b8d98)', padding: 16 }}>
                {t('yuanCode.mcpConfig.noExternal', '暂无外部 MCP 服务器配置')}
              </div>
            )}
            {externalServers.map(record => (
              <div className={styles.skillCard} key={record.id}>
                <div className={styles.skillName}>{record.name}</div>
                <div className={styles.skillDesc}>{record.command} {record.args}</div>
                <div className={styles.skillMeta}>
                  <span className={styles.skillTag}>
                    {record.enabled
                      ? t('yuanCode.mcpConfig.enabled', '已启用')
                      : t('yuanCode.mcpConfig.disabled', '已禁用')}
                  </span>
                  {statuses[record.id] && (
                    <span className={styles.skillTag} style={{ color: 'var(--color-success, #30a46c)' }}>
                      {t('yuanCode.mcpConfig.connected', '已连接')}
                    </span>
                  )}
                  {record.is_builtin && (
                    <span className={styles.skillTag}>{t('yuanCode.mcpConfig.builtinTag', '预置')}</span>
                  )}
                </div>
                <div style={{ marginTop: 8, display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                  <label className={styles.toggleSwitch} style={{ flex: 1, minWidth: 80 }}>
                    <input
                      type="checkbox"
                      checked={record.enabled}
                      onChange={e => handleToggleExternal(record, e.target.checked)}
                      style={{ display: 'none' }}
                    />
                    <span className={styles.toggleKnob} />
                  </label>
                  {statuses[record.id] ? (
                    <button
                      className={styles.btnGold}
                      onClick={() => handleDisconnectExternal(record.id)}
                    >
                      {t('yuanCode.mcpConfig.disconnect', '断开')}
                    </button>
                  ) : (
                    <button
                      className={styles.btnPrimary}
                      onClick={() => handleConnectExternal(record.id)}
                      disabled={!record.enabled}
                    >
                      {t('yuanCode.mcpConfig.connect', '连接')}
                    </button>
                  )}
                  {!record.is_builtin && (
                    <button
                      className={styles.btnDanger}
                      onClick={() => handleDeleteExternal(record.id)}
                    >
                      {t('yuanCode.mcpConfig.delete', '删除')}
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
