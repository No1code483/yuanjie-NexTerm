import { useEffect, useState } from 'react';
import i18n from '@/i18n';

/**
 * useRTL hook（C2.4 RTL 布局支持）
 *
 * 功能：
 *   - 检测当前语言是否为 RTL（Right-To-Left）
 *   - 监听 i18n 语言变化，自动更新 RTL 状态
 *   - 供组件判断是否需要翻转方向性图标 / 布局
 *
 * RTL 语言列表：
 *   - ar（阿拉伯语）
 *   - he（希伯来语）
 *   - fa（波斯语）
 *   - ur（乌尔都语）
 *
 * 当前 NexTerm 支持的 6 语言中：
 *   - LTR：zh / en / ja / ko / ru
 *   - RTL：ar（阿拉伯语，C2.8 已启用实验性支持）
 * he / fa / ur 为未来扩展预留。
 *
 * 关联文档：功能展望/体验深化/02_多语言切换_i18n体系_未来展望.md §2.7
 */

const RTL_LANGUAGES = ['ar', 'he', 'fa', 'ur'];

function isRTLLanguage(lang: string): boolean {
  const base = lang.split('-')[0].toLowerCase();
  return RTL_LANGUAGES.includes(base);
}

export function useRTL(): boolean {
  const [isRTL, setIsRTL] = useState<boolean>(() => isRTLLanguage(i18n.language));

  useEffect(() => {
    const handler = (lng: string) => {
      setIsRTL(isRTLLanguage(lng));
    };
    i18n.on('languageChanged', handler);
    return () => {
      i18n.off('languageChanged', handler);
    };
  }, []);

  return isRTL;
}

/** 获取当前语言的文本方向 */
export function useTextDirection(): 'rtl' | 'ltr' {
  return useRTL() ? 'rtl' : 'ltr';
}

export default useRTL;
