/**
 * TitleBarSettingsPanel 窗口控制设置面板（M3.② 迁移自 TitleBar）
 *
 * 功能：
 *   - 从顶部导航栏齿轮入口迁移至个人中心/设置列表
 *   - 包含：标题栏风格选择 + 扩展按钮可见性 + 按钮排序 + 快捷键展示
 *   - 复用 useTitleBarStore 状态管理，与原 TitleBar 设置面板功能完全一致
 *
 * 关联文档：已有功能/14_系统工具/AI会话模型管理_导航栏_主题_语言_四项细节补充规划 M3.②
 */

import { t } from 'i18next';
import { useTitleBarStore, DEFAULT_SHORTCUTS } from '@/stores/titleBarStore';
import styles from './TitleBarSettingsPanel.module.css';

export function TitleBarSettingsPanel() {
  const {
    style,
    buttonVisibility,
    buttonOrder,
    setStyle,
    toggleButtonVisibility,
    moveButton,
  } = useTitleBarStore();

  return (
    <div className={styles.container}>
      {/* C3.4：风格选择 */}
      <div className={styles.settingsGroup}>
        <label className={styles.settingsLabel}>{t('components.TitleBar.k8')}</label>
        <select
          value={style}
          onChange={(e) => setStyle(e.target.value as 'system' | 'custom' | 'minimal')}
          className={styles.settingsSelect}
        >
          <option value="system">{t('components.TitleBar.k9')}</option>
          <option value="custom">{t('components.TitleBar.k10')}</option>
          <option value="minimal">{t('components.TitleBar.k11')}</option>
        </select>
      </div>

      {/* C3.2：扩展按钮可见性 */}
      <div className={styles.settingsGroup}>
        <label className={styles.settingsLabel}>{t('components.TitleBar.k12')}</label>
        {(['pin', 'devtools', 'screenshot', 'record'] as const).map(btn => (
          <label key={btn} className={styles.checkboxRow}>
            <input
              type="checkbox"
              checked={buttonVisibility[btn]}
              onChange={() => toggleButtonVisibility(btn)}
            />
            <span>{t(`components.TitleBar.k${{ pin: 3, devtools: 4, screenshot: 5, record: 6 }[btn]}`)}</span>
          </label>
        ))}
      </div>

      {/* C3.3：按钮排序 */}
      <div className={styles.settingsGroup}>
        <label className={styles.settingsLabel}>{t('components.TitleBar.k13')}</label>
        {buttonOrder.map(btn => (
          <div key={btn} className={styles.sortRow}>
            <span>{t(`components.TitleBar.k${
              { minimize: 14, maximize: 15, close: 16, pin: 3, devtools: 4, screenshot: 5, record: 6 }[btn]
            }`)}</span>
            <div className={styles.sortBtns}>
              <button type="button" onClick={() => moveButton(btn, 'up')} className={styles.sortBtn}>↑</button>
              <button type="button" onClick={() => moveButton(btn, 'down')} className={styles.sortBtn}>↓</button>
            </div>
          </div>
        ))}
      </div>

      {/* C3.6：快捷键展示 */}
      <div className={styles.settingsGroup}>
        <label className={styles.settingsLabel}>{t('components.TitleBar.k23')}</label>
        <div className={styles.shortcutRow}>
          <span className={styles.shortcutLabel}>{t('common.maximize')}</span>
          <span className={styles.shortcutKey}>{DEFAULT_SHORTCUTS.maximize}</span>
        </div>
        <div className={styles.shortcutRow}>
          <span className={styles.shortcutLabel}>{t('components.TitleBar.k3')}</span>
          <span className={styles.shortcutKey}>{DEFAULT_SHORTCUTS.pin}</span>
        </div>
        <div className={styles.shortcutRow}>
          <span className={styles.shortcutLabel}>{t('components.TitleBar.k5')}</span>
          <span className={styles.shortcutKey}>{DEFAULT_SHORTCUTS.screenshot}</span>
        </div>
        <div className={styles.shortcutRow}>
          <span className={styles.shortcutLabel}>{t('components.TitleBar.k4')}</span>
          <span className={styles.shortcutKey}>{DEFAULT_SHORTCUTS.devtools}</span>
        </div>
        <div className={styles.shortcutRow}>
          <span className={styles.shortcutLabel}>{t('components.TitleBar.k6')}</span>
          <span className={styles.shortcutKey}>{DEFAULT_SHORTCUTS.record}</span>
        </div>
      </div>
    </div>
  );
}

export default TitleBarSettingsPanel;
