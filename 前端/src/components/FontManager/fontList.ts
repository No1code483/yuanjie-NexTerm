/**
 * C1.2 字体清单 + 字体加载器（v1.51.5）
 *
 * 架构说明：
 *   - 5 个内置字体声明（不打包字体文件，依赖系统已安装的字体，自动 fallback）
 *   - 用户上传的自定义字体通过 @font-face 动态加载（asset:// 协议）
 *   - 字体文件存储在 %APPDATA%/NexTerm/fonts/
 *
 * 关联文档：功能展望/体验深化/01_主题自定义系统_未来展望.md §2.2
 */

export interface FontMeta {
  /** 字体家族名（CSS font-family 值，含引号） */
  family: string;
  /** 显示名（i18n key 后缀，用于翻译） */
  labelKey: string;
  /** 字体来源：builtin（内置声明，依赖系统已装）/ system（系统字体）/ custom（用户上传） */
  source: 'builtin' | 'system' | 'custom';
  /** 字体文件名（仅 source=custom 有值） */
  filename?: string;
  /** 字体文件 asset:// URL（仅 source=custom 有值，用于 @font-face url） */
  url?: string;
  /** 字体格式（仅 source=custom 有值：truetype/opentype/woff/woff2） */
  format?: string;
}

/**
 * 内置字体清单（5 个等宽字体声明）
 *
 * 说明：不打包字体文件，仅声明 CSS font-family。
 * 如果系统已安装该字体则自动使用；否则 fallback 到下一个或 monospace。
 */
export const BUILTIN_FONTS: FontMeta[] = [
  {
    family: "'JetBrains Mono', 'Consolas', monospace",
    labelKey: 'components.FontManager.builtin.jetbrainsMono',
    source: 'builtin'
  },
  {
    family: "'Cascadia Code', 'Consolas', monospace",
    labelKey: 'components.FontManager.builtin.cascadiaCode',
    source: 'builtin'
  },
  {
    family: "'Fira Code', 'Consolas', monospace",
    labelKey: 'components.FontManager.builtin.firaCode',
    source: 'builtin'
  },
  {
    family: "'Source Code Pro', 'Consolas', monospace",
    labelKey: 'components.FontManager.builtin.sourceCodePro',
    source: 'builtin'
  },
  {
    family: "'Consolas', 'Courier New', monospace",
    labelKey: 'components.FontManager.builtin.consolas',
    source: 'system'
  }
];

/** @font-face <style> 元素 ID */
const FONT_FACE_STYLE_ID = 'nexterm-custom-fonts-style';

/**
 * 为自定义字体动态注入 @font-face 规则
 * @param fonts 自定义字体清单
 */
export function loadCustomFonts(fonts: FontMeta[]): void {
  if (typeof document === 'undefined') return;

  let style = document.getElementById(FONT_FACE_STYLE_ID) as HTMLStyleElement | null;
  if (!style) {
    style = document.createElement('style');
    style.id = FONT_FACE_STYLE_ID;
    document.head.appendChild(style);
  }

  const cssRules = fonts
    .filter((f) => f.source === 'custom' && f.url && f.format)
    .map((f) => {
      // family_name 用作 font-family（不含引号，因为用户上传的文件名可能含空格）
      const family = f.family.replace(/'/g, '');
      return `@font-face {
  font-family: '${family}';
  src: url('${f.url}') format('${f.format}');
  font-display: swap;
}`;
    })
    .join('\n\n');

  style.textContent = cssRules;
}

/**
 * 清除所有自定义字体 @font-face 规则
 */
export function clearCustomFonts(): void {
  if (typeof document === 'undefined') return;
  const style = document.getElementById(FONT_FACE_STYLE_ID);
  if (style) style.textContent = '';
}

/** @font-face format() 值映射 */
export const FONT_FORMAT_MAP: Record<string, string> = {
  ttf: 'truetype',
  otf: 'opentype',
  woff: 'woff',
  woff2: 'woff2'
};
