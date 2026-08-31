import { t } from "i18next";
import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { intelligence } from '@/lib/ipc';
import type { ActivityLog } from '@/types';
import styles from '../Profile.module.css';
interface ActivityTimelineProps {
  userId: number | undefined;
  enabled: boolean;
  limit?: number;
}
const MODULE_LABEL_MAP: Record<string, string> = {
  knowledge_base: t("components.intelligence.ActivityPanel.k1"),
  resume: t("components.intelligence.ActivityPanel.k5"),
  quote: t("components.intelligence.ActivityPanel.k6"),
  profile: t("components.intelligence.DashboardPanel.k105"),
  terminal: t("components.intelligence.ActivityPanel.k8"),
  ai_chat: t("components.intelligence.ActivityPanel.k7"),
  chat: t("components.intelligence.ActivityPanel.k7"),
  editor: t("profile.ActivityTimeline.k1"),
  spyglass: t("components.intelligence.DashboardPanel.k103"),
  todo: t("components.intelligence.ActivityPanel.k2"),
  journal: t("components.intelligence.ActivityPanel.k3"),
  timer: t("components.intelligence.ActivityPanel.k4"),
  game: t("components.AddExtensionModal.k11"),
  search: t("common.search"),
  news: t("components.intelligence.ActivityPanel.k9"),
  linux: 'Linux',
  xin: t("components.FloatingXin.k26")
};
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
  'AI对话': t("components.intelligence.ActivityPanel.k7"),
  '活跃心跳': t("profile.ActivityTimeline.k2")
};
const HEARTBEAT_KEYWORDS = [t("profile.ActivityTimeline.k2"), 'heartbeat', 'heart_beat', 'ping'];
const isHeartbeat = (log: ActivityLog): boolean => {
  return HEARTBEAT_KEYWORDS.some(kw => log.operation?.toLowerCase().includes(kw.toLowerCase()));
};
const formatDetail = (detail: string, maxLen = 30): string => {
  if (!detail) return '';
  if (detail.startsWith('/')) {
    const parts = detail.split('/').filter(Boolean);
    if (parts.length <= 2) return detail;
    return '…/' + parts.slice(-2).join('/');
  }
  if (detail.length > maxLen) {
    return detail.slice(0, maxLen) + '…';
  }
  return detail;
};
export default function ActivityTimeline({
  userId,
  enabled,
  limit = 8
}: ActivityTimelineProps) {
  const navigate = useNavigate();
  const [logs, setLogs] = useState<ActivityLog[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [totalCount, setTotalCount] = useState(0);
  const fetchLogs = useCallback(async () => {
    if (!enabled || !userId) return;
    setLoading(true);
    setError('');
    try {
      const res = await intelligence.queryActivityLogs({
        userId: String(userId),
        limit: limit * 5,
        offset: 0
      });
      if (res.code === 0 && res.data) {
        const all = res.data.logs || [];
        const filtered = all.filter(log => !isHeartbeat(log));
        setLogs(filtered.slice(0, limit));
        setTotalCount(res.data.total || filtered.length);
      } else {
        setError(res.message || t("profile.ActivityTimeline.k3"));
      }
    } catch (err) {
      setError(t("profile.ActivityTimeline.k4", {
        err: err
      }));
    } finally {
      setLoading(false);
    }
  }, [userId, enabled, limit]);
  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);
  const formatRelative = (ts: string): string => {
    try {
      // 修复时区偏移：无时区标识的时间戳统一视为 UTC（与写入端 toISOString 保持一致）
      const normalized = ts.replace(' ', 'T');
      const date = (normalized.includes('Z') || normalized.includes('+'))
        ? new Date(normalized)
        : new Date(normalized + 'Z');
      const now = Date.now();
      const diff = now - date.getTime();
      if (diff < 60000) return t("lib.utils.k1");
      if (diff < 3600000) return t("ai.utils.k1", {
        arg0: Math.floor(diff / 60000)
      });
      if (diff < 86400000) return t("profile.ActivityTimeline.k5", {
        arg0: Math.floor(diff / 3600000)
      });
      if (diff < 604800000) return t("profile.ActivityTimeline.k6", {
        arg0: Math.floor(diff / 86400000)
      });
      return date.toLocaleDateString();
    } catch {
      return ts;
    }
  };
  const moduleColor = (m: string): string => {
    const map: Record<string, string> = {
      knowledge_base: '#00F0FF',
      resume: '#B026FF',
      quote: '#FF006E',
      profile: '#00FF00',
      terminal: '#FFA500',
      ai_chat: '#00F0FF',
      chat: '#00F0FF',
      editor: '#B026FF',
      spyglass: '#00F0FF',
      todo: '#FFB454',
      journal: '#7FD962',
      timer: '#E26D6D',
      game: '#FF006E',
      search: '#00F0FF',
      news: '#FFB454',
      linux: '#7FD962',
      xin: '#B026FF'
    };
    return map[m] || '#888';
  };
  const moduleLabel = (m: string): string => MODULE_LABEL_MAP[m] || m;
  const opLabel = (op: string): string => OP_LABEL_MAP[op] || op;
  return <div className={`${styles.infoCard} ${styles.timelineCard}`}>
      <div className={styles.cardTitle}>
        <span>{t("profile.ActivityTimeline.k7")}</span>
        <span className={styles.cardTitleMeta}>[{logs.length}/{totalCount}]</span>
        <div style={{
        marginLeft: 'auto',
        display: 'flex',
        alignItems: 'center',
        gap: 8
      }}>
          <button className={styles.viewMoreBtn} onClick={() => navigate('/spyglass?tab=activity')} title={t("profile.ActivityTimeline.k8")}>
            {t("profile.ActivityTimeline.k9")}
          </button>
          <button className={styles.refreshTimelineBtn} onClick={fetchLogs} disabled={loading} title={t("common.refresh")}>
            {loading ? '⟳' : '↻'}
          </button>
        </div>
      </div>

      {error && <div className={styles.timelineError}>{error}</div>}

      {!loading && logs.length === 0 && !error && <div className={styles.timelineEmpty}>
          <span className={styles.timelineEmptyPrompt}>$</span>
          <span>{t("components.intelligence.ActivityPanel.k33")}</span>
        </div>}

      <div className={styles.timelineList}>
        {logs.map((log, idx) => <div key={log.id || idx} className={styles.timelineItem}>
            <span className={styles.timelineIndex}>{String(idx + 1).padStart(3, '0')}</span>
            <span className={styles.timelineModule} style={{
          color: moduleColor(log.module),
          borderColor: moduleColor(log.module) + '40'
        }}>
              {moduleLabel(log.module)}
            </span>
            <span className={styles.timelineOp}>{opLabel(log.operation)}</span>
            {log.detail && <span className={styles.timelineDetail} title={log.detail}>
                {formatDetail(log.detail)}
              </span>}
            <span className={styles.timelineTime}>{formatRelative(log.timestamp)}</span>
          </div>)}
      </div>
    </div>;
}