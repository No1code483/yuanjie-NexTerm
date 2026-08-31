/**
 * C4 §2.1.2 颜色对比度工具
 *
 * 实现 WCAG 2.1 相对亮度 + 对比度比计算：
 *   contrast = (L_lighter + 0.05) / (L_darker + 0.05)
 *
 * 设计依据：功能展望/体验深化/04_无障碍_A11y合规.md §2.1.2 / §2.1.3
 *
 * 用法：
 *   calculateContrast('#00FF00', '#000000')        // 15.3
 *   meetsAAContrast('#00FF00', '#000000')           // true  (>= 4.5)
 *   meetsAAAContrast('#00FF00', '#000000')          // true  (>= 7)
 */

interface RGB {
  r: number;
  g: number;
  b: number;
}

/** 解析 #RGB / #RRGGBB / rgb()/rgba() 颜色为 RGB 对象 */
export function parseColor(color: string): RGB | null {
  if (!color) return null;
  const trimmed = color.trim().toLowerCase();

  // #RRGGBB / #RGB
  const hexMatch = trimmed.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i);
  if (hexMatch) {
    let hex = hexMatch[1];
    if (hex.length === 3) {
      hex = hex.split('').map(c => c + c).join('');
    }
    const num = parseInt(hex, 16);
    return {
      r: (num >> 16) & 0xff,
      g: (num >> 8) & 0xff,
      b: num & 0xff,
    };
  }

  // rgb()/rgba()
  const rgbMatch = trimmed.match(/^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/i);
  if (rgbMatch) {
    return {
      r: parseInt(rgbMatch[1], 10),
      g: parseInt(rgbMatch[2], 10),
      b: parseInt(rgbMatch[3], 10),
    };
  }

  return null;
}

/** 计算相对亮度（WCAG 2.1 公式） */
export function getRelativeLuminance({ r, g, b }: RGB): number {
  const [R, G, B] = [r, g, b].map(c => {
    const srgb = c / 255;
    return srgb <= 0.03928 ? srgb / 12.92 : Math.pow((srgb + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * R + 0.7152 * G + 0.0722 * B;
}

/** 计算两色对比度比（1.0 ~ 21.0） */
export function calculateContrast(foreground: string, background: string): number {
  const fg = parseColor(foreground);
  const bg = parseColor(background);
  if (!fg || !bg) return 0;

  const l1 = getRelativeLuminance(fg);
  const l2 = getRelativeLuminance(bg);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

/** 是否满足 WCAG AA（>= 4.5，普通文本） */
export function meetsAAContrast(foreground: string, background: string): boolean {
  return calculateContrast(foreground, background) >= 4.5;
}

/** 是否满足 WCAG AAA（>= 7，普通文本） */
export function meetsAAAContrast(foreground: string, background: string): boolean {
  return calculateContrast(foreground, background) >= 7;
}

/** 是否满足大文本 AA（>= 3.0，18pt 或 14pt 加粗） */
export function meetsAALargeContrast(foreground: string, background: string): boolean {
  return calculateContrast(foreground, background) >= 3.0;
}
