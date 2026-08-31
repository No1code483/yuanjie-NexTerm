import { t } from "i18next";
/**
 * 主题选择器组件（v1.51.4 体验深化 Phase 1）
 *
 * 功能：
 *   - 10 套预设主题卡片网格
 *   - 每个卡片显示主题预览（背景/文字/强调色块）
 *   - 点击卡片即时切换主题
 *   - 当前主题高亮标记
 *   - 系统主题跟随开关
 *
 * 关联文档：功能展望/体验深化/01_主题自定义系统_详细设计.md §2.2.2
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { useThemeStore, PRESET_THEMES, type ThemeName } from '../../stores/themeStore';
import { ThemeEditor } from '../ThemeEditor/ThemeEditor';
import { FontManager } from '../FontManager/FontManager';
import { useSystemTheme } from '../../hooks/useSystemTheme';
import { MODULE_THEME_TARGETS } from '../../hooks/useModuleTheme'; // C1.4 模块级主题
import { customThemeList, customThemeDelete, customThemeMigrateLocal, type CustomTheme } from '../../lib/customThemeIpc'; // C1.5 自定义主题持久化
import styles from './ThemeSelector.module.css';
export function ThemeSelector() {
  const {
    themeName,
    followSystem,
    setTheme,
    toggleFollowSystem,
    systemThemeDark,
    systemThemeLight,
    setSystemThemeDark,
    setSystemThemeLight,
    animationsEnabled,
    animationDuration,
    setAnimationsEnabled,
    setAnimationDuration,
    moduleThemes,
    moduleThemeEnabled,
    setModuleTheme,
    clearModuleTheme,
    setModuleThemeEnabled
  } = useThemeStore();
  const [editorOpen, setEditorOpen] = useState(false);
  const [fontManagerOpen, setFontManagerOpen] = useState(false);
  const systemScheme = useSystemTheme();

  // M4.②：拆分「选中」与「激活」两个状态
  // selectedThemeId：选中待应用（控制选择框高亮边框）
  // themeName（=activeThemeId）：当前真实生效（控制"当前"标签）
  const [selectedThemeId, setSelectedThemeId] = useState<string>(themeName);
  const clickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 同步外部主题变化到选中态
  useEffect(() => {
    setSelectedThemeId(themeName);
  }, [themeName]);

  /** M4.②：单击 — 仅选中（移动选择框），不切换主题 */
  const handleCardClick = useCallback((themeName: string) => {
    // 双击检测：250ms 内的第二次点击 → 双击（真实切换）
    if (clickTimeoutRef.current) {
      clearTimeout(clickTimeoutRef.current);
      clickTimeoutRef.current = null;
      // 双击：真实切换主题
      setSelectedThemeId(themeName);
      setTheme(themeName as ThemeName);
      return;
    }
    // 单击：延迟 250ms 确认（若期间无第二次点击则为单击）
    clickTimeoutRef.current = setTimeout(() => {
      setSelectedThemeId(themeName);
      clickTimeoutRef.current = null;
    }, 250);
  }, [setTheme]);

  /** M4.②：「应用」按钮 — 等效双击，真实切换到当前选中主题 */
  const handleApplyTheme = useCallback(() => {
    setTheme(selectedThemeId as ThemeName);
  }, [selectedThemeId, setTheme]);

  /** M4.②：「取消」按钮 — 选择框回到当前激活主题 */
  const handleCancelSelection = useCallback(() => {
    setSelectedThemeId(themeName);
  }, [themeName]);

  // ===== C1.5：已保存的自定义主题列表 =====
  const [savedThemes, setSavedThemes] = useState<CustomTheme[]>([]);
  const [loadingSaved, setLoadingSaved] = useState(false);
  const [savedError, setSavedError] = useState<string | null>(null);

  /** 加载已保存的自定义主题列表 */
  const refreshSavedThemes = useCallback(async () => {
    setLoadingSaved(true);
    setSavedError(null);
    try {
      const list = await customThemeList();
      setSavedThemes(list);
    } catch (e) {
      setSavedError((e as Error).message);
    } finally {
      setLoadingSaved(false);
    }
  }, []);

  // 首次挂载时：检测 localStorage 旧数据并迁移，然后加载列表
  useEffect(() => {
    (async () => {
      // 一次性迁移：检测 localStorage 中的旧 custom-themes
      try {
        const raw = localStorage.getItem('nexterm-custom-themes');
        if (raw) {
          await customThemeMigrateLocal(raw);
          localStorage.removeItem('nexterm-custom-themes');
          console.log('[C1.5] localStorage 自定义主题已迁移到数据库');
        }
      } catch (e) {
        console.warn('[C1.5] localStorage 迁移失败（不影响功能）:', e);
      }
      await refreshSavedThemes();
    })();
  }, [refreshSavedThemes]);

  // 主题编辑器关闭后刷新列表（可能新增/更新了主题）
  useEffect(() => {
    if (!editorOpen) {
      refreshSavedThemes();
    }
  }, [editorOpen, refreshSavedThemes]);

  /** 应用已保存的自定义主题（加载其 variables 到当前主题覆盖） */
  function handleApplySavedTheme(theme: CustomTheme) {
    try {
      const overrides = JSON.parse(theme.variables) as Record<string, string>;
      // 通过 themeStore 的 setCustomVariable 逐个应用
      const store = useThemeStore.getState();
      // 先清空旧覆盖，再应用新的（保证切换主题时干净）
      store.resetCustomVariables();
      Object.entries(overrides).forEach(([k, v]) => store.setCustomVariable(k, v));
      // 同时切换基础主题（让 CSS 变量基线正确）
      if (theme.base_theme) {
        store.setTheme(theme.base_theme as ThemeName);
      }
    } catch (e) {
      console.error('[C1.5] 应用自定义主题失败:', e);
    }
  }

  /** 删除已保存的自定义主题 */
  async function handleDeleteSavedTheme(id: number, name: string) {
    if (!window.confirm(t('components.ThemeSelector.ThemeSelector.k28', { name }))) return;
    try {
      await customThemeDelete(id);
      await refreshSavedThemes();
    } catch (e) {
      setSavedError((e as Error).message);
    }
  }
  return <div className={styles.container}>
      {/* 头部：标题 + 系统跟随开关 */}
      <div className={styles.header}>
        <div className={styles.headerText}>
          <div className={styles.title}>{t("components.ThemeSelector.ThemeSelector.k1")}</div>
          <div className={styles.subtitle}>{t("components.ThemeSelector.ThemeSelector.k2")} {PRESET_THEMES.length} {t("components.ThemeSelector.ThemeSelector.k3")}</div>
        </div>
        <label className={styles.followToggle}>
          <input type="checkbox" checked={followSystem} onChange={toggleFollowSystem} className={styles.followCheckbox} />
          <span className={styles.followLabel}>{t("components.ThemeSelector.ThemeSelector.k4")}</span>
        </label>
      </div>

      {/* 主题卡片网格 */}
      <div className={styles.themeGrid}>
        {PRESET_THEMES.map(theme => {
        const isActive = themeName === theme.name;
        const isSelected = selectedThemeId === theme.name;
        const isSystemActive = themeName === 'system' && theme.name === 'terminal';
        const isDisabled = followSystem && themeName === 'system';
        return <div key={theme.name} className={`${styles.themeCard} ${isSelected ? styles.active : ''} ${isDisabled ? styles.disabled : ''}`} onClick={() => !isDisabled && handleCardClick(theme.name)} style={{
          '--card-bg': theme.preview.bg,
          '--card-text': theme.preview.text,
          '--card-accent': theme.preview.accent,
          '--card-secondary': theme.preview.secondary
        } as React.CSSProperties}>
              {/* 预览区域 */}
              <div className={styles.previewArea}>
                <div className={styles.previewHeader}>
                  <div className={styles.previewDot} style={{
                background: theme.preview.accent
              }} />
                  <div className={styles.previewDot} style={{
                background: theme.preview.secondary
              }} />
                </div>
                <div className={styles.previewText}>
                  <div className={styles.previewTitle} style={{
                color: theme.preview.accent
              }}>
                    {theme.label}
                  </div>
                  <div className={styles.previewDesc} style={{
                color: theme.preview.text
              }}>
                    {theme.description}
                  </div>
                </div>
                <div className={styles.previewSample}>
                  <span style={{
                color: theme.preview.text
              }}>Aa</span>
                  <span style={{
                color: theme.preview.accent
              }}>{t("components.ThemeSelector.ThemeSelector.k5")}</span>
                  <span style={{
                color: theme.preview.secondary
              }}>{t("components.ThemeSelector.ThemeSelector.k6")}</span>
                </div>
              </div>

              {/* 卡片底部：主题名 + 状态标记 + M4.②应用/取消按钮 */}
              <div className={styles.cardFooter}>
                <span className={styles.themeLabel}>{theme.label}</span>
                <div className={styles.themeBadges}>
                  {theme.isLight && <span className={styles.lightBadge}>{t("components.ThemeSelector.ThemeSelector.k7")}</span>}
                  {isActive && <span className={styles.activeBadge}>{t("components.ThemeSelector.ThemeSelector.k8")}</span>}
                  {isSystemActive && <span className={styles.systemBadge}>{t("components.GroupChatOrchestrationPanel.k2")}</span>}
                </div>
              </div>
              {isSelected && !isActive && !isDisabled && (
                <div className={styles.cardActions}>
                  <button type="button" className={styles.applyBtn} onClick={(e) => { e.stopPropagation(); handleApplyTheme(); }}>
                    {t("components.ThemeSelector.ThemeSelector.k5")}
                  </button>
                  <button type="button" className={styles.cancelBtn} onClick={(e) => { e.stopPropagation(); handleCancelSelection(); }}>
                    {t("components.ThemeSelector.ThemeSelector.k6")}
                  </button>
                </div>
              )}
            </div>;
      })}
      </div>

      {/* 提示信息 */}
      {followSystem && <div className={styles.hint}>
          {t("components.ThemeSelector.ThemeSelector.k9")}
        </div>}

      {/* C1.3：系统跟随时的 dark/light 主题选择器 */}
      {followSystem && (
        <div className={styles.systemThemeMapping}>
          <div className={styles.mappingRow}>
            <label className={styles.mappingLabel}>
              {t("components.ThemeSelector.ThemeSelector.k12")}
              {systemScheme === 'dark' && (
                <span className={styles.currentSchemeBadge}>
                  {t("components.ThemeSelector.ThemeSelector.k14")}
                </span>
              )}
            </label>
            <select
              className={styles.mappingSelect}
              value={systemThemeDark}
              onChange={(e) => setSystemThemeDark(e.target.value)}
            >
              {PRESET_THEMES.filter(th => !th.isLight).map(th => (
                <option key={th.name} value={th.name}>{th.label}</option>
              ))}
            </select>
          </div>
          <div className={styles.mappingRow}>
            <label className={styles.mappingLabel}>
              {t("components.ThemeSelector.ThemeSelector.k13")}
              {systemScheme === 'light' && (
                <span className={styles.currentSchemeBadge}>
                  {t("components.ThemeSelector.ThemeSelector.k14")}
                </span>
              )}
            </label>
            <select
              className={styles.mappingSelect}
              value={systemThemeLight}
              onChange={(e) => setSystemThemeLight(e.target.value)}
            >
              {PRESET_THEMES.filter(th => th.isLight).map(th => (
                <option key={th.name} value={th.name}>{th.label}</option>
              ))}
            </select>
          </div>
        </div>
      )}

      {/* C1.5：已保存的自定义主题列表 */}
      <div className={styles.savedThemesSection}>
        <div className={styles.savedThemesTitle}>
          {t("components.ThemeSelector.ThemeSelector.k26")}
        </div>
        {loadingSaved && (
          <div className={styles.savedHint}>...</div>
        )}
        {savedError && (
          <div className={styles.savedError}>{savedError}</div>
        )}
        {!loadingSaved && !savedError && savedThemes.length === 0 && (
          <div className={styles.savedHint}>
            {t("components.ThemeSelector.ThemeSelector.k27")}
          </div>
        )}
        {savedThemes.length > 0 && (
          <div className={styles.savedThemesList}>
            {savedThemes.map(theme => (
              <div key={theme.id} className={styles.savedThemeItem}>
                <button
                  type="button"
                  className={styles.savedThemeApply}
                  onClick={() => handleApplySavedTheme(theme)}
                  title={t("components.ThemeSelector.ThemeSelector.k29")}
                >
                  <span className={styles.savedThemeName}>{theme.name}</span>
                  <span className={styles.savedThemeBase}>
                    {t("components.ThemeSelector.ThemeSelector.k30")} {theme.base_theme}
                  </span>
                </button>
                <button
                  type="button"
                  className={styles.savedThemeDelete}
                  onClick={() => handleDeleteSavedTheme(theme.id, theme.name)}
                  title={t("components.ThemeSelector.ThemeSelector.k25")}
                >
                  ×
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* C1.1：自定义主题按钮 */}
      <button
        type="button"
        className={styles.customBtn}
        onClick={() => setEditorOpen(true)}
      >
        <span className={styles.customIcon}>⚙</span>
        <span>{t("components.ThemeSelector.ThemeSelector.k10")}</span>
      </button>

      {/* C1.2：字体管理按钮 */}
      <button
        type="button"
        className={styles.fontBtn}
        onClick={() => setFontManagerOpen(true)}
      >
        <span className={styles.customIcon}>Aa</span>
        <span>{t("components.ThemeSelector.ThemeSelector.k11")}</span>
      </button>

      {/* C1.6：动画与过渡配置 */}
      <div className={styles.animationConfig}>
        <label className={styles.animToggleRow}>
          <input
            type="checkbox"
            checked={animationsEnabled}
            onChange={(e) => setAnimationsEnabled(e.target.checked)}
            className={styles.animCheckbox}
          />
          <span className={styles.animToggleLabel}>
            {t("components.ThemeSelector.ThemeSelector.k15")}
          </span>
        </label>
        <div className={styles.animDurationRow}>
          <span className={styles.animDurationLabel}>
            {t("components.ThemeSelector.ThemeSelector.k16")}
          </span>
          <div className={styles.animDurationBtns}>
            {([
              { v: 100, key: 'k17' },
              { v: 200, key: 'k18' },
              { v: 400, key: 'k19' }
            ] as const).map(opt => (
              <button
                key={opt.v}
                type="button"
                className={`${styles.animDurationBtn} ${animationDuration === opt.v ? styles.animDurationActive : ''}`}
                onClick={() => setAnimationDuration(opt.v)}
                disabled={!animationsEnabled}
              >
                {t(`components.ThemeSelector.ThemeSelector.${opt.key}`)}
              </button>
            ))}
          </div>
        </div>
        {!animationsEnabled && (
          <div className={styles.animHint}>
            {t("components.ThemeSelector.ThemeSelector.k20")}
          </div>
        )}
      </div>

      {/* C1.4：模块级主题配置 */}
      <div className={styles.moduleThemeConfig}>
        <label className={styles.moduleToggleRow}>
          <input
            type="checkbox"
            checked={moduleThemeEnabled}
            onChange={(e) => setModuleThemeEnabled(e.target.checked)}
            className={styles.moduleCheckbox}
          />
          <span className={styles.moduleToggleLabel}>
            {t("components.ThemeSelector.ThemeSelector.k23")}
          </span>
        </label>
        <div className={styles.subtitle} style={{ paddingLeft: 22 }}>
          {t("components.ThemeSelector.ThemeSelector.k22")}
        </div>
        {moduleThemeEnabled && (
          <div className={styles.moduleList}>
            {MODULE_THEME_TARGETS.map(target => {
              const currentValue = moduleThemes?.[target.key] || '';
              return (
                <div key={target.key} className={styles.moduleRow}>
                  <label className={styles.moduleRowLabel}>
                    {t(target.labelKey)}
                  </label>
                  <select
                    className={styles.moduleRowSelect}
                    value={currentValue}
                    onChange={(e) => {
                      if (e.target.value === '') {
                        clearModuleTheme(target.key);
                      } else {
                        setModuleTheme(target.key, e.target.value);
                      }
                    }}
                  >
                    <option value="">
                      {t("components.ThemeSelector.ThemeSelector.k24")}
                    </option>
                    {PRESET_THEMES.map(th => (
                      <option key={th.name} value={th.name}>{th.label}</option>
                    ))}
                  </select>
                  {currentValue !== '' && (
                    <button
                      type="button"
                      className={styles.moduleClearBtn}
                      onClick={() => clearModuleTheme(target.key)}
                      title={t("components.ThemeSelector.ThemeSelector.k25")}
                    >
                      ×
                    </button>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* C1.1：主题编辑器 Modal */}
      <ThemeEditor isOpen={editorOpen} onClose={() => setEditorOpen(false)} />

      {/* C1.2：字体管理 Modal */}
      <FontManager isOpen={fontManagerOpen} onClose={() => setFontManagerOpen(false)} />
    </div>;
}
export default ThemeSelector;