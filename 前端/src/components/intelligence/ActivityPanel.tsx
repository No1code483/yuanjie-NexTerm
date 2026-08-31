import { t } from "i18next";
import { useState, useEffect, useCallback } from 'react';
import { intelligence } from '@/lib/ipc';
import { useAuthStore } from '@/stores/authStore';
import type { ActivityLog, ActivityStats } from '@/types';
import { time } from '@/lib/utils';
import styles from './Intelligence.module.css';
const MODULE_OPTIONS = [{
  key: 'all',
  label: t("common.all")
}, {
  key: 'knowledge_base',
  label: t("components.intelligence.ActivityPanel.k1")
}, {
  key: 'todo',
  label: t("components.intelligence.ActivityPanel.k2")
}, {
  key: 'journal',
  label: t("components.intelligence.ActivityPanel.k3")
}, {
  key: 'timer',
  label: t("components.intelligence.ActivityPanel.k4")
}, {
  key: 'resume',
  label: t("components.intelligence.ActivityPanel.k5")
}, {
  key: 'quote',
  label: t("components.intelligence.ActivityPanel.k6")
}, {
  key: 'ai_chat',
  label: t("components.intelligence.ActivityPanel.k7")
}, {
  key: 'game',
  label: t("components.AddExtensionModal.k11")
}, {
  key: 'terminal',
  label: t("components.intelligence.ActivityPanel.k8")
}, {
  key: 'search',
  label: t("common.search")
}, {
  key: 'news',
  label: t("components.intelligence.ActivityPanel.k9")
}];
const OP_LABEL_MAP: Record<string, string> = {
  '浏览页面': t("components.intelligence.ActivityPanel.k10"),
  'add_entry': t("components.intelligence.ActivityPanel.k11"),
  'delete_entry': t("components.intelligence.ActivityPanel.k12"),
  '打开文件': t("components.intelligence.ActivityPanel.k13"),
  '打开链接': t("components.intelligence.ActivityPanel.k14"),
  '查看条目': t("components.intelligence.ActivityPanel.k15"),
  'import_files': t("components.intelligence.ActivityPanel.k16"),
  '智能建议': t("components.intelligence.ActivityPanel.k17"),
  '行为分析': t("components.intelligence.ActivityPanel.k18"),
  'AI对话': t("components.intelligence.ActivityPanel.k7")
};
const PAGE_SIZE = 30;
function getRemarkClass(remark?: string): string {
  if (!remark) return styles.remarkNormal;
  const lower = remark.toLowerCase();
  if (lower.includes('error') || lower.includes(t("common.failed")) || lower.includes(t("common.error"))) return styles.remarkError;
  if (lower.includes('success') || lower.includes(t("common.success")) || lower.includes(t("common.finish"))) return styles.remarkSuccess;
  return styles.remarkNormal;
}
export default function ActivityPanel() {
  const {
    user
  } = useAuthStore();
  const [logs, setLogs] = useState<ActivityLog[]>([]);
  const [loading, setLoading] = useState(false);
  const [moduleFilter, setModuleFilter] = useState('all');
  const [startTime, setStartTime] = useState('');
  const [endTime, setEndTime] = useState('');
  const [page, setPage] = useState(0);

  // ===== 使用统计 =====
  const [statsPeriod, setStatsPeriod] = useState<'today' | 'week' | 'month'>('today');
  const [stats, setStats] = useState<ActivityStats | null>(null);
  const [statsLoading, setStatsLoading] = useState(false);
  const loadStats = useCallback(async () => {
    setStatsLoading(true);
    try {
      const userId = user?.id ? String(user.id) : '1';
      const res = await intelligence.getActivityStats(userId, statsPeriod);
      if (res?.data) {
        // 后端返回 Vec<(String, i64)> 会被 Tauri 序列化为 Record<string, number>
        const data = res.data as unknown as ActivityStats;
        setStats(data);
      }
    } catch (e) {
      console.error('加载使用统计失败:', e);
    } finally {
      setStatsLoading(false);
    }
  }, [statsPeriod, user]);
  useEffect(() => {
    loadStats();
  }, [loadStats]);
  const load = useCallback(async () => {
    setLoading(true);
    try {
      const userId = user?.id ? String(user.id) : '1';
      const res = await intelligence.queryActivityLogs({
        userId,
        module: moduleFilter === 'all' ? undefined : moduleFilter,
        startTime: startTime || undefined,
        endTime: endTime || undefined,
        limit: PAGE_SIZE,
        offset: page * PAGE_SIZE
      });
      if (res?.data?.logs) setLogs(res.data.logs);
    } catch (e) {
      console.error('加载活动日志失败:', e);
    } finally {
      setLoading(false);
    }
  }, [moduleFilter, startTime, endTime, page, user]);
  useEffect(() => {
    load();
  }, [load]);
  useEffect(() => {
    setPage(0);
  }, [moduleFilter, startTime, endTime]);
  return <div className={styles.panel}>
      {/* Toolbar */}
      <div className={styles.activityToolbar}>
        <div className={styles.activityFilters}>
          {MODULE_OPTIONS.map(mod => <button key={mod.key} className={`${styles.modFilter} ${moduleFilter === mod.key ? styles.modFilterActive : ''}`} onClick={() => setModuleFilter(mod.key)}>
              {mod.label}
            </button>)}
        </div>
        <div className={styles.activityFilters}>
          <input type="date" className={styles.timeRangeInput} value={startTime} onChange={e => setStartTime(e.target.value)} placeholder={t("common.start")} />
          <span className={styles.timeRangeSep}>{t("components.intelligence.ActivityPanel.k19")}</span>
          <input type="date" className={styles.timeRangeInput} value={endTime} onChange={e => setEndTime(e.target.value)} placeholder={t("components.intelligence.ActivityPanel.k20")} />
        </div>
      </div>

      {/* ===== 使用统计概览 ===== */}
      <div className={styles.statsSection}>
        <div className={styles.statsHeader}>
          <h4 className={styles.statsTitle}>{t("components.intelligence.ActivityPanel.k21")}</h4>
          <div className={styles.statsPeriod}>
            {(['today', 'week', 'month'] as const).map(p => <button key={p} className={`${styles.periodBtn} ${statsPeriod === p ? styles.periodBtnActive : ''}`} onClick={() => setStatsPeriod(p)}>
                {p === 'today' ? t("components.intelligence.ActivityPanel.k22") : p === 'week' ? t("components.intelligence.ActivityPanel.k23") : t("components.intelligence.ActivityPanel.k24")}
              </button>)}
          </div>
        </div>
        {statsLoading ? <div className={styles.loading}>{t("components.intelligence.ActivityPanel.k25")}</div> : stats ? <div className={styles.statsBody}>
            {/* Top Stat Cards */}
            <div className={styles.statsCardRow}>
              <div className={styles.statsCard}>
                <div className={styles.statsCardLabel}>{t("components.intelligence.ActivityPanel.k26")}</div>
                <div className={styles.statsCardValue}>{stats.totalOperations}</div>
              </div>
              <div className={styles.statsCard}>
                <div className={styles.statsCardLabel}>{t("components.intelligence.ActivityPanel.k27")}</div>
                <div className={styles.statsCardValue}>{Object.keys(stats.byModule || {}).length}</div>
              </div>
              <div className={styles.statsCard}>
                <div className={styles.statsCardLabel}>{t("components.intelligence.ActivityPanel.k28")}</div>
                <div className={styles.statsCardValue}>{Object.keys(stats.byOperation || {}).length}</div>
              </div>
            </div>

            {/* Module Breakdown */}
            <div className={styles.breakdownSection}>
              <div className={styles.breakdownTitle}>{t("components.intelligence.ActivityPanel.k29")}</div>
              <div className={styles.breakdownBars}>
                {Object.entries(stats.byModule || {}).sort(([, a], [, b]) => b - a).map(([mod, count]) => {
              const pct = stats.totalOperations > 0 ? Math.round(count / stats.totalOperations * 100) : 0;
              return <div key={mod} className={styles.breakdownRow}>
                        <span className={styles.breakdownLabel}>{mod}</span>
                        <div className={styles.breakdownBarWrap}>
                          <div className={styles.breakdownBar} style={{
                    width: `${Math.max(pct, 2)}%`
                  }} />
                        </div>
                        <span className={styles.breakdownCount}>{count}</span>
                      </div>;
            })}
              </div>
            </div>

            {/* Operation Type Breakdown */}
            <div className={styles.breakdownSection}>
              <div className={styles.breakdownTitle}>{t("components.intelligence.ActivityPanel.k30")}</div>
              <div className={styles.breakdownBars}>
                {Object.entries(stats.byOperation || {}).sort(([, a], [, b]) => b - a).map(([op, count]) => {
              const pct = stats.totalOperations > 0 ? Math.round(count / stats.totalOperations * 100) : 0;
              const label = OP_LABEL_MAP[op] || op;
              return <div key={op} className={styles.breakdownRow}>
                        <span className={styles.breakdownLabel}>{label}</span>
                        <div className={styles.breakdownBarWrap}>
                          <div className={styles.breakdownBar} style={{
                    width: `${Math.max(pct, 2)}%`,
                    background: '#39BAE6'
                  }} />
                        </div>
                        <span className={styles.breakdownCount}>{count}</span>
                      </div>;
            })}
              </div>
            </div>
          </div> : <div className={styles.emptyState}><div className={styles.emptyText}>{t("components.intelligence.ActivityPanel.k31")}</div></div>}
      </div>

      {/* Table */}
      {loading ? <div className={styles.loading}>{t("components.intelligence.ActivityPanel.k32")}</div> : logs.length === 0 ? <div className={styles.emptyState}>
          <div className={styles.emptyIcon}>📋</div>
          <div className={styles.emptyText}>{t("components.intelligence.ActivityPanel.k33")}</div>
          <div className={styles.emptyHint}>{t("components.intelligence.ActivityPanel.k34")}</div>
        </div> : <>
          <table className={styles.activityTable}>
            <thead>
              <tr>
                <th>{t("components.intelligence.ActivityPanel.k35")}</th>
                <th>{t("components.intelligence.ActivityPanel.k36")}</th>
                <th>{t("common.operation")}</th>
                <th>{t("components.intelligence.ActivityPanel.k37")}</th>
              </tr>
            </thead>
            <tbody>
              {logs.map(log => <tr key={log.id}>
                  <td className={styles.activityTime}>{time.formatUtcToLocal(log.timestamp)}</td>
                  <td><span className={styles.activityModule}>{log.module}</span></td>
                  <td className={styles.activityOperation}>{log.operation}</td>
                  <td className={`${styles.activityRemark} ${getRemarkClass(log.remark)}`}>
                    {log.remark || '-'}
                  </td>
                </tr>)}
            </tbody>
          </table>

          {/* Pagination */}
          <div className={styles.pagination}>
            <button className={styles.pageBtn} disabled={page === 0} onClick={() => setPage(p => Math.max(0, p - 1))}>
              {t("components.intelligence.ActivityPanel.k38")}
            </button>
            <span className={styles.pageInfo}>{t("components.intelligence.ActivityPanel.k39")} {page + 1} {t("components.intelligence.ActivityPanel.k40")}</span>
            <button className={styles.pageBtn} disabled={logs.length < PAGE_SIZE} onClick={() => setPage(p => p + 1)}>
              {t("components.intelligence.ActivityPanel.k41")}
            </button>
          </div>
        </>}
    </div>;
}