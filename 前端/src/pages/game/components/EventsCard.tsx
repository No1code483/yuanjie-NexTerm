import { t } from "i18next";
/**
 * EventsCard - 事件与任务卡（Task 7.7）
 *
 * 展示：
 *   - 即将完工建筑列表（upcoming_buildings，前 5 个）
 *   - 突破提示（breakthrough_hint：可突破高亮 / 原因 / 冷却倒计时）
 *   - 近期事件流（recent_events，前 10 个，标题 + 描述 + 相对时间）
 *
 * Props：{ events: EventsAndTasks | null; loading: boolean }
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react';
import type { CSSProperties } from 'react';
import type { EventsAndTasks } from '@/types/game';
interface EventsCardProps {
  events: EventsAndTasks | null;
  loading: boolean;
  /** 突破考验点击回调；传入时「可突破」提示框变为可点击按钮，触发云端 API 考验流程 */
  onBreakthroughClick?: () => void;
}

/** 相对时间格式化（X 分钟前 / X 小时前 / X 天前） */
function formatRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  if (diff < 0) return t("lib.utils.k1");
  const seconds = Math.floor(diff / 1000);
  if (seconds < 60) return t("lib.utils.k1");
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return t("game.components.EventsCard.k1", {
    minutes: minutes
  });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return t("game.components.EventsCard.k2", {
    hours: hours
  });
  const days = Math.floor(hours / 24);
  if (days < 30) return t("game.components.EventsCard.k3", {
    days: days
  });
  const months = Math.floor(days / 30);
  if (months < 12) return t("game.components.EventsCard.k4", {
    months: months
  });
  const years = Math.floor(months / 12);
  return t("game.components.EventsCard.k5", {
    years: years
  });
}

/** 冷却剩余时间格式化（X 小时 Y 分） */
function formatCooldown(ms: number): string {
  if (ms <= 0) return '';
  const totalMinutes = Math.floor(ms / 60000);
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (hours > 0) return t("game.components.EventsCard.k6", {
    hours: hours,
    minutes: minutes
  });
  if (minutes > 0) return t("game.components.EventsCard.k7", {
    minutes: minutes
  });
  const seconds = Math.floor(ms / 1000);
  return t("game.components.EventsCard.k8", {
    seconds: seconds
  });
}
export default function EventsCard({
  events,
  loading,
  onBreakthroughClick
}: EventsCardProps) {
  // 派生展示数据
  const data = useMemo(() => {
    if (!events) {
      return {
        upcoming: [],
        hint: null as null | {
          can_breakthrough: boolean;
          reason: string;
          cooldown_remaining: number;
        },
        recent: []
      };
    }
    return {
      upcoming: events.upcoming_buildings.slice(0, 5),
      hint: events.breakthrough_hint,
      recent: events.recent_events.slice(0, 10)
    };
  }, [events]);

  // 加载中骨架屏
  if (loading || !events) {
    return <div style={cardStyle}>
        <h3 style={titleStyle}>{t("game.components.EventsCard.k9")}</h3>
        <div style={{
        ...skeletonStyle,
        height: '40px'
      }} />
        <div style={{
        ...skeletonStyle,
        height: '40px'
      }} />
        <div style={{
        ...skeletonStyle,
        height: '40px'
      }} />
      </div>;
  }
  const {
    upcoming,
    hint,
    recent
  } = data;
  return <div style={cardStyle}>
      <h3 style={titleStyle}>{t("game.components.EventsCard.k9")}</h3>

      {/* 突破提示 */}
      {hint && (() => {
        const clickable = hint.can_breakthrough && !!onBreakthroughClick;
        const Tag = clickable ? 'button' : 'div';
        return <Tag
            style={{
              ...hintBoxStyle(hint.can_breakthrough),
              ...(clickable ? clickableHintStyle : null)
            }}
            onClick={clickable ? onBreakthroughClick : undefined}
            type={clickable ? 'button' : undefined}
          >
            <div style={hintHeaderStyle}>
              <span style={hintIconStyle(hint.can_breakthrough)}>
                {hint.can_breakthrough ? '✨' : '⏳'}
              </span>
              <span style={hintTitleStyle(hint.can_breakthrough)}>
                {hint.can_breakthrough ? t("game.components.EventsCard.k10") : t("game.components.EventsCard.k11")}
              </span>
              {clickable && <span style={hintActionStyle}>›</span>}
            </div>
            <div style={hintBodyStyle}>
              {hint.can_breakthrough ? t("game.components.EventsCard.k12") : hint.reason || (hint.cooldown_remaining > 0 ? t("game.components.EventsCard.k13", {
            arg0: formatCooldown(hint.cooldown_remaining)
          }) : t("game.components.EventsCard.k14"))}
            </div>
          </Tag>;
      })()}

      {/* 即将完工建筑 */}
      {upcoming.length > 0 && <>
          <div style={sectionLabelStyle}>{t("game.components.EventsCard.k15")}</div>
          <div style={upcomingListStyle}>
            {upcoming.map(b => <div key={b.building_id} style={upcomingItemStyle}>
                <div style={upcomingHeaderStyle}>
                  <span style={upcomingNameStyle}>{b.name}</span>
                  <span style={upcomingProgressTextStyle}>
                    {b.progress.toFixed(1)}%
                  </span>
                </div>
                <div style={progressBarStyle}>
                  <div style={{
              ...progressFillStyle,
              width: `${Math.min(100, Math.max(0, b.progress))}%`
            }} />
                </div>
                {b.remaining_points > 0 && <div style={remainingStyle}>
                    {t("game.components.EventsCard.k16")}{b.remaining_points.toLocaleString()}
                  </div>}
              </div>)}
          </div>
        </>}

      {/* 近期事件流 */}
      {recent.length > 0 && <>
          <div style={sectionLabelStyle}>{t("game.components.EventsCard.k17")}</div>
          <div style={eventListStyle}>
            {recent.map(e => <div key={e.event_id} style={eventItemStyle}>
                <div style={eventHeaderStyle}>
                  <span style={eventTitleStyle}>· {e.title}</span>
                  <span style={eventTimeStyle}>{formatRelativeTime(e.timestamp)}</span>
                </div>
                {e.description && <div style={eventDescStyle}>{e.description}</div>}
              </div>)}
          </div>
        </>}

      {/* 空状态 */}
      {upcoming.length === 0 && recent.length === 0 && <div style={emptyStyle}>
          {t("game.components.EventsCard.k18")}<br />
          <span style={{
        fontSize: '12px',
        opacity: 0.7
      }}>
            {t("game.components.EventsCard.k19")}
          </span>
        </div>}
    </div>;
}

// ===== 内联样式 =====

const cardStyle: CSSProperties = {
  padding: '20px',
  background: 'var(--nt-bg-secondary)',
  border: '1px solid var(--nt-border-color)',
  borderRadius: 'var(--nt-radius-lg)',
  backdropFilter: 'blur(10px)',
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
  minHeight: '280px',
  transition: 'all 0.3s'
};
const titleStyle: CSSProperties = {
  margin: 0,
  fontFamily: 'var(--nt-font-chinese)',
  fontSize: '18px',
  color: 'var(--nt-primary)',
  textShadow: '0 0 8px rgba(0, 240, 255, 0.4)',
  borderBottom: '1px solid var(--nt-border-subtle)',
  paddingBottom: '8px'
};
const sectionLabelStyle: CSSProperties = {
  fontSize: '12px',
  color: 'var(--nt-text-secondary)',
  fontFamily: 'var(--nt-font-mono)',
  marginTop: '4px',
  letterSpacing: '0.5px'
};

// === 突破提示 ===
const hintBoxStyle = (canBreakthrough: boolean): CSSProperties => ({
  padding: '10px 12px',
  background: canBreakthrough ? 'rgba(255, 215, 0, 0.08)' : 'rgba(106, 106, 138, 0.06)',
  border: `1px solid ${canBreakthrough ? 'rgba(255, 215, 0, 0.4)' : 'rgba(106, 106, 138, 0.3)'}`,
  borderRadius: 'var(--nt-radius-md)',
  display: 'flex',
  flexDirection: 'column',
  gap: '4px'
});
/** 可点击的突破提示样式（覆盖 button 默认样式 + hover 高亮）。 */
const clickableHintStyle: CSSProperties = {
  cursor: 'pointer',
  textAlign: 'left',
  width: '100%',
  fontFamily: 'inherit',
  fontSize: 'inherit',
  color: 'inherit',
  outline: 'none',
  transition: 'all var(--nt-transition-normal)'
};
const hintActionStyle: CSSProperties = {
  marginLeft: 'auto',
  fontSize: '16px',
  color: 'var(--nt-warning)',
  fontWeight: 600
};
const hintHeaderStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px'
};
const hintIconStyle = (canBreakthrough: boolean): CSSProperties => ({
  fontSize: '16px',
  filter: canBreakthrough ? 'drop-shadow(0 0 4px rgba(255, 215, 0, 0.6))' : 'none'
});
const hintTitleStyle = (canBreakthrough: boolean): CSSProperties => ({
  fontSize: '14px',
  fontWeight: 600,
  color: canBreakthrough ? 'var(--nt-warning)' : 'var(--nt-text-secondary)',
  fontFamily: 'var(--nt-font-chinese)'
});
const hintBodyStyle: CSSProperties = {
  fontSize: '12px',
  color: 'var(--nt-text-secondary)',
  fontFamily: 'var(--nt-font-mono)',
  lineHeight: 1.5
};

// === 即将完工 ===
const upcomingListStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px'
};
const upcomingItemStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
  padding: '8px 10px',
  background: 'rgba(0, 240, 255, 0.04)',
  border: '1px solid var(--nt-border-subtle)',
  borderRadius: 'var(--nt-radius-md)'
};
const upcomingHeaderStyle: CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  gap: '8px'
};
const upcomingNameStyle: CSSProperties = {
  fontSize: '13px',
  color: 'var(--nt-text-primary)',
  fontFamily: 'var(--nt-font-chinese)'
};
const upcomingProgressTextStyle: CSSProperties = {
  fontSize: '11px',
  color: 'var(--nt-warning)',
  fontFamily: 'var(--nt-font-mono)',
  fontWeight: 600
};
const remainingStyle: CSSProperties = {
  fontSize: '11px',
  color: 'var(--nt-text-muted)',
  fontFamily: 'var(--nt-font-mono)'
};
const progressBarStyle: CSSProperties = {
  width: '100%',
  height: '4px',
  background: 'var(--nt-gray-medium)',
  borderRadius: '2px',
  overflow: 'hidden'
};
const progressFillStyle: CSSProperties = {
  height: '100%',
  background: 'linear-gradient(90deg, var(--nt-warning), var(--nt-primary))',
  borderRadius: '2px',
  transition: 'width 0.4s ease',
  boxShadow: '0 0 6px rgba(255, 215, 0, 0.4)'
};

// === 近期事件 ===
const eventListStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
  maxHeight: '280px',
  overflowY: 'auto',
  paddingRight: '4px'
};
const eventItemStyle: CSSProperties = {
  padding: '6px 10px',
  background: 'rgba(176, 38, 255, 0.04)',
  borderLeft: '2px solid var(--nt-secondary)',
  borderRadius: '0 var(--nt-radius-md) var(--nt-radius-md) 0'
};
const eventHeaderStyle: CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  gap: '8px'
};
const eventTitleStyle: CSSProperties = {
  fontSize: '12px',
  color: 'var(--nt-text-primary)',
  fontFamily: 'var(--nt-font-chinese)'
};
const eventTimeStyle: CSSProperties = {
  fontSize: '10px',
  color: 'var(--nt-text-muted)',
  fontFamily: 'var(--nt-font-mono)',
  whiteSpace: 'nowrap'
};
const eventDescStyle: CSSProperties = {
  fontSize: '11px',
  color: 'var(--nt-text-secondary)',
  fontFamily: 'var(--nt-font-mono)',
  marginTop: '2px',
  lineHeight: 1.4,
  opacity: 0.85
};
const emptyStyle: CSSProperties = {
  textAlign: 'center',
  padding: '24px',
  color: 'var(--nt-text-muted)',
  fontFamily: 'var(--nt-font-chinese)',
  fontSize: '14px',
  lineHeight: 1.6
};
const skeletonStyle: CSSProperties = {
  background: 'linear-gradient(90deg, var(--nt-text-muted) 0%, rgba(106, 106, 138, 0.2) 50%, var(--nt-text-muted) 100%)',
  backgroundSize: '200% 100%',
  borderRadius: 'var(--nt-radius-sm)',
  animation: 'skeletonPulse 1.5s ease-in-out infinite'
};