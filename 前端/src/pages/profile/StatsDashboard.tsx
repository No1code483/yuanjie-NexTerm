import { t } from "i18next";
import { useUserStats, formatUsageDuration, asciiProgress } from '@/hooks/useUserStats';
import styles from '../Profile.module.css';
interface StatsDashboardProps {
  userId: number | undefined;
  enabled: boolean;
  resumeCount?: number;
  quoteCount?: number;
  kbCount?: number;
}
export default function StatsDashboard({
  userId,
  enabled,
  resumeCount = 0,
  quoteCount = 0,
  kbCount = 0
}: StatsDashboardProps) {
  const {
    stats
  } = useUserStats(userId, enabled);
  const usageHours = stats.totalUsageSecs / 3600;
  const level = Math.floor(Math.log2(usageHours + 1)) + 1;
  const nextLevelHours = Math.pow(2, level);
  const levelProgress = Math.min(100, usageHours / nextLevelHours * 100);
  const streakProgress = Math.min(100, stats.streakDays / 7 * 100);
  const resumeProgress = Math.min(100, resumeCount / 5 * 100);
  const quoteProgress = Math.min(100, quoteCount / 10 * 100);
  return <div className={`${styles.infoCard} ${styles.statsCard}`}>
      <div className={styles.cardTitle}>{t("profile.StatsDashboard.k1")}</div>

      <div className={styles.statsGrid}>
        <div className={styles.statTile}>
          <div className={styles.statTileRow}>
            <div className={styles.statTileLabel}>{t("profile.StatsDashboard.k2")}</div>
            <div className={styles.statTileValueWrap}>
              <span className={styles.statTileValue}>{stats.loginCount}</span>
              <span className={styles.statTileUnit}>{t("profile.StatsDashboard.k3")}</span>
            </div>
          </div>
          <div className={styles.statTileBar}>{asciiProgress(Math.min(100, stats.loginCount / 50 * 100), 10)}</div>
        </div>

        <div className={styles.statTile}>
          <div className={styles.statTileRow}>
            <div className={styles.statTileLabel}>{t("profile.StatsDashboard.k4")}</div>
            <div className={styles.statTileValueWrap}>
              <span className={styles.statTileValue}>{formatUsageDuration(stats.totalUsageSecs)}</span>
              <span className={styles.statTileUnit}>{'Lv.'}{level}</span>
            </div>
          </div>
          <div className={styles.statTileBar}>{asciiProgress(levelProgress, 10)}</div>
        </div>

        <div className={styles.statTile}>
          <div className={styles.statTileRow}>
            <div className={styles.statTileLabel}>{t("profile.StatsDashboard.k5")}</div>
            <div className={styles.statTileValueWrap}>
              <span className={styles.statTileValue}>{stats.streakDays}</span>
              <span className={styles.statTileUnit}>{t("components.FocusMode.k9")}</span>
            </div>
          </div>
          <div className={styles.statTileBar}>{asciiProgress(streakProgress, 10)}</div>
        </div>

        <div className={styles.statTile}>
          <div className={styles.statTileRow}>
            <div className={styles.statTileLabel}>{t("profile.StatsDashboard.k6")}</div>
            <div className={styles.statTileValueWrap}>
              <span className={styles.statTileValue}>{kbCount}</span>
              <span className={styles.statTileUnit}>{t("ai.ChatPanel.k19")}</span>
            </div>
          </div>
          <div className={styles.statTileBar}>{asciiProgress(Math.min(100, kbCount / 50 * 100), 10)}</div>
        </div>

        <div className={styles.statTile}>
          <div className={styles.statTileRow}>
            <div className={styles.statTileLabel}>{t("profile.StatsDashboard.k7")}</div>
            <div className={styles.statTileValueWrap}>
              <span className={styles.statTileValue}>{resumeCount}</span>
              <span className={styles.statTileUnit}>{t("profile.StatsDashboard.k8")}</span>
            </div>
          </div>
          <div className={styles.statTileBar}>{asciiProgress(resumeProgress, 10)}</div>
        </div>

        <div className={styles.statTile}>
          <div className={styles.statTileRow}>
            <div className={styles.statTileLabel}>{t("profile.StatsDashboard.k9")}</div>
            <div className={styles.statTileValueWrap}>
              <span className={styles.statTileValue}>{quoteCount}</span>
              <span className={styles.statTileUnit}>{t("ai.ChatPanel.k19")}</span>
            </div>
          </div>
          <div className={styles.statTileBar}>{asciiProgress(quoteProgress, 10)}</div>
        </div>
      </div>

      {stats.firstLoginAt && <div className={styles.statsFooter}>
          <span className={styles.statsFooterLabel}>{t("profile.StatsDashboard.k10")}</span>
          <span className={styles.statsFooterValue}>
            {new Date(stats.firstLoginAt).toLocaleString()}
          </span>
        </div>}
    </div>;
}