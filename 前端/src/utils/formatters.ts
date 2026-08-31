/**
 * 本地化格式化工具（C2.5 数字/日期/时间本地化）
 *
 * 功能：
 *   - 日期格式化（formatDate）
 *   - 时间格式化（formatTime）
 *   - 日期时间格式化（formatDateTime）
 *   - 相对时间格式化（formatRelativeTime）
 *   - 数字格式化（formatNumber）
 *   - 货币格式化（formatCurrency）
 *   - 文件大小格式化（formatFileSize）
 *
 * 实现：
 *   - 基于 Intl.DateTimeFormat / Intl.NumberFormat / Intl.RelativeTimeFormat
 *   - 自动根据 i18n.language 选择 locale
 *   - 提供 hook 版本（useFormatters）以便响应语言变化
 *
 * 关联文档：功能展望/体验深化/02_多语言切换_i18n体系_未来展望.md §2.4
 */

import { useMemo } from 'react';
import i18n from '@/i18n';

/** 语言代码 → Intl locale 映射 */
const LANG_TO_LOCALE: Record<string, string> = {
  zh: 'zh-CN',
  en: 'en-US',
  ja: 'ja-JP',
  ko: 'ko-KR',
  ru: 'ru-RU',
  ar: 'ar-SA',
};

/** 获取当前语言对应的 Intl locale */
export function getCurrentLocale(): string {
  const lang = i18n.language ?? 'zh';
  return LANG_TO_LOCALE[lang] || 'zh-CN';
}

/** 日期格式化选项 */
export interface FormatDateOptions {
  year?: 'numeric' | '2-digit';
  month?: 'numeric' | '2-digit' | 'long' | 'short' | 'narrow';
  day?: 'numeric' | '2-digit';
  weekday?: 'long' | 'short' | 'narrow';
}

/** 时间格式化选项 */
export interface FormatTimeOptions {
  hour?: 'numeric' | '2-digit';
  minute?: 'numeric' | '2-digit';
  second?: 'numeric' | '2-digit';
  hour12?: boolean;
}

/** 格式化日期（仅日期部分） */
export function formatDate(date: Date | number | string, options: FormatDateOptions = {}): string {
  const d = new Date(date);
  if (isNaN(d.getTime())) return '';
  const locale = getCurrentLocale();
  const formatter = new Intl.DateTimeFormat(locale, {
    year: options.year ?? 'numeric',
    month: options.month ?? '2-digit',
    day: options.day ?? '2-digit',
    weekday: options.weekday,
  });
  return formatter.format(d);
}

/** 格式化时间（仅时间部分） */
export function formatTime(date: Date | number | string, options: FormatTimeOptions = {}): string {
  const d = new Date(date);
  if (isNaN(d.getTime())) return '';
  const locale = getCurrentLocale();
  const formatter = new Intl.DateTimeFormat(locale, {
    hour: options.hour ?? '2-digit',
    minute: options.minute ?? '2-digit',
    second: options.second ?? '2-digit',
    hour12: options.hour12 ?? false,
  });
  return formatter.format(d);
}

/** 格式化日期时间（日期 + 时间） */
export function formatDateTime(
  date: Date | number | string,
  options: FormatDateOptions & FormatTimeOptions = {}
): string {
  const d = new Date(date);
  if (isNaN(d.getTime())) return '';
  const locale = getCurrentLocale();
  const formatter = new Intl.DateTimeFormat(locale, {
    year: options.year ?? 'numeric',
    month: options.month ?? '2-digit',
    day: options.day ?? '2-digit',
    hour: options.hour ?? '2-digit',
    minute: options.minute ?? '2-digit',
    second: options.second ?? '2-digit',
    hour12: options.hour12 ?? false,
    weekday: options.weekday,
  });
  return formatter.format(d);
}

/** 格式化相对时间（如"3 小时前"、"2 天后"） */
export function formatRelativeTime(date: Date | number | string): string {
  const d = new Date(date);
  if (isNaN(d.getTime())) return '';
  const locale = getCurrentLocale();
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });
  const now = Date.now();
  const diffMs = d.getTime() - now;
  const absMs = Math.abs(diffMs);

  // 时间单位换算（ms → sec → min → hour → day → month → year）
  const sec = 1000;
  const min = 60 * sec;
  const hour = 60 * min;
  const day = 24 * hour;
  const month = 30 * day;
  const year = 365 * day;

  if (absMs < min) {
    return rtf.format(Math.round(diffMs / sec), 'second');
  } else if (absMs < hour) {
    return rtf.format(Math.round(diffMs / min), 'minute');
  } else if (absMs < day) {
    return rtf.format(Math.round(diffMs / hour), 'hour');
  } else if (absMs < month) {
    return rtf.format(Math.round(diffMs / day), 'day');
  } else if (absMs < year) {
    return rtf.format(Math.round(diffMs / month), 'month');
  } else {
    return rtf.format(Math.round(diffMs / year), 'year');
  }
}

/** 数字格式化选项 */
export interface FormatNumberOptions {
  minimumFractionDigits?: number;
  maximumFractionDigits?: number;
  notation?: 'standard' | 'scientific' | 'engineering' | 'compact';
  compactDisplay?: 'short' | 'long';
}

/** 格式化数字（千位分隔符等） */
export function formatNumber(value: number, options: FormatNumberOptions = {}): string {
  const locale = getCurrentLocale();
  const formatter = new Intl.NumberFormat(locale, {
    minimumFractionDigits: options.minimumFractionDigits,
    maximumFractionDigits: options.maximumFractionDigits,
    notation: options.notation,
    compactDisplay: options.compactDisplay,
  });
  return formatter.format(value);
}

/** 货币格式化 */
export function formatCurrency(
  value: number,
  currency: string = 'CNY',
  options: { minimumFractionDigits?: number; maximumFractionDigits?: number } = {}
): string {
  const locale = getCurrentLocale();
  const formatter = new Intl.NumberFormat(locale, {
    style: 'currency',
    currency,
    minimumFractionDigits: options.minimumFractionDigits ?? 2,
    maximumFractionDigits: options.maximumFractionDigits ?? 2,
  });
  return formatter.format(value);
}

/** 文件大小格式化（自动选择 B/KB/MB/GB 单位） */
export function formatFileSize(bytes: number): string {
  if (bytes < 0) return '';
  if (bytes === 0) return formatNumber(0) + ' B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const unitIndex = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, unitIndex);
  return `${formatNumber(value, { maximumFractionDigits: 2 })} ${units[unitIndex]}`;
}

/** Hook 版本：响应语言变化自动更新格式化器 */
export function useFormatters() {
  const locale = getCurrentLocale();
  return useMemo(() => ({
    formatDate,
    formatTime,
    formatDateTime,
    formatRelativeTime,
    formatNumber,
    formatCurrency,
    formatFileSize,
    locale,
  }), [locale]);
}
