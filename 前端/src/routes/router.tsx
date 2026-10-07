import { t } from "i18next";
import { createBrowserRouter, Navigate } from 'react-router-dom';
import type { RouteObject } from 'react-router-dom';
import Layout from '../layouts/Layout';
import { ROUTES } from './routes';

export const legacyRoutes = [
  // 根路径重定向到首页
  {
    index: true,
    element: <Navigate to={ROUTES.HOME} replace />
  },
  // 首页路由（含 6 条子路由）已迁入 boards.home 插件（阶段3 批次1b-1）
  // AI会话路由（含 3 条子路由）已迁入 boards.ai 插件（阶段3 批次2b-1）
  // 知识库路由已迁入 boards.knowledge 插件（阶段3 批次2a-1）
  // 终端 3 条路由（/terminal、/terminal/terminal、/terminal/linux —— 同页 Tab）已迁入
  // boards.terminal 插件（阶段3 批次3a）；TERMINAL_YUANCODE 已迁入 terminal.yuancode
  // 插件（阶段3 批次3c）
  // 命令手册路由（5 个 TERMINAL_MANUAL 路由）已迁入 boards.terminal 插件（D4）
  // 回收站路由已迁入 customs.recycle 插件（D4）
  // 搜索路由已迁入 customs.search 插件（D4）
  // 底层智能路由（Spyglass）已迁入 customs.intelligence 插件（D4）
  // 小欣路由（阶段3 批次4a-1：已迁入 plugins/boards/xin，由插件贡献）
  // 游戏路由（阶段3 批次4b：已迁入 plugins/boards/game，由插件贡献）
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
  }
] satisfies RouteObject[];

const router = createBrowserRouter([{
  path: ROUTES.ROOT,
  element: <Layout />,
  children: legacyRoutes,
}]);
export default router;