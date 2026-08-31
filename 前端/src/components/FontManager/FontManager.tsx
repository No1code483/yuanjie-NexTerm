/**
 * FontManager 字体管理（C1.2 / v1.51.5）
 *
 * 功能：
 *   - Modal 弹窗形式（点击 ThemeSelector 中的"字体管理"按钮触发）
 *   - 字体选择：5 个内置字体 + 用户上传的自定义字体
 *   - 字号调节：4 个预设按钮（12/14/16/18）+ 12-24px 滑块
 *   - 字体上传：调用 Tauri dialog 选择 .ttf/.otf/.woff/.woff2 文件
 *   - 字体删除：删除已上传的自定义字体
 *   - 实时预览：选择字体/字号时立即应用到 DOM
 *
 * 关联文档：功能展望/体验深化/01_主题自定义系统_未来展望.md §2.2
 */

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { t } from 'i18next';
import { useThemeStore } from '../../stores/themeStore';
import {
  BUILTIN_FONTS,
  FONT_FORMAT_MAP,
  loadCustomFonts,
  type FontMeta,
} from './fontList';
import styles from './FontManager.module.css';

/** 后端返回的自定义字体结构 */
interface CustomFont {
  filename: string;
  family_name: string;
  file_size: number;
  format: string; // ttf / otf / woff / woff2
}

/** 后端统一响应 */
interface ApiResponse<T> {
  code: number;
  message: string;
  data?: T;
}

/** 字号预设 */
const FONT_SIZE_PRESETS = [12, 14, 16, 18];

export interface FontManagerProps {
  /** 是否显示 */
  isOpen: boolean;
  /** 关闭回调 */
  onClose: () => void;
}

export function FontManager({ isOpen, onClose }: FontManagerProps) {
  const { fontFamily, fontSize, setFontFamily, setFontSize } = useThemeStore();
  const [customFonts, setCustomFonts] = useState<CustomFont[]>([]);
  const [loading, setLoading] = useState(false);
  const [toast, setToast] = useState<{ type: 'success' | 'error'; msg: string } | null>(null);

  /** 显示 toast 提示 */
  const showToast = useCallback((type: 'success' | 'error', msg: string) => {
    setToast({ type, msg });
  }, []);

  /** 刷新自定义字体列表 */
  const refreshCustomFonts = useCallback(async () => {
    setLoading(true);
    try {
      // 获取字体目录路径（用于拼接字体文件完整路径）
      const dirRes = await invoke<ApiResponse<string>>('font_get_dir');
      const fontsDirPath = dirRes.code === 0 && dirRes.data ? dirRes.data : '';

      const listRes = await invoke<ApiResponse<CustomFont[]>>('font_list_custom');
      if (listRes.code === 0 && listRes.data) {
        setCustomFonts(listRes.data);
        // 注入 @font-face 规则
        if (fontsDirPath) {
          const fontsMeta: FontMeta[] = listRes.data.map((f) => {
            // 拼接绝对路径并转为 asset:// URL
            const fullPath = `${fontsDirPath}/${f.filename}`.replace(/\\/g, '/');
            return {
              family: f.family_name,
              labelKey: '',
              source: 'custom',
              filename: f.filename,
              url: convertFileSrc(fullPath),
              format: FONT_FORMAT_MAP[f.format] || 'truetype',
            };
          });
          loadCustomFonts(fontsMeta);
        }
      }
    } catch (e) {
      showToast('error', (e as Error).message || t('components.FontManager.k1'));
    } finally {
      setLoading(false);
    }
  }, [showToast, t]);

  // 打开 Modal 时加载字体列表
  useEffect(() => {
    if (isOpen) {
      refreshCustomFonts();
    }
  }, [isOpen, refreshCustomFonts]);

  // ESC 关闭
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [isOpen, onClose]);

  // toast 自动消失
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(null), 2500);
    return () => clearTimeout(timer);
  }, [toast]);

  if (!isOpen) return null;

  /** 上传字体文件 */
  async function handleUpload() {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [
          { name: t('components.FontManager.k2'), extensions: ['ttf', 'otf', 'woff', 'woff2'] },
        ],
      });
      if (!selected) return;

      const res = await invoke<ApiResponse<CustomFont>>('font_upload', { srcPath: selected });
      if (res.code === 0) {
        showToast('success', t('components.FontManager.k3', { name: res.data?.family_name || '' }));
        await refreshCustomFonts();
      } else {
        showToast('error', res.message || t('components.FontManager.k4'));
      }
    } catch (e) {
      showToast('error', (e as Error).message || t('components.FontManager.k4'));
    }
  }

  /** 删除字体 */
  async function handleDelete(filename: string, familyName: string) {
    if (!window.confirm(t('components.FontManager.k5', { name: familyName }))) return;
    try {
      const res = await invoke<ApiResponse<null>>('font_delete', { filename });
      if (res.code === 0) {
        showToast('success', t('components.FontManager.k6'));
        // 如果当前正在使用被删除的字体，回退到默认
        if (fontFamily === familyName) {
          setFontFamily(BUILTIN_FONTS[0].family);
        }
        await refreshCustomFonts();
      } else {
        showToast('error', res.message || t('components.FontManager.k7'));
      }
    } catch (e) {
      showToast('error', (e as Error).message || t('components.FontManager.k7'));
    }
  }

  /** 选择字体（内置或自定义） */
  function handleSelectFont(family: string) {
    setFontFamily(family);
  }

  /** 格式化文件大小 */
  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }

  /** 判断是否为当前选中字体 */
  function isFontActive(family: string): boolean {
    // fontFamily 可能是 "'JetBrains Mono', monospace" 这种复合值
    // 而 custom 字体的 family 是纯名字
    if (fontFamily === family) return true;
    // 检查 fontFamily 是否包含该 family（用引号包围或开头）
    return fontFamily.includes(`'${family}'`) || fontFamily.startsWith(family);
  }

  return (
    <div className={styles.overlay} onClick={onClose}>
      <div className={styles.dialog} onClick={(e) => e.stopPropagation()}>
        {/* 标题栏 */}
        <div className={styles.header}>
          <span className={styles.title}>{t('components.FontManager.k8')}</span>
          <button className={styles.closeBtn} onClick={onClose} title={t('components.FontManager.k9')}>
            ×
          </button>
        </div>

        {/* 主体内容 */}
        <div className={styles.body}>
          {/* ===== 字体选择 ===== */}
          <section className={styles.section}>
            <h3 className={styles.sectionTitle}>{t('components.FontManager.k10')}</h3>

            <h4 className={styles.subsectionTitle}>{t('components.FontManager.k11')}</h4>
            <div className={styles.fontGrid}>
              {BUILTIN_FONTS.map((font) => (
                <button
                  key={font.family}
                  className={`${styles.fontCard} ${isFontActive(font.family) ? styles.active : ''}`}
                  onClick={() => handleSelectFont(font.family)}
                  style={{ fontFamily: font.family }}
                >
                  <div className={styles.fontPreview}>Aa Bb 123</div>
                  <div className={styles.fontLabel}>{t(font.labelKey)}</div>
                </button>
              ))}
            </div>

            <h4 className={styles.subsectionTitle}>
              {t('components.FontManager.k12')}
              <button
                className={styles.uploadBtn}
                onClick={handleUpload}
                disabled={loading}
                title={t('components.FontManager.k13')}
              >
                + {t('components.FontManager.k14')}
              </button>
            </h4>
            {customFonts.length === 0 ? (
              <div className={styles.emptyHint}>
                {t('components.FontManager.k15')}
              </div>
            ) : (
              <div className={styles.customFontList}>
                {customFonts.map((font) => (
                  <div
                    key={font.filename}
                    className={`${styles.customFontItem} ${isFontActive(font.family_name) ? styles.active : ''}`}
                  >
                    <button
                      className={styles.customFontInfo}
                      onClick={() => handleSelectFont(font.family_name)}
                      style={{ fontFamily: `'${font.family_name}', monospace` }}
                    >
                      <div className={styles.customFontPreview}>Aa Bb 123</div>
                      <div className={styles.customFontMeta}>
                        <span className={styles.customFontName}>{font.family_name}</span>
                        <span className={styles.customFontDetail}>
                          .{font.format} · {formatSize(font.file_size)}
                        </span>
                      </div>
                    </button>
                    <button
                      className={styles.deleteBtn}
                      onClick={() => handleDelete(font.filename, font.family_name)}
                      title={t('components.FontManager.k16')}
                    >
                      ×
                    </button>
                  </div>
                ))}
              </div>
            )}
          </section>

          {/* ===== 字号调节 ===== */}
          <section className={styles.section}>
            <h3 className={styles.sectionTitle}>{t('components.FontManager.k17')}</h3>

            <div className={styles.fontSizePresets}>
              {FONT_SIZE_PRESETS.map((size) => (
                <button
                  key={size}
                  className={`${styles.presetBtn} ${fontSize === size ? styles.active : ''}`}
                  onClick={() => setFontSize(size)}
                >
                  {size}px
                </button>
              ))}
            </div>

            <div className={styles.fontSizeSlider}>
              <input
                type="range"
                min={12}
                max={24}
                step={1}
                value={fontSize}
                onChange={(e) => setFontSize(parseInt(e.target.value, 10))}
                className={styles.slider}
              />
              <div className={styles.sizeValue}>{fontSize}px</div>
            </div>
          </section>

          {/* ===== 预览 ===== */}
          <section className={styles.section}>
            <h3 className={styles.sectionTitle}>{t('components.FontManager.k18')}</h3>
            <div
              className={styles.previewBox}
              style={{ fontFamily: fontFamily, fontSize: `${fontSize}px` }}
            >
              <p>{t('components.FontManager.k19')}</p>
              <pre className={styles.previewCode}>{`fn main() {
    println!("Hello, NexTerm!");
    let x = 42;
    println!("x = {}", x);
}`}</pre>
            </div>
          </section>
        </div>

        {/* 底部操作栏 */}
        <div className={styles.footer}>
          <span className={styles.hint}>{t('components.FontManager.k20')}</span>
          <button className={styles.closeFooterBtn} onClick={onClose}>
            {t('components.FontManager.k21')}
          </button>
        </div>

        {/* toast */}
        {toast && (
          <div className={`${styles.toast} ${styles[toast.type]}`}>{toast.msg}</div>
        )}
      </div>
    </div>
  );
}

export default FontManager;
