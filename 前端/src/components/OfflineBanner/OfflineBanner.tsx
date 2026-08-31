/**
 * OfflineBanner — 全局离线状态横幅（A5 离线同步 Phase 3 Task 1）
 *
 * 功能：
 * - 离线时在 TitleBar 下方固定显示警示横幅
 * - 显示 📴 图标 + 离线提示 + 上次检查时间（相对时间，30s 刷新）
 * - 提供「立即检测」按钮触发 checkNow()
 * - 在线时收起（height 0 + opacity 0，带过渡动画）
 *
 * 终端黑客风格：黑底 #000000 + 警示红 #FF0000 + 等宽字体 + 闪烁/发光
 *
 * 数据来源：useNetworkStatus hook（独立于 syncStore，直接调用 sync IPC）
 * i18n：useTranslation + t(key, { defaultValue }) 形式，资源缺失时回退中文
 *
 * 设计文档：功能展望/平台级增强/04_离线与同步机制.md §Phase 3
 */

import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useNetworkStatus } from '@/hooks/useNetworkStatus';
import styles from './OfflineBanner.module.css';

/** 格式化相对时间（上次检查：N 分钟前） */
function formatRelativeTime(
  ts: number | null,
  now: number,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string {
  if (ts == null) return '';
  const diff = Math.max(0, now - ts);
  const secs = Math.floor(diff / 1000);
  if (secs < 60) {
    return t('components.OfflineBanner.justNow', { defaultValue: '刚刚' });
  }
  const mins = Math.floor(secs / 60);
  if (mins < 60) {
    return t('components.OfflineBanner.minutesAgo', {
      minutes: mins,
      defaultValue: '{{minutes}} 分钟前',
    });
  }
  const hours = Math.floor(mins / 60);
  if (hours < 24) {
    return t('components.OfflineBanner.hoursAgo', {
      hours,
      defaultValue: '{{hours}} 小时前',
    });
  }
  const days = Math.floor(hours / 24);
  return t('components.OfflineBanner.daysAgo', {
    days,
    defaultValue: '{{days}} 天前',
  });
}

export function OfflineBanner() {
  const { t } = useTranslation();
  const { isOnline, lastChecked, checkNow } = useNetworkStatus();

  // 离线时每 30s 刷新相对时间文本
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (isOnline) return;
    const id = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(id);
  }, [isOnline]);

  const lastCheckedText = formatRelativeTime(lastChecked, now, t);
  const retryLabel = t('components.OfflineBanner.retry', { defaultValue: '立即检测' });

  return (
    <div
      className={`${styles.banner} ${isOnline ? styles.hidden : styles.visible}`}
      role="status"
      aria-live="polite"
      aria-hidden={isOnline}
    >
      <span className={styles.icon} aria-hidden="true">📴</span>
      <span className={styles.text}>
        {t('components.OfflineBanner.offlineMessage', {
          defaultValue: '当前离线，部分功能受限',
        })}
      </span>
      {lastCheckedText && (
        <span className={styles.lastChecked}>
          {t('components.OfflineBanner.lastCheckedLabel', {
            time: lastCheckedText,
            defaultValue: '上次检查：{{time}}',
          })}
        </span>
      )}
      <button
        type="button"
        className={styles.retryBtn}
        onClick={checkNow}
        aria-label={retryLabel}
        title={retryLabel}
      >
        {retryLabel}
      </button>
    </div>
  );
}

export default OfflineBanner;
