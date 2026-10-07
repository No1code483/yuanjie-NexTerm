// xin.review L2 —— 会话复盘功能域（局部 state + handlers + JSX）。当前人格经 core 注入。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { xinOrchestration } from '../../ipc';
import styles from '../../Xin.module.css';
import type { ConversationReview, TopicTrend, GrowthTrajectory, HeatmapData } from '../../xin/types';
import type { XinCore } from '../../core';

export function useReview(core: XinCore, active: boolean) {
  const { activePersona } = core;
  const [review, setReview] = useState<ConversationReview | null>(null);
  const [reviewPeriod, setReviewPeriod] = useState('week');
  const [topicTrends, setTopicTrends] = useState<TopicTrend[]>([]);
  const [growthTrajectory, setGrowthTrajectory] = useState<GrowthTrajectory | null>(null);
  const [heatmap, setHeatmap] = useState<HeatmapData | null>(null);
  const [reviewLoading, setReviewLoading] = useState(false);

  const loadReview = async () => {
    setReviewLoading(true);
    try {
      const [reviewRes, trendRes, growthRes, heatmapRes] = await Promise.all([xinOrchestration.reviewGenerate({ period_type: reviewPeriod, persona_id: activePersona.id }), xinOrchestration.reviewTopicTrends(activePersona.id, reviewPeriod, 7), xinOrchestration.reviewGrowthTrajectory(activePersona.id, reviewPeriod, 7), xinOrchestration.reviewHeatmap(activePersona.id, reviewPeriod)]);
      if (reviewRes?.data) setReview(reviewRes.data);
      if (trendRes?.data?.trends) setTopicTrends(trendRes.data.trends);
      if (growthRes?.data) setGrowthTrajectory(growthRes.data);
      if (heatmapRes?.data) setHeatmap(heatmapRes.data);
    } catch {/* silent */} finally {
      setReviewLoading(false);
    }
  };

  useEffect(() => {
    if (active && !review) loadReview();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const renderReview = () => <div className={styles.tabContent}>
      <div style={{
      display: 'flex',
      gap: '8px',
      marginBottom: '12px',
      alignItems: 'center'
    }}>
        <span className={styles.inputLabel}>{t("Xin.k129")}</span>
        <select className={styles.select} value={reviewPeriod} onChange={e => {
        setReviewPeriod(e.target.value);
        loadReview();
      }}>
          <option value="day">{t("components.FocusMode.k9")}</option>
          <option value="week">{t("Xin.k130")}</option>
          <option value="month">{t("Xin.k131")}</option>
          <option value="year">{t("Xin.k132")}</option>
        </select>
        <button className={styles.btn} onClick={loadReview} disabled={reviewLoading}>
          {reviewLoading ? t("components.intelligence.DashboardPanel.k67") : t("Xin.k133")}
        </button>
      </div>

      {reviewLoading && <div className={styles.loading}>{t("Xin.k134")}</div>}

      {review && <>
          <div className={styles.card}>
            <div className={styles.panelTitle}>
              {t("Xin.k135")} {review.period_label}
            </div>
            <p style={{
          fontSize: '11px',
          color: '#6a6a8a',
          marginBottom: '8px'
        }}>
              {review.date_from} → {review.date_to}
            </p>
            <div className={styles.settingsGrid}>
              <div className={styles.settingItem}>
                <span className={styles.inputLabel}>{t("Xin.k136")}</span>
                <span className={styles.badgeOn}>{review.conversation_count}</span>
              </div>
              <div className={styles.settingItem}>
                <span className={styles.inputLabel}>{t("Xin.k137")}</span>
                <span className={styles.badgeOn}>{review.total_messages}</span>
              </div>
              <div className={styles.settingItem}>
                <span className={styles.inputLabel}>Token</span>
                <span>{review.total_tokens.toLocaleString()}</span>
              </div>
              <div className={styles.settingItem}>
                <span className={styles.inputLabel}>{t("components.intelligence.BehaviorPanel.k8")}</span>
                <span>{review.most_active_hours.map(h => `${h}h`).join(', ')}</span>
              </div>
            </div>
            {review.generated_summary && <div style={{
          marginTop: '10px',
          padding: '10px',
          background: 'rgba(0,240,255,0.02)',
          borderRadius: '4px',
          border: '1px solid rgba(0,240,255,0.15)'
        }}>
                <span style={{
            fontSize: '10px',
            color: '#00F0FF'
          }}>{t("Xin.k138")}</span>
                <p style={{
            fontSize: '11px',
            color: '#c8c8dd',
            margin: '6px 0 0',
            lineHeight: '1.5'
          }}>
                  {review.generated_summary}
                </p>
              </div>}
            {review.dominant_topics.length > 0 && <div style={{
          marginTop: '8px'
        }}>
                <span style={{
            fontSize: '10px',
            color: '#6a6a8a'
          }}>{t("Xin.k139")}</span>
                <div style={{
            display: 'flex',
            gap: '6px',
            flexWrap: 'wrap',
            marginTop: '4px'
          }}>
                  {review.dominant_topics.map((t, i) => <span key={i} style={{
              fontSize: '10px',
              padding: '2px 8px',
              background: 'rgba(255,180,84,0.1)',
              borderRadius: '3px',
              color: '#FFD700',
              border: '1px solid rgba(255,180,84,0.2)'
            }}>
                      {t.topic} ({t.count})
                    </span>)}
                </div>
              </div>}
          </div>

          {topicTrends.length > 0 && <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k140")}</div>
              {topicTrends.slice(0, 5).map((trend, i) => <div key={i} style={{
          marginBottom: '8px'
        }}>
                  <span style={{
            fontSize: '10px',
            color: '#00F0FF'
          }}>{trend.topic}</span>
                  <div className={styles.trendLine}>
                    {trend.data_points.map((pt, j) => <div key={j} className={styles.trendPointWrapper} title={`${pt.date_label}: ${pt.value.toFixed(1)}`}>
                        <div className={styles.trendPoint} style={{
                height: `${Math.max(pt.value * 40, 4)}px`
              }} />
                        <span className={styles.trendLabel}>{pt.date_label.slice(-2)}</span>
                      </div>)}
                  </div>
                </div>)}
            </div>}

          {growthTrajectory && <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k141")}</div>
              <div className={styles.settingsGrid}>
                {[{
            label: t("Xin.k142"),
            data: growthTrajectory.emotion_trend
          }, {
            label: t("Xin.k143"),
            data: growthTrajectory.empathy_trend
          }, {
            label: t("lib.xinChatEngine.k68"),
            data: growthTrajectory.creativity_trend
          }, {
            label: t("ai.ModelManager.k38"),
            data: growthTrajectory.conversation_frequency
          }, {
            label: 'Token',
            data: growthTrajectory.token_usage_trend
          }].map(({
            label,
            data
          }) => <div key={label} className={styles.settingItem} style={{
            flexDirection: 'column',
            alignItems: 'flex-start',
            gap: '4px'
          }}>
                    <span className={styles.inputLabel}>{label}</span>
                    <div className={styles.miniTrend}>
                      {data.map((pt, j) => <div key={j} className={styles.trendPointWrapper}>
                          <div className={styles.trendPoint} style={{
                  height: `${Math.max(pt.value * 20, 2)}px`
                }} />
                        </div>)}
                    </div>
                    <span style={{
              fontSize: '9px',
              color: '#6a6a8a'
            }}>
                      {data[0]?.date_label} → {data[data.length - 1]?.date_label}
                    </span>
                  </div>)}
              </div>
            </div>}

          {heatmap && <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k144")}</div>
              <div className={styles.heatmapGrid}>
                {[t("Xin.k145"), t("Xin.k146"), t("Xin.k147"), t("Xin.k148"), t("Xin.k149"), t("Xin.k150"), t("Xin.k151")].map((day, d) => <div key={day} className={styles.heatmapRow}>
                    <span className={styles.heatmapDayLabel}>{day}</span>
                    {Array.from({
            length: 24
          }, (_, h) => {
            const cell = heatmap.cells.find(c => c.day_of_week === d && c.hour === h);
            const intensity = cell ? cell.intensity / Math.max(heatmap.max_intensity, 1) : 0;
            return <div key={h} className={styles.heatmapCell} style={{
              background: `rgba(0, 240, 255, ${Math.min(intensity * 0.8, 0.8)})`
            }} title={t("Xin.k152", {
              day: day,
              h: h,
              arg0: cell?.count || 0
            })} />;
          })}
                  </div>)}
              </div>
              <div style={{
          fontSize: '9px',
          color: '#6a6a8a',
          marginTop: '6px',
          textAlign: 'center'
        }}>
                {t("Xin.k153")} {heatmap.max_intensity}{t("ai.ChatPanel.k19")}
              </div>
            </div>}
        </>}

      {!review && !reviewLoading && <div style={{
      textAlign: 'center',
      padding: '40px',
      color: '#6a6a8a'
    }}>
          <p style={{
        fontSize: '13px'
      }}>{t("Xin.k154")}</p>
          <p style={{
        fontSize: '11px'
      }}>{t("Xin.k155")}</p>
        </div>}
    </div>;

  return { renderReview };
}
