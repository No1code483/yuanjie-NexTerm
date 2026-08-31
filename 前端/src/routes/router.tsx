import { t } from "i18next";
import { createBrowserRouter, Navigate } from 'react-router-dom';
import Layout from '../layouts/Layout';
import { ROUTES } from './routes';
import { withAuthGuard } from './AuthGuard';

// 动态导入组件并包装权限守卫
const createLazyRoute = (importPath: string, routePath: string) => ({
  lazy: async () => {
    const {
      default: Component
    } = await import(/* @vite-ignore */importPath);
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
    ...createLazyRoute('../pages/Home', ROUTES.HOME)
  },
  // 首页子路由
  {
    path: ROUTES.HOME_NEWS,
    ...createLazyRoute('../pages/Home', ROUTES.HOME_NEWS)
  }, {
    path: ROUTES.HOME_TODO,
    ...createLazyRoute('../pages/Home', ROUTES.HOME_TODO)
  }, {
    path: ROUTES.HOME_LOG,
    ...createLazyRoute('../pages/Home', ROUTES.HOME_LOG)
  }, {
    path: ROUTES.HOME_TIMER,
    ...createLazyRoute('../pages/Home', ROUTES.HOME_TIMER)
  }, {
    path: ROUTES.HOME_TIMER_SHORT,
    ...createLazyRoute('../pages/Home', ROUTES.HOME_TIMER_SHORT)
  }, {
    path: ROUTES.HOME_TIMER_LONG,
    ...createLazyRoute('../pages/Home', ROUTES.HOME_TIMER_LONG)
  },
  // 个人中心路由
  {
    path: ROUTES.PROFILE,
    ...createLazyRoute('../pages/Profile', ROUTES.PROFILE)
  },
  // 个人中心子路由
  {
    path: ROUTES.PROFILE_ACCOUNT,
    ...createLazyRoute('../pages/Profile', ROUTES.PROFILE_ACCOUNT)
  }, {
    path: ROUTES.PROFILE_RESUME,
    ...createLazyRoute('../pages/Profile', ROUTES.PROFILE_RESUME)
  }, {
    path: ROUTES.PROFILE_SETTING,
    ...createLazyRoute('../pages/Profile', ROUTES.PROFILE_SETTING)
  }, {
    path: ROUTES.PROFILE_LOGOUT,
    ...createLazyRoute('../pages/Profile', ROUTES.PROFILE_LOGOUT)
  }, {
    path: ROUTES.PROFILE_QUOTE,
    ...createLazyRoute('../pages/Profile', ROUTES.PROFILE_QUOTE)
  },
  // AI会话路由
  {
    path: ROUTES.AI,
    ...createLazyRoute('../pages/AI', ROUTES.AI)
  },
  // AI会话子路由
  {
    path: ROUTES.AI_MODEL,
    ...createLazyRoute('../pages/AI', ROUTES.AI_MODEL)
  }, {
    path: ROUTES.AI_CHAT,
    ...createLazyRoute('../pages/AI', ROUTES.AI_CHAT)
  }, {
    path: ROUTES.AI_GROUP,
    ...createLazyRoute('../pages/AI', ROUTES.AI_GROUP)
  },
  // 知识库路由
  {
    path: ROUTES.KNOWLEDGE,
    ...createLazyRoute('../pages/Knowledge', ROUTES.KNOWLEDGE)
  },
  // 终端路由
  {
    path: ROUTES.TERMINAL,
    ...createLazyRoute('../pages/Terminal', ROUTES.TERMINAL)
  },
  // 终端子路由
  {
    path: ROUTES.TERMINAL_TERMINAL,
    ...createLazyRoute('../pages/Terminal', ROUTES.TERMINAL_TERMINAL)
  }, {
    path: ROUTES.TERMINAL_YUANCODE,
    ...createLazyRoute('../pages/YuanCode', ROUTES.TERMINAL_YUANCODE)
  }, {
    path: ROUTES.TERMINAL_LINUX,
    ...createLazyRoute('../pages/Terminal', ROUTES.TERMINAL_LINUX)
  },
  // 命令手册路由
  {
    path: ROUTES.TERMINAL_MANUAL,
    ...createLazyRoute('../pages/CommandManual', ROUTES.TERMINAL_MANUAL)
  }, {
    path: ROUTES.TERMINAL_MANUAL_TERMINAL,
    ...createLazyRoute('../pages/CommandManual', ROUTES.TERMINAL_MANUAL_TERMINAL)
  }, {
    path: ROUTES.TERMINAL_MANUAL_YUANCODE,
    ...createLazyRoute('../pages/CommandManual', ROUTES.TERMINAL_MANUAL_YUANCODE)
  }, {
    path: ROUTES.TERMINAL_MANUAL_LINUX,
    ...createLazyRoute('../pages/CommandManual', ROUTES.TERMINAL_MANUAL_LINUX)
  },
  // 命令手册-快捷键路由
  {
    path: ROUTES.TERMINAL_MANUAL_SHORTCUTS,
    ...createLazyRoute('../pages/CommandManual', ROUTES.TERMINAL_MANUAL_SHORTCUTS)
  },
  // 小欣路由
  {
    path: ROUTES.XIN,
    ...createLazyRoute('../pages/Xin', ROUTES.XIN)
  },
  // 游戏路由（数据预览页）
  {
    path: ROUTES.GAME,
    ...createLazyRoute('../pages/game/GamePreview', ROUTES.GAME)
  },
  // 游戏 3D 路由（阶段8 完整实现，本次仅占位）
  {
    path: ROUTES.GAME_PLAY,
    ...createLazyRoute('../pages/game3d/Game3D', ROUTES.GAME_PLAY)
  },
  // 游戏-知识领域映射设置页（阶段10 Task 10.3）
  {
    path: ROUTES.GAME_PLAY_MAPPING,
    ...createLazyRoute('../pages/game3d/KnowledgeMapping', ROUTES.GAME_PLAY_MAPPING)
  },
  // 回收站路由
  {
    path: ROUTES.RECYCLE,
    ...createLazyRoute('../pages/Recycle', ROUTES.RECYCLE)
  },
  // 搜索路由
  {
    path: ROUTES.SEARCH,
    ...createLazyRoute('../pages/Search', ROUTES.SEARCH)
  },
  // 底层智能路由
  {
    path: ROUTES.SPYGLASS,
    ...createLazyRoute('../pages/Intelligence', ROUTES.SPYGLASS)
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