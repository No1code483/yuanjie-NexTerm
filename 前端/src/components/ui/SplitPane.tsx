import { t } from "i18next";
/**
 * SplitPane - 可拖拽分割面板组件（v2）
 * 参考 VSCode SplitView + Sash 架构设计
 *
 * 设计原则：
 * 1. Sash 分隔条始终可见（1px 细线），hover 时高亮
 * 2. 折叠/展开按钮内嵌在 Sash 区域，不突兀
 * 3. 拖拽时显示拖拽指示器
 * 4. 尺寸持久化 localStorage
 */
import { useState, useCallback, useRef, useEffect, type ReactNode } from 'react';
import styles from './SplitPane.module.css';
export interface SplitPaneProps {
  direction: 'horizontal' | 'vertical';
  first: ReactNode;
  second: ReactNode;
  defaultFirstSize?: number;
  firstMinSize?: number;
  firstMaxSize?: number;
  secondMinSize?: number;
  firstCollapsible?: boolean;
  secondCollapsible?: boolean;
  firstCollapsed?: boolean;
  secondCollapsed?: boolean;
  onFirstSizeChange?: (size: number) => void;
  onFirstCollapsedChange?: (collapsed: boolean) => void;
  onSecondCollapsedChange?: (collapsed: boolean) => void;
  storageKey?: string;
  className?: string;
}
export default function SplitPane({
  direction,
  first,
  second,
  defaultFirstSize = 260,
  firstMinSize = 100,
  firstMaxSize = 800,
  secondMinSize = 150,
  firstCollapsible = true,
  secondCollapsible = true,
  firstCollapsed = false,
  secondCollapsed = false,
  onFirstSizeChange,
  onFirstCollapsedChange,
  onSecondCollapsedChange,
  storageKey,
  className
}: SplitPaneProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  // 初始尺寸：优先从 localStorage 恢复
  const [firstSize, setFirstSize] = useState(() => {
    if (storageKey) {
      try {
        const stored = localStorage.getItem(`sp_${storageKey}`);
        if (stored) return Number(stored);
      } catch {/* ignore */}
    }
    return defaultFirstSize;
  });

  // 折叠状态（受控 + 非受控混合）
  const [isFirstCollapsed, setIsFirstCollapsed] = useState(firstCollapsed);
  const [isSecondCollapsed, setIsSecondCollapsed] = useState(secondCollapsed);
  const [isDragging, setIsDragging] = useState(false);
  const firstSizeRef = useRef(firstSize);
  const dragStartRef = useRef({
    pos: 0,
    size: 0
  });
  const savedSizeRef = useRef(defaultFirstSize); // 折叠前保存的尺寸

  useEffect(() => {
    firstSizeRef.current = firstSize;
  }, [firstSize]);

  /** 保存尺寸到 state + localStorage + 回调 */
  const commitSize = useCallback((size: number) => {
    setFirstSize(size);
    onFirstSizeChange?.(size);
    if (storageKey) {
      try {
        localStorage.setItem(`sp_${storageKey}`, String(size));
      } catch {/* ignore */}
    }
  }, [storageKey, onFirstSizeChange]);

  /** 折叠/展开第一个面板 */
  const toggleFirstCollapse = useCallback(() => {
    setIsFirstCollapsed(prev => {
      if (!prev) {
        savedSizeRef.current = firstSizeRef.current;
      } else {
        commitSize(savedSizeRef.current);
      }
      onFirstCollapsedChange?.(!prev);
      return !prev;
    });
  }, [commitSize, onFirstCollapsedChange]);

  /** 折叠/展开第二个面板 */
  const toggleSecondCollapse = useCallback(() => {
    setIsSecondCollapsed(prev => {
      onSecondCollapsedChange?.(!prev);
      return !prev;
    });
  }, [onSecondCollapsedChange]);

  /* ========== Sash 拖拽逻辑 ========== */
  const handleSashMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
    const isH = direction === 'horizontal';
    dragStartRef.current = {
      pos: isH ? e.clientX : e.clientY,
      size: firstSizeRef.current
    };
    const onMove = (ev: MouseEvent) => {
      const currentPos = isH ? ev.clientX : ev.clientY;
      const delta = currentPos - dragStartRef.current.pos;
      let newSize = dragStartRef.current.size + delta;

      // 约束范围
      const containerSize = isH ? containerRef.current?.clientWidth ?? window.innerWidth : containerRef.current?.clientHeight ?? window.innerHeight;
      newSize = Math.max(firstMinSize, Math.min(newSize, firstMaxSize, containerSize - secondMinSize));
      commitSize(newSize);
    };
    const onUp = () => {
      setIsDragging(false);
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      document.body.style.pointerEvents = '';
    };
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
    document.body.style.cursor = isH ? 'col-resize' : 'row-resize';
    document.body.style.userSelect = 'none';
  }, [direction, firstMinSize, firstMaxSize, secondMinSize, commitSize]);
  const isHorizontal = direction === 'horizontal';

  /* ========== 计算面板样式 ========== */
  const firstStyle: React.CSSProperties = isFirstCollapsed ? {
    width: 0,
    height: 0,
    minWidth: 0,
    minHeight: 0,
    overflow: 'hidden',
    flexShrink: 0
  } : isHorizontal ? {
    width: firstSize,
    minWidth: firstMinSize,
    flexShrink: 0
  } : {
    height: firstSize,
    minHeight: firstMinSize,
    flexShrink: 0
  };
  const secondStyle: React.CSSProperties = isSecondCollapsed ? {
    width: 0,
    height: 0,
    minWidth: 0,
    minHeight: 0,
    overflow: 'hidden',
    flex: '0 0 auto'
  } : {
    flex: 1,
    minWidth: secondMinSize,
    minHeight: secondMinSize,
    overflow: 'hidden'
  };
  return <div ref={containerRef} className={`${styles.root} ${isHorizontal ? styles.h : styles.v} ${className || ''}`}>
      {/* ===== 第一个面板 ===== */}
      <div className={`${styles.pane} ${isFirstCollapsed ? styles.paneHidden : ''}`} style={firstStyle}>
        {first}
      </div>

      {/* ===== Sash 分隔条（含折叠按钮）===== */}
      {!isFirstCollapsed && !isSecondCollapsed && <div className={`${styles.sash} ${isHorizontal ? styles.sashH : styles.sashV} ${isDragging ? styles.sashDrag : ''}`} onMouseDown={handleSashMouseDown}>
          {/* 第一个面板折叠按钮 */}
          {firstCollapsible && <button className={`${styles.foldBtn} ${isHorizontal ? styles.foldBtnRight : styles.foldBtnDown}`} onClick={e => {
        e.stopPropagation();
        toggleFirstCollapse();
      }} title={t("components.ui.SplitPane.k1")}>
              {isHorizontal ? '◀' : '▲'}
            </button>}
          {/* 第二个面板折叠按钮 */}
          {secondCollapsible && <button className={`${styles.foldBtn} ${isHorizontal ? styles.foldBtnLeft : styles.foldBtnUp}`} onClick={e => {
        e.stopPropagation();
        toggleSecondCollapse();
      }} title={t("components.ui.SplitPane.k1")}>
              {isHorizontal ? '▶' : '▼'}
            </button>}
        </div>}

      {/* ===== 第一个面板折叠时：展开按钮 ===== */}
      {isFirstCollapsed && <button className={`${styles.expandBar} ${isHorizontal ? styles.expandBarV : styles.expandBarH}`} onClick={toggleFirstCollapse} title={t("common.expand")}>
          <span className={styles.expandIcon}>{isHorizontal ? '▶' : '▼'}</span>
        </button>}

      {/* ===== 第二个面板折叠时：展开按钮 ===== */}
      {isSecondCollapsed && !isFirstCollapsed && <button className={`${styles.expandBar} ${isHorizontal ? styles.expandBarVLeft : styles.expandBarHTop}`} onClick={toggleSecondCollapse} title={t("common.expand")}>
          <span className={styles.expandIcon}>{isHorizontal ? '◀' : '▲'}</span>
        </button>}

      {/* ===== 第二个面板 ===== */}
      <div className={`${styles.pane} ${isSecondCollapsed ? styles.paneHidden : ''}`} style={secondStyle}>
        {second}
      </div>
    </div>;
}