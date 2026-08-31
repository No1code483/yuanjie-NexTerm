import { t } from "i18next";
/**
 * AuthGuard 路由守卫组件
 * 
 * v2.0 重构：统一使用 RBAC 权限系统
 * 
 * 重构前的问题：
 * - 混合使用了路由权限系统（canAccessRoute）和临时账号权限系统（isGuest）
 * - 硬编码了终端白名单逻辑（/terminal/manual, /terminal/yuancode, /terminal/linux）
 * - 权限检查逻辑分散，难以维护
 * 
 * 重构后的优势：
 * - 统一使用 usePermissions hook 进行权限检查
 * - 使用 getRouteResource 将路由映射到资源名称
 * - 白名单逻辑已整合到 DEFAULT_PERMISSIONS.guest 配置中
 * - 权限检查逻辑集中化，易于维护和扩展
 */

import React, { ReactNode } from 'react';
import { Navigate, useLocation } from 'react-router-dom';
import { ROUTES, getRouteTitle, getRouteResource } from './routes';
import { useAuthStore } from '../stores/authStore';
import { usePermissions } from '../hooks/usePermissions';
interface AuthGuardProps {
  children: ReactNode;
  routePath: string;
}
export const AuthGuard: React.FC<AuthGuardProps> = ({
  children,
  routePath
}) => {
  const location = useLocation();
  const {
    isAuthenticated
  } = useAuthStore();

  // 使用统一的 RBAC 权限系统
  const {
    can,
    isLoading,
    permissions
  } = usePermissions(); // 需要 permissions 判断初始状态

  // 设置页面标题
  React.useEffect(() => {
    const title = getRouteTitle(routePath as any);
    document.title = t("routes.AuthGuard.k1", {
      title: title
    });
  }, [routePath]);

  // ===== 第一步：认证检查 =====
  if (!isAuthenticated) {
    return <Navigate to={ROUTES.ROOT} replace state={{
      from: location
    }} />;
  }

  // ===== 第一步五：权限加载状态检查 (新增) =====
  // 如果权限正在加载中，或权限尚未初始化，临时放行
  // 关键在于：isLoading 初始值是 false，但 permissions 初始值是 null
  // 所以必须同时检查两者，避免在第一次渲染时误判为"权限不足"
  if (isLoading || !permissions) {
    console.log(`[AuthGuard] ⏳ 权限未就绪 (isLoading=${isLoading}, permissions=${permissions ? 'loaded' : 'null'})，临时放行...`);
    return <>{children}</>;
  }

  // ===== 第二步：权限检查 (统一使用 RBAC 系统) =====
  // 获取当前路由对应的资源名称
  const routeStr = typeof routePath === 'string' ? routePath : '';
  const resource = getRouteResource(routeStr);

  // 使用统一的权限检查方法
  // - 自动处理白名单逻辑（如 terminal_manual 对临时账号允许 read）
  // - 自动处理角色权限差异（admin/user/guest）
  // - 无需硬编码任何路由路径
  const hasAccess = can(resource, 'read');
  if (!hasAccess) {
    console.group(`[AuthGuard] 🔒 权限拒绝`);
    console.log('路由:', routeStr);
    console.log('资源:', resource);
    console.log('操作: read');
    console.log('权限矩阵 keys:', permissions ? Object.keys(permissions) : 'null');
    console.log(`资源 ${resource} 权限:`, permissions ? JSON.stringify(permissions[resource]) : 'null');
    console.log(`资源 'terminal' 权限:`, permissions ? JSON.stringify(permissions['terminal']) : 'null');
    console.groupEnd();
    return <div className="nt-flex nt-flex-center nt-full-height">
        <div className="nt-card">
          <h2 className="nt-title">{t("errors.permissionDenied")}</h2>
          <p>{t("routes.AuthGuard.k2")}</p>
          <button className="nt-button" onClick={() => window.history.back()}>
            {t("common.back")}
          </button>
        </div>
      </div>;
  }

  // ===== 第三步：权限通过，渲染子组件 =====
  return <>{children}</>;
};

/**
 * 高阶组件：为页面组件添加路由守卫
 * 
 * @param Component 页面组件
 * @param routePath 当前路由路径
 * @returns 包装后的组件（带权限检查）
 */
export function withAuthGuard(Component: React.ComponentType, routePath: string) {
  return function AuthWrappedComponent(props: any) {
    return <AuthGuard routePath={routePath}>
        <Component {...props} />
      </AuthGuard>;
  };
}