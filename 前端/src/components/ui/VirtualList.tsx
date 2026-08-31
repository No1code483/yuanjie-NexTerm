import { t } from "i18next";
import { useCallback, useEffect } from 'react';
import { List, useListRef } from 'react-window';
import styles from './VirtualList.module.css';

// ============================================================
// 通用虚拟列表 — 终端黑客风格
// 基于 react-window v2，替代大数据量 .map() 直接渲染
// ============================================================

interface RowProps {
  data: any;
}
interface VirtualListProps<T> {
  /** 数据源 */
  items: T[];
  /** 单项渲染函数 */
  renderItem: (item: T, index: number) => React.ReactNode;
  /** 单项高度（固定值或函数） */
  rowHeight?: number | ((index: number) => number);
  /** 容器宽度（默认100%） */
  width?: string | number;
  /** 容器高度 */
  height: number;
  /** 滚动到指定索引 */
  scrollToIndex?: number;
  /** 额外渲染行数 */
  overscanCount?: number;
  /** 空状态文本 */
  emptyText?: string;
  /** 是否显示加载遮罩 */
  loading?: boolean;
  /** 自定义className */
  className?: string;
}

/** 虚拟列表（固定/可变高度） */
export function VirtualList<T>({
  items,
  renderItem,
  rowHeight = 40,
  width = '100%',
  height,
  scrollToIndex,
  overscanCount = 5,
  emptyText = t("common.noData"),
  loading = false,
  className = ''
}: VirtualListProps<T>) {
  const listRef = useListRef(null);
  useEffect(() => {
    if (scrollToIndex !== undefined && listRef.current) {
      listRef.current.scrollToRow({
        index: scrollToIndex,
        align: 'start'
      });
    }
  }, [scrollToIndex, listRef]);
  const RowComponent = useCallback(({
    index,
    style
  }: {
    index: number;
    style: React.CSSProperties;
  }) => {
    const item = items[index];
    return <div style={style}>
          {renderItem(item, index)}
        </div>;
  }, [items, renderItem]);
  if (items.length === 0 && !loading) {
    return <div className={styles.empty}>{emptyText}</div>;
  }
  return <div className={`${styles.virtualContainer} ${className}`} style={{
    width,
    height
  }}>
      {loading && <div className={styles.loadingOverlay}>{t("common.loading")}</div>}
      <List listRef={listRef} className="react-window-list" style={{
      width: '100%',
      height
    }} rowCount={items.length} rowHeight={rowHeight} rowComponent={RowComponent as any} rowProps={{} as RowProps} overscanCount={overscanCount} />
    </div>;
}
export default VirtualList;