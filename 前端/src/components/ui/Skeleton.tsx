import { t } from "i18next";
import { useTranslation } from 'react-i18next';
import { memo } from 'react';
import styles from './Skeleton.module.css';
interface SkeletonProps {
  className?: string;
  style?: React.CSSProperties;
}

/** 文本行骨架 */
export const SkeletonText = memo(function SkeletonText({
  lines = 3,
  shortLast = false
}: {
  lines?: number;
  shortLast?: boolean;
}) {
  const { t } = useTranslation();
  return <div className={styles.textLines} role="status" aria-label={t("common.loading")} aria-busy="true">
      {Array.from({
      length: lines
    }).map((_, i) => {
      const isLast = i === lines - 1;
      const variant = isLast && shortLast ? styles.textShort : i === 0 ? styles.textMedium : '';
      return <div key={i} className={`${styles.skeleton} ${styles.text} ${variant}`} />;
    })}
    </div>;
});

/** 卡片/矩形骨架 */
export const SkeletonCard = memo(function SkeletonCard({
  width,
  height,
  className
}: SkeletonProps & {
  width?: string | number;
  height?: string | number;
}) {
  const { t } = useTranslation();
  return <div className={`${styles.skeleton} ${styles.card} ${className ?? ''}`} style={{
    width,
    height
  }} role="status" aria-label={t("common.loading")} aria-busy="true" />;
});

/** 圆形骨架（头像等） */
export const SkeletonCircle = memo(function SkeletonCircle({
  size = 40
}: {
  size?: number;
}) {
  return <div className={`${styles.skeleton} ${styles.circle}`} style={{
    width: size,
    height: size
  }} />;
});

/** 列表项骨架 */
export const SkeletonListItem = memo(function SkeletonListItem({
  hasAvatar = false
}: {
  hasAvatar?: boolean;
}) {
  return <div className={styles.listItem}>
      {hasAvatar && <SkeletonCircle size={32} />}
      <div style={{
      flex: 1
    }}>
        <div className={`${styles.skeleton} ${styles.text}`} />
        <div className={`${styles.skeleton} ${styles.text} ${styles.textShort}`} />
      </div>
    </div>;
});

/** 列表骨架 */
export const SkeletonList = memo(function SkeletonList({
  count = 5,
  hasAvatar = false
}: {
  count?: number;
  hasAvatar?: boolean;
}) {
  const { t } = useTranslation();
  return <div role="status" aria-label={t("common.loading")} aria-busy="true">
      {Array.from({
      length: count
    }).map((_, i) => <SkeletonListItem key={i} hasAvatar={hasAvatar} />)}
    </div>;
});

/** 表格行骨架 */
export function SkeletonTableRow({
  cols = 4
}: {
  cols?: number;
}) {
  return <div className={styles.tableRow}>
      {Array.from({
      length: cols
    }).map((_, i) => <div key={i} className={styles.skeleton} style={{
      flex: i === 0 ? 2 : 1
    }} />)}
    </div>;
}

/** 表格骨架 */
export function SkeletonTable({
  rows = 5,
  cols = 4
}: {
  rows?: number;
  cols?: number;
}) {
  return <div role="status" aria-label={t("common.loading")} aria-busy="true">
      {Array.from({
      length: rows
    }).map((_, i) => <SkeletonTableRow key={i} cols={cols} />)}
    </div>;
}

/** 页面骨架容器 */
export function SkeletonPage({
  children,
  className
}: SkeletonProps & {
  children: React.ReactNode;
}) {
  return <div className={`${styles.page} ${className ?? ''}`} role="status" aria-label={t("components.ui.Skeleton.k1")} aria-busy="true">
      {children}
    </div>;
}

/** 通用骨架块 */
export function SkeletonBlock({
  width,
  height,
  className,
  style
}: SkeletonProps & {
  width?: string | number;
  height?: string | number;
}) {
  return <div className={`${styles.skeleton} ${className ?? ''}`} style={{
    width,
    height,
    ...style
  }} role="status" aria-label={t("common.loading")} aria-busy="true" />;
}