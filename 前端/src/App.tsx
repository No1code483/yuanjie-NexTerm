import { t } from "i18next";
import { useTranslation } from 'react-i18next';
import { useState, useEffect, useRef, Suspense } from 'react';
import { RouterProvider } from 'react-router-dom';
import LoginModal from './pages/LoginModal';
import router from './routes/router';
import { TimerProvider } from './contexts/TimerContext';
import { HighContrastProvider, useHighContrast } from './contexts/HighContrastContext';
import ErrorBoundary from './components/ErrorBoundary';
import CyberpunkOverlay from './components/CyberpunkOverlay';
import OfflineBanner from './components/OfflineBanner/OfflineBanner';
import { SkeletonPage, SkeletonCard, SkeletonText } from './components/ui/Skeleton';
import { useAuthStore } from './stores/authStore';
import { auth } from './lib/ipc';
import { initPerfMonitor, markStartupBegin, markStartupEnd } from './lib/perfMonitor';
// T2.1.7: Monaco Editor loader 配置（side-effect import，配置加载路径，Monaco 本身在 <Editor> 挂载时才加载）
import './lib/monaco';
import { useFocusManagement } from './hooks/useFocusManagement';
import { useA11yStore } from './stores/a11yStore';
import { initAriaAnnouncer } from './utils/ariaAnnouncer';

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
  // C2.4：useTranslation hook 触发语言切换时的全局重渲染
  // 在根节点订阅 i18n 变化，语言切换时整个组件树重渲染，
  // 子组件使用全局 t() 时会自动返回新语言的值。
  useTranslation();
  const {
    isAuthenticated,
    token,
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

  // 首屏就绪后标记启动结束（v1.52 性能基线）
  useEffect(() => {
    if (isInitialized) {
      markStartupEnd('interactive');
    }
  }, [isInitialized]);
  const initializeAuth = async () => {
    try {
      console.log('[App] � 开始初始化认证...');
      if (!token) {
        console.log('[App] 未找到 token，显示登录界面');
        setIsInitialized(true);
        return;
      }
      console.log('[App] � 发现已保存的 token，尝试恢复会话...');
      const result = await (auth as any).restoreSession(token);
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
  return <HighContrastProvider>
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
                <RouterProvider router={router} />
              </Suspense>
            </TimerProvider>}
        </div>
      </div>
    </ErrorBoundary>
    </HighContrastProvider>;
}
function HighContrastKeyboardShortcut() {
  const {
    toggleHighContrast
  } = useHighContrast();
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