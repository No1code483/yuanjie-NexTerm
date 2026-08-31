import { useTranslation } from 'react-i18next';
/**
 * LanguageSwitcher 紧凑型语言切换器（C2.2）
 *
 * 功能：
 *   - 紧凑的图标按钮 + 下拉菜单形式（适合顶部栏/标题栏嵌入）
 *   - 显示当前语言的 2 字母代码（ZH/EN/JA/KO）
 *   - 点击展开下拉菜单，列出所有支持的语言
 *   - 选中后立即切换 + 持久化（C2.4：useTranslation 自动重渲染，无需刷新）
 *   - ja/ko 标记「部分翻译」badge
 *   - 点击外部自动收起
 *   - ESC 键关闭
 *
 * 与 LanguageSelector 的区别：
 *   - LanguageSelector：设置页内卡片式（4 语言大卡片）
 *   - LanguageSwitcher：顶部栏紧凑下拉（2 字母代码 + 菜单）
 *
 * 关联文档：功能展望/体验深化/02_多语言切换_i18n体系_未来展望.md §2.1
 */

import { useState, useRef, useEffect, useId } from 'react';
import { getCurrentLanguage, changeLanguage, SUPPORTED_LANGUAGES } from '@/i18n';
import styles from './LanguageSwitcher.module.css';

export interface LanguageSwitcherProps {
  /** 自定义容器类名 */
  className?: string;
  /** 尺寸 */
  size?: 'small' | 'medium';
}

export function LanguageSwitcher({ className = '', size = 'medium' }: LanguageSwitcherProps) {
  const { t } = useTranslation();
  const [current, setCurrent] = useState(getCurrentLanguage());
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const menuId = useId();

  // 同步外部语言变化（如其他组件切换后）
  useEffect(() => {
    const handler = () => setCurrent(getCurrentLanguage());
    window.addEventListener('languagechange', handler);
    return () => window.removeEventListener('languagechange', handler);
  }, []);

  // 点击外部收起
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [isOpen]);

  // ESC 收起
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setIsOpen(false);
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [isOpen]);

  const handleSelect = (code: string) => {
    if (code === current) {
      setIsOpen(false);
      return;
    }
    changeLanguage(code);
    setCurrent(code);
    setIsOpen(false);
    // 派发自定义事件，通知其他组件
    window.dispatchEvent(new CustomEvent('languagechange', { detail: { code } }));
    // C2.4：useTranslation hook 自动触发全局重渲染，无需 reload
  };

  const currentLang = SUPPORTED_LANGUAGES.find(l => l.code === current) ?? SUPPORTED_LANGUAGES[0];
  const currentCodeUpper = current.toUpperCase();

  return (
    <div
      ref={containerRef}
      className={`${styles.container} ${styles[size]} ${className}`}
    >
      <button
        type="button"
        className={`${styles.trigger} ${isOpen ? styles.open : ''}`}
        onClick={() => setIsOpen(!isOpen)}
        aria-label={t('a11y.languageSwitch')}
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-controls={isOpen ? menuId : undefined}
        title={currentLang.nativeLabel}
      >
        <span className={styles.codeBadge}>{currentCodeUpper}</span>
        <span className={styles.arrow}>{isOpen ? '▲' : '▼'}</span>
      </button>

      {isOpen && (
        <div id={menuId} className={styles.menu} role="listbox">
          {SUPPORTED_LANGUAGES.map(lang => {
            const isActive = current === lang.code;
            const isExperimental = lang.code === 'ja' || lang.code === 'ko';
            return (
              <div
                key={lang.code}
                role="option"
                aria-selected={isActive}
                className={`${styles.option} ${isActive ? styles.active : ''}`}
                onClick={() => handleSelect(lang.code)}
              >
                <div className={styles.optionMain}>
                  <span className={styles.optionLabel}>{lang.label}</span>
                  <span className={styles.optionNative}>{lang.nativeLabel}</span>
                </div>
                {isExperimental && (
                  <span className={styles.experimentalTag}>
                    {t('components.LanguageSelector.LanguageSelector.k5')}
                  </span>
                )}
                {isActive && <span className={styles.checkMark}>✓</span>}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

export default LanguageSwitcher;
