import { t } from "i18next";
import { useEffect, useCallback } from 'react';
import styles from './ShortcutPanel.module.css';
interface Shortcut {
  key: string;
  desc: string;
  category: string;
}
const SHORTCUTS: Shortcut[] = [{
  key: 'Ctrl+P',
  desc: t("components.ShortcutPanel.k1"),
  category: t("components.ShortcutPanel.k2")
}, {
  key: 'Ctrl+H',
  desc: t("components.ShortcutPanel.k3"),
  category: t("components.ShortcutPanel.k2")
}, {
  key: 'Ctrl+T',
  desc: t("components.ShortcutPanel.k4"),
  category: t("components.ShortcutPanel.k2")
}, {
  key: 'Ctrl+K',
  desc: t("components.ShortcutPanel.k5"),
  category: t("components.ShortcutPanel.k2")
}, {
  key: 'Ctrl+A',
  desc: t("components.ShortcutPanel.k6"),
  category: t("components.ShortcutPanel.k2")
}, {
  key: 'Ctrl+X',
  desc: t("components.ShortcutPanel.k7"),
  category: t("components.ShortcutPanel.k2")
}, {
  key: 'Ctrl+S',
  desc: t("components.ShortcutPanel.k8"),
  category: t("components.ShortcutPanel.k2")
}, {
  key: 'Ctrl+?',
  desc: t("components.ShortcutPanel.k9"),
  category: t("components.ShortcutPanel.k10")
}, {
  key: 'Esc',
  desc: t("components.ShortcutPanel.k11"),
  category: t("components.ShortcutPanel.k10")
}, {
  key: 'F1',
  desc: t("components.ShortcutPanel.k12"),
  category: t("components.ShortcutPanel.k13")
}, {
  key: 'Ctrl+Enter',
  desc: t("components.ShortcutPanel.k14"),
  category: t("components.PermissionRestricted.k2")
}, {
  key: 'Ctrl+N',
  desc: t("components.ShortcutPanel.k15"),
  category: t("components.PermissionRestricted.k2")
}, {
  key: 'Ctrl+D',
  desc: t("components.ShortcutPanel.k16"),
  category: t("components.ShortcutPanel.k10")
}, {
  key: 'Tab',
  desc: t("components.ShortcutPanel.k17"),
  category: t("components.ShortcutPanel.k13")
}, {
  key: 'Ctrl+L',
  desc: t("components.ShortcutPanel.k18"),
  category: t("components.ShortcutPanel.k13")
}, {
  key: '↑ / ↓',
  desc: t("components.ShortcutPanel.k19"),
  category: t("components.ShortcutPanel.k13")
}, {
  key: 'Ctrl+Z',
  desc: t("components.ShortcutPanel.k20"),
  category: t("components.ShortcutPanel.k21")
}, {
  key: 'Ctrl+Y',
  desc: t("components.ShortcutPanel.k22"),
  category: t("components.ShortcutPanel.k21")
}, {
  key: 'Space',
  desc: t("components.ShortcutPanel.k23"),
  category: t("components.intelligence.DashboardPanel.k104")
}, {
  key: 'Ctrl+0',
  desc: t("components.ShortcutPanel.k24"),
  category: t("components.intelligence.DashboardPanel.k104")
}];
const CATEGORY_ICONS: Record<string, string> = {
  '导航': '🧭',
  '全局': '🌐',
  '终瑞': '🖥️',
  'AI会话': '💬',
  '编辑器': '✍️',
  '计时器': '⏱️'
};
interface ShortcutPanelProps {
  open: boolean;
  onClose: () => void;
}
export default function ShortcutPanel({
  open,
  onClose
}: ShortcutPanelProps) {
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Escape' && open) {
      onClose();
    }
  }, [open, onClose]);
  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
  if (!open) return null;
  const categories = [...new Set(SHORTCUTS.map(s => s.category))];
  return <div className={styles.overlay} onClick={onClose}>
      <div className={styles.panel} onClick={e => e.stopPropagation()}>
        <div className={styles.header}>
          <div className={styles.headerLeft}>
            <span className={styles.headerIcon}>⌨️</span>
            <span className={styles.headerTitle}>{t("components.ShortcutPanel.k25")}</span>
          </div>
          <button className={styles.closeBtn} onClick={onClose}>✕</button>
        </div>
        <div className={styles.body}>
          {categories.map(cat => <div key={cat} className={styles.category}>
              <div className={styles.categoryTitle}>
                <span className={styles.categoryIcon}>{CATEGORY_ICONS[cat] ?? '🔧'}</span>
                <span>{cat}</span>
              </div>
              <div className={styles.shortcutList}>
                {SHORTCUTS.filter(s => s.category === cat).map((s, i) => <div key={i} className={styles.shortcutItem}>
                    <kbd className={styles.keyBadge}>{s.key}</kbd>
                    <span className={styles.shortcutDesc}>{s.desc}</span>
                  </div>)}
              </div>
            </div>)}
        </div>
        <div className={styles.footer}>
          {t("components.ShortcutPanel.k26")} <kbd className={styles.keyBadge}>Ctrl+?</kbd> {t("components.ShortcutPanel.k27")}
        </div>
      </div>
    </div>;
}