import { t } from "i18next";
/**
 * RealmCard - 境界信息卡（Task 7.4）
 *
 * 展示：
 *   - 大境界中文名 + 小境界阶位
 *   - 道基品质 + 系数
 *   - 修为进度条（当前大境界内进度）
 *   - 突破状态（可突破 N 题 / 已达仙境界）
 *   - 总修为 + 文明等级
 *
 * Props：{ realmInfo: RealmInfo | null; loading: boolean }
 *
 * change-id: game-3d-rebuild-refactor
 */
import type { CSSProperties } from 'react';
import type { RealmInfo, GameRealmMajor, GameRealmMinor, GameDaoFoundation } from '@/types/game';

// ===== 中文名映射（自包含，不依赖旧版 game/types.ts）=====

const REALM_MAJOR_NAMES: Record<GameRealmMajor, string> = {
  mortal: t("game.components.RealmCard.k1"),
  qi_refining: t("game.components.RealmCard.k2"),
  foundation_building: t("game.components.RealmCard.k3"),
  golden_core: t("game.components.RealmCard.k4"),
  nascent_soul: t("game.components.RealmCard.k5"),
  spirit_transformation: t("game.components.RealmCard.k6"),
  unity: t("game.components.RealmCard.k7"),
  mahayana: t("game.components.RealmCard.k8"),
  tribulation: t("game.components.RealmCard.k9"),
  immortal: t("game.components.RealmCard.k10")
};
const REALM_MINOR_NAMES: Record<GameRealmMinor, string> = {
  early: t("game.components.RealmCard.k11"),
  middle: t("game.components.RealmCard.k12"),
  complete: t("game.components.RealmCard.k13")
};
const DAO_FOUNDATION_NAMES: Record<GameDaoFoundation, string> = {
  white: t("game.components.RealmCard.k14"),
  blue: t("game.components.RealmCard.k15"),
  red: t("game.components.RealmCard.k16"),
  purple: t("game.components.RealmCard.k17"),
  black: t("game.components.RealmCard.k18")
};
interface RealmCardProps {
  realmInfo: RealmInfo | null;
  loading: boolean;
}
export default function RealmCard({
  realmInfo,
  loading
}: RealmCardProps) {
  // 加载中骨架屏
  if (loading || !realmInfo) {
    return <div style={cardStyle}>
        <h3 style={titleStyle}>{t("game.components.RealmCard.k19")}</h3>
        <div style={{
        ...skeletonStyle,
        width: '60%'
      }} />
        <div style={{
        ...skeletonStyle,
        width: '40%'
      }} />
        <div style={{
        ...skeletonStyle,
        height: '6px',
        marginTop: '8px'
      }} />
        <div style={{
        ...skeletonStyle,
        width: '50%'
      }} />
      </div>;
  }
  const {
    realm_major,
    realm_minor,
    dao_foundation,
    dao_multiplier,
    total_xp,
    realm_xp,
    realm_xp_lower,
    realm_xp_upper,
    realm_ordinal,
    next_realm_major,
    breakthrough_question_count,
    civilization_level
  } = realmInfo;

  // 修为进度计算（当前大境界内进度百分比）
  const realmRange = realm_xp_upper - realm_xp_lower;
  const realmProgress = realmRange > 0 ? (realm_xp - realm_xp_lower) / realmRange * 100 : 0;
  const progressClamped = Math.max(0, Math.min(100, realmProgress));

  // 突破状态判断
  const isImmortal = realm_major === 'immortal';
  const canBreakthrough = !isImmortal && breakthrough_question_count > 0;
  return <div style={cardStyle}>
      <h3 style={titleStyle}>{t("game.components.RealmCard.k19")}</h3>

      {/* 境界名称 */}
      <div style={realmNameRowStyle}>
        <span style={realmMajorStyle}>
          {REALM_MAJOR_NAMES[realm_major]}
          <span style={realmMinorStyle}> · {REALM_MINOR_NAMES[realm_minor]}</span>
        </span>
        <span style={ordinalBadgeStyle}>{t("components.intelligence.ActivityPanel.k39")} {realm_ordinal} {t("game.components.RealmCard.k20")}</span>
      </div>

      {/* 道基 */}
      <div style={infoRowStyle}>
        <span style={labelStyle}>{t("game.components.RealmCard.k21")}</span>
        <span style={daoBadgeStyle(dao_foundation)}>
          {DAO_FOUNDATION_NAMES[dao_foundation]}（×{dao_multiplier.toFixed(1)}）
        </span>
      </div>

      {/* 修为进度条 */}
      <div style={progressBlockStyle}>
        <div style={progressLabelRowStyle}>
          <span style={labelStyle}>{t("game.components.RealmCard.k22")}</span>
          <span style={progressTextStyle}>
            {realm_xp.toLocaleString()} / {realm_xp_upper.toLocaleString()}
          </span>
        </div>
        <div style={progressBarStyle}>
          <div style={{
          ...progressFillStyle,
          width: `${progressClamped}%`
        }} />
        </div>
        <div style={progressHintStyle}>
          {t("game.components.RealmCard.k23")} {progressClamped.toFixed(1)}%
          {next_realm_major && <> {t("game.components.RealmCard.k24")}{REALM_MAJOR_NAMES[next_realm_major]}</>}
        </div>
      </div>

      {/* 突破状态 */}
      <div style={breakthroughRowStyle(canBreakthrough)}>
        {isImmortal ? <span>{t("game.components.RealmCard.k25")}</span> : canBreakthrough ? <span>{t("game.components.RealmCard.k26")} {breakthrough_question_count}</span> : <span>{t("game.components.RealmCard.k27")}</span>}
      </div>

      {/* 总修为 + 文明等级 */}
      <div style={footerRowStyle}>
        <div style={footerItemStyle}>
          <span style={footerLabelStyle}>{t("game.components.RealmCard.k28")}</span>
          <span style={footerValueStyle}>{total_xp.toLocaleString()}</span>
        </div>
        <div style={footerItemStyle}>
          <span style={footerLabelStyle}>{t("game.components.RealmCard.k29")}</span>
          <span style={footerValueStyle}>Lv.{civilization_level}</span>
        </div>
      </div>
    </div>;
}

// ===== 内联样式（避免为单一组件引入 CSS Module）=====

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
const realmNameRowStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '8px'
};
const realmMajorStyle: CSSProperties = {
  fontFamily: 'var(--nt-font-chinese)',
  fontSize: '22px',
  fontWeight: 600,
  color: 'var(--nt-text-bright)',
  textShadow: '0 0 10px rgba(0, 240, 255, 0.5)'
};
const realmMinorStyle: CSSProperties = {
  fontSize: '14px',
  color: 'var(--nt-text-secondary)',
  fontWeight: 400
};
const ordinalBadgeStyle: CSSProperties = {
  padding: '2px 10px',
  fontSize: '11px',
  color: 'var(--nt-secondary)',
  background: 'rgba(176, 38, 255, 0.1)',
  border: '1px solid rgba(176, 38, 255, 0.35)',
  borderRadius: 'var(--nt-radius-sm)',
  fontFamily: 'var(--nt-font-mono)'
};
const infoRowStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '8px'
};
const labelStyle: CSSProperties = {
  fontSize: '13px',
  color: 'var(--nt-text-secondary)',
  fontFamily: 'var(--nt-font-mono)'
};
const daoBadgeStyle = (dao: GameDaoFoundation): CSSProperties => {
  // 道基颜色映射（白→灰、蓝→蓝、红→红、紫→紫、黑→金）
  const colorMap: Record<GameDaoFoundation, string> = {
    white: '#c8c8dd',
    blue: '#01a0e4',
    red: '#FF006E',
    purple: '#B026FF',
    black: '#FFD700'
  };
  const color = colorMap[dao];
  return {
    padding: '3px 10px',
    fontSize: '13px',
    color,
    background: `${color}1A`,
    border: `1px solid ${color}80`,
    borderRadius: 'var(--nt-radius-sm)',
    fontFamily: 'var(--nt-font-mono)',
    fontWeight: 600
  };
};
const progressBlockStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
  marginTop: '4px'
};
const progressLabelRowStyle: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '8px'
};
const progressTextStyle: CSSProperties = {
  fontSize: '12px',
  color: 'var(--nt-text-bright)',
  fontFamily: 'var(--nt-font-mono)'
};
const progressBarStyle: CSSProperties = {
  width: '100%',
  height: '6px',
  background: 'var(--nt-gray-medium)',
  borderRadius: '3px',
  overflow: 'hidden'
};
const progressFillStyle: CSSProperties = {
  height: '100%',
  background: 'linear-gradient(90deg, var(--nt-primary), var(--nt-secondary))',
  borderRadius: '3px',
  transition: 'width 0.4s ease',
  boxShadow: '0 0 8px rgba(0, 240, 255, 0.5)'
};
const progressHintStyle: CSSProperties = {
  fontSize: '11px',
  color: 'var(--nt-text-muted)',
  fontFamily: 'var(--nt-font-mono)',
  marginTop: '2px'
};
const breakthroughRowStyle = (canBreakthrough: boolean): CSSProperties => ({
  padding: '8px 12px',
  fontSize: '13px',
  fontFamily: 'var(--nt-font-mono)',
  borderRadius: 'var(--nt-radius-md)',
  textAlign: 'center',
  background: canBreakthrough ? 'rgba(255, 215, 0, 0.08)' : 'rgba(106, 106, 138, 0.08)',
  border: `1px solid ${canBreakthrough ? 'rgba(255, 215, 0, 0.4)' : 'rgba(106, 106, 138, 0.3)'}`,
  color: canBreakthrough ? 'var(--nt-warning)' : 'var(--nt-text-muted)'
});
const footerRowStyle: CSSProperties = {
  display: 'flex',
  gap: '16px',
  marginTop: 'auto',
  paddingTop: '12px',
  borderTop: '1px solid var(--nt-border-subtle)'
};
const footerItemStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '2px',
  flex: 1
};
const footerLabelStyle: CSSProperties = {
  fontSize: '11px',
  color: 'var(--nt-text-muted)',
  fontFamily: 'var(--nt-font-mono)'
};
const footerValueStyle: CSSProperties = {
  fontSize: '16px',
  color: 'var(--nt-primary)',
  fontFamily: 'var(--nt-font-mono)',
  fontWeight: 600,
  textShadow: '0 0 6px rgba(0, 240, 255, 0.4)'
};
const skeletonStyle: CSSProperties = {
  height: '14px',
  background: 'linear-gradient(90deg, var(--nt-text-muted) 0%, rgba(106, 106, 138, 0.2) 50%, var(--nt-text-muted) 100%)',
  backgroundSize: '200% 100%',
  borderRadius: 'var(--nt-radius-sm)',
  animation: 'skeletonPulse 1.5s ease-in-out infinite'
};