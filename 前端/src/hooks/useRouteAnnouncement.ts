import { useEffect } from 'react';
import { useLocation } from 'react-router-dom';

/**
 * C4 §2.4.3 路由切换通知
 * 设计依据：功能展望/体验深化/04_无障碍_A11y合规.md §2.4.3
 *
 * 作用：路由变化时，通过 aria-live 区域向屏幕阅读器播报新路由名称，
 * 让视障用户感知页面切换（否则屏幕阅读器用户不知道 SPA 已经切换了页面）。
 *
 * 依赖：Layout 内必须存在 announcer 节点：
 * `<div id="route-announcer" role="status" aria-live="polite" className="sr-only" />`
 *
 * 用法：在 Layout 组件内调用 `useRouteAnnouncement()`（Layout 在 RouterProvider 内，useLocation 可用）。
 */

// 路由前缀 → 可读名称映射（后续可迁移至 i18n 资源文件）
const ROUTE_NAMES: Record<string, string> = {
  '/home': '首页',
  '/ai': 'AI 会话',
  '/knowledge': '知识库',
  '/terminal': '终端',
  '/xin': '小欣',
  '/game': '游戏',
  '/profile': '个人中心',
  '/recycle': '回收站',
  '/search': '搜索',
  '/spyglass': '底层智能',
};

function getRouteName(pathname: string): string {
  // 按前缀匹配，取最长匹配项（避免 /terminal/manual 误匹配到其他短前缀）
  const matched = Object.keys(ROUTE_NAMES)
    .filter(prefix => pathname.startsWith(prefix))
    .sort((a, b) => b.length - a.length)[0];
  return matched ? ROUTE_NAMES[matched] : '页面';
}

export function useRouteAnnouncement(): void {
  const location = useLocation();

  useEffect(() => {
    const announcer = document.getElementById('route-announcer');
    if (!announcer) return;

    // 先清空再设置，触发 aria-live 重新播报（相同文本不会重复播报）
    announcer.textContent = '';
    // setTimeout 50ms 确保屏幕阅读器检测到 DOM 变化
    window.setTimeout(() => {
      announcer.textContent = `已进入：${getRouteName(location.pathname)}`;
    }, 50);
  }, [location.pathname]);
}
