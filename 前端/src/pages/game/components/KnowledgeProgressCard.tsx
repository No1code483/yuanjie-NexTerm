import { t } from "i18next";
/**
 * KnowledgeProgressCard - 知识领域积分卡（Task 7.6）
 *
 * 展示 12 知识领域的积分进度：
 *   - 每行：领域名称 + 当前积分 + 等级（1-8）+ 进度条 + 近7天 sparkline
 *   - 进度计算：(points - thresholds[level-1]) / (thresholds[level] - thresholds[level-1])
 *   - 等级 8（≥1,000,000）视为已满级，进度固定 100%
 *   - 领域↔建筑大类对应关系提示（小字显示在领域名下方）
 *   - 底部统计：日均/周累（基于 trend 数据）
 *
 * Props：{ domains, progress, loading, trend? }
 *
 * 注：sparkline 为纯 SVG 自绘（无图表库依赖），数据源为 game_get_points_trend IPC。
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react';
import type { CSSProperties } from 'react';
import type { GameKnowledgeDomain, GameKnowledgeProgress, GameKnowledgeDomainId, GameBuildingCategory, PointsTrend } from '@/types/game';
import { GAME_DOMAIN_LEVEL_THRESHOLDS, GAME_DOMAIN_IDS } from '@/types/game';
interface KnowledgeProgressCardProps {
  domains: GameKnowledgeDomain[];
  progress: GameKnowledgeProgress[];
  loading: boolean;
  /** 近 N 天积分趋势（sparkline 数据源）；未传入时不渲染趋势图 */
  trend?: PointsTrend | null;
}

/** 建筑大类中文名映射（领域↔建筑大类提示用）。 */
const BUILDING_CATEGORY_NAMES: Record<GameBuildingCategory, string> = {
  house: '房屋',
  town: '城镇',
  city: '城池',
  kingdom: '王国',
  palace: '宫殿',
  technology: '科技',
  sect: '宗门',
  immortal: '修仙',
};

/**
 * Sparkline - 纯 SVG 自绘迷你折线图。
 *
 * 无图表库依赖，固定宽度，自适应数据范围。
 * 空数据或全零数据渲染为平直线。
 */
function Sparkline({
  data,
  width = 64,
  height = 20,
  color = 'var(--nt-primary)'
}: {
  data: number[];
  width?: number;
  height?: number;
  color?: string;
}) {
  if (data.length === 0) {
    return <svg width={width} height={height} style={{ display: 'block' }} />;
  }
  const max = Math.max(...data, 0);
  const min = Math.min(...data, 0);
  const range = max - min || 1;
  const step = data.length > 1 ? width / (data.length - 1) : 0;
  const points = data
    .map((v, i) => {
      const x = i * step;
      const y = height - ((v - min) / range) * (height - 2) - 1;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(' ');
  return (
    <svg width={width} height={height} style={{ display: 'block', flexShrink: 0 }}>
      <polyline
        points={points}
        fill="none"
        stroke={color}
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        style={{ filter: `drop-shadow(0 0 2px ${color})` }}
      />
    </svg>
  );
}

/** 计算单个领域的当前等级进度百分比（0-100） */
function calcLevelProgress(points: number, level: number): number {
  // 等级 8 视为满级
  if (level >= 8) return 100;

  // thresholds 索引：thresholds[level-1] = 当前等级下界，thresholds[level] = 下一等级下界
  const lowerBound = GAME_DOMAIN_LEVEL_THRESHOLDS[level - 1] ?? 0;
  const upperBound = GAME_DOMAIN_LEVEL_THRESHOLDS[level] ?? lowerBound;
  const range = upperBound - lowerBound;
  if (range <= 0) return 100;
  const pct = (points - lowerBound) / range * 100;
  return Math.max(0, Math.min(100, pct));
}
export default function KnowledgeProgressCard({
  domains,
  progress,
  loading,
  trend
}: KnowledgeProgressCardProps) {
  // 合并 domains + progress + trend 为渲染数据
  const rows = useMemo(() => {
    // 以 domains 为顺序基准，没有 progress 的领域默认 0 积分
    const progressMap = new Map<GameKnowledgeDomainId, GameKnowledgeProgress>();
    for (const p of progress) {
      progressMap.set(p.domain_id, p);
    }

    // 优先按 domains 顺序，若 domains 为空则回退到 GAME_DOMAIN_IDS
    const orderedDomains = domains.length > 0 ? [...domains].sort((a, b) => a.sort_order - b.sort_order) : GAME_DOMAIN_IDS.map(id => ({
      id,
      name: id,
      name_en: id,
      description: '',
      building_category: 'house' as const,
      sort_order: 0
    }));
    return orderedDomains.map(d => {
      const p = progressMap.get(d.id);
      const points = p?.points ?? 0;
      const level = p?.level ?? 1;
      const progressPct = calcLevelProgress(points, level);
      // 近7天趋势数据（trend.trend[domain_id] 为 number[]，缺日补 0）
      const trendSeries = trend?.trend?.[d.id] ?? [];
      return {
        domain: d,
        points,
        level,
        progressPct,
        trendSeries,
        buildingCategoryName: BUILDING_CATEGORY_NAMES[d.building_category] ?? d.building_category
      };
    });
  }, [domains, progress, trend]);

  // 总积分
  const totalPoints = useMemo(() => rows.reduce((sum, r) => sum + r.points, 0), [rows]);

  // 日均/周累统计（基于 trend.daily_totals）
  const stats = useMemo(() => {
    if (!trend || trend.daily_totals.length === 0) {
      return { dailyAvg: 0, weeklyTotal: 0, hasData: false };
    }
    const weeklyTotal = trend.daily_totals.reduce((sum, v) => sum + v, 0);
    const dailyAvg = Math.round(weeklyTotal / trend.daily_totals.length);
    return { dailyAvg, weeklyTotal, hasData: true };
  }, [trend]);

  // 加载中骨架屏
  if (loading) {
    return <div style={cardStyle}>
        <h3 style={titleStyle}>{t("game.components.KnowledgeProgressCard.k1")}</h3>
        <div style={{
        ...skeletonStyle,
        width: '50%',
        height: '14px'
      }} />
        {[1, 2, 3, 4].map(i => <div key={i} style={{
        ...skeletonStyle,
        height: '32px'
      }} />)}
      </div>;
  }
  return <div style={cardStyle}>
      <h3 style={titleStyle}>{t("game.components.KnowledgeProgressCard.k1")}</h3>

      {/* 总积分 */}
      <div style={totalRowStyle}>
        <span style={totalLabelStyle}>{t("game.components.KnowledgeProgressCard.k2")}</span>
        <span style={totalValueStyle}>{totalPoints.toLocaleString()}</span>
      </div>

      {/* 12 领域进度条 */}
      <div style={domainListStyle}>
        {rows.map(({
        domain,
        points,
        level,
        progressPct,
        trendSeries,
        buildingCategoryName
      }) => <div key={domain.id} style={domainItemStyle}>
            <div style={domainHeaderStyle}>
              <div style={domainNameColStyle}>
                <span style={domainNameStyle}>{domain.name}</span>
                <span style={domainCategoryHintStyle}>▸ {buildingCategoryName}</span>
              </div>
              <div style={domainMetaStyle}>
                {trendSeries.length > 0 && <Sparkline data={trendSeries} color={levelBadgeColor(level)} />}
                <span style={pointsStyle}>{points.toLocaleString()}</span>
                <span style={levelBadgeStyle(level)}>Lv.{level}</span>
              </div>
            </div>
            <div style={progressBarStyle}>
              <div style={{
            ...progressFillStyle,
            width: `${progressPct}%`
          }} />
            </div>
          </div>)}
      </div>

      {/* 底部统计：日均/周累 */}
      {stats.hasData && <div style={statsRowStyle}>
          <div style={statsItemStyle}>
            <span style={statsLabelStyle}>{t("game.components.KnowledgeProgressCard.k3")}</span>
            <span style={statsValueStyle}>{stats.dailyAvg.toLocaleString()}</span>
          </div>
          <div style={statsItemStyle}>
            <span style={statsLabelStyle}>{t("game.components.KnowledgeProgressCard.k4")}</span>
            <span style={statsValueStyle}>{stats.weeklyTotal.toLocaleString()}</span>
          </div>
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
const totalRowStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '8px 12px',
  background: 'rgba(0, 240, 255, 0.06)',
  border: '1px solid var(--nt-border-color)',
  borderRadius: 'var(--nt-radius-md)'
};
const totalLabelStyle: CSSProperties = {
  fontSize: '13px',
  color: 'var(--nt-text-secondary)',
  fontFamily: 'var(--nt-font-mono)'
};
const totalValueStyle: CSSProperties = {
  fontSize: '18px',
  fontWeight: 600,
  color: 'var(--nt-primary)',
  fontFamily: 'var(--nt-font-mono)',
  textShadow: '0 0 8px rgba(0, 240, 255, 0.4)'
};
const domainListStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  overflowY: 'auto',
  maxHeight: '320px',
  paddingRight: '4px'
};
const domainItemStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px'
};
const domainHeaderStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '8px'
};
const domainNameStyle: CSSProperties = {
  fontSize: '13px',
  color: 'var(--nt-text-primary)',
  fontFamily: 'var(--nt-font-chinese)'
};
const domainNameColStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '1px',
  minWidth: 0
};
const domainCategoryHintStyle: CSSProperties = {
  fontSize: '10px',
  color: 'var(--nt-text-muted)',
  fontFamily: 'var(--nt-font-mono)',
  opacity: 0.7
};
const domainMetaStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px'
};
const pointsStyle: CSSProperties = {
  fontSize: '12px',
  color: 'var(--nt-text-secondary)',
  fontFamily: 'var(--nt-font-mono)'
};
/** 等级→颜色映射（sparkline 与 levelBadgeStyle 共用）。 */
const levelBadgeColor = (level: number): string => {
  const colorMap: Record<number, string> = {
    1: 'var(--nt-text-muted)',
    2: 'var(--nt-text-secondary)',
    3: 'var(--nt-secondary)',
    4: 'var(--nt-primary)',
    5: 'var(--nt-warning)',
    6: 'var(--nt-accent)',
    7: 'var(--nt-accent-hover)',
    8: 'var(--nt-warning-hover)'
  };
  return colorMap[level] ?? 'var(--nt-text-muted)';
};
const levelBadgeStyle = (level: number): CSSProperties => {
  const color = levelBadgeColor(level);
  return {
    padding: '2px 8px',
    fontSize: '11px',
    color,
    background: `${color === 'var(--nt-text-muted)' ? 'rgba(106, 106, 138, 0.1)' : 'rgba(0, 240, 255, 0.1)'}`,
    border: `1px solid ${color}80`,
    borderRadius: 'var(--nt-radius-sm)',
    fontFamily: 'var(--nt-font-mono)',
    fontWeight: 600
  };
};
const statsRowStyle: CSSProperties = {
  display: 'flex',
  gap: '16px',
  marginTop: 'auto',
  paddingTop: '10px',
  borderTop: '1px solid var(--nt-border-subtle)'
};
const statsItemStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '2px',
  flex: 1
};
const statsLabelStyle: CSSProperties = {
  fontSize: '11px',
  color: 'var(--nt-text-muted)',
  fontFamily: 'var(--nt-font-mono)'
};
const statsValueStyle: CSSProperties = {
  fontSize: '15px',
  color: 'var(--nt-primary)',
  fontFamily: 'var(--nt-font-mono)',
  fontWeight: 600,
  textShadow: '0 0 6px rgba(0, 240, 255, 0.4)'
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
  background: 'linear-gradient(90deg, var(--nt-primary), var(--nt-secondary))',
  borderRadius: '2px',
  transition: 'width 0.4s ease',
  boxShadow: '0 0 6px rgba(0, 240, 255, 0.5)'
};
const skeletonStyle: CSSProperties = {
  background: 'linear-gradient(90deg, var(--nt-text-muted) 0%, rgba(106, 106, 138, 0.2) 50%, var(--nt-text-muted) 100%)',
  backgroundSize: '200% 100%',
  borderRadius: 'var(--nt-radius-sm)',
  animation: 'skeletonPulse 1.5s ease-in-out infinite'
};