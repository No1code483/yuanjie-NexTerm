import { t } from "i18next";
import { useNavigate, useLocation } from 'react-router-dom';
import { ROUTES, RoutePath, ROUTE_META, RouteMeta } from './routes';

/**
 * 自定义导航钩子
 * 提供便捷的路由导航功能
 */
export function useNavigation() {
  const navigate = useNavigate();
  const location = useLocation();
  const navigateTo = (path: RoutePath) => {
    navigate(path);
  };
  const navigateBack = () => {
    navigate(-1);
  };
  const navigateForward = () => {
    navigate(1);
  };
  const replace = (path: RoutePath) => {
    navigate(path, {
      replace: true
    });
  };
  const reload = () => {
    navigate(0);
  };

  // 快捷导航方法
  const navigateToHome = () => navigateTo(ROUTES.HOME);
  const navigateToProfile = () => navigateTo(ROUTES.PROFILE);
  const navigateToAI = () => navigateTo(ROUTES.AI);
  const navigateToKnowledge = () => navigateTo(ROUTES.KNOWLEDGE);
  const navigateToTerminal = () => navigateTo(ROUTES.TERMINAL);
  const navigateToXin = () => navigateTo(ROUTES.XIN);
  const navigateToGame = () => navigateTo(ROUTES.GAME);
  const navigateToRecycle = () => navigateTo(ROUTES.RECYCLE);
  const navigateToSearch = () => navigateTo(ROUTES.SEARCH);
  return {
    navigateTo,
    navigateBack,
    navigateForward,
    replace,
    reload,
    currentPath: location.pathname,
    // 快捷导航
    navigateToHome,
    navigateToProfile,
    navigateToAI,
    navigateToKnowledge,
    navigateToTerminal,
    navigateToXin,
    navigateToGame,
    navigateToRecycle,
    navigateToSearch
  };
}

/**
 * 检查当前路由是否匹配指定路径
 */
export function useRouteMatch(path: RoutePath): boolean {
  const location = useLocation();
  return location.pathname === path;
}

/**
 * 获取当前路由的元数据
 */
export function useRouteMeta() {
  const location = useLocation();

  // 获取对应的主路由路径
  const getMainRoutePath = (routePath: RoutePath): string => {
    // 如果是主路由，直接返回
    if (Object.values(ROUTES).includes(routePath as any)) {
      return routePath;
    }

    // 如果是子路由，找到对应的主路由
    for (const mainPath of Object.values(ROUTES).slice(0, 11)) {
      // 前11个是主路由
      if (routePath.startsWith(mainPath)) {
        return mainPath;
      }
    }

    // 默认返回首页
    return ROUTES.HOME;
  };
  const mainPath = getMainRoutePath(location.pathname as RoutePath);
  return (ROUTE_META as Record<string, RouteMeta>)[mainPath] || {
    title: t("xin.mockData.k1"),
    description: '',
    requiresAuth: true,
    allowedForTemp: false
  };
}