/**
 * ThemePreviewPanel 主题预览面板（C1.1 主题编辑器组件）
 *
 * 功能：
 *   - 实时反映当前主题变量变化（无需"应用"按钮）
 *   - 模拟典型 UI 组件：标题/段落/按钮/输入框/列表/卡片/标签/终端文本
 *   - 通过引用 --nt-* 语义层变量自动跟随主题变化
 *
 * 关联文档：功能展望/体验深化/01_主题自定义系统_未来展望.md §2.1 ThemePreviewPanel
 */

import { t } from 'i18next';
import styles from './ThemePreviewPanel.module.css';

export interface ThemePreviewPanelProps {
  /** 可选的额外 className */
  className?: string;
}

/**
 * 预览面板：渲染一组典型 UI 组件，所有颜色/字体/几何引用 CSS 变量
 * 当 ThemeEditor 修改变量时，本面板自动更新外观
 */
export function ThemePreviewPanel({ className }: ThemePreviewPanelProps) {
  return (
    <div className={`${styles.container} ${className || ''}`}>
      <div className={styles.header}>
        <span className={styles.title}>{t('components.ThemeEditor.ThemePreviewPanel.k1')}</span>
        <span className={styles.subtitle}>{t('components.ThemeEditor.ThemePreviewPanel.k2')}</span>
      </div>

      <div className={styles.previewContent}>
        {/* 标题与正文 */}
        <section className={styles.section}>
          <h3 className={styles.previewTitle}>{t('components.ThemeEditor.ThemePreviewPanel.k3')}</h3>
          <p className={styles.previewText}>{t('components.ThemeEditor.ThemePreviewPanel.k4')}</p>
          <p className={styles.previewMuted}>{t('components.ThemeEditor.ThemePreviewPanel.k5')}</p>
        </section>

        {/* 按钮组 */}
        <section className={styles.section}>
          <div className={styles.sectionLabel}>{t('components.ThemeEditor.ThemePreviewPanel.k6')}</div>
          <div className={styles.buttonRow}>
            <button className={styles.btnPrimary}>{t('components.ThemeEditor.ThemePreviewPanel.k7')}</button>
            <button className={styles.btnSecondary}>{t('components.ThemeEditor.ThemePreviewPanel.k8')}</button>
            <button className={styles.btnDanger}>{t('components.ThemeEditor.ThemePreviewPanel.k9')}</button>
            <button className={styles.btnWarning}>{t('components.ThemeEditor.ThemePreviewPanel.k10')}</button>
          </div>
        </section>

        {/* 输入框 */}
        <section className={styles.section}>
          <div className={styles.sectionLabel}>{t('components.ThemeEditor.ThemePreviewPanel.k11')}</div>
          <input
            type="text"
            className={styles.input}
            placeholder={t('components.ThemeEditor.ThemePreviewPanel.k12')}
            readOnly
          />
        </section>

        {/* 列表 */}
        <section className={styles.section}>
          <div className={styles.sectionLabel}>{t('components.ThemeEditor.ThemePreviewPanel.k13')}</div>
          <ul className={styles.list}>
            <li className={styles.listItem}>
              <span className={styles.listDot} />
              <span className={styles.listText}>{t('components.ThemeEditor.ThemePreviewPanel.k14')}</span>
              <span className={styles.badgeActive}>{t('components.ThemeEditor.ThemePreviewPanel.k15')}</span>
            </li>
            <li className={styles.listItem}>
              <span className={styles.listDotSecondary} />
              <span className={styles.listText}>{t('components.ThemeEditor.ThemePreviewPanel.k16')}</span>
              <span className={styles.badgeMuted}>{t('components.ThemeEditor.ThemePreviewPanel.k17')}</span>
            </li>
            <li className={styles.listItem}>
              <span className={styles.listDotWarning} />
              <span className={styles.listText}>{t('components.ThemeEditor.ThemePreviewPanel.k18')}</span>
              <span className={styles.badgeWarning}>{t('components.ThemeEditor.ThemePreviewPanel.k19')}</span>
            </li>
          </ul>
        </section>

        {/* 卡片 */}
        <section className={styles.section}>
          <div className={styles.sectionLabel}>{t('components.ThemeEditor.ThemePreviewPanel.k20')}</div>
          <div className={styles.card}>
            <div className={styles.cardHeader}>
              <span className={styles.cardTitle}>{t('components.ThemeEditor.ThemePreviewPanel.k21')}</span>
              <span className={styles.cardMeta}>v1.53</span>
            </div>
            <div className={styles.cardBody}>
              {t('components.ThemeEditor.ThemePreviewPanel.k22')}
            </div>
          </div>
        </section>

        {/* 终端 ANSI 色彩 */}
        <section className={styles.section}>
          <div className={styles.sectionLabel}>{t('components.ThemeEditor.ThemePreviewPanel.k23')}</div>
          <div className={styles.terminal}>
            <div className={styles.terminalLine}>
              <span className={styles.ansiRed}>●</span>
              <span className={styles.ansiGreen}>●</span>
              <span className={styles.ansiYellow}>●</span>
              <span className={styles.ansiBlue}>●</span>
              <span className={styles.ansiMagenta}>●</span>
              <span className={styles.ansiCyan}>●</span>
              <span className={styles.ansiWhite}>●</span>
            </div>
            <div className={styles.terminalLine}>
              <span className={styles.prompt}>$</span>
              <span className={styles.terminalText}>{t('components.ThemeEditor.ThemePreviewPanel.k24')}</span>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}

export default ThemePreviewPanel;
