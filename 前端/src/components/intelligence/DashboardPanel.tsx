import { t } from "i18next";
import { useState, useEffect, useCallback, useRef } from 'react';
import { intelligence, ipc } from '@/lib/ipc';
import { useAuthStore } from '@/stores/authStore';
import { time } from '@/lib/utils';
import type { DashboardData, RealtimeStats, ActivityStats, ActivityLog } from '@/types';
import styles from './Intelligence.module.css';
interface Props {
  period: 'day' | 'week' | 'month';
  onPeriodChange: (p: 'day' | 'week' | 'month') => void;
}

/** 延迟 hover 提示弹窗（悬停整个卡片 1 秒后显示） */
function StatCard({
  label,
  value,
  explanation,
  valueStyle
}: {
  label: string;
  value: React.ReactNode;
  explanation: string;
  valueStyle?: React.CSSProperties;
}) {
  const [show, setShow] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const handleEnter = () => {
    timerRef.current = setTimeout(() => setShow(true), 1000);
  };
  const handleLeave = () => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setShow(false);
  };
  return <div className={styles.statCard} onMouseEnter={handleEnter} onMouseLeave={handleLeave}>
      <span className={styles.statLabelHint}>{label}</span>
      {show && <span className={styles.statTooltip}>{explanation}</span>}
      <div className={styles.statValue} style={valueStyle}>{value}</div>
    </div>;
}

/** 2.3 心流状态检测：基于实时数据判断当前心流状态 */
function detectFlowState(realtime: RealtimeStats | null): {
  level: 'deep' | 'light' | 'scattered' | 'idle';
  label: string;
  icon: string;
  color: string;
  desc: string;
  tip: string;
} {
  if (!realtime) {
    return {
      level: 'idle',
      label: t("components.intelligence.DashboardPanel.k1"),
      icon: '⏳',
      color: '#555',
      desc: t("components.intelligence.DashboardPanel.k2"),
      tip: t("components.intelligence.DashboardPanel.k3")
    };
  }
  const lastTs = realtime.last_activity_at ? new Date(realtime.last_activity_at).getTime() : 0;
  const secsSinceLast = lastTs ? (Date.now() - lastTs) / 1000 : Infinity;
  const opsToday = realtime.operations_today;
  const focusModule = realtime.current_focus_module;
  const modulesUsed = realtime.modules_used_today;
  const activeSecs = realtime.active_duration_today_secs;

  // 5 分钟以上无活动 → 休息
  if (secsSinceLast > 300) {
    return {
      level: 'idle',
      label: t("components.intelligence.DashboardPanel.k4"),
      icon: '😴',
      color: '#7A8BE8',
      desc: t("components.intelligence.DashboardPanel.k5", {
        arg0: time.formatDurationSecs(secsSinceLast)
      }),
      tip: t("components.intelligence.DashboardPanel.k6")
    };
  }
  // 60 秒内无活动 → 待机
  if (secsSinceLast > 60) {
    return {
      level: 'scattered',
      label: t("components.intelligence.DashboardPanel.k7"),
      icon: '🌫️',
      color: '#FFB454',
      desc: t("components.intelligence.DashboardPanel.k8", {
        arg0: Math.round(secsSinceLast)
      }),
      tip: t("components.intelligence.DashboardPanel.k9")
    };
  }
  // 今日使用模块 ≥ 5 个 → 分散
  if (modulesUsed >= 5) {
    return {
      level: 'scattered',
      label: t("components.intelligence.DashboardPanel.k10"),
      icon: '🔀',
      color: '#FFB454',
      desc: t("components.intelligence.DashboardPanel.k11", {
        modulesUsed: modulesUsed
      }),
      tip: t("components.intelligence.DashboardPanel.k12")
    };
  }
  // 专注单一模块 + 操作密度高 + 持续时间 ≥ 15 分钟 → 深度心流
  if (focusModule && opsToday >= 30 && activeSecs >= 900 && modulesUsed <= 3) {
    return {
      level: 'deep',
      label: t("components.intelligence.DashboardPanel.k13"),
      icon: '🌊',
      color: '#7FD962',
      desc: t("components.intelligence.DashboardPanel.k14", {
        focusModule: focusModule
      }),
      tip: t("components.intelligence.DashboardPanel.k15")
    };
  }
  // 60 秒内有活动 → 轻度专注
  return {
    level: 'light',
    label: t("components.intelligence.DashboardPanel.k16"),
    icon: '🎯',
    color: '#39BAE6',
    desc: focusModule ? t("components.intelligence.DashboardPanel.k17", {
      focusModule: focusModule
    }) : t("components.intelligence.DashboardPanel.k18"),
    tip: t("components.intelligence.DashboardPanel.k19")
  };
}

/** 2.5 生产力画像：基于已有数据本地计算生成用户画像标签 */
interface PortraitTag {
  icon: string;
  label: string;
  desc: string;
  color: string;
}
function buildPortraitTags(data: DashboardData | null, realtime: RealtimeStats | null, cumulative: DashboardData | null): PortraitTag[] {
  const tags: PortraitTag[] = [];
  if (!data && !realtime) return tags;

  // 时段偏好（基于热力图）
  if (data?.hourly_heatmap && data.hourly_heatmap.length > 0) {
    const hourCounts: Record<number, number> = {};
    data.hourly_heatmap.forEach(h => {
      const hr = parseInt(h.hour.slice(0, 2), 10);
      if (!isNaN(hr)) hourCounts[hr] = (hourCounts[hr] || 0) + h.count;
    });
    const nightOps = [23, 0, 1, 2, 3, 4, 5].reduce((s, h) => s + (hourCounts[h] || 0), 0);
    const morningOps = [6, 7, 8, 9].reduce((s, h) => s + (hourCounts[h] || 0), 0);
    const total = Object.values(hourCounts).reduce((s, c) => s + c, 0);
    if (total > 0) {
      if (nightOps / total > 0.3) {
        tags.push({
          icon: '🦉',
          label: t("components.intelligence.DashboardPanel.k20"),
          desc: t("components.intelligence.DashboardPanel.k21"),
          color: '#B026FF'
        });
      } else if (morningOps / total > 0.35) {
        tags.push({
          icon: '☀️',
          label: t("components.intelligence.DashboardPanel.k22"),
          desc: t("components.intelligence.DashboardPanel.k23"),
          color: '#FFB84D'
        });
      }
    }
  }

  // 模块多样性
  if (realtime?.modules_used_today != null) {
    if (realtime.modules_used_today >= 6) {
      tags.push({
        icon: '🎭',
        label: t("components.intelligence.DashboardPanel.k24"),
        desc: t("components.intelligence.DashboardPanel.k25", {
          modules_used_today: realtime.modules_used_today
        }),
        color: '#39BAE6'
      });
    } else if (realtime.modules_used_today <= 2 && realtime.modules_used_today > 0) {
      tags.push({
        icon: '🎯',
        label: t("components.intelligence.DashboardPanel.k26"),
        desc: t("components.intelligence.DashboardPanel.k27"),
        color: '#7FD962'
      });
    }
  }

  // 操作密度
  if (realtime?.operations_today != null) {
    if (realtime.operations_today >= 150) {
      tags.push({
        icon: '⚡',
        label: t("components.intelligence.DashboardPanel.k28"),
        desc: t("components.intelligence.DashboardPanel.k29", {
          operations_today: realtime.operations_today
        }),
        color: '#FFB454'
      });
    } else if (realtime.operations_today > 0 && realtime.operations_today < 20) {
      tags.push({
        icon: '🌱',
        label: t("components.intelligence.DashboardPanel.k30"),
        desc: t("components.intelligence.DashboardPanel.k31"),
        color: '#7A8BE8'
      });
    }
  }

  // 生产力评分
  if (data?.productivity_score != null && data.productivity_score > 0) {
    if (data.productivity_score >= 75) {
      tags.push({
        icon: '🌟',
        label: t("components.intelligence.DashboardPanel.k32"),
        desc: t("components.intelligence.DashboardPanel.k33", {
          arg0: data.productivity_score.toFixed(0)
        }),
        color: '#7FD962'
      });
    } else if (data.productivity_score < 40) {
      tags.push({
        icon: '🚀',
        label: t("components.intelligence.DashboardPanel.k34"),
        desc: t("components.intelligence.DashboardPanel.k35"),
        color: '#E26D6D'
      });
    }
  }

  // 累计经验
  if (cumulative?.total_operations != null) {
    if (cumulative.total_operations >= 5000) {
      tags.push({
        icon: '🏛️',
        label: t("components.intelligence.DashboardPanel.k36"),
        desc: t("components.intelligence.DashboardPanel.k37", {
          total_operations: cumulative.total_operations
        }),
        color: '#FFD700'
      });
    } else if (cumulative.total_operations < 100) {
      tags.push({
        icon: '🐣',
        label: t("components.intelligence.DashboardPanel.k38"),
        desc: t("components.intelligence.DashboardPanel.k39"),
        color: '#7FD962'
      });
    }
  }

  // 深度专注
  if (realtime?.active_duration_today_secs != null && realtime.current_focus_module) {
    if (realtime.active_duration_today_secs >= 7200 && realtime.modules_used_today <= 3) {
      tags.push({
        icon: '🌊',
        label: t("components.intelligence.DashboardPanel.k40"),
        desc: t("components.intelligence.DashboardPanel.k41", {
          arg0: time.formatDurationSecs(realtime.active_duration_today_secs)
        }),
        color: '#00F0FF'
      });
    }
  }
  return tags;
}
export default function DashboardPanel({
  period,
  onPeriodChange
}: Props) {
  const {
    user
  } = useAuthStore();
  const [data, setData] = useState<DashboardData | null>(null);
  const [cumulative, setCumulative] = useState<DashboardData | null>(null);
  const [realtime, setRealtime] = useState<RealtimeStats | null>(null);
  const [activityStats, setActivityStats] = useState<ActivityStats | null>(null);
  const [loading, setLoading] = useState(false);
  // 2.2 下钻式时间线
  const [activityLogs, setActivityLogs] = useState<ActivityLog[]>([]);
  const [expandedLogId, setExpandedLogId] = useState<number | null>(null);
  const [logFilterModule, setLogFilterModule] = useState<string>('');
  const [, setForceTick] = useState(0); // 用于刷新心流状态显示
  const flowState = detectFlowState(realtime);

  // 2.4 对话式查询：用户用自然语言询问自己的数据
  interface ChatQaItem {
    question: string;
    answer: string;
    loading?: boolean;
  }
  const [chatQuestion, setChatQuestion] = useState('');
  const [chatHistory, setChatHistory] = useState<ChatQaItem[]>([]);
  const chatScrollRef = useRef<HTMLDivElement>(null);

  // 2.5 生产力画像
  const portraitTags = buildPortraitTags(data, realtime, cumulative);

  // 4.1 每日简报
  interface DailyBriefing {
    date: string;
    briefing: string;
    todo_count: number;
    todo_completed: number;
    journal_words: number;
    news_count: number;
    timer_sessions: number;
    total_focus_seconds: number;
    generated_at: string;
  }
  const [briefing, setBriefing] = useState<DailyBriefing | null>(null);
  const [briefingLoading, setBriefingLoading] = useState(false);
  const [briefingExpanded, setBriefingExpanded] = useState(true);
  const load = useCallback(async () => {
    setLoading(true);
    try {
      const userId = user?.id ? String(user.id) : '1';
      const [dRes, cRes, rRes, aRes] = await Promise.all([intelligence.getDashboard(userId, period), intelligence.getDashboard(userId, 'all'), intelligence.getRealtimeStats(userId), intelligence.getActivityStats(userId, period)]);
      if (dRes?.data) {
        setData(dRes.data);
      }
      if (cRes?.data) {
        setCumulative(cRes.data);
      }
      if (rRes?.data) {
        setRealtime(rRes.data);
      }
      if (aRes?.data) {
        setActivityStats(aRes.data);
      }
    } catch (e) {
      console.error('加载仪表盘失败:', e);
    } finally {
      setLoading(false);
    }
  }, [period, user]);
  useEffect(() => {
    load();
  }, [load]);
  useEffect(() => {
    const interval = setInterval(load, 1000);
    return () => clearInterval(interval);
  }, [load]);

  // 2.2 加载活动时间线（不与仪表盘同步每秒刷新，独立 10 秒刷新一次）
  const loadActivityLogs = useCallback(async () => {
    const userId = user?.id ? String(user.id) : '1';
    try {
      const res = await intelligence.queryActivityLogs({
        userId,
        module: logFilterModule || undefined,
        limit: 20,
        offset: 0
      });
      if (res?.data?.logs) {
        setActivityLogs(res.data.logs);
      }
    } catch (e) {
      console.error('加载活动时间线失败:', e);
    }
  }, [user, logFilterModule]);
  useEffect(() => {
    loadActivityLogs();
    const iv = setInterval(loadActivityLogs, 10000);
    return () => clearInterval(iv);
  }, [loadActivityLogs]);

  // 2.3 每 15 秒强制刷新一次心流状态显示（用于检测空闲过渡）
  useEffect(() => {
    const iv = setInterval(() => setForceTick(t => t + 1), 15000);
    return () => clearInterval(iv);
  }, []);

  // 4.1 加载每日简报（只在仪表盘首次加载时获取一次，5 分钟刷新一次）
  useEffect(() => {
    const loadBriefing = async () => {
      setBriefingLoading(true);
      try {
        const res = await intelligence.dailyBriefing();
        if (res?.code === 0 && res?.data) {
          setBriefing(res.data);
        }
      } catch {
        // 静默失败
      } finally {
        setBriefingLoading(false);
      }
    };
    loadBriefing();
    const iv = setInterval(loadBriefing, 5 * 60 * 1000);
    return () => clearInterval(iv);
  }, []);

  // 2.4 对话式查询：构建数据上下文 + 调用 LLM
  const buildDataContext = useCallback((): string => {
    const parts: string[] = [];
    parts.push(t("components.intelligence.DashboardPanel.k42", {
      arg0: period === 'day' ? t("components.intelligence.ActivityPanel.k22") : period === 'week' ? t("components.intelligence.DashboardPanel.k43") : t("components.intelligence.DashboardPanel.k44")
    }));
    if (data) {
      parts.push(t("components.intelligence.DashboardPanel.k45", {
        arg0: time.formatDurationSecs(data.total_active_secs)
      }));
      parts.push(t("components.intelligence.DashboardPanel.k46", {
        total_operations: data.total_operations
      }));
      parts.push(t("components.intelligence.DashboardPanel.k47", {
        arg0: data.productivity_score.toFixed(0)
      }));
      if (data.top_modules && data.top_modules.length > 0) {
        parts.push(t("components.intelligence.DashboardPanel.k48", {
          arg0: data.top_modules.slice(0, 5).map(([m, c]) => `${m}(${c})`).join(', ')
        }));
      }
      if (data.peak_hours && data.peak_hours.length > 0) {
        parts.push(t("components.intelligence.DashboardPanel.k49", {
          arg0: data.peak_hours.join(', ')
        }));
      }
      if (data.daily_trend && data.daily_trend.length > 0) {
        parts.push(t("components.intelligence.DashboardPanel.k50", {
          arg0: time.formatDurationSecs(Math.round(data.total_active_secs / data.daily_trend.length))
        }));
      }
    }
    if (realtime) {
      parts.push(t("components.intelligence.DashboardPanel.k51", {
        operations_today: realtime.operations_today
      }));
      parts.push(t("components.intelligence.DashboardPanel.k52", {
        arg0: time.formatDurationSecs(realtime.active_duration_today_secs)
      }));
      parts.push(t("components.intelligence.DashboardPanel.k53", {
        arg0: realtime.current_focus_module || t("common.none")
      }));
      parts.push(t("components.intelligence.DashboardPanel.k54", {
        modules_used_today: realtime.modules_used_today
      }));
    }
    if (cumulative) {
      parts.push(t("components.intelligence.DashboardPanel.k55", {
        arg0: time.formatDurationSecs(Math.round(cumulative.total_active_hours * 3600))
      }));
      parts.push(t("components.intelligence.DashboardPanel.k56", {
        total_operations: cumulative.total_operations
      }));
    }
    if (activityStats) {
      parts.push(t("components.intelligence.DashboardPanel.k57", {
        arg0: period === 'day' ? t("components.intelligence.ActivityPanel.k22") : period === 'week' ? t("components.intelligence.DashboardPanel.k43") : t("components.intelligence.DashboardPanel.k44")
      }));
      const modEntries = Object.entries(activityStats.byModule || {}).sort(([, a], [, b]) => b - a).slice(0, 5);
      modEntries.forEach(([m, c]) => parts.push(t("components.intelligence.DashboardPanel.k58", {
        m: m,
        c: c
      })));
    }
    return parts.join('\n');
  }, [period, data, realtime, cumulative, activityStats]);
  const handleChatSubmit = useCallback(async () => {
    const q = chatQuestion.trim();
    if (!q) return;
    const ctx = buildDataContext();
    const prompt = t("components.intelligence.DashboardPanel.k59", {
      ctx: ctx,
      q: q
    });
    const qaItem: ChatQaItem = {
      question: q,
      answer: '',
      loading: true
    };
    setChatHistory(prev => [...prev, qaItem]);
    setChatQuestion('');
    setTimeout(() => chatScrollRef.current?.scrollTo({
      top: 999999,
      behavior: 'smooth'
    }), 50);
    try {
      const res = await ipc.invoke<string>('intelligence_query_local_llm', {
        prompt
      });
      const answer = res?.code === 0 && res?.data ? res.data : t("components.intelligence.DashboardPanel.k60");
      setChatHistory(prev => prev.map((item, i) => i === prev.length - 1 ? {
        ...item,
        answer,
        loading: false
      } : item));
    } catch (err) {
      setChatHistory(prev => prev.map((item, i) => i === prev.length - 1 ? {
        ...item,
        answer: t("components.FloatingBall.k51") + (err instanceof Error ? err.message : t("errors.unknown")),
        loading: false
      } : item));
    } finally {
      setTimeout(() => chatScrollRef.current?.scrollTo({
        top: 999999,
        behavior: 'smooth'
      }), 50);
    }
  }, [chatQuestion, buildDataContext]);
  const handleChatKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleChatSubmit();
    }
  };
  const scoreColor = (data?.productivity_score ?? 0) >= 70 ? '#7FD962' : (data?.productivity_score ?? 0) >= 40 ? '#FFB454' : '#E26D6D';
  if (loading && !data) {
    return <div className={styles.loading}>{t("components.intelligence.DashboardPanel.k61")}</div>;
  }
  if (!data) {
    return <div className={styles.emptyState}><div className={styles.emptyText}>{t("common.noData")}</div></div>;
  }

  // 始终展示仪表盘布局（无数据时显示 0，与本周/本月保持一致）

  const dailyTrendItems = data.daily_trend.map(d => [d.date, d.active_secs / 60 / 60] as const);
  const maxActiveHours = Math.max(...dailyTrendItems.map(([, v]) => v), 1);
  const topFiles = data.top_files;
  const hourlyActivity: number[] = Array(24).fill(0);
  data.hourly_heatmap.forEach(h => {
    const hourNum = parseInt(h.hour.slice(0, 2), 10);
    if (!isNaN(hourNum) && hourNum >= 0 && hourNum < 24) {
      hourlyActivity[hourNum] = h.count;
    }
  });
  return <div className={styles.panel}>
      {/* 顶部栏：周期选择 + 统计概览 */}
      <div className={styles.topBar}>
        <div className={styles.periodSelector}>
          {(['day', 'week', 'month'] as const).map(p => <button key={p} className={`${styles.periodBtn} ${period === p ? styles.periodBtnActive : ''}`} onClick={() => onPeriodChange(p)}>
              {p === 'day' ? t("components.intelligence.ActivityPanel.k22") : p === 'week' ? t("components.intelligence.DashboardPanel.k43") : t("components.intelligence.DashboardPanel.k44")}
            </button>)}
        </div>
        <span className={styles.topStat}>
          <StatCard label={t("components.intelligence.DashboardPanel.k62")} value={time.formatDurationSecs(Math.round((cumulative?.total_active_hours ?? 0) * 3600))} explanation={t("components.intelligence.DashboardPanel.k63")} />
        </span>
        <span className={styles.topStat}>
          <StatCard label={t("components.intelligence.DashboardPanel.k64")} value={cumulative?.total_operations ?? 0} explanation={t("components.intelligence.DashboardPanel.k65")} />
        </span>
      </div>

      {/* 4.1 每日简报 */}
      {(briefing || briefingLoading) && <div className={styles.briefingCard}>
          <div className={styles.briefingHeader} onClick={() => setBriefingExpanded(!briefingExpanded)}>
            <span className={styles.briefingIcon}>📰</span>
            <span className={styles.briefingTitle}>{t("components.intelligence.DashboardPanel.k66")}</span>
            {briefing && <span className={styles.briefingDate}>{briefing.date}</span>}
            <span className={styles.briefingToggle}>{briefingExpanded ? '▼' : '▶'}</span>
          </div>
          {briefingLoading ? <div className={styles.briefingLoading}>{t("components.intelligence.DashboardPanel.k67")}</div> : briefing && <>
              <div className={styles.briefingContent}>
                {briefing.briefing}
              </div>
              <div className={styles.briefingStats}>
                <span className={styles.briefingStat}>📋 {briefing.todo_completed}/{briefing.todo_count} {t("components.intelligence.ActivityPanel.k2")}</span>
                <span className={styles.briefingStat}>📖 {briefing.journal_words} {t("components.intelligence.DashboardPanel.k68")}</span>
                <span className={styles.briefingStat}>📰 {briefing.news_count} {t("components.intelligence.DashboardPanel.k69")}</span>
                <span className={styles.briefingStat}>⏱️ {briefing.timer_sessions} {t("components.intelligence.DashboardPanel.k70")}</span>
                <span className={styles.briefingStat}>🎯 {time.formatDurationSecs(briefing.total_focus_seconds)}</span>
              </div>
            </>}
        </div>}

      {/* 主内容区（红框）：左侧实时+评分 / 右侧热力图 */}
      <div className={styles.flowStateCard} style={{
      '--flow-color': flowState.color
    } as React.CSSProperties}>
        <div className={styles.flowStateLeft}>
          <span className={styles.flowStateIcon}>{flowState.icon}</span>
          <div className={styles.flowStateInfo}>
            <div className={styles.flowStateLabel} style={{
            color: flowState.color
          }}>{flowState.label}</div>
            <div className={styles.flowStateDesc}>{flowState.desc}</div>
          </div>
        </div>
        <div className={styles.flowStateTip}>💡 {flowState.tip}</div>
      </div>

      {/* 2.4 对话式查询 */}
      <div className={styles.sectionCard}>
        <div className={styles.sectionTitle}>
          <span>{t("components.intelligence.DashboardPanel.k71")}</span>
          <span className={styles.chatHint}>{t("components.intelligence.DashboardPanel.k72")}</span>
        </div>
        <div className={styles.chatQAContainer}>
          <div className={styles.chatQAMessages} ref={chatScrollRef}>
            {chatHistory.length === 0 ? <div className={styles.chatQAEmpty}>
                <div className={styles.chatQAEmptyIcon}>🔍</div>
                <p>{t("components.intelligence.DashboardPanel.k73")}</p>
                <div className={styles.chatQASuggestions}>
                  <button className={styles.chatQASugBtn} onClick={() => setChatQuestion(t("components.intelligence.DashboardPanel.k74"))}>
                    {t("components.intelligence.DashboardPanel.k74")}
                  </button>
                  <button className={styles.chatQASugBtn} onClick={() => setChatQuestion(t("components.intelligence.DashboardPanel.k75"))}>
                    {t("components.intelligence.DashboardPanel.k76")}
                  </button>
                  <button className={styles.chatQASugBtn} onClick={() => setChatQuestion(t("components.intelligence.DashboardPanel.k77"))}>
                    {t("components.intelligence.DashboardPanel.k77")}
                  </button>
                  <button className={styles.chatQASugBtn} onClick={() => setChatQuestion(t("components.intelligence.DashboardPanel.k78"))}>
                    {t("components.intelligence.DashboardPanel.k79")}
                  </button>
                </div>
              </div> : chatHistory.map((qa, i) => <div key={i} className={styles.chatQAItem}>
                  <div className={styles.chatQAQuestion}>
                    <span className={styles.chatQABadge}>{t("components.intelligence.DashboardPanel.k80")}</span>
                    <span className={styles.chatQAText}>{qa.question}</span>
                  </div>
                  <div className={styles.chatQAAnswer}>
                    <span className={styles.chatQABadgeAns}>{t("components.intelligence.DashboardPanel.k81")}</span>
                    {qa.loading ? <span className={styles.chatQALoading}>{t("components.intelligence.DashboardPanel.k82")}</span> : <span className={styles.chatQAText}>{qa.answer}</span>}
                  </div>
                </div>)}
          </div>
          <div className={styles.chatQAInputBar}>
            <textarea className={styles.chatQAInput} value={chatQuestion} onChange={e => setChatQuestion(e.target.value)} onKeyDown={handleChatKeyDown} placeholder={t("components.intelligence.DashboardPanel.k83")} rows={1} />
            <button className={styles.chatQASendBtn} onClick={handleChatSubmit} disabled={!chatQuestion.trim()}>{t("components.intelligence.DashboardPanel.k84")}</button>
          </div>
        </div>
      </div>

      <div className={styles.mainArea}>
        <div className={styles.mainLeft}>
          {/* 实时数据 — 标签和数值随周期变化 */}
          <div className={styles.realtimeRow}>
            <div className={styles.statCard}>
              <div className={styles.statLabel}>
                {period === 'day' ? t("components.intelligence.DashboardPanel.k85") : period === 'week' ? t("components.intelligence.DashboardPanel.k86") : t("components.intelligence.DashboardPanel.k87")}
              </div>
              <div className={styles.statValue} style={{
              color: '#39BAE6'
            }}>
                {period === 'day' ? realtime ? `${realtime.operations_today}` : '-' : data ? `${data.total_operations}` : '-'}
              </div>
            </div>
            <div className={styles.statCard}>
              <div className={styles.statLabel}>
                {period === 'day' ? t("components.intelligence.DashboardPanel.k88") : period === 'week' ? t("components.intelligence.DashboardPanel.k89") : t("components.intelligence.DashboardPanel.k90")}
              </div>
              <div className={styles.statValue} style={{
              color: '#39BAE6'
            }}>
                {period === 'day' ? realtime ? time.formatDurationSecs(realtime.active_duration_today_secs) : '-' : data ? time.formatDurationSecs(data.total_active_secs) : '-'}
              </div>
            </div>
          </div>
          {/* 生产力评分 */}
          <div className={styles.gaugeCard}>
            <div className={styles.gaugeTitle}>{t("components.intelligence.DashboardPanel.k91")}</div>
            {data.productivity_score > 0 ? <svg className={styles.gaugeSvg} width="140" height="80" viewBox="0 0 140 80">
                <defs>
                  <linearGradient id="scoreGrad" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" stopColor="#E26D6D" />
                    <stop offset="50%" stopColor="#FFB454" />
                    <stop offset="100%" stopColor="#7FD962" />
                  </linearGradient>
                </defs>
                <path d="M 10 70 A 60 60 0 0 1 130 70" fill="none" stroke="#1F2A35" strokeWidth="10" strokeLinecap="round" />
                <path d="M 10 70 A 60 60 0 0 1 130 70" fill="none" stroke="url(#scoreGrad)" strokeWidth="10" strokeLinecap="round" strokeDasharray={`${data.productivity_score / 100 * 188} 188`} />
                <text x="70" y="62" textAnchor="middle" fontSize="28" fontWeight="700" fill={scoreColor}>
                  {data.productivity_score.toFixed(0)}%
                </text>
              </svg> : <svg className={styles.gaugeSvg} width="140" height="80" viewBox="0 0 140 80">
                <path d="M 10 70 A 60 60 0 0 1 130 70" fill="none" stroke="#1F2A35" strokeWidth="10" strokeLinecap="round" />
                <text x="70" y="62" textAnchor="middle" fontSize="28" fontWeight="700" fill="#555">-</text>
              </svg>}
            <div className={styles.gaugeSub}>
              {data.productivity_score >= 70 ? t("components.intelligence.DashboardPanel.k92") : data.productivity_score >= 40 ? t("components.intelligence.DashboardPanel.k93") : data.productivity_score > 0 ? t("components.intelligence.DashboardPanel.k94") : t("components.intelligence.DashboardPanel.k95")}
            </div>
          </div>
        </div>

        {/* 右侧：4×6 热力图矩阵 */}
        {data.hourly_heatmap && data.hourly_heatmap.length > 0 && <div className={styles.heatmapCompactCard}>
            <div className={styles.chartTitle}>{t("components.intelligence.DashboardPanel.k96")}</div>
            <div className={styles.heatmapMatrix}>
              {Array.from({
            length: 24
          }, (_, h) => <div key={h} className={styles.heatmapCell} style={{
            background: hourlyActivity[h] > 0 ? `rgba(57, 186, 230, ${Math.min(hourlyActivity[h] / Math.max(...hourlyActivity, 1), 1) * 0.8 + 0.1})` : '#0F1419'
          }} title={t("components.intelligence.DashboardPanel.k97", {
            h: h,
            arg0: h + 1,
            arg1: hourlyActivity[h]
          })}>
                  <span className={styles.heatmapCellLabel}>{h}</span>
                </div>)}
            </div>
          </div>}
      </div>

      {/* Activity Trend Line Chart (SVG) */}
      {data.daily_trend && data.daily_trend.length > 0 && <div className={styles.chartCard}>
          <div className={styles.chartTitle}>{t("components.intelligence.DashboardPanel.k98")}</div>
          <svg className={styles.svgChart} viewBox={`0 0 ${Math.max(data.daily_trend.length * 40, 300)} 160`} height="160">
            <defs>
              <linearGradient id="chartGradient" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="#39BAE6" stopOpacity="0.3" />
                <stop offset="100%" stopColor="#39BAE6" stopOpacity="0" />
              </linearGradient>
            </defs>
            <line x1="40" y1="10" x2="40" y2="140" stroke="#2D3A45" strokeWidth="1" />
            <line x1="40" y1="140" x2={Math.max(data.daily_trend.length * 40 + 20, 300)} y2="140" stroke="#2D3A45" strokeWidth="1" />
            {(() => {
          const points = dailyTrendItems.map(([, v], i) => {
            const x = 50 + i * 38;
            const y = 140 - v / maxActiveHours * 120;
            return `${x},${y}`;
          }).join(' ');
          const areaPath = `M 50 140 ${dailyTrendItems.map(([, v], i) => {
            const x = 50 + i * 38;
            const y = 140 - v / maxActiveHours * 120;
            return `L ${x} ${y}`;
          }).join(' ')} L ${50 + (dailyTrendItems.length - 1) * 38} 140 Z`;
          return <>
                  <path d={areaPath} className={styles.chartArea} />
                  <polyline points={points} className={styles.chartLine} />
                  {dailyTrendItems.map(([, v], i) => {
              const x = 50 + i * 38;
              const y = 140 - v / maxActiveHours * 120;
              return <circle key={i} cx={x} cy={y} r="3" className={styles.chartDot} />;
            })}
                </>;
        })()}
          </svg>
        </div>}

      {/* Top Modules (CSS Bar Chart) */}
      {data.top_modules && data.top_modules.length > 0 && <div className={styles.chartCard}>
          <div className={styles.chartTitle}>{'🤖'} {t("components.intelligence.DashboardPanel.k99")}</div>
          <div className={styles.barChart}>
            {data.top_modules.slice(0, 8).map(([module, count]) => {
          const maxVal = Math.max(...data.top_modules.map(([, c]) => c), 1);
          return <div key={module} className={styles.barCol}>
                  <span className={styles.barValue}>{count}</span>
                  <div className={styles.barFill} style={{
              height: `${count / maxVal * 100}%`,
              background: `linear-gradient(180deg, #39BAE6, #1F5A7A)`
            }} />
                  <span className={styles.barLabel}>{module.length > 6 ? module.slice(0, 6) + '…' : module}</span>
                </div>;
        })}
          </div>
        </div>}

      {/* Top Files */}
      {data.top_files && data.top_files.length > 0 && <div className={styles.sectionCard}>
          <div className={styles.sectionTitle}>{t("components.intelligence.DashboardPanel.k100")}</div>
          <div className={styles.fileList}>
            {topFiles.map((f, i) => <div key={`${f.file_name}-${i}`} className={styles.fileItem}>
                <span className={styles.fileRank}>#{i + 1}</span>
                <div className={styles.fileInfo}>
                  <span className={styles.fileName}>{f.file_name}</span>
                  {f.kb_path && f.kb_path !== f.file_name && <span className={styles.fileKbPath} title={t("components.intelligence.DashboardPanel.k101")}>{f.kb_path}</span>}
                  {f.local_path && <span className={styles.fileLocalPath} title={t("components.intelligence.DashboardPanel.k102")}>{f.local_path}</span>}
                </div>
              </div>)}
          </div>
        </div>}

      {/* 时间黑洞分析 */}
      {activityStats && activityStats.totalOperations > 0 && (() => {
      const moduleEntries = Object.entries(activityStats.byModule || {}).sort(([, a], [, b]) => b - a).slice(0, 6);
      const opEntries = Object.entries(activityStats.byOperation || {}).sort(([, a], [, b]) => b - a).slice(0, 5);
      const maxModuleCount = Math.max(...moduleEntries.map(([, c]) => c), 1);
      const maxOpCount = Math.max(...opEntries.map(([, c]) => c), 1);
      const moduleNames: Record<string, string> = {
        knowledge_base: t("components.intelligence.ActivityPanel.k1"),
        spyglass: t("components.intelligence.DashboardPanel.k103"),
        ai_chat: t("components.intelligence.ActivityPanel.k7"),
        terminal: t("components.intelligence.ActivityPanel.k8"),
        resume: t("components.intelligence.ActivityPanel.k5"),
        quote: t("components.intelligence.ActivityPanel.k6"),
        todo: t("components.intelligence.ActivityPanel.k2"),
        journal: t("components.intelligence.ActivityPanel.k3"),
        timer: t("components.intelligence.DashboardPanel.k104"),
        game: t("components.AddExtensionModal.k11"),
        search: t("common.search"),
        profile: t("components.intelligence.DashboardPanel.k105"),
        home: t("components.intelligence.DashboardPanel.k106"),
        xin: t("components.FloatingXin.k26")
      };
      return <div className={styles.sectionCard}>
            <div className={styles.sectionTitle}>{t("components.intelligence.DashboardPanel.k107")}</div>
            <p className={styles.blackHoleDesc}>
              {period === 'day' ? t("components.intelligence.ActivityPanel.k22") : period === 'week' ? t("components.intelligence.DashboardPanel.k43") : t("components.intelligence.DashboardPanel.k44")}{t("components.GroupChatOrchestrationPanel.k26")} {activityStats.totalOperations} {t("components.intelligence.DashboardPanel.k108")}
            </p>
            <div className={styles.blackHoleSection}>
              <span className={styles.blackHoleSubTitle}>{t("components.intelligence.DashboardPanel.k109")}</span>
              {moduleEntries.map(([mod, count]) => <div key={mod} className={styles.blackHoleRow}>
                  <span className={styles.blackHoleLabel}>{moduleNames[mod] || mod}</span>
                  <div className={styles.blackHoleBar}>
                    <div className={styles.blackHoleBarFill} style={{
                width: `${count / maxModuleCount * 100}%`
              }} />
                  </div>
                  <span className={styles.blackHoleCount}>{count}</span>
                </div>)}
            </div>
            {opEntries.length > 0 && <div className={styles.blackHoleSection}>
                <span className={styles.blackHoleSubTitle}>{t("components.intelligence.DashboardPanel.k110")}</span>
                {opEntries.map(([op, count]) => <div key={op} className={styles.blackHoleRow}>
                    <span className={styles.blackHoleLabel}>{op}</span>
                    <div className={styles.blackHoleBar}>
                      <div className={styles.blackHoleBarFillOp} style={{
                width: `${count / maxOpCount * 100}%`
              }} />
                    </div>
                    <span className={styles.blackHoleCount}>{count}</span>
                  </div>)}
              </div>}
          </div>;
    })()}

      {/* 2.2 下钻式活动时间线 */}
      <div className={styles.sectionCard}>
        <div className={styles.sectionTitle}>
          <span>{t("components.intelligence.DashboardPanel.k111")}</span>
          <select className={styles.timelineFilter} value={logFilterModule} onChange={e => {
          setLogFilterModule(e.target.value);
          setExpandedLogId(null);
        }}>
            <option value="">{t("components.intelligence.DashboardPanel.k112")}</option>
            <option value="knowledge_base">{t("components.intelligence.ActivityPanel.k1")}</option>
            <option value="spyglass">{t("components.intelligence.DashboardPanel.k103")}</option>
            <option value="ai_chat">{t("components.intelligence.ActivityPanel.k7")}</option>
            <option value="terminal">{t("components.intelligence.ActivityPanel.k8")}</option>
            <option value="resume">{t("components.intelligence.ActivityPanel.k5")}</option>
            <option value="quote">{t("components.intelligence.ActivityPanel.k6")}</option>
            <option value="todo">{t("components.intelligence.ActivityPanel.k2")}</option>
            <option value="journal">{t("components.intelligence.ActivityPanel.k3")}</option>
            <option value="timer">{t("components.intelligence.DashboardPanel.k104")}</option>
            <option value="home">{t("components.intelligence.DashboardPanel.k106")}</option>
            <option value="profile">{t("components.intelligence.DashboardPanel.k105")}</option>
            <option value="search">{t("common.search")}</option>
            <option value="xin">{t("components.FloatingXin.k26")}</option>
            <option value="game">{t("components.AddExtensionModal.k11")}</option>
          </select>
        </div>
        {activityLogs.length === 0 ? <div className={styles.timelineEmpty}>{t("components.intelligence.ActivityPanel.k33")}</div> : <div className={styles.timelineList}>
            {activityLogs.map((log, idx) => {
          const isExpanded = expandedLogId === log.id;
          const isLast = idx === 0;
          const moduleNames: Record<string, string> = {
            knowledge_base: t("components.intelligence.ActivityPanel.k1"),
            spyglass: t("components.intelligence.DashboardPanel.k103"),
            ai_chat: t("components.intelligence.ActivityPanel.k7"),
            terminal: t("components.intelligence.ActivityPanel.k8"),
            resume: t("components.intelligence.ActivityPanel.k5"),
            quote: t("components.intelligence.ActivityPanel.k6"),
            todo: t("components.intelligence.ActivityPanel.k2"),
            journal: t("components.intelligence.ActivityPanel.k3"),
            timer: t("components.intelligence.DashboardPanel.k104"),
            game: t("components.AddExtensionModal.k11"),
            search: t("common.search"),
            profile: t("components.intelligence.DashboardPanel.k105"),
            home: t("components.intelligence.DashboardPanel.k106"),
            xin: t("components.FloatingXin.k26")
          };
          const moduleName = moduleNames[log.module] || log.module;
          // 修复时区偏移：无时区标识的时间戳统一视为 UTC
          const normalized = log.timestamp.replace(' ', 'T');
          const ts = (normalized.includes('Z') || normalized.includes('+'))
            ? new Date(normalized)
            : new Date(normalized + 'Z');
          const timeStr = `${String(ts.getHours()).padStart(2, '0')}:${String(ts.getMinutes()).padStart(2, '0')}:${String(ts.getSeconds()).padStart(2, '0')}`;
          return <div key={log.id} className={`${styles.timelineItem} ${isExpanded ? styles.timelineItemExpanded : ''} ${isLast ? styles.timelineItemLatest : ''}`} onClick={() => setExpandedLogId(isExpanded ? null : log.id)}>
                  <div className={styles.timelineDot} />
                  <div className={styles.timelineContent}>
                    <div className={styles.timelineHeader}>
                      <span className={styles.timelineTime}>{timeStr}</span>
                      <span className={styles.timelineModule}>{moduleName}</span>
                      <span className={styles.timelineOp}>{log.operation}</span>
                      {log.durationSecs > 0 && <span className={styles.timelineDur}>{time.formatDurationSecs(log.durationSecs)}</span>}
                      {isLast && <span className={styles.timelineLatestBadge}>{t("components.intelligence.DashboardPanel.k113")}</span>}
                    </div>
                    {isExpanded && log.detail && <div className={styles.timelineDetail}>{log.detail}</div>}
                    {isExpanded && log.remark && <div className={styles.timelineRemark}>{t("components.intelligence.DashboardPanel.k114")} {log.remark}</div>}
                    {isExpanded && !log.detail && !log.remark && <div className={styles.timelineDetailEmpty}>{t("components.intelligence.DashboardPanel.k115")}</div>}
                  </div>
                </div>;
        })}
          </div>}
      </div>

      {/* 2.5 生产力画像 */}
      <div className={styles.sectionCard}>
        <div className={styles.sectionTitle}>
          <span>{t("components.intelligence.DashboardPanel.k116")}</span>
          <span className={styles.chatHint}>{t("components.intelligence.DashboardPanel.k117")}</span>
        </div>
        {portraitTags.length === 0 ? <div className={styles.timelineEmpty}>{t("components.intelligence.DashboardPanel.k118")}</div> : <div className={styles.portraitTagsGrid}>
            {portraitTags.map((tag, i) => <div key={i} className={styles.portraitTagCard} style={{
          '--tag-color': tag.color
        } as React.CSSProperties}>
                <span className={styles.portraitTagIcon}>{tag.icon}</span>
                <div className={styles.portraitTagInfo}>
                  <span className={styles.portraitTagLabel}>{tag.label}</span>
                  <span className={styles.portraitTagDesc}>{tag.desc}</span>
                </div>
              </div>)}
          </div>}
      </div>

    </div>;
}