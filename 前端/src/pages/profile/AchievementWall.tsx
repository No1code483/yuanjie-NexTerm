import { t } from "i18next";
import { useUserStats } from '@/hooks/useUserStats';
import styles from '../Profile.module.css';
export interface Achievement {
  id: string;
  icon: string;
  name: string;
  desc: string;
  // 解锁条件（任一满足即解锁；返回 null 表示已解锁）
  check: (ctx: AchievementContext) => boolean;
}
interface AchievementContext {
  loginCount: number;
  streakDays: number;
  totalUsageSecs: number;
  resumeCount: number;
  quoteCount: number;
  kbCount: number;
}
interface AchievementWallProps {
  userId: number | undefined;
  enabled: boolean;
  resumeCount?: number;
  quoteCount?: number;
  kbCount?: number;
}

/**
 * 成就墙 - 终端风格徽章网格
 * 基于 useUserStats + 外部计数判断解锁状态
 */
const ACHIEVEMENTS: Achievement[] = [{
  id: 'newbie',
  icon: '🥚',
  name: t("profile.AchievementWall.k1"),
  desc: t("profile.AchievementWall.k2"),
  check: c => c.loginCount >= 1
}, {
  id: 'streak7',
  icon: '🌅',
  name: t("profile.AchievementWall.k3"),
  desc: t("profile.AchievementWall.k4"),
  check: c => c.streakDays >= 7
}, {
  id: 'uptime1h',
  icon: '⚡',
  name: t("profile.AchievementWall.k5"),
  desc: t("profile.AchievementWall.k6"),
  check: c => c.totalUsageSecs >= 3600
}, {
  id: 'resume1',
  icon: '📝',
  name: t("profile.AchievementWall.k7"),
  desc: t("profile.AchievementWall.k8"),
  check: c => c.resumeCount >= 1
}, {
  id: 'quote5',
  icon: '💬',
  name: t("profile.AchievementWall.k9"),
  desc: t("profile.AchievementWall.k10"),
  check: c => c.quoteCount >= 5
}, {
  id: 'kb10',
  icon: '📚',
  name: t("profile.AchievementWall.k11"),
  desc: t("profile.AchievementWall.k12"),
  check: c => c.kbCount >= 10
}, {
  id: 'login30',
  icon: '🏆',
  name: t("profile.AchievementWall.k13"),
  desc: t("profile.AchievementWall.k14"),
  check: c => c.loginCount >= 30
}, {
  id: 'uptime24h',
  icon: '👑',
  name: t("profile.AchievementWall.k15"),
  desc: t("profile.AchievementWall.k16"),
  check: c => c.totalUsageSecs >= 86400
}];
export default function AchievementWall({
  userId,
  enabled,
  resumeCount = 0,
  quoteCount = 0,
  kbCount = 0
}: AchievementWallProps) {
  const {
    stats
  } = useUserStats(userId, enabled);
  const ctx: AchievementContext = {
    loginCount: stats.loginCount,
    streakDays: stats.streakDays,
    totalUsageSecs: stats.totalUsageSecs,
    resumeCount,
    quoteCount,
    kbCount
  };
  const unlocked = ACHIEVEMENTS.filter(a => a.check(ctx)).length;
  return <div className={`${styles.infoCard} ${styles.achievementCard}`}>
      <div className={styles.cardTitle}>
        <span>{t("profile.AchievementWall.k17")}</span>
        <span className={styles.cardTitleMeta}>[{unlocked}/{ACHIEVEMENTS.length}]</span>
      </div>

      <div className={styles.achievementGrid}>
        {ACHIEVEMENTS.map(a => {
        const isUnlocked = a.check(ctx);
        return <div key={a.id} className={`${styles.achievementBadge} ${isUnlocked ? styles.badgeUnlocked : styles.badgeLocked}`} title={a.desc}>
              <div className={styles.badgeIcon}>{isUnlocked ? a.icon : '🔒'}</div>
              <div className={styles.badgeName}>{a.name}</div>
              <div className={styles.badgeDesc}>{a.desc}</div>
            </div>;
      })}
      </div>
    </div>;
}