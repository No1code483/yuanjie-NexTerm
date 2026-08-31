import { useTranslation } from 'react-i18next';
/**
 * 语言选择器组件（v1.51.4 体验深化 Phase 1 - C2 i18n）
 *
 * 功能：
 *   - 9 种语言卡片式切换（M5.③：中文/繁中/英/日/韩/俄/阿/法/西）
 *   - 当前语言高亮标记
 *   - ja/ko/fr/es 标记为「部分翻译」，缺失 key 自动 fallback 到 en
 *   - 切换后即时生效（C2.4：useTranslation 自动重渲染，无需刷新页面）
 *
 * M5.① 语言卡显示名走 i18n：t(`language.name.${code}`)（按当前界面语言翻译）
 * M5.② UI 减重：删除左下角重复语言名、非中文字卡去小字、卡高减半、内容居中
 *
 * 关联文档：功能展望/体验深化/02_多语言切换_i18n体系_未来展望.md
 */

import { useState } from 'react';
import { getCurrentLanguage, changeLanguage, SUPPORTED_LANGUAGES } from '@/i18n';
import styles from './LanguageSelector.module.css';

/** 语言额外元数据（SUPPORTED_LANGUAGES 之外的 UI 信息） */
const LANG_EXTRAS: Record<string, { icon: string; experimental?: boolean }> = {
  zh: { icon: '中' },
  'zh-TW': { icon: '繁' },
  en: { icon: 'En' },
  ja: { icon: '日', experimental: true },
  ko: { icon: '한', experimental: true },
  ru: { icon: 'ru' },
  ar: { icon: 'ar' },
  // M5.③：fr/es 为 en 基底 + 核心组真翻译，标记部分翻译
  fr: { icon: 'Fr', experimental: true },
  es: { icon: 'Es', experimental: true },
};

/** M5.①：仅中文卡（简/繁）保留小字说明，其余卡无小字 */
function hasSmallText(code: string): boolean {
  return code === 'zh' || code === 'zh-TW';
}

export function LanguageSelector() {
  const { t } = useTranslation();
  const [current, setCurrent] = useState(getCurrentLanguage());
  const [switching, setSwitching] = useState(false);

  const handleChange = (code: string) => {
    if (code === current || switching) return;
    setSwitching(true);
    changeLanguage(code);
    setCurrent(code);
    // C2.4：useTranslation hook 自动触发全局重渲染，无需 reload
    setSwitching(false);
  };

  return <div className={styles.container}>
      {/* 头部：标题 */}
      <div className={styles.header}>
        <div className={styles.headerText}>
          <div className={styles.title}>{t("components.LanguageSelector.LanguageSelector.k1")}</div>
          <div className={styles.subtitle}>{t("components.LanguageSelector.LanguageSelector.k2")}</div>
        </div>
      </div>

      {/* 语言卡片网格 */}
      <div className={styles.langGrid}>
        {SUPPORTED_LANGUAGES.map(lang => {
        const isActive = current === lang.code;
        const extra = LANG_EXTRAS[lang.code];
        // M5.①：语言名走 i18n，按当前界面语言翻译显示
        const displayName = t(`language.name.${lang.code}`, { defaultValue: lang.label });
        const showSmall = hasSmallText(lang.code);
        return <button key={lang.code} className={`${styles.langCard} ${isActive ? styles.active : ''}`} onClick={() => handleChange(lang.code)} disabled={switching}>
              {/* 预览区域：图标 + 大字（+ 仅中文字卡的小字说明） */}
              <div className={styles.previewArea}>
                <div className={styles.previewIcon}>{extra?.icon ?? lang.code}</div>
                <div className={styles.previewText}>
                  <div className={styles.previewTitle}>{displayName}</div>
                  {showSmall && <div className={styles.previewDesc}>{lang.nativeLabel}</div>}
                </div>
              </div>

              {/* 卡片底部：仅状态标记（M5.② 已删除左下角重复语言名） */}
              {(extra?.experimental || isActive) && (
                <div className={styles.cardFooter}>
                  <div className={styles.badges}>
                    {extra?.experimental && (
                      <span className={styles.experimentalBadge} title={t("components.LanguageSelector.LanguageSelector.k5")}>
                        {t("components.LanguageSelector.LanguageSelector.k5")}
                      </span>
                    )}
                    {isActive && <span className={styles.activeBadge}>{t("components.LanguageSelector.LanguageSelector.k3")}</span>}
                  </div>
                </div>
              )}
            </button>;
      })}
      </div>

      {/* 提示信息 */}
      <div className={styles.hint}>
        {t("components.LanguageSelector.LanguageSelector.k4")}
      </div>
    </div>;
}
export default LanguageSelector;
