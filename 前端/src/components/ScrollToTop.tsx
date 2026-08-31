import { useEffect } from 'react';
import { useLocation } from 'react-router-dom';

/**
 * 路由切换时重置主内容区滚动位置
 * 解决 BUG-003/BUG-011/BUG-002：导航栏被覆盖、侧边栏截断
 *
 * 原理：主内容区（#main-content）和侧边栏（#sidebar）各自独立滚动，
 * 路由切换时将两者的 scrollTop 重置为 0，避免旧滚动位置导致内容偏移。
 */
export default function ScrollToTop() {
  const { pathname } = useLocation();

  useEffect(() => {
    const mainContent = document.getElementById('main-content');
    if (mainContent) {
      mainContent.scrollTop = 0;
    }

    const sidebar = document.getElementById('sidebar');
    if (sidebar) {
      sidebar.scrollTop = 0;
    }
  }, [pathname]);

  return null;
}
