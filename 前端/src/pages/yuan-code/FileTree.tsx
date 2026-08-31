import { t } from "i18next";
import type { FileNode } from '../YuanCode';
import React from 'react';
import styles from '../YuanCode.module.css';
interface FileTreeProps {
  fileTree: FileNode[];
  activeFilePath: string | null;
  onToggleDir: (node: FileNode) => void;
  onOpenFile: (node: FileNode) => void;
  onCreateFile: () => void;
  onCreateDir: () => void;
  onRefresh: () => void;
}
const FILE_ICONS: Record<string, string> = {
  py: '🐍',
  js: '🟨',
  ts: '🔷',
  tsx: '⚛️',
  jsx: '⚛️',
  rs: '🦀',
  go: '🐹',
  java: '☕',
  c: '⚙️',
  cpp: '⚙️',
  h: '⚙️',
  hpp: '⚙️',
  cs: '🎯',
  rb: '💎',
  php: '🐘',
  swift: '🦅',
  kt: '📱',
  scala: '🔴',
  lua: '🌙',
  r: '📊',
  sql: '🗄️',
  html: '🌐',
  css: '🎨',
  scss: '🎨',
  less: '🎨',
  json: '📋',
  xml: '📰',
  yaml: '⚡',
  yml: '⚡',
  toml: '⚙️',
  md: '📝',
  txt: '📄',
  sh: '💻',
  bash: '💻',
  fish: '💻',
  dockerfile: '🐳',
  gitignore: '🔧'
};
export default function FileTree({
  fileTree,
  activeFilePath,
  onToggleDir,
  onOpenFile,
  onCreateFile,
  onCreateDir,
  onRefresh
}: FileTreeProps) {
  const getIcon = (name: string, isDir: boolean): string => {
    if (isDir) return '📁';
    const ext = name.split('.').pop()?.toLowerCase() || '';
    return FILE_ICONS[ext] || '📄';
  };
  const renderNode = (node: FileNode, depth: number = 0): React.JSX.Element => {
    const isActive = !node.is_dir && node.path === activeFilePath;
    return <div key={node.path}>
        <div className={`${styles.floatTreeItem} ${isActive ? styles.floatTreeItemActive : ''}`} style={{
        paddingLeft: `${8 + depth * 12}px`
      }} onClick={() => node.is_dir ? onToggleDir(node) : onOpenFile(node)} title={node.path}>
          <span className={styles.floatTreeIcon}>{getIcon(node.name, node.is_dir)}</span>
          <span>{node.name}</span>
        </div>
        {node.is_dir && node.expanded && node.children && <div>
            {node.children.map(child => renderNode(child, depth + 1))}
          </div>}
      </div>;
  };
  return <div className={styles.sidebar}>
      <div className={styles.sidebarHeader}>
        <span>Explorer</span>
        <div className={styles.sidebarHeaderActions}>
          <button className={styles.sidebarHeaderBtn} onClick={onCreateFile} title={t("yuan-code.FileTree.k1")}>+</button>
          <button className={styles.sidebarHeaderBtn} onClick={onCreateDir} title={t("Knowledge.k152")}>📁</button>
          <button className={styles.sidebarHeaderBtn} onClick={onRefresh} title={t("common.refresh")}>↻</button>
        </div>
      </div>
      <div className={styles.fileTree}>
        {fileTree.length === 0 ? <div style={{
        padding: '16px',
        color: 'var(--nt-text-muted)',
        fontSize: '12px',
        textAlign: 'center'
      }}>
            {t("yuan-code.FileTree.k2")}
          </div> : fileTree.map(node => renderNode(node))}
      </div>
    </div>;
}