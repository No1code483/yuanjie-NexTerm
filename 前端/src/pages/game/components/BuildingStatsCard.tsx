import { t } from "i18next";
/**
 * BuildingStatsCard - 建筑统计卡（Task 7.5）
 *
 * 展示：
 *   - 顶部统计行：总数 / 已完成数 / 建造中数
 *   - 8 大类分类统计网格
 *   - 建造中建筑列表（前 5 个，显示名称 + 进度条）
 *
 * Props：{ buildings: GameBuilding[]; loading: boolean }
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react';
import type { CSSProperties } from 'react';
import type { GameBuilding, GameBuildingCategory } from '@/types/game';
import { GAME_BUILDING_CATEGORIES } from '@/types/game';

// ===== 8 大类中文名映射 =====
const CATEGORY_NAMES: Record<GameBuildingCategory, string> = {
  house: t("game.components.BuildingStatsCard.k1"),
  town: t("game.components.BuildingStatsCard.k2"),
  city: t("game.components.BuildingStatsCard.k3"),
  kingdom: t("game.components.BuildingStatsCard.k4"),
  palace: t("game.components.BuildingStatsCard.k5"),
  technology: t("game.components.BuildingStatsCard.k6"),
  sect: t("game.components.BuildingStatsCard.k7"),
  immortal: t("game.components.BuildingStatsCard.k8")
};

// 8 大类图标（emoji 占位，阶段8 替换为 3D 模型）
const CATEGORY_ICONS: Record<GameBuildingCategory, string> = {
  house: '🏠',
  town: '🏘️',
  city: '🏯',
  kingdom: '🏰',
  palace: '🏛️',
  technology: '⚙️',
  sect: '⛩️',
  immortal: '🏔️'
};
interface BuildingStatsCardProps {
  buildings: GameBuilding[];
  loading: boolean;
}
export default function BuildingStatsCard({
  buildings,
  loading
}: BuildingStatsCardProps) {
  // 派生数据
  const stats = useMemo(() => {
    const total = buildings.length;
    const completed = buildings.filter(b => b.status === 'completed').length;
    const building = buildings.filter(b => b.status === 'building').length;
    const planning = buildings.filter(b => b.status === 'planning').length;

    // 8 大类分类统计
    const categoryStats = GAME_BUILDING_CATEGORIES.reduce((acc, cat) => {
      acc[cat] = buildings.filter(b => b.building_category === cat).length;
      return acc;
    }, {} as Record<GameBuildingCategory, number>);

    // 建造中建筑（前 5 个，按进度降序）
    const buildingList = buildings.filter(b => b.status === 'building').sort((a, b) => b.build_progress - a.build_progress).slice(0, 5);
    return {
      total,
      completed,
      building,
      planning,
      categoryStats,
      buildingList
    };
  }, [buildings]);

  // 加载中骨架屏
  if (loading) {
    return <div style={cardStyle}>
        <h3 style={titleStyle}>{t("game.components.BuildingStatsCard.k9")}</h3>
        <div style={statsRowStyle}>
          {[1, 2, 3].map(i => <div key={i} style={{
          ...skeletonStyle,
          height: '32px',
          flex: 1
        }} />)}
        </div>
        <div style={categoryGridStyle}>
          {GAME_BUILDING_CATEGORIES.map(cat => <div key={cat} style={{
          ...skeletonStyle,
          height: '40px'
        }} />)}
        </div>
      </div>;
  }
  const {
    total,
    completed,
    building,
    planning,
    categoryStats,
    buildingList
  } = stats;
  return <div style={cardStyle}>
      <h3 style={titleStyle}>{t("game.components.BuildingStatsCard.k9")}</h3>

      {/* 顶部统计行 */}
      <div style={statsRowStyle}>
        <div style={statItemStyle}>
          <span style={statValueStyle}>{total}</span>
          <span style={statLabelStyle}>{t("game.components.BuildingStatsCard.k10")}</span>
        </div>
        <div style={statItemStyle}>
          <span style={{
          ...statValueStyle,
          color: 'var(--nt-success)'
        }}>{completed}</span>
          <span style={statLabelStyle}>{t("game.components.BuildingStatsCard.k11")}</span>
        </div>
        <div style={statItemStyle}>
          <span style={{
          ...statValueStyle,
          color: 'var(--nt-warning)'
        }}>{building}</span>
          <span style={statLabelStyle}>{t("game.components.BuildingStatsCard.k12")}</span>
        </div>
        <div style={statItemStyle}>
          <span style={{
          ...statValueStyle,
          color: 'var(--nt-text-muted)'
        }}>{planning}</span>
          <span style={statLabelStyle}>{t("CommandManual.k216")}</span>
        </div>
      </div>

      {/* 8 大类分类统计 */}
      <div style={sectionLabelStyle}>{t("game.components.BuildingStatsCard.k13")}</div>
      <div style={categoryGridStyle}>
        {GAME_BUILDING_CATEGORIES.map(cat => <div key={cat} style={categoryItemStyle(categoryStats[cat] > 0)} title={CATEGORY_NAMES[cat]}>
            <span style={categoryIconStyle}>{CATEGORY_ICONS[cat]}</span>
            <span style={categoryNameStyle}>{CATEGORY_NAMES[cat]}</span>
            <span style={categoryCountStyle(categoryStats[cat] > 0)}>
              {categoryStats[cat]}
            </span>
          </div>)}
      </div>

      {/* 建造中建筑列表 */}
      {buildingList.length > 0 && <>
          <div style={sectionLabelStyle}>{t("game.components.BuildingStatsCard.k14")}</div>
          <div style={buildingListStyle}>
            {buildingList.map(b => <div key={b.id} style={buildingItemStyle}>
                <div style={buildingHeaderStyle}>
                  <span style={buildingNameStyle}>{b.name}</span>
                  <span style={buildingProgressTextStyle}>
                    {b.build_progress.toFixed(1)}%
                  </span>
                </div>
                <div style={progressBarStyle}>
                  <div style={{
              ...progressFillStyle,
              width: `${Math.min(100, Math.max(0, b.build_progress))}%`
            }} />
                </div>
              </div>)}
          </div>
        </>}

      {total === 0 && <div style={emptyStyle}>
          {t("game.components.BuildingStatsCard.k15")}<br />
          <span style={{
        fontSize: '12px',
        opacity: 0.7
      }}>
            {t("game.components.BuildingStatsCard.k16")}
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
const statsRowStyle: CSSProperties = {
  display: 'flex',
  gap: '8px',
  justifyContent: 'space-between'
};
const statItemStyle: CSSProperties = {
  flex: 1,
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  gap: '2px',
  padding: '8px 4px',
  background: 'rgba(0, 240, 255, 0.04)',
  border: '1px solid var(--nt-border-subtle)',
  borderRadius: 'var(--nt-radius-md)'
};
const statValueStyle: CSSProperties = {
  fontSize: '20px',
  fontWeight: 600,
  color: 'var(--nt-primary)',
  fontFamily: 'var(--nt-font-mono)',
  textShadow: '0 0 6px rgba(0, 240, 255, 0.4)'
};
const statLabelStyle: CSSProperties = {
  fontSize: '11px',
  color: 'var(--nt-text-muted)',
  fontFamily: 'var(--nt-font-mono)'
};
const sectionLabelStyle: CSSProperties = {
  fontSize: '12px',
  color: 'var(--nt-text-secondary)',
  fontFamily: 'var(--nt-font-mono)',
  marginTop: '4px',
  letterSpacing: '0.5px'
};
const categoryGridStyle: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'repeat(4, 1fr)',
  gap: '6px'
};
const categoryItemStyle = (hasBuildings: boolean): CSSProperties => ({
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  gap: '2px',
  padding: '8px 4px',
  background: hasBuildings ? 'rgba(0, 240, 255, 0.08)' : 'rgba(106, 106, 138, 0.05)',
  border: `1px solid ${hasBuildings ? 'var(--nt-border-color)' : 'var(--nt-border-subtle)'}`,
  borderRadius: 'var(--nt-radius-md)',
  opacity: hasBuildings ? 1 : 0.5
});
const categoryIconStyle: CSSProperties = {
  fontSize: '18px',
  lineHeight: 1
};
const categoryNameStyle: CSSProperties = {
  fontSize: '11px',
  color: 'var(--nt-text-secondary)',
  fontFamily: 'var(--nt-font-chinese)'
};
const categoryCountStyle = (hasBuildings: boolean): CSSProperties => ({
  fontSize: '14px',
  fontWeight: 600,
  color: hasBuildings ? 'var(--nt-primary)' : 'var(--nt-text-muted)',
  fontFamily: 'var(--nt-font-mono)'
});
const buildingListStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px'
};
const buildingItemStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px'
};
const buildingHeaderStyle: CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
  alignItems: 'center',
  gap: '8px'
};
const buildingNameStyle: CSSProperties = {
  fontSize: '13px',
  color: 'var(--nt-text-primary)',
  fontFamily: 'var(--nt-font-chinese)'
};
const buildingProgressTextStyle: CSSProperties = {
  fontSize: '11px',
  color: 'var(--nt-warning)',
  fontFamily: 'var(--nt-font-mono)',
  fontWeight: 600
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