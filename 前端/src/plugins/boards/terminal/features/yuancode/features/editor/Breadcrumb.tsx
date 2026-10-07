import { t } from "i18next";
import { useEffect, useState, useCallback } from 'react';
import styles from './Breadcrumb.module.css';
interface BreadcrumbProps {
  activeFile: string | null;
  onPathClick?: (path: string) => void;
  onSymbolClick?: (symbol: string, line: number) => void;
}
interface BreadcrumbSegment {
  name: string;
  path: string;
  type: 'folder' | 'file' | 'symbol';
  line?: number;
}
export default function Breadcrumb({
  activeFile,
  onPathClick,
  onSymbolClick
}: BreadcrumbProps) {
  const [segments, setSegments] = useState<BreadcrumbSegment[]>([]);
  const parsePath = useCallback((filePath: string | null) => {
    if (!filePath) {
      setSegments([]);
      return;
    }
    const parts = filePath.replace(/\\/g, '/').split('/');
    const result: BreadcrumbSegment[] = [];
    let accumulatedPath = '';
    for (let i = 0; i < parts.length; i++) {
      accumulatedPath = accumulatedPath ? `${accumulatedPath}/${parts[i]}` : parts[i];
      const isLast = i === parts.length - 1;
      result.push({
        name: parts[i],
        path: accumulatedPath,
        type: isLast ? 'file' : 'folder'
      });
    }
    setSegments(result);
  }, []);
  useEffect(() => {
    parsePath(activeFile);
  }, [activeFile, parsePath]);
  const handleClick = (segment: BreadcrumbSegment) => {
    if (segment.type === 'symbol' && segment.line && onSymbolClick) {
      onSymbolClick(segment.name, segment.line);
    } else if (onPathClick) {
      onPathClick(segment.path);
    }
  };
  if (segments.length === 0) {
    return <div className={styles.breadcrumb}>
        <span className={styles.empty}>{t("yuan-code.Breadcrumb.k1")}</span>
      </div>;
  }
  return <div className={styles.breadcrumb}>
      {segments.map((segment, index) => <span key={`${segment.path}-${index}`} className={styles.segmentWrapper}>
          {index > 0 && <span className={styles.separator}>›</span>}
          <span className={`${styles.segment} ${segment.type === 'file' ? styles.file : styles.folder}`} onClick={() => handleClick(segment)} title={segment.path}>
            {getIcon(segment.type)}
            {segment.name}
          </span>
        </span>)}
    </div>;
}
function getIcon(type: string): string {
  switch (type) {
    case 'folder':
      return '📁 ';
    case 'file':
      return '📄 ';
    case 'symbol':
      return '🔹 ';
    default:
      return '';
  }
}