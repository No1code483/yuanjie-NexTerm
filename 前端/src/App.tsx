import { t } from "i18next";
import { useTranslation } from 'react-i18next';
import { useState, useEffect, useRef, Suspense } from 'react';
import { RouterProvider } from 'react-router-dom';
import LoginModal from '@/plugins/customs/auth/LoginModal';
import router, { legacyRoutes } from './routes/router';
import { TimerProvider } from './plugins/boards/home/features/timer/context';
import ErrorBoundary from './components/ErrorBoundary';
import CyberpunkOverlay from './components/CyberpunkOverlay';
import OfflineBanner from './components/OfflineBanner/OfflineBanner';
import { SkeletonPage, SkeletonCard, SkeletonText } from './components/ui/Skeleton';
import { useAuthStore } from './kernel/state/authStore';
import { auth } from './plugins/customs/auth/ipc/auth';
import { initPerfMonitor, markStartupBegin, markStartupEnd } from './lib/perfMonitor';
// T2.1.7: Monaco Editor loader 配置（side-effect import，配置加载路径，Monaco 本身在 <Editor> 挂载时才加载）
import './lib/monaco';
import { useFocusManagement } from './hooks/useFocusManagement';
import { useA11yStore } from './kernel/state/a11yStore';
import { initAriaAnnouncer } from './utils/ariaAnnouncer';
// 插件化重构（v2 并轨后）：插件路由为唯一路径 —— App 启动即经 buildApp 装配插件路由与导航，
// 静态路由表（routes/router.tsx）仅保留根路径重定向与 404 兜底
import { createBrowserRouter } from 'react-router-dom';
import { listen } from '@tauri-apps/api/event';
import Layout from './layouts/Layout';
import { buildApp } from './kernel/registry/buildApp';
// BUG-035：SlotRenderer 的 useRegistry 依赖 RegistryProvider 上下文（缺失时首页
// 新闻/待办/日志/计时 modal 渲染即抛 'useRegistry 必须在 <RegistryProvider> 内使用' 白屏）
import { RegistryProvider } from './kernel/registry/context';
import PluginManagerPage from './kernel/plugin-manager/PluginManagerPage';
import type { KernelEvent } from './kernel/types';

// 页面懒加载时的骨架屏回退
const PageLoadingFallback = () => <SkeletonPage>
    <SkeletonCard height={48} />
    <div style={{
    height: 16
  }} />
    <SkeletonText lines={4} />
    <div style={{
    height: 24
  }} />
    <SkeletonCard height={200} />
    <div style={{
    height: 16
  }} />
    <SkeletonText lines={3} shortLast />
  </SkeletonPage>;
const LoadingScreen = () => <div role="status" aria-label={t("common.loading")} style={{
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  height: '100%',
  backgroundColor: '#000000',
  color: '#00FF00',
  fontFamily: 'Consolas, monospace'
}}>
    <div style={{
    width: '50px',
    height: '50px',
    border: '3px solid #003300',
    borderTopColor: '#00FF00',
    borderRadius: '50%',
    animation: 'spin 1s linear infinite',
    marginBottom: '20px'
  }} />
    <p style={{
    fontSize: '14px'
  }}>{t("app.k1")}</p>
    <style>{`@keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }`}</style>
  </div>;

// 标记应用启动开始（模块加载最早时机，v1.52 性能基线）
markStartupBegin();
function App() {
  const [isInitialized, setIsInitialized] = useState(false);
  const initRef = useRef(false);
  // 插件路由实例（null = 未启用/未就绪 → 回退现有 router）
  const [pluginRouter, setPluginRouter] = useState<ReturnType<typeof createBrowserRouter> | null>(null);
  // C2.4：useTranslation hook 触发语言切换时的全局重渲染
  // 在根节点订阅 i18n 变化，语言切换时整个组件树重渲染，
  // 子组件使用全局 t() 时会自动返回新语言的值。
  useTranslation();
  const {
    isAuthenticated,
    login: storeLogin
  } = useAuthStore();
  // C4 §2.3.1 键盘焦点管理：给 body 添加 using-keyboard class（供 :focus-visible 之外的逻辑判断输入模式）
  useFocusManagement();
  // C4 §2.7 / §2.5：订阅 A11y 偏好（字号 + 减弱动画），状态变化时根节点重渲染，
  // a11yStore 内部已同步把 fontSize/reducedMotion 写到 <html> 属性，CSS 即时响应。
  useA11yStore();
  // C4 §2.4.5：初始化全局 ARIA Live 通知区（polite + assertive 各一个）
  useEffect(() => {
    initAriaAnnouncer();
  }, []);
  useEffect(() => {
    if (initRef.current) return;
    initRef.current = true;
    console.log('[App] 🚀 应用启动...');
    initPerfMonitor();
    initializeAuth();
  }, []);

  // 插件路由装配：buildApp 装配插件路由（包裹现有 Layout 外壳），失败降级回退现有路由
  // F6：订阅内核 state-changed → 重跑 buildApp 重建路由/导航（07 契约 §九交互）；
  //     管理页 route 注入（内核级归属，管理插件的 UI 不得被被管理插件持有）
  useEffect(() => {
    let cancelled = false;
    const rebuild = () => {
      buildApp()
        .then(({ router: routes, registry, navTree }) => {
          if (cancelled) return;
          const legacyFallback = legacyRoutes.filter((route) => route.path === '*');
          const nextRouter = createBrowserRouter([{
            path: '/',
            // BUG-035：整棵路由树包在 RegistryProvider 内——Home 页 SlotRenderer
            // （home.news/todo/journal/timer 插槽）经 useRegistry 读取插槽组件；
            // navTree 同源注入，供 Layout 侧边栏注册表驱动（K3）
            element: <RegistryProvider registry={registry} navTree={navTree}><Layout /></RegistryProvider>,
            children: [
              ...legacyRoutes.filter((route) => route.path !== '*'),
              ...routes.filter((route) => route.index !== true),
              // 插件管理页（内核级归属，防管理死锁；手稿 20260926 ②：入口置于全局侧边栏）
              { path: '/plugin-manager', element: <PluginManagerPage /> },
              ...legacyFallback,
            ],
          }]);
          const currentLocation = `${window.location.pathname}${window.location.search}${window.location.hash}`;
          void nextRouter.navigate(currentLocation, { replace: true });
          setPluginRouter(nextRouter);
        })
        .catch((e) => console.warn('[plugin] buildApp 失败，回退现有路由:', e));
    };
    rebuild();
    const unlisten = listen<KernelEvent>('k://event', (ev) => {
      if (ev.payload.name === 'kernel:plugin.state-changed') rebuild();
    });
    return () => {
      cancelled = true;
      void unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!import.meta.env.DEV) return;
    const navigateForGate = (event: KeyboardEvent) => {
      if (event.ctrlKey && event.altKey && event.shiftKey && event.key === 'G') {
        window.history.pushState({}, '', '/hello');
        window.dispatchEvent(new PopStateEvent('popstate'));
      }
    };
    window.addEventListener('keydown', navigateForGate);
    return () => window.removeEventListener('keydown', navigateForGate);
  }, []);

  // 首屏就绪后标记启动结束（v1.52 性能基线）
  useEffect(() => {
    if (isInitialized) {
      markStartupEnd('interactive');
    }
  }, [isInitialized]);
  const initializeAuth = async () => {
    try {
      console.log('[App] 🚀 开始初始化认证...');
      // BUG-023 修复（吸收自 v1 线 f5e6179）：等待 zustand persist 异步水合完成后再判断 token。
      // token 经 tauriStorage 异步恢复，useEffect 立即执行时往往尚未水合，
      // 旧逻辑会误判「未找到 token」提前显示登录页（dev 热重载随机弹回的根因），
      // 同时导致 restoreSession 从未真正执行。
      if (!useAuthStore.persist.hasHydrated()) {
        await new Promise<void>((resolve) => {
          const unsub = useAuthStore.persist.onFinishHydration(() => {
            unsub();
            resolve();
          });
        });
      }
      const token = useAuthStore.getState().token;
      if (!token) {
        console.log('[App] 未找到 token，显示登录界面');
        setIsInitialized(true);
        return;
      }
      console.log('[App] 📡 发现已保存的 token，尝试恢复会话...');
      const result = await auth.restoreSession(token);
      if (result.code === 0 && result.data) {
        console.log('[App] ✅ 会话恢复成功');
        storeLogin(result.data.user, result.data.token);
      } else {
        console.log('[App] ⚠️ 会话已失效，需要重新登录');
        useAuthStore.getState().logout();
      }
      setIsInitialized(true);
    } catch (error) {
      console.error('[App] ❌ 认证初始化失败:', error);
      useAuthStore.getState().logout();
      setIsInitialized(true);
    }
  };
  const handleLogin = (user?: any, token?: string) => {
    console.log('[App] ✅ 登录成功');
    if (user && token) {
      storeLogin(user, token);
    }
  };
  const renderErrorFallback = () => <div role="alert" style={{
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    backgroundColor: '#000000',
    color: '#FF0000',
    fontFamily: 'Consolas, monospace',
    padding: '40px'
  }}>
      <h1 style={{
      fontSize: '48px',
      marginBottom: '20px'
    }}>⚠️</h1>
      <h2 style={{
      fontSize: '24px',
      marginBottom: '16px'
    }}>{t("common.serviceUnavailable")}</h2>
      <p style={{
      color: '#CCCCCC',
      marginBottom: '24px',
      textAlign: 'center'
    }}>
        {t("app.k2")}<br />
        {t("app.k3")}
      </p>
      <button onClick={() => window.location.reload()} style={{
      padding: '12px 32px',
      backgroundColor: '#000000',
      border: '2px solid #FF0000',
      color: '#FF0000',
      fontSize: '16px',
      cursor: 'pointer',
      fontFamily: 'Consolas, monospace'
    }}>
        {t("app.k4")}
      </button>
    </div>;
  return <>
    <ErrorBoundary fallback={renderErrorFallback()}>
      {/* C4 §2.3.3 跳过导航链接：键盘用户可跳过 Layout 顶部导航直接进入主内容（#main-content 在 Layout 内） */}
      <a href="#main-content" className="skip-link">{t('common.skipToMain')}</a>
      <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', overflow: 'hidden' }}>
        {/* A5 Phase 3 Task 1：全局离线横幅 — 主内容区上方，所有页面可见（不在 Route 内） */}
        <OfflineBanner />
        <div style={{ flex: 1, overflow: 'hidden', position: 'relative' }}>
          <HighContrastKeyboardShortcut />
          <CyberpunkOverlay enabled={isAuthenticated} intensity={0.03} />
          {!isInitialized ? <LoadingScreen /> : !isAuthenticated ? <LoginModal onLogin={handleLogin} /> : <TimerProvider>
              <Suspense fallback={<PageLoadingFallback />}>
                <RouterProvider router={pluginRouter ?? router} />
              </Suspense>
            </TimerProvider>}
        </div>
      </div>
    </ErrorBoundary>
    </>;
}
function HighContrastKeyboardShortcut() {
  const { toggleHighContrast } = useA11yStore();
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.key === 'H') {
        e.preventDefault();
        toggleHighContrast();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleHighContrast]);
  return null;
}
export default App;
if (typeof window !== 'undefined') {
  window.addEventListener('unhandledrejection', event => {
    console.error('[Global] ❌ 未处理的 Promise rejection:', event.reason);
  });
  window.addEventListener('error', event => {
    console.error('[Global] ❌ 全局错误:', event.error);
  });
}