/**
 * useSystemTheme Hook（C1.3 / v1.51.5）
 *
 * 监听系统 prefers-color-scheme 变化，返回当前系统主题（'dark' | 'light'）。
 *
 * 用法：
 *   const systemTheme = useSystemTheme();
 *   if (systemTheme === 'dark') { ... }
 *
 * 关联文档：功能展望/体验深化/01_主题自定义系统_未来展望.md §2.3
 */

import { useEffect, useState } from 'react';

export type SystemTheme = 'light' | 'dark';

/**
 * 监听系统 prefers-color-scheme
 * @returns 当前系统主题（dark/light），undefined 表示尚未检测
 */
export function useSystemTheme(): SystemTheme | undefined {
  const [systemTheme, setSystemTheme] = useState<SystemTheme | undefined>(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return undefined;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  });

  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return;

    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = (e: MediaQueryListEvent) => {
      setSystemTheme(e.matches ? 'dark' : 'light');
    };

    // 立即同步一次（防止初始状态过期）
    setSystemTheme(mq.matches ? 'dark' : 'light');

    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  return systemTheme;
}

export default useSystemTheme;
