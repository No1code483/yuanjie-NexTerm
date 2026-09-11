import { t } from "i18next";
import type { ComponentType } from 'react';
import { createBrowserRouter, Navigate } from 'react-router-dom';
import Layout from '../layouts/Layout';
import { ROUTES } from './routes';
import { withAuthGuard } from './AuthGuard';

type RouteLoader = () => Promise<{ default: ComponentType }>;

// 动态导入组件并包装权限守卫
const createLazyRoute = (loader: RouteLoader, routePath: string) => ({
  lazy: async () => {
    const {
      default: Component
    } = await loader();
    const GuardedComponent = withAuthGuard(Component, routePath);
    return {
      Component: GuardedComponent
    };
  }
});
const router = createBrowserRouter([{
  path: ROUTES.ROOT,
  element: <Layout />,
  children: [
  // 根路径重定向到首页
  {
    index: true,
    element: <Navigate to={ROUTES.HOME} replace />
  },
  // 首页路由
  {
    path: ROUTES.HOME,
    ...createLazyRoute(() => import('../pages/Home'), ROUTES.HOME)
  },
  // 首页子路由
  {
    path: ROUTES.HOME_NEWS,
    ...createLazyRoute(() => import('../pages/Home'), ROUTES.HOME_NEWS)
  }, {
    path: ROUTES.HOME_TODO,
    ...createLazyRoute(() => import('../pages/Home'), ROUTES.HOME_TODO)
  }, {
    path: ROUTES.HOME_LOG,
    ...createLazyRoute(() => import('../pages/Home'), ROUTES.HOME_LOG)
  }, {
    path: ROUTES.HOME_TIMER,
    ...createLazyRoute(() => import('../pages/Home'), ROUTES.HOME_TIMER)
  }, {
    path: ROUTES.HOME_TIMER_SHORT,
    ...createLazyRoute(() => import('../pages/Home'), ROUTES.HOME_TIMER_SHORT)
  }, {
    path: ROUTES.HOME_TIMER_LONG,
    ...createLazyRoute(() => import('../pages/Home'), ROUTES.HOME_TIMER_LONG)
  },
  // 个人中心路由
  {
    path: ROUTES.PROFILE,
    ...createLazyRoute(() => import('../pages/Profile'), ROUTES.PROFILE)
  },
  // 个人中心子路由
  {
    path: ROUTES.PROFILE_ACCOUNT,
    ...createLazyRoute(() => import('../pages/Profile'), ROUTES.PROFILE_ACCOUNT)
  }, {
    path: ROUTES.PROFILE_RESUME,
    ...createLazyRoute(() => import('../pages/Profile'), ROUTES.PROFILE_RESUME)
  }, {
    path: ROUTES.PROFILE_SETTING,
    ...createLazyRoute(() => import('../pages/Profile'), ROUTES.PROFILE_SETTING)
  }, {
    path: ROUTES.PROFILE_LOGOUT,
    ...createLazyRoute(() => import('../pages/Profile'), ROUTES.PROFILE_LOGOUT)
  }, {
    path: ROUTES.PROFILE_QUOTE,
    ...createLazyRoute(() => import('../pages/Profile'), ROUTES.PROFILE_QUOTE)
  },
  // AI会话路由
  {
    path: ROUTES.AI,
    ...createLazyRoute(() => import('../pages/AI'), ROUTES.AI)
  },
  // AI会话子路由
  {
    path: ROUTES.AI_MODEL,
    ...createLazyRoute(() => import('../pages/AI'), ROUTES.AI_MODEL)
  }, {
    path: ROUTES.AI_CHAT,
    ...createLazyRoute(() => import('../pages/AI'), ROUTES.AI_CHAT)
  }, {
    path: ROUTES.AI_GROUP,
    ...createLazyRoute(() => import('../pages/AI'), ROUTES.AI_GROUP)
  },
  // 知识库路由
  {
    path: ROUTES.KNOWLEDGE,
    ...createLazyRoute(() => import('../pages/Knowledge'), ROUTES.KNOWLEDGE)
  },
  // 终端路由
  {
    path: ROUTES.TERMINAL,
    ...createLazyRoute(() => import('../pages/Terminal'), ROUTES.TERMINAL)
  },
  // 终端子路由
  {
    path: ROUTES.TERMINAL_TERMINAL,
    ...createLazyRoute(() => import('../pages/Terminal'), ROUTES.TERMINAL_TERMINAL)
  }, {
    path: ROUTES.TERMINAL_YUANCODE,
    ...createLazyRoute(() => import('../pages/YuanCode'), ROUTES.TERMINAL_YUANCODE)
  }, {
    path: ROUTES.TERMINAL_LINUX,
    ...createLazyRoute(() => import('../pages/Terminal'), ROUTES.TERMINAL_LINUX)
  },
  // 命令手册路由
  {
    path: ROUTES.TERMINAL_MANUAL,
    ...createLazyRoute(() => import('../pages/CommandManual'), ROUTES.TERMINAL_MANUAL)
  }, {
    path: ROUTES.TERMINAL_MANUAL_TERMINAL,
    ...createLazyRoute(() => import('../pages/CommandManual'), ROUTES.TERMINAL_MANUAL_TERMINAL)
  }, {
    path: ROUTES.TERMINAL_MANUAL_YUANCODE,
    ...createLazyRoute(() => import('../pages/CommandManual'), ROUTES.TERMINAL_MANUAL_YUANCODE)
  }, {
    path: ROUTES.TERMINAL_MANUAL_LINUX,
    ...createLazyRoute(() => import('../pages/CommandManual'), ROUTES.TERMINAL_MANUAL_LINUX)
  },
  // 命令手册-快捷键路由
  {
    path: ROUTES.TERMINAL_MANUAL_SHORTCUTS,
    ...createLazyRoute(() => import('../pages/CommandManual'), ROUTES.TERMINAL_MANUAL_SHORTCUTS)
  },
  // 小欣路由
  {
    path: ROUTES.XIN,
    ...createLazyRoute(() => import('../pages/Xin'), ROUTES.XIN)
  },
  // 游戏路由（数据预览页）
  {
    path: ROUTES.GAME,
    ...createLazyRoute(() => import('../pages/game/GamePreview'), ROUTES.GAME)
  },
  // 游戏 3D 路由（阶段8 完整实现，本次仅占位）
  {
    path: ROUTES.GAME_PLAY,
    ...createLazyRoute(() => import('../pages/game3d/Game3D'), ROUTES.GAME_PLAY)
  },
  // 游戏-知识领域映射设置页（阶段10 Task 10.3）
  {
    path: ROUTES.GAME_PLAY_MAPPING,
    ...createLazyRoute(() => import('../pages/game3d/KnowledgeMapping'), ROUTES.GAME_PLAY_MAPPING)
  },
  // 回收站路由
  {
    path: ROUTES.RECYCLE,
    ...createLazyRoute(() => import('../pages/Recycle'), ROUTES.RECYCLE)
  },
  // 搜索路由
  {
    path: ROUTES.SEARCH,
    ...createLazyRoute(() => import('../pages/Search'), ROUTES.SEARCH)
  },
  // 底层智能路由
  {
    path: ROUTES.SPYGLASS,
    ...createLazyRoute(() => import('../pages/Intelligence'), ROUTES.SPYGLASS)
  },
  // 404 页面
  {
    path: '*',
    element: <div className="nt-flex nt-flex-center nt-full-height">
            <div className="nt-card">
              <h2 className="nt-title">{t("routes.router.k1")}</h2>
              <p>{t("routes.router.k2")}</p>
              <button className="nt-button" onClick={() => window.location.href = ROUTES.HOME}>
                {t("components.ShortcutPanel.k3")}
              </button>
            </div>
          </div>
  }]
}]);
export default router;