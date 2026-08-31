import { t } from "i18next";
import { useState, useEffect, useCallback } from 'react';
import { intelligence } from '@/lib/ipc';
import { useAuthStore } from '@/stores/authStore';
import type { BehaviorReport } from '@/types';
import styles from './Intelligence.module.css';
const TREND_ICONS: Record<string, string> = {
  rising: '↑',
  stable: '→',
  declining: '↓'
};
const TREND_CLASSES: Record<string, string> = {
  rising: styles.trendUp,
  stable: styles.trendStable,
  declining: styles.trendDown
};
export default function BehaviorPanel() {
  const {
    user
  } = useAuthStore();
  const [report, setReport] = useState<BehaviorReport | null>(null);
  const [loading, setLoading] = useState(false);
  const load = useCallback(async () => {
    setLoading(true);
    try {
      const userId = user?.id ? String(user.id) : '1';
      const res = await intelligence.analyzeBehavior(userId, new Date().toISOString().slice(0, 10));
      if (res?.data) setReport(res.data);
    } catch (e) {
      console.error('加载行为分析失败:', e);
    } finally {
      setLoading(false);
    }
  }, [user]);
  useEffect(() => {
    load();
  }, [load]);
  if (loading && !report) {
    return <div className={styles.loading}>{t("components.intelligence.BehaviorPanel.k1")}</div>;
  }
  if (!report) {
    return <div className={styles.emptyState}><div className={styles.emptyText}>{t("components.intelligence.BehaviorPanel.k2")}</div></div>;
  }
  const focusScoreColor = report.focus_score >= 70 ? '#7FD962' : report.focus_score >= 40 ? '#FFB454' : '#E26D6D';
  return <div className={styles.panel}>
      {/* Focus Score Gauge */}
      <div className={styles.gaugeSection}>
        <div className={styles.gaugeCard}>
          <div className={styles.gaugeTitle}>{t("components.intelligence.BehaviorPanel.k3")}</div>
          <svg className={styles.gaugeSvg} width="140" height="80" viewBox="0 0 140 80">
            <defs>
              <linearGradient id="focusGrad" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="#E26D6D" />
                <stop offset="50%" stopColor="#FFB454" />
                <stop offset="100%" stopColor="#7FD962" />
              </linearGradient>
            </defs>
            <path d="M 10 70 A 60 60 0 0 1 130 70" fill="none" stroke="#1F2A35" strokeWidth="10" strokeLinecap="round" />
            <path d="M 10 70 A 60 60 0 0 1 130 70" fill="none" stroke="url(#focusGrad)" strokeWidth="10" strokeLinecap="round" strokeDasharray={`${report.focus_score / 100 * 188} 188`} />
            <text x="70" y="62" textAnchor="middle" fontSize="28" fontWeight="700" fill={focusScoreColor}>
              {report.focus_score.toFixed(0)}%
            </text>
          </svg>
          <div className={styles.gaugeSub}>{report.focus_verdict}</div>
        </div>

        {/* Consistency Score */}
        <div className={styles.gaugeCard}>
          <div className={styles.gaugeTitle}>{t("components.intelligence.BehaviorPanel.k4")}</div>
          <div className={styles.gaugeValue} style={{
          color: report.consistency_score >= 70 ? '#7FD962' : report.consistency_score >= 40 ? '#FFB454' : '#E26D6D',
          fontSize: 36
        }}>
            {report.consistency_score.toFixed(0)}%
          </div>
          <div className={styles.gaugeSub}>{report.consistency_verdict}</div>
        </div>
      </div>

      {/* Overview Stats */}
      <div className={styles.overviewGrid}>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>{t("components.intelligence.BehaviorPanel.k5")}</div>
          <div className={styles.statValue}>{report.distraction_count}</div>
          <div style={{
          fontSize: 11,
          color: '#555D68',
          marginTop: 4
        }}>{report.distraction_verdict}</div>
        </div>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>{t("components.intelligence.BehaviorPanel.k6")}</div>
          <div className={styles.statValue}>{report.error_rate.toFixed(1)}%</div>
          <div style={{
          fontSize: 11,
          color: '#555D68',
          marginTop: 4
        }}>{report.error_verdict}</div>
        </div>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>{t("components.intelligence.BehaviorPanel.k7")}</div>
          <div className={styles.statValue}>{report.module_diversity}</div>
          <div style={{
          fontSize: 11,
          color: '#555D68',
          marginTop: 4
        }}>{report.module_diversity_verdict}</div>
        </div>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>{t("components.intelligence.BehaviorPanel.k8")}</div>
          <div className={styles.statValue} style={{
          fontSize: 14
        }}>{report.active_period || '-'}</div>
          <div style={{
          fontSize: 11,
          color: '#555D68',
          marginTop: 4
        }}>
            {report.peak_hour_label || ''}
          </div>
        </div>
      </div>

      {/* Knowledge Base Stats */}
      <div className={styles.chartCard}>
        <div className={styles.chartTitle}>{t("components.intelligence.BehaviorPanel.k9")}</div>
        <div className={styles.barChart} style={{
        flexDirection: 'row',
        gap: 24
      }}>
          <div className={styles.barCol} style={{
          alignItems: 'flex-start'
        }}>
            <span className={styles.barValue}>{report.kb_entry_count}</span>
            <div className={styles.barFill} style={{
            width: `${Math.min(report.kb_entry_count * 10, 200)}px`,
            height: 16,
            background: 'linear-gradient(90deg, #39BAE6, #1F5A7A)',
            borderRadius: 3
          }} />
            <span className={styles.barLabel}>{t("components.intelligence.BehaviorPanel.k10")}</span>
          </div>
          <div className={styles.barCol} style={{
          alignItems: 'flex-start'
        }}>
            <span className={styles.barValue}>{report.kb_avg_duration_human}</span>
            <div className={styles.barFill} style={{
            width: `${Math.min(report.kb_avg_duration_secs / 6, 200)}px`,
            height: 16,
            background: 'linear-gradient(90deg, #7FD962, #3A7A3A)',
            borderRadius: 3
          }} />
            <span className={styles.barLabel}>{t("components.intelligence.BehaviorPanel.k11")}</span>
          </div>
        </div>
      </div>

      {/* Analysis Summary */}
      <div className={styles.sectionCard}>
        <div className={styles.sectionTitle}>{t("components.intelligence.BehaviorPanel.k12")}</div>
        <div className={styles.fileList}>
          <div className={styles.fileItem} style={{
          flexDirection: 'column',
          alignItems: 'flex-start'
        }}>
            <span className={styles.fileName} style={{
            whiteSpace: 'pre-wrap',
            lineHeight: 1.6
          }}>
              {report.summary}
            </span>
          </div>
        </div>
      </div>

      {/* Comparison Trend */}
      {report.comparison && <div className={styles.chartCard}>
          <div className={styles.chartTitle}>
            {t("components.intelligence.BehaviorPanel.k13")} {report.comparison.period_label}
            <span className={TREND_CLASSES[report.comparison.overall_trend] || ''} style={{
          marginLeft: 8,
          fontSize: 14
        }}>
              {TREND_ICONS[report.comparison.overall_trend] || '→'} {report.comparison.overall_trend}
            </span>
          </div>
          <div className={styles.trendGrid}>
            <div className={styles.trendCard}>
              <div className={styles.trendLabel}>{t("components.intelligence.BehaviorPanel.k14")}</div>
              <div className={styles.trendValue} style={{
            color: report.comparison.focus_change >= 0 ? '#7FD962' : '#E26D6D'
          }}>
                {report.comparison.focus_change >= 0 ? '+' : ''}{report.comparison.focus_change.toFixed(0)}%
              </div>
            </div>
            <div className={styles.trendCard}>
              <div className={styles.trendLabel}>{t("components.intelligence.BehaviorPanel.k15")}</div>
              <div className={styles.trendValue} style={{
            color: report.comparison.distraction_change <= 0 ? '#7FD962' : '#E26D6D'
          }}>
                {report.comparison.distraction_change >= 0 ? '+' : ''}{report.comparison.distraction_change.toFixed(0)}%
              </div>
            </div>
            <div className={styles.trendCard}>
              <div className={styles.trendLabel}>{t("components.intelligence.BehaviorPanel.k16")}</div>
              <div className={styles.trendValue} style={{
            color: report.comparison.error_change <= 0 ? '#7FD962' : '#E26D6D'
          }}>
                {report.comparison.error_change >= 0 ? '+' : ''}{report.comparison.error_change.toFixed(0)}%
              </div>
            </div>
            <div className={styles.trendCard}>
              <div className={styles.trendLabel}>{t("components.intelligence.BehaviorPanel.k17")}</div>
              <div className={styles.trendValue} style={{
            color: report.comparison.consistency_change >= 0 ? '#7FD962' : '#E26D6D'
          }}>
                {report.comparison.consistency_change >= 0 ? '+' : ''}{report.comparison.consistency_change.toFixed(0)}%
              </div>
            </div>
          </div>
        </div>}
    </div>;
}