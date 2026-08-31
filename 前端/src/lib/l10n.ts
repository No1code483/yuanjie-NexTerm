/**
 * L10n 本地化工具（C2.6 / v1.51.9）
 *
 * 基于 Intl API 实现日期/数字/货币的本地化格式化。
 * 自动根据当前 i18n 语言切换 locale。
 *
 * 用法：
 *   import { formatDate, formatNumber, formatCurrency, formatRelativeTime } from '@/lib/l10n';
 *   formatDate(new Date())        // 2026年7月22日 / Jul 22, 2026 / 2026年7月22日
 *   formatNumber(1234567.89)      // 1,234,567.89 / 1.234.567,89
 *   formatCurrency(99.5, 'CNY')   // ¥99.50 / ¥99.50
 *
 * 关联文档：功能展望/体验深化/02_多语言切换_i18n体系_未来展望.md §C2.6
 */

import i18n from '../i18n';

/** 语言代码 → Intl locale 映射 */
const LANG_TO_LOCALE: Record<string, string> = {
  zh: 'zh-CN',
  en: 'en-US',
  ja: 'ja-JP',
  ko: 'ko-KR',
  ar: 'ar-SA',
  he: 'he-IL',
  fa: 'fa-IR',
};

/** 获取当前 i18n 语言对应的 Intl locale */
function getCurrentLocale(): string {
  const lang = i18n.language?.split('-')[0] || 'zh';
  return LANG_TO_LOCALE[lang] || 'zh-CN';
}

/**
 * 格式化日期
 * @param date 日期对象/时间戳/日期字符串
 * @param options Intl.DateTimeFormatOptions
 * @returns 本地化的日期字符串
 */
export function formatDate(
  date: Date | number | string,
  options: Intl.DateTimeFormatOptions = { year: 'numeric', month: 'long', day: 'numeric' },
): string {
  const d = typeof date === 'string' ? new Date(date) : typeof date === 'number' ? new Date(date) : date;
  try {
    return new Intl.DateTimeFormat(getCurrentLocale(), options).format(d);
  } catch {
    return d.toISOString().split('T')[0];
  }
}

/**
 * 格式化日期时间
 * @param date 日期对象/时间戳/日期字符串
 * @returns 本地化的日期时间字符串
 */
export function formatDateTime(
  date: Date | number | string,
): string {
  return formatDate(date, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/**
 * 格式化相对时间（如"3小时前"、"2天后"）
 * @param date 日期对象/时间戳/日期字符串
 * @returns 本地化的相对时间字符串
 */
export function formatRelativeTime(date: Date | number | string): string {
  const d = typeof date === 'string' ? new Date(date) : typeof date === 'number' ? new Date(date) : date;
  const now = Date.now();
  const diff = (d.getTime() - now) / 1000; // 秒
  try {
    const rtf = new Intl.RelativeTimeFormat(getCurrentLocale(), { numeric: 'auto' });
    const absDiff = Math.abs(diff);
    if (absDiff < 60) return rtf.format(Math.round(diff), 'second');
    if (absDiff < 3600) return rtf.format(Math.round(diff / 60), 'minute');
    if (absDiff < 86400) return rtf.format(Math.round(diff / 3600), 'hour');
    if (absDiff < 2592000) return rtf.format(Math.round(diff / 86400), 'day');
    if (absDiff < 31536000) return rtf.format(Math.round(diff / 2592000), 'month');
    return rtf.format(Math.round(diff / 31536000), 'year');
  } catch {
    return formatDate(d);
  }
}

/**
 * 格式化数字
 * @param number 数字
 * @param options Intl.NumberFormatOptions
 * @returns 本地化的数字字符串
 */
export function formatNumber(
  number: number,
  options: Intl.NumberFormatOptions = {},
): string {
  try {
    return new Intl.NumberFormat(getCurrentLocale(), options).format(number);
  } catch {
    return String(number);
  }
}

/**
 * 格式化百分比
 * @param ratio 比率（0-1）
 * @param decimals 小数位数
 * @returns 本地化的百分比字符串
 */
export function formatPercent(ratio: number, decimals = 1): string {
  return formatNumber(ratio, {
    style: 'percent',
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
}

/**
 * 格式化文件大小
 * @param bytes 字节数
 * @returns 本地化的文件大小字符串（如 "1.5 MB"）
 */
export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${formatNumber(bytes)} B`;
  if (bytes < 1024 * 1024) return `${formatNumber(bytes / 1024, { maximumFractionDigits: 1 })} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${formatNumber(bytes / (1024 * 1024), { maximumFractionDigits: 2 })} MB`;
  return `${formatNumber(bytes / (1024 * 1024 * 1024), { maximumFractionDigits: 2 })} GB`;
}

/**
 * 格式化货币
 * @param amount 金额
 * @param currency ISO 4217 货币代码（CNY/USD/JPY/KRW 等）
 * @returns 本地化的货币字符串
 */
export function formatCurrency(
  amount: number,
  currency: string = 'CNY',
): string {
  try {
    return new Intl.NumberFormat(getCurrentLocale(), {
      style: 'currency',
      currency,
    }).format(amount);
  } catch {
    return `${currency} ${formatNumber(amount)}`;
  }
}
