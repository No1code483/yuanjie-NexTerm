import { t } from "i18next";
import { useEffect, useRef } from 'react';
import { useLocation } from 'react-router-dom';
import { intelligence } from '@/lib/ipc';
import { useAuthStore } from '@/stores/authStore';

/** 路径 → 模块名映射 */
const ROUTE_MODULE_MAP: Record<string, string> = {
  '/home': t("components.intelligence.DashboardPanel.k106"),
  '/profile': t("components.intelligence.DashboardPanel.k105"),
  '/ai': t("components.PermissionRestricted.k2"),
  '/knowledge': t("components.intelligence.ActivityPanel.k1"),
  '/terminal': t("components.intelligence.ActivityPanel.k8"),
  '/xin': t("components.FloatingXin.k26"),
  '/game': t("components.AddExtensionModal.k11"),
  '/search': t("common.search"),
  '/spyglass': t("components.intelligence.DashboardPanel.k103"),
  '/recycle': t("components.PermissionRestricted.k4")
};
function resolveModule(pathname: string): string {
  if (ROUTE_MODULE_MAP[pathname]) return ROUTE_MODULE_MAP[pathname];
  const sorted = Object.keys(ROUTE_MODULE_MAP).sort((a, b) => b.length - a.length);
  for (const prefix of sorted) {
    if (pathname.startsWith(prefix + '/') || pathname === prefix) return ROUTE_MODULE_MAP[prefix];
  }
  return t("hooks.useActivityTracker.k1");
}

/**
 * 页面访问活动追踪 Hook
 * - 路由变化时记录页面停留时长（durationSecs）
 * - 每 60 秒发送心跳持续累计活跃时长
 */
export function useActivityTracker() {
  const {
    user
  } = useAuthStore();
  const location = useLocation();

  // 用单个 ref 打包所有可变状态，避免多 useEffect 竞态
  const stateRef = useRef({
    lastPath: '',
    sessionStart: Date.now(),
    currentModule: '',
    debounceTimer: null as ReturnType<typeof setTimeout> | null,
    heartbeatTimer: null as ReturnType<typeof setInterval> | null,
    user
  });
  // 始终保持 user 引用最新
  stateRef.current.user = user;

  /** 发送活动日志 */
  const emitLog = (module: string, operation: string, detail: string, durationSecs: number) => {
    ;
    (async () => {
      try {
        const u = stateRef.current.user;
        const userId = u?.id ? String(u.id) : '1';
        // 修复时区偏移：使用 UTC 时间戳并带 'Z' 后缀，确保前端读取时能正确转换为本地时间
        const timestamp = new Date().toISOString();
        await intelligence.logActivity(userId, timestamp, module, operation, detail, undefined, durationSecs);
      } catch {/* 静默 */}
    })();
  };

  /** 启动 / 重置心跳（每 60s） */
  const ensureHeartbeat = (module: string) => {
    const s = stateRef.current;
    if (s.heartbeatTimer) clearInterval(s.heartbeatTimer);
    s.heartbeatTimer = setInterval(() => {
      emitLog(module, t("hooks.useActivityTracker.k2"), location.pathname, 60);
    }, 60000);
  };
  const stopHeartbeat = () => {
    if (stateRef.current.heartbeatTimer) {
      clearInterval(stateRef.current.heartbeatTimer);
      stateRef.current.heartbeatTimer = null;
    }
  };

  /** 记录上一页停留时长，切换到新页 */
  const commitPageSwitch = (newPath: string, newModule: string) => {
    const s = stateRef.current;
    const now = Date.now();
    const stayedSecs = Math.round((now - s.sessionStart) / 1000);
    if (s.lastPath && stayedSecs > 2 && s.currentModule) {
      emitLog(s.currentModule, t("hooks.useActivityTracker.k3"), s.lastPath, stayedSecs);
    }
    s.lastPath = newPath;
    s.sessionStart = now;
    s.currentModule = newModule;
  };

  // 单一 useEffect：处理路由切换 + 初始化 + 清理
  useEffect(() => {
    const currentPath = location.pathname;
    const s = stateRef.current;

    // 首次初始化
    if (!s.lastPath) {
      s.lastPath = currentPath;
      s.sessionStart = Date.now();
      s.currentModule = resolveModule(currentPath);
      ensureHeartbeat(s.currentModule);

      // 页面关闭兜底
      const handleBeforeUnload = () => {
        const stayedSecs = Math.round((Date.now() - s.sessionStart) / 1000);
        if (stayedSecs > 2 && s.currentModule) {
          const u = s.user;
          const userId = u?.id ? String(u.id) : '1';
          // 修复时区偏移：使用 UTC 时间戳并带 'Z' 后缀
          const timestamp = new Date().toISOString();
          const body = JSON.stringify({
            record: {
              userId,
              timestamp,
              module: s.currentModule,
              operation: t("hooks.useActivityTracker.k3"),
              detail: s.lastPath,
              remark: null,
              durationSecs: stayedSecs
            }
          });
          navigator.sendBeacon('/api/intelligence/log', body);
        }
      };
      window.addEventListener('beforeunload', handleBeforeUnload);
      return () => {
        window.removeEventListener('beforeunload', handleBeforeUnload);
        stopHeartbeat();
        if (s.debounceTimer) clearTimeout(s.debounceTimer);
      };
    }

    // 后续路由切换
    if (currentPath === s.lastPath) return;
    if (s.debounceTimer) clearTimeout(s.debounceTimer);
    s.debounceTimer = setTimeout(() => {
      const moduleName = resolveModule(currentPath);
      commitPageSwitch(currentPath, moduleName);
      ensureHeartbeat(moduleName);
    }, 800);
    return () => {
      if (s.debounceTimer) clearTimeout(s.debounceTimer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [location.pathname]);
}