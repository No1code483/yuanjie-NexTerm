/**
 * PerfDashboard — 周度性能仪表盘组件（T2.4.3）
 *
 * 功能：
 * 1. 查询后端 perf_metrics 表的聚合统计（perf_get_metrics_summary）
 * 2. 展示各指标的 avg / p50 / p95 / min / max / latest
 * 3. 点击指标名查看时间序列趋势（perf_get_metric_timeseries）
 * 4. 支持时间窗口切换（7/14/30 天）
 *
 * 关联文档：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §5.2
 * 后端命令：perf_commands.rs::perf_get_metrics_summary / perf_get_metric_timeseries
 *
 * 使用方式：
 *   import { PerfDashboard } from '@/components/PerfDashboard/PerfDashboard'
 *   <PerfDashboard />
 *
 * 设计原则：
 * - 静默降级：非 Tauri 环境显示提示，不报错
 * - 自包含：不依赖外部图表库，用 CSS bar 实现简易趋势图
 * - 无侵入：可嵌入任意页面（如设置页 / 诊断面板）
 */

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import styles from './PerfDashboard.module.css';

// ===== 类型定义（对齐后端 perf_commands.rs）=====

import type { ReactElement } from 'react'

interface ApiResponse<T> {
  code: number;
  message: string;
  data?: T;
}

interface PerfMetricSummary {
  metric_name: string;
  sample_count: number;
  avg_ms: number;
  min_ms: number;
  max_ms: number;
  p50_ms: number;
  p95_ms: number;
  latest_recorded_at: string;
  latest_value_ms: number;
}

interface PerfMetricPoint {
  recorded_at: string;
  metric_value_ms: number;
  route: string | null;
}

// ===== IPC 封装 =====

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const res = await invoke<ApiResponse<T>>(cmd, args);
  if (res.code !== 0) {
    throw new Error(res.message || `命令 ${cmd} 调用失败`);
  }
  return res.data as T;
}

function isTauriEnv(): boolean {
  return (
    typeof window !== 'undefined' &&
    ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)
  );
}

// ===== 常量 =====

const TIME_WINDOWS = [
  { days: 7, label: '近 7 天' },
  { days: 14, label: '近 14 天' },
  { days: 30, label: '近 30 天' },
] as const;

// 指标名中文映射（常见指标）
const METRIC_LABELS: Record<string, string> = {
  startup_total_ms: '启动总耗时',
  startup_setup_ms: '启动 setup 阶段',
  startup_boot_ms: '启动 boot 阶段（含 Tauri）',
  fcp_ms: '首次内容绘制 FCP',
  lcp_ms: '最大内容绘制 LCP',
  cls: '累积布局偏移 CLS',
  ttfb: '首字节时间 TTFB',
  inp: '交互延迟 INP',
  ipc_duration_ms: 'IPC 调用耗时',
  ipc_error_ms: 'IPC 错误耗时',
  route_switch_ms: '路由切换耗时',
};

function getMetricLabel(name: string): string {
  return METRIC_LABELS[name] || name;
}

function formatMs(ms: number): string {
  if (ms < 1) return `${ms.toFixed(2)} ms`;
  if (ms < 100) return `${ms.toFixed(1)} ms`;
  return `${Math.round(ms)} ms`;
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return iso;
  }
}

// ===== 主组件 =====

export function PerfDashboard(): ReactElement {
  const [tauriReady] = useState(isTauriEnv());
  const [days, setDays] = useState<number>(7);
  const [summaries, setSummaries] = useState<PerfMetricSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 时间序列状态
  const [selectedMetric, setSelectedMetric] = useState<string | null>(null);
  const [timeseries, setTimeseries] = useState<PerfMetricPoint[]>([]);
  const [tsLoading, setTsLoading] = useState(false);

  // 查询聚合统计
  const fetchSummary = useCallback(async () => {
    if (!tauriReady) return;
    setLoading(true);
    setError(null);
    try {
      const data = await call<PerfMetricSummary[]>('perf_get_metrics_summary', { days });
      setSummaries(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setSummaries([]);
    } finally {
      setLoading(false);
    }
  }, [tauriReady, days]);

  // 查询时间序列
  const fetchTimeseries = useCallback(async (metricName: string) => {
    if (!tauriReady) return;
    setTsLoading(true);
    try {
      const data = await call<PerfMetricPoint[]>('perf_get_metric_timeseries', {
        metricName,
        limit: 100,
      });
      setTimeseries(data);
    } catch {
      setTimeseries([]);
    } finally {
      setTsLoading(false);
    }
  }, [tauriReady]);

  useEffect(() => {
    void fetchSummary();
  }, [fetchSummary]);

  const handleMetricClick = (metricName: string) => {
    if (selectedMetric === metricName) {
      setSelectedMetric(null);
      setTimeseries([]);
      return;
    }
    setSelectedMetric(metricName);
    void fetchTimeseries(metricName);
  };

  // 非 Tauri 环境
  if (!tauriReady) {
    return (
      <div className={styles.container}>
        <div className={styles.header}>
          <h2 className={styles.title}>性能仪表盘</h2>
        </div>
        <div className={styles.emptyState}>
          <p className={styles.emptyIcon}>📊</p>
          <p>性能仪表盘需在 Tauri 桌面环境中运行</p>
          <p className={styles.emptyHint}>
            浏览器开发模式下无法访问本地 perf_metrics 数据库
          </p>
        </div>
      </div>
    );
  }

  // 计算趋势图的最大值（用于归一化 bar 高度）
  const tsMax = timeseries.length > 0
    ? Math.max(...timeseries.map((p) => p.metric_value_ms))
    : 1;

  return (
    <div className={styles.container}>
      {/* 头部 */}
      <div className={styles.header}>
        <h2 className={styles.title}>性能仪表盘</h2>
        <div className={styles.controls}>
          {TIME_WINDOWS.map((tw) => (
            <button
              key={tw.days}
              className={`${styles.windowBtn} ${days === tw.days ? styles.windowBtnActive : ''}`}
              onClick={() => setDays(tw.days)}
            >
              {tw.label}
            </button>
          ))}
          <button
            className={styles.refreshBtn}
            onClick={() => void fetchSummary()}
            disabled={loading}
          >
            {loading ? '刷新中…' : '↻ 刷新'}
          </button>
        </div>
      </div>

      {/* 错误提示 */}
      {error && (
        <div className={styles.errorBar}>
          ⚠️ 查询失败: {error}
        </div>
      )}

      {/* 聚合统计表 */}
      {summaries.length === 0 && !loading && !error ? (
        <div className={styles.emptyState}>
          <p className={styles.emptyIcon}>📈</p>
          <p>暂无性能数据</p>
          <p className={styles.emptyHint}>
            请正常使用应用 1-2 分钟，启动耗时 / Web Vitals / IPC 耗时将自动采集
          </p>
        </div>
      ) : (
        <div className={styles.tableWrapper}>
          <table className={styles.table}>
            <thead>
              <tr>
                <th>指标</th>
                <th>样本数</th>
                <th>平均值</th>
                <th>P50</th>
                <th>P95</th>
                <th>最小</th>
                <th>最大</th>
                <th>最新值</th>
                <th>记录时间</th>
              </tr>
            </thead>
            <tbody>
              {summaries.map((s) => (
                <tr
                  key={s.metric_name}
                  className={`${styles.row} ${selectedMetric === s.metric_name ? styles.rowSelected : ''}`}
                  onClick={() => handleMetricClick(s.metric_name)}
                >
                  <td className={styles.metricNameCell}>
                    <span className={styles.metricLabel}>{getMetricLabel(s.metric_name)}</span>
                    <span className={styles.metricKey}>{s.metric_name}</span>
                  </td>
                  <td>{s.sample_count}</td>
                  <td>{formatMs(s.avg_ms)}</td>
                  <td>{formatMs(s.p50_ms)}</td>
                  <td className={styles.p95Cell}>{formatMs(s.p95_ms)}</td>
                  <td>{formatMs(s.min_ms)}</td>
                  <td>{formatMs(s.max_ms)}</td>
                  <td className={styles.latestCell}>{formatMs(s.latest_value_ms)}</td>
                  <td className={styles.timeCell}>{formatTime(s.latest_recorded_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* 时间序列趋势图 */}
      {selectedMetric && (
        <div className={styles.trendSection}>
          <h3 className={styles.trendTitle}>
            {getMetricLabel(selectedMetric)} — 趋势
            <span className={styles.trendSubtitle}>（{selectedMetric}，最近 {timeseries.length} 条）</span>
          </h3>
          {tsLoading ? (
            <div className={styles.trendLoading}>加载中…</div>
          ) : timeseries.length === 0 ? (
            <div className={styles.trendEmpty}>暂无时间序列数据</div>
          ) : (
            <div className={styles.chart}>
              <div className={styles.chartBars}>
                {timeseries.map((p, i) => {
                  const heightPct = tsMax > 0 ? (p.metric_value_ms / tsMax) * 100 : 0;
                  return (
                    <div
                      key={i}
                      className={styles.bar}
                      style={{ height: `${Math.max(heightPct, 2)}%` }}
                      title={`${formatTime(p.recorded_at)}: ${formatMs(p.metric_value_ms)}${p.route ? ` (${p.route})` : ''}`}
                    />
                  );
                })}
              </div>
              <div className={styles.chartAxis}>
                <span>{formatTime(timeseries[0]?.recorded_at || '')}</span>
                <span>{formatTime(timeseries[timeseries.length - 1]?.recorded_at || '')}</span>
              </div>
              <div className={styles.chartLegend}>
                <span>最大: {formatMs(tsMax)}</span>
                <span>最小: {formatMs(Math.min(...timeseries.map((p) => p.metric_value_ms)))}</span>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default PerfDashboard;
