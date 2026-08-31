// D1 v3.1 Task 3.2.11: MCP 工具调用 UI（调用历史 + 结果展示）
// 选择内置工具 → 输入参数 → 调用 → 展示结果 + 调用历史
import { t } from "i18next";
import { useState, useCallback, useEffect, useMemo } from 'react';
import { yuanMcp } from '@/lib/ipc';
import type { McpRegisteredTool, McpToolCallResult, McpToolCallHistoryEntry } from '@/lib/ipc';
import styles from '../YuanCode.module.css';

export default function McpToolCall() {
  const [tools, setTools] = useState<McpRegisteredTool[]>([]);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [argsText, setArgsText] = useState<string>('{}');
  const [result, setResult] = useState<McpToolCallResult | null>(null);
  const [calling, setCalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [latencyMs, setLatencyMs] = useState<number | null>(null);
  const [history, setHistory] = useState<McpToolCallHistoryEntry[]>([]);

  const loadTools = useCallback(async () => {
    try {
      const res = await yuanMcp.listBuiltinTools();
      if (res.code === 0 && res.data) {
        setTools(res.data);
        if (res.data.length > 0 && !selectedKey) {
          setSelectedKey(`${res.data[0].server_id}::${res.data[0].tool.name}`);
        }
      }
    } catch (e) {
      setError(String(e));
    }
  }, [selectedKey]);

  const loadHistory = useCallback(async () => {
    try {
      const res = await yuanMcp.listCallHistory();
      if (res.code === 0 && res.data) {
        setHistory(res.data);
      }
    } catch (e) {
      // 历史加载失败不阻塞主流程
      console.warn('[McpToolCall] 加载历史失败', e);
    }
  }, []);

  useEffect(() => {
    loadTools();
    loadHistory();
  }, [loadTools, loadHistory]);

  // 按服务器分组
  const groupedTools = useMemo(() => {
    const map: Record<string, McpRegisteredTool[]> = {};
    for (const t of tools) {
      (map[t.server_id] = map[t.server_id] || []).push(t);
    }
    return map;
  }, [tools]);

  const selectedTool = useMemo(() => {
    if (!selectedKey) return null;
    const [sid, tname] = selectedKey.split('::');
    return tools.find(t => t.server_id === sid && t.tool.name === tname) || null;
  }, [selectedKey, tools]);

  const handleCall = useCallback(async () => {
    if (!selectedTool) return;
    setCalling(true);
    setError(null);
    setResult(null);
    setLatencyMs(null);
    let parsedArgs: Record<string, unknown> = {};
    try {
      parsedArgs = argsText.trim() ? JSON.parse(argsText) : {};
    } catch (e) {
      setError(t('yuanCode.mcpToolCall.invalidJson', '参数 JSON 解析失败') + ': ' + String(e));
      setCalling(false);
      return;
    }
    const start = performance.now();
    try {
      const res = await yuanMcp.callBuiltinTool(
        selectedTool.server_id,
        selectedTool.tool.name,
        parsedArgs
      );
      if (res.code === 0 && res.data) {
        setResult(res.data);
        setLatencyMs(Math.round(performance.now() - start));
      } else {
        setError(res.message || t('yuanCode.mcpToolCall.callFailed', '调用失败'));
      }
      // 刷新历史
      loadHistory();
    } catch (e) {
      setError(String(e));
    } finally {
      setCalling(false);
    }
  }, [selectedTool, argsText, loadHistory]);

  const formatTime = (ts: number) => {
    try {
      return new Date(ts * 1000).toLocaleTimeString();
    } catch {
      return String(ts);
    }
  };

  const resultText = result?.content?.map(c => c.text || c.data || '').join('\n') || '';

  return (
    <div className={styles.panelContainer}>
      <div className={styles.panelHeader}>
        <span className={styles.panelTitle}>{t('yuanCode.mcpToolCall.title', 'MCP 工具调用')}</span>
        <span className={styles.panelBadge}>{tools.length}</span>
      </div>
      <div className={styles.panelBody} style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        {error && (
          <div style={{ color: 'var(--color-danger, #e5484d)', fontSize: 13 }}>{error}</div>
        )}
        <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
          <div style={{ flex: '1 1 240px', minWidth: 240 }}>
            <label className={styles.formLabel}>
              {t('yuanCode.mcpToolCall.selectTool', '选择工具')}
            </label>
            <select
              className={styles.formSelect}
              value={selectedKey || ''}
              onChange={e => {
                setSelectedKey(e.target.value);
                setResult(null);
                setLatencyMs(null);
                setError(null);
              }}
            >
              {Object.entries(groupedTools).map(([sid, list]) => (
                <optgroup label={sid} key={sid}>
                  {list.map(item => (
                    <option key={`${item.server_id}::${item.tool.name}`} value={`${item.server_id}::${item.tool.name}`}>
                      {item.tool.name}
                    </option>
                  ))}
                </optgroup>
              ))}
            </select>
          </div>
          <div style={{ flex: '1 1 240px', minWidth: 240 }}>
            <label className={styles.formLabel}>
              {t('yuanCode.mcpToolCall.args', '参数（JSON）')}
            </label>
            <textarea
              className={styles.formInput}
              style={{ minHeight: 60, fontFamily: 'monospace' }}
              value={argsText}
              onChange={e => setArgsText(e.target.value)}
              placeholder='{"key": "value"}'
            />
          </div>
        </div>
        {selectedTool && (
          <div style={{ fontSize: 12, color: 'var(--color-muted, #8b8d98)' }}>
            {selectedTool.tool.description}
          </div>
        )}
        <button
          className={styles.btnPrimary}
          disabled={!selectedTool || calling}
          onClick={handleCall}
        >
          {calling
            ? t('yuanCode.mcpToolCall.calling', '调用中...')
            : t('yuanCode.mcpToolCall.call', '调用工具')}
        </button>
        {latencyMs !== null && (
          <div style={{ fontSize: 12, color: 'var(--color-muted, #8b8d98)' }}>
            {t('yuanCode.mcpToolCall.latency', '延迟')}: {latencyMs}ms
            {latencyMs < 500 && (
              <span style={{ color: 'var(--color-success, #30a46c)', marginLeft: 8 }}>
                {t('yuanCode.mcpToolCall.below500', '< 500ms 达标')}
              </span>
            )}
          </div>
        )}
        {result && (
          <div>
            <div className={styles.formLabel}>
              {t('yuanCode.mcpToolCall.result', '调用结果')}
              {result.isError && (
                <span style={{ color: 'var(--color-danger, #e5484d)', marginLeft: 8 }}>
                  ({t('yuanCode.mcpToolCall.errorResult', '错误')})
                </span>
              )}
            </div>
            <pre
              style={{
                background: 'var(--color-bg-secondary, #1a1a1a)',
                padding: 10,
                borderRadius: 6,
                fontSize: 12,
                overflow: 'auto',
                maxHeight: 240,
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-word'
              }}
            >
              {resultText}
            </pre>
          </div>
        )}
        <div>
          <div className={styles.formLabel} style={{ display: 'flex', justifyContent: 'space-between' }}>
            <span>{t('yuanCode.mcpToolCall.history', '调用历史')}</span>
            <button
              className={styles.sidebarHeaderBtn}
              onClick={loadHistory}
              style={{ fontSize: 11 }}
            >
              {t('yuanCode.mcpToolCall.refresh', '刷新')}
            </button>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, maxHeight: 200, overflow: 'auto' }}>
            {history.length === 0 && (
              <div style={{ color: 'var(--color-muted, #8b8d98)', fontSize: 12, padding: 8 }}>
                {t('yuanCode.mcpToolCall.noHistory', '暂无调用历史')}
              </div>
            )}
            {history.map((h, i) => (
              <div
                key={i}
                style={{
                  fontSize: 12,
                  padding: '6px 8px',
                  background: 'var(--color-bg-secondary, #1a1a1a)',
                  borderRadius: 4,
                  borderLeft: `3px solid ${h.success ? 'var(--color-success, #30a46c)' : 'var(--color-danger, #e5484d)'}`
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                  <span>
                    <strong>{h.server_name}</strong>/{h.tool_name}
                  </span>
                  <span style={{ color: 'var(--color-muted, #8b8d98)' }}>
                    {h.latency_ms}ms · {formatTime(h.called_at)}
                  </span>
                </div>
                {h.error && (
                  <div style={{ color: 'var(--color-danger, #e5484d)' }}>{h.error}</div>
                )}
                {h.result_preview && (
                  <div style={{ color: 'var(--color-muted, #8b8d98)', marginTop: 2 }}>
                    {h.result_preview.slice(0, 120)}
                    {h.result_preview.length > 120 ? '...' : ''}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
