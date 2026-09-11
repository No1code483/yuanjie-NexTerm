import { t } from "i18next";
import { useTranslation } from 'react-i18next';
import { useState, useEffect, useCallback, useRef } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { useTimer } from '@/contexts/TimerContext';
import { useNavigation } from '@/routes/useNavigation';
import { extensionManager } from '../utils/extensionManager';
import { registerBuiltinExtensions } from '../utils/builtinExtensions';
import { NexTermAdapter } from '../types/extensions';
import { RecycleProvider, useRecycleContext } from '@/contexts/RecycleContext';
import AddExtensionModal from '../components/AddExtensionModal';
import FloatingBall from '../components/FloatingBall';
import SelectionToolbar from '../components/SelectionToolbar';
import { useActivityTracker } from '@/hooks/useActivityTracker';
import { useRouteAnnouncement } from '@/hooks/useRouteAnnouncement';
import { useModuleTheme } from '@/hooks/useModuleTheme'; // C1.4 模块级主题
import { useAuthStore } from '@/stores/authStore';
import ScrollToTop from '@/components/ScrollToTop';
import TitleBar from '@/components/TitleBar/TitleBar';
import { windowControl, checkIsTauri } from '@/lib/tauri';
import styles from './Layout.module.css';
interface Bookmark {
  id: string;
  label: string;
  icon: string;
  iconUrl: string;
  url: string;
}
const BOOKMARKS_KEY = 'nexterm_bookmarks';
function loadBookmarks(): Bookmark[] {
  try {
    const raw = localStorage.getItem(BOOKMARKS_KEY);
    if (raw) {
      return (JSON.parse(raw) as Bookmark[]).map(bookmark => ({
        ...bookmark,
        iconUrl: ''
      }));
    }
  } catch {/* ignore */}
  const defaults: Bookmark[] = [{
    id: 'github',
    label: 'GitHub',
    icon: '🐙',
    iconUrl: '',
    url: 'https://github.com'
  }, {
    id: 'huggingface',
    label: 'HuggingFace',
    icon: '🤗',
    iconUrl: '',
    url: 'https://huggingface.co'
  }, {
    id: 'owasp',
    label: 'OWASP',
    icon: '🛡️',
    iconUrl: '',
    url: 'https://owasp.org'
  }];
  localStorage.setItem(BOOKMARKS_KEY, JSON.stringify(defaults));
  return defaults;
}
function saveBookmarks(bookmarks: Bookmark[]) {
  localStorage.setItem(BOOKMARKS_KEY, JSON.stringify(bookmarks));
}
export default function Layout() {
  const [currentTime, setCurrentTime] = useState({
    short: '00:00:00',
    long: t("components.NexTermTimer.k3")
  });
  const [isExpanded, setIsExpanded] = useState(false);
  const [extensions, setExtensions] = useState<NexTermAdapter[]>([]);
  const [, setMountedExtensions] = useState<Set<string>>(new Set());
  const [showAddModal, setShowAddModal] = useState(false);
  const [bookmarks, setBookmarks] = useState<Bookmark[]>(loadBookmarks);

  // 窄屏响应式：导航栏溢出处理
  const navContainerRef = useRef<HTMLDivElement>(null);
  const [visibleNavCount, setVisibleNavCount] = useState(7);
  const [showMoreMenu, setShowMoreMenu] = useState(false);
  const allNavItems = [
    t("components.intelligence.DashboardPanel.k106"),
    t("components.PermissionRestricted.k2"),
    t("components.intelligence.ActivityPanel.k1"),
    t("components.intelligence.ActivityPanel.k8"),
    t("components.FloatingXin.k26"),
    t("components.AddExtensionModal.k11"),
    t("common.search")
  ];

  useEffect(() => {
    const calculateVisibleNav = () => {
      if (!navContainerRef.current) return;
      const containerWidth = navContainerRef.current.offsetWidth;
      // 估算每个导航项的宽度（约 60px，已减小间距）
      const itemWidth = 60;
      const maxItems = Math.floor(containerWidth / itemWidth);
      setVisibleNavCount(Math.max(2, Math.min(maxItems, allNavItems.length)));
    };

    calculateVisibleNav();
    window.addEventListener('resize', calculateVisibleNav);
    return () => window.removeEventListener('resize', calculateVisibleNav);
  }, []);
  const {
    isTiming,
    timerData,
    activeCountdown
  } = useTimer();
  const location = useLocation();
  const navigate = useNavigate();
  const isTauri = checkIsTauri();
  const {
    navigateToHome,
    navigateToAI,
    navigateToKnowledge,
    navigateToTerminal,
    navigateToXin,
    navigateToGame,
    navigateToProfile,
    navigateToRecycle,
    navigateToSearch
  } = useNavigation();

  // 合并标题栏：拖动窗口处理
  const handleHeaderDragStart = useCallback((e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('button, a, input, select, [role="button"]')) return;
    if (!isTauri) return;
    windowControl.startDragging();
  }, [isTauri]);

  const handleHeaderDoubleClick = useCallback(() => {
    if (!isTauri) return;
    windowControl.maximize();
  }, [isTauri]);

  // 全局页面访问追踪 — 记录用户浏览行为，为仪表盘提供真实数据
  useActivityTracker();
  // C4 §2.4.3 路由切换通知：向屏幕阅读器播报新路由名称
  useRouteAnnouncement();
  // C1.4 模块级主题：根据当前路由在 <html> 上同步 data-module-theme 属性
  useModuleTheme();
  // C2.4：订阅 i18n 变化，语言切换时触发 Layout 及其子树重渲染
  useTranslation();

  // 初始化扩展系统
  useEffect(() => {
    const initializeExtensions = async () => {
      try {
        // 注册内置扩展
        await registerBuiltinExtensions(extensionManager);

        // 获取所有扩展
        const allExtensions = extensionManager.getAll();
        setExtensions(allExtensions);

        // 监听扩展状态变化
        extensionManager.on('extensionMounted', (id: string) => {
          setMountedExtensions(prev => new Set([...prev, id]));
        });
        extensionManager.on('extensionUnmounted', (id: string) => {
          setMountedExtensions(prev => {
            const newSet = new Set(prev);
            newSet.delete(id);
            return newSet;
          });
        });

        // 自动挂载启用的扩展
        await extensionManager.mountAll();
        console.log('扩展系统初始化完成');
      } catch (error) {
        console.error('扩展系统初始化失败:', error);
      }
    };
    initializeExtensions();
  }, []);
  useEffect(() => {
    function onBookmarkAdd(e: Event) {
      const detail = (e as CustomEvent).detail as Bookmark;
      setBookmarks(prev => {
        if (prev.some(b => b.url === detail.url)) return prev;
        const next = [...prev, detail];
        saveBookmarks(next);
        return next;
      });
    }
    function onBookmarkRemove() {
      setBookmarks(loadBookmarks());
    }
    window.addEventListener('bookmark-add', onBookmarkAdd);
    window.addEventListener('bookmark-remove', onBookmarkRemove);
    return () => {
      window.removeEventListener('bookmark-add', onBookmarkAdd);
      window.removeEventListener('bookmark-remove', onBookmarkRemove);
    };
  }, []);

  // 根据当前路由获取侧边栏内容
  const getSidebarContent = () => {
    const path = location.pathname;
    const searchParams = new URLSearchParams(location.search);
    const currentTab = searchParams.get('tab');
    if (path.startsWith('/home')) {
      return {
        title: t("layout.k1"),
        items: [{
          id: 'news',
          label: t("components.intelligence.ActivityPanel.k9"),
          icon: '📰',
          route: '/home',
          active: currentTab === 'news'
        }, {
          id: 'todo',
          label: t("components.intelligence.ActivityPanel.k2"),
          icon: '✅',
          route: '/home',
          active: currentTab === 'todo'
        }, {
          id: 'log',
          label: t("components.intelligence.ActivityPanel.k3"),
          icon: '📝',
          route: '/home',
          active: currentTab === 'log'
        }, {
          id: 'timer',
          label: t("components.intelligence.ActivityPanel.k4"),
          icon: '⏱️',
          route: '/home',
          active: currentTab === 'timer'
        }]
      };
    }
    if (path.startsWith('/ai')) {
      return {
        title: t("components.PermissionRestricted.k2"),
        items: [{
          id: 'model',
          label: t("layout.k2"),
          icon: 'bot',
          route: '/ai',
          active: !currentTab || currentTab === 'model',
          vertical: true
        }, {
          id: 'agent',
          label: t("layout.k3"),
          icon: 'cube',
          route: '/ai',
          active: currentTab === 'agent',
          vertical: true
        }, {
          id: 'chat',
          label: t("layout.k4"),
          icon: 'chat',
          route: '/ai',
          active: currentTab === 'chat',
          vertical: true
        }, {
          id: 'group',
          label: t("layout.k5"),
          icon: 'users',
          route: '/ai',
          active: currentTab === 'group',
          vertical: true
        }]
      };
    }
    if (path.startsWith('/knowledge')) {
      return {
        title: t("components.intelligence.ActivityPanel.k1"),
        items: [],
        actionButtons: [{
          id: 'insert-template',
          label: t("layout.k6"),
          icon: '📋'
        }, {
          id: 'relation-graph',
          label: t("layout.k7"),
          icon: '🕸️'
        }]
      };
    }
    if (path.startsWith('/terminal')) {
      const isManualPage = path.startsWith('/terminal/manual');
      return {
        title: t("layout.k8"),
        items: [{
          id: 'terminal',
          label: t("components.intelligence.ActivityPanel.k8"),
          icon: '🖥️',
          route: '/terminal',
          active: !isManualPage && (!currentTab || currentTab === 'terminal')
        }, {
          id: 'yuancode',
          label: 'Yuan Code',
          icon: '◈',
          route: '/terminal/yuancode',
          active: currentTab === 'yuancode'
        }, {
          id: 'linux',
          label: 'Linux',
          icon: '🐧',
          route: '/terminal/linux',
          active: currentTab === 'linux'
        }, {
          id: 'manual',
          label: t("components.PermissionRestricted.k9"),
          icon: '📖',
          route: '/terminal/manual',
          active: isManualPage
        }]
      };
    }
    if (path.startsWith('/profile')) {
      return {
        title: t("components.intelligence.DashboardPanel.k105"),
        items: [{
          id: 'account',
          label: t("layout.k9"),
          icon: '👤',
          route: '/profile',
          active: !currentTab || currentTab === 'account'
        }, {
          id: 'resume',
          label: t("components.intelligence.ActivityPanel.k5"),
          icon: '📄',
          route: '/profile',
          active: currentTab === 'resume'
        }, {
          id: 'quote',
          label: t("components.intelligence.ActivityPanel.k6"),
          icon: '💬',
          route: '/profile',
          active: currentTab === 'quote'
        }, {
          id: 'setting',
          label: t("common.settings"),
          icon: '⚙️',
          route: '/profile',
          active: currentTab === 'setting'
        }, {
          id: 'logout',
          label: t("layout.k10"),
          icon: '🚪',
          isDanger: true
        }]
      };
    }
    if (path.startsWith('/recycle')) {
      return {
        title: t("components.PermissionRestricted.k4"),
        isRecyclePage: true,
        items: []
      };
    }
    if (path.startsWith('/game')) {
      return {
        title: t("layout.k11"),
        items: [{
          id: 'catalog',
          label: t("layout.k12"),
          icon: '🏗️',
          route: '/game',
          active: !currentTab || currentTab === 'catalog'
        }, {
          id: 'character',
          label: t("layout.k13"),
          icon: '🧘',
          route: '/game',
          active: currentTab === 'character'
        }, {
          id: 'achievements',
          label: t("layout.k14"),
          icon: '🏆',
          route: '/game',
          active: currentTab === 'achievements'
        }, {
          id: 'knowledge_rewards',
          label: t("layout.k15"),
          icon: '📚',
          route: '/game',
          active: currentTab === 'knowledge_rewards'
        }]
      };
    }
    if (path.startsWith('/xin')) {
      return {
        title: t("layout.k16"),
        items: [{
          id: 'chat',
          label: t("layout.k17"),
          icon: '💬',
          route: '/xin',
          active: !currentTab || currentTab === 'chat'
        }, {
          id: 'memory',
          label: t("layout.k18"),
          icon: '🧠',
          route: '/xin',
          active: currentTab === 'memory'
        }, {
          id: 'mood',
          label: t("layout.k19"),
          icon: '🌊',
          route: '/xin',
          active: currentTab === 'mood'
        }, {
          id: 'productivity',
          label: t("layout.k20"),
          icon: '⏱️',
          route: '/xin',
          active: currentTab === 'productivity'
        }, {
          id: 'briefing',
          label: t("layout.k21"),
          icon: '📊',
          route: '/xin',
          active: currentTab === 'briefing'
        }, {
          id: 'compaction',
          label: t("layout.k22"),
          icon: '🗜️',
          route: '/xin',
          active: currentTab === 'compaction'
        }, {
          id: 'dream',
          label: t("layout.k23"),
          icon: '🌙',
          route: '/xin',
          active: currentTab === 'dream'
        }, {
          id: 'checkpoint',
          label: t("layout.k24"),
          icon: '💾',
          route: '/xin',
          active: currentTab === 'checkpoint'
        }, {
          id: 'search',
          label: t("common.search"),
          icon: '🔍',
          route: '/xin',
          active: currentTab === 'search'
        }, {
          id: 'review',
          label: t("layout.k25"),
          icon: '📈',
          route: '/xin',
          active: currentTab === 'review'
        }, {
          id: 'skill',
          label: t("layout.k26"),
          icon: '⚡',
          route: '/xin',
          active: currentTab === 'skill'
        }, {
          id: 'tool',
          label: t("layout.k27"),
          icon: '🔧',
          route: '/xin',
          active: currentTab === 'tool'
        }]
      };
    }
    if (path.startsWith('/search')) {
      return {
        title: t("layout.k28"),
        items: bookmarks.map(b => ({
          ...b,
          active: false
        }))
      };
    }
    if (path.startsWith('/spyglass')) {
      return {
        title: t("components.intelligence.DashboardPanel.k103"),
        items: [{
          id: 'dashboard',
          label: t("layout.k29"),
          icon: '📊',
          route: '/spyglass',
          active: !currentTab || currentTab === 'dashboard'
        }, {
          id: 'suggestions',
          label: t("components.intelligence.ActivityPanel.k17"),
          icon: '💡',
          route: '/spyglass',
          active: currentTab === 'suggestions'
        }, {
          id: 'behavior',
          label: t("components.intelligence.ActivityPanel.k18"),
          icon: '🧠',
          route: '/spyglass',
          active: currentTab === 'behavior'
        }, {
          id: 'activity',
          label: t("layout.k30"),
          icon: '📋',
          route: '/spyglass',
          active: currentTab === 'activity'
        }, {
          id: 'settings',
          label: t("common.settings"),
          icon: '⚙️',
          route: '/spyglass',
          active: currentTab === 'settings'
        }]
      };
    }

    // 默认占位内容
    return {
      title: t("layout.k11"),
      items: [{
        id: 'placeholder',
        label: t("layout.k31"),
        icon: '🔧',
        route: path,
        active: true
      }]
    };
  };
  const sidebarContent = getSidebarContent();

  // 处理侧边栏项目点击
  const handleSidebarItemClick = (item: any) => {
    if (item.id === 'logout') {
      // 使用 authStore.logout() 安全清除 Token（tauriStorage），
      // 同时清除遗留的 localStorage 数据
      useAuthStore.getState().logout();
      localStorage.removeItem('nt_token');
      localStorage.removeItem('nt_temp_account');
      window.location.reload();
      return;
    }

    // 搜索页收藏网站：派发事件通知 Search 组件导航到目标网址
    const isSearchPage = location.pathname.startsWith('/search');
    if (isSearchPage && item.url) {
      window.dispatchEvent(new CustomEvent('search-navigate', {
        detail: item.url
      }));
      return;
    }
    if (item.route) {
      // 检查权限
      const isTemp = localStorage.getItem('nt_temp_account') === 'true';
      const routePath = `${item.route}?tab=${item.id}`;

      // 临时账号权限检查
      if (isTemp) {
        // 终端完全锁住
        if (item.id === 'terminal' || item.id === 'cmd' || item.id === 'xincode') {
          alert(t("layout.k32"));
          return;
        }

        // 个人中心：简历锁住
        if (item.id === 'resume') {
          alert(t("layout.k33"));
          return;
        }

        // AI会话：模型管理、Agent只读（允许访问但只读）
        // 个人中心：账号信息只读（允许访问但只读）
        // 这些功能允许访问，但会在对应页面组件中设置为只读模式
      }

      // 如果是首页功能，使用模态框方式打开
      if (item.route === '/home') {
        // 使用URL参数来触发Home组件的模态框
        navigate(`${item.route}?modal=${item.id}`);
      } else {
        // 其他页面正常导航，如果是临时账号且需要只读模式，传递只读标记
        let finalRoutePath = routePath;
        if (isTemp && (item.id === 'model' || item.id === 'agent' || item.id === 'account')) {
          finalRoutePath = `${routePath}&readonly=true`;
        }
        navigate(finalRoutePath);
      }
    }
  };

  // 处理导航栏点击
  const handleNavClick = (page: string) => {
    switch (page) {
      case t("components.intelligence.DashboardPanel.k106"):
        navigateToHome();
        break;
      case t("components.PermissionRestricted.k2"):
        navigateToAI();
        break;
      case t("components.intelligence.ActivityPanel.k1"):
        navigateToKnowledge();
        break;
      case t("components.intelligence.ActivityPanel.k8"):
        navigateToTerminal();
        break;
      case t("components.FloatingXin.k26"):
        navigateToXin();
        break;
      case t("components.AddExtensionModal.k11"):
        navigateToGame();
        break;
      case t("common.search"):
        navigateToSearch();
        break;
      default:
        navigateToHome();
    }
  };

  // 处理扩展添加完成
  const handleExtensionAdded = (adapter: NexTermAdapter) => {
    setExtensions(prev => [...prev, adapter]);
    console.log(`扩展添加成功: ${adapter.name}`);
  };
  useEffect(() => {
    const timer = setInterval(() => {
      const now = new Date();
      const hours = String(now.getHours()).padStart(2, '0');
      const minutes = String(now.getMinutes()).padStart(2, '0');
      const seconds = String(now.getSeconds()).padStart(2, '0');
      setCurrentTime(prev => ({
        ...prev,
        short: `${hours}:${minutes}:${seconds}`
      }));
    }, 1000);
    return () => clearInterval(timer);
  }, []);
  function RecycleActions() {
    const {
      selectedFiles,
      clearSelection
    } = useRecycleContext();
    return <div className={styles.recycleActions}>
        <button className={styles.recycleActionPrimary} disabled={selectedFiles.length === 0} onClick={() => window.dispatchEvent(new CustomEvent('recycle-restore'))} aria-label={t("common.restore")}>
          {t("layout.k34")}{selectedFiles.length})
        </button>
        <button className={styles.recycleActionDanger} disabled={selectedFiles.length === 0} onClick={() => window.dispatchEvent(new CustomEvent('recycle-delete'))} aria-label={t("common.permanentDelete")}>
          {t("layout.k35")}{selectedFiles.length})
        </button>
        <button className={styles.recycleActionDanger} onClick={() => window.dispatchEvent(new CustomEvent('recycle-clear'))} aria-label={t("common.clearAll")}>
          {t("common.clearAll")}
        </button>
        <button className={styles.recycleActionSecondary} onClick={() => window.dispatchEvent(new CustomEvent('recycle-select-all'))} aria-label={t("common.selectAll")}>
          {t("common.selectAll")}
        </button>
        <button className={styles.recycleActionSecondary} disabled={selectedFiles.length === 0} onClick={() => clearSelection()} aria-label={t("common.deselect")}>
          {t("common.deselect")}
        </button>
      </div>;
  }
  const handleFaviconError = (e: React.SyntheticEvent<HTMLImageElement>) => {
    const img = e.currentTarget;
    img.style.display = 'none';
    const next = img.nextElementSibling as HTMLElement | null;
    if (next) next.style.display = 'inline';
  };
  return <RecycleProvider>
    <ScrollToTop />
    <div style={{
      height: '100vh',
      display: 'flex',
      flexDirection: 'column'
    }}>
      {/* 合并标题栏 + 顶部导航栏（拖动区域覆盖整个 header） */}
      <header className={styles.header} role="banner" onMouseDown={handleHeaderDragStart} onDoubleClick={handleHeaderDoubleClick}>
        {/* 左侧：应用图标 + 标题 */}
        <div className={styles.headerLeft}>
          <span className={styles.appLogo} aria-hidden="true">◈</span>
          <span className={styles.appTitle}>
            <span>NexTerm</span>
            <span>元界</span>
          </span>
        </div>

        {/* 中间：主导航 */}
        <nav className={styles.navContainer} ref={navContainerRef} role="navigation" aria-label={t('a11y.mainNav')}>
          {allNavItems.slice(0, visibleNavCount).map(item => <div key={item} onClick={() => handleNavClick(item)} onKeyDown={e => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              handleNavClick(item);
            }
          }} className={`${styles.navItem} ${location.pathname.includes(getNavPath(item)) ? styles.navItemActive : ''}`} role="button" tabIndex={0} aria-current={location.pathname.includes(getNavPath(item)) ? 'page' : undefined}>
              {item}
            </div>)}
        </nav>

        {/* 窄屏响应式："…"按钮在导航区与时间区之间 */}
        {visibleNavCount < allNavItems.length && <div style={{ position: 'relative' }}>
            <div className={styles.navMoreButton} onClick={() => setShowMoreMenu(!showMoreMenu)} role="button" tabIndex={0} aria-label={t("common.more")}>
              ⋯
            </div>
            {showMoreMenu && <div className={styles.navDropdown}>
                {allNavItems.slice(visibleNavCount).map(item => <div key={item} className={`${styles.navDropdownItem} ${location.pathname.includes(getNavPath(item)) ? styles.navDropdownItemActive : ''}`} onClick={() => {
                handleNavClick(item);
                setShowMoreMenu(false);
              }} role="button" tabIndex={0}>
                    {item}
                  </div>)}
              </div>}
          </div>}

        {/* 时间显示 */}
        <div className={styles.timeDisplay}>
          {isTiming ? <span>
              {timerData.time} {timerData.days}
            </span> : currentTime.short}
          {activeCountdown && <span className={styles.countdownInline}>
              {activeCountdown.shortName} {(() => {
              const target = new Date(activeCountdown.targetDate);
              const now = new Date();
              const diff = Math.ceil((target.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
              return t("layout.k36", {
                diff: diff
              });
            })()}
            </span>}
        </div>

        {/* 右侧：原 TitleBar 窗口控制按钮 */}
        <TitleBar />
      </header>
      
      {/* 主体内容区域 */}
      <div style={{
        display: 'flex',
        flex: 1,
        overflow: 'hidden'
      }}>
        {/* 侧边栏 */}
        <aside id="sidebar" className={styles.sidebar} role="navigation" aria-label={t('a11y.sidebar')}>
          {/* 页面特定内容区域 */}
          <div style={{
            flex: sidebarContent.items.length > 0 || (sidebarContent as any).isRecyclePage ? 1 : undefined,
            padding: '0 10px',
            overflow: 'auto'
          }}>
            <div className={styles.sidebarTitle}>
              {sidebarContent.title}
            </div>

            {(sidebarContent as any).isRecyclePage ? <RecycleActions /> : <div className={styles.sidebarItems}>
                {sidebarContent.items.map(item => {
                const isDanger = (item as any).isDanger || false;
                const iconName = (item as any).icon || '';
                const isSvgIcon = ['bot', 'cube', 'chat', 'users'].includes(iconName);
                const renderSvgIcon = () => {
                  const stroke = isDanger ? '#FF0040' : item.active ? '#00F0FF' : 'rgba(184, 184, 208, 0.7)';
                  switch (iconName) {
                    case 'bot':
                      return <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" style={{
                        color: stroke
                      }}>
                            <rect x="3" y="8" width="18" height="12" rx="2" />
                            <path d="M12 2v4" />
                            <path d="M8 6h8" />
                            <circle cx="9" cy="14" r="1.5" fill="currentColor" />
                            <circle cx="15" cy="14" r="1.5" fill="currentColor" />
                            <path d="M9 18h6" />
                            <path d="M3 14v-2" />
                            <path d="M21 14v-2" />
                          </svg>;
                    case 'cube':
                      return <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" style={{
                        color: stroke
                      }}>
                            <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
                            <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
                            <line x1="12" y1="22.08" x2="12" y2="12" />
                          </svg>;
                    case 'chat':
                      return <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" style={{
                        color: stroke
                      }}>
                            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
                          </svg>;
                    case 'users':
                      return <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" style={{
                        color: stroke
                      }}>
                            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
                            <circle cx="9" cy="7" r="4" />
                            <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
                            <path d="M16 3.13a4 4 0 0 1 0 7.75" />
                          </svg>;
                    default:
                      return null;
                  }
                };
                const label = item.label;
                const renderLabel = () => {
                  // 四字功能项统一 2×2 排布（不再依赖 isVertical 标记）
                  if (label.length === 4) {
                    return <div className={styles.verticalLabel}>
                          <span>{label.slice(0, 2)}</span>
                          <span>{label.slice(2, 4)}</span>
                        </div>;
                  }
                  return <span style={{
                    fontSize: '13px',
                    marginLeft: (item as any).iconUrl || isSvgIcon ? '8px' : '0'
                  }}>{label}</span>;
                };
                // 四字功能项统一使用 vertical 样式（2×2 排布）
                const useVerticalStyle = label.length === 4;
                return <div key={item.id} onClick={() => handleSidebarItemClick(item)} onKeyDown={e => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault();
                    handleSidebarItemClick(item);
                  }
                }} className={`
                        ${styles.sidebarItem}
                        ${item.active ? styles.sidebarItemActive : ''}
                        ${isDanger ? styles.sidebarItemDanger : ''}
                        ${isDanger && item.active ? styles.sidebarItemDangerActive : ''}
                        ${useVerticalStyle ? styles.sidebarItemVertical : ''}
                      `} role="button" tabIndex={0}>
                      {(item as any).iconUrl ? <img src={(item as any).iconUrl} alt="" style={{
                    width: 16,
                    height: 16,
                    flexShrink: 0
                  }} onError={handleFaviconError} /> : isSvgIcon ? <div className={styles.sidebarIconWrap}>
                          {renderSvgIcon()}
                        </div> : <span style={{
                    fontSize: '16px',
                    display: (item as any).iconUrl ? 'none' : 'inline'
                  }}>{item.icon}</span>}
                      {renderLabel()}
                    </div>;
              })}
              </div>}
          </div>

          {/* 功能按钮区（知识库等页面专用） */}
          {location.pathname.startsWith('/knowledge') && <div className={styles.actionButtons}>
              <div className={`${styles.actionBtn} ${localStorage.getItem('kb_library') !== 'study' ? styles.actionBtnActive : ''}`} onClick={() => {
              localStorage.setItem('kb_library', 'material');
              window.dispatchEvent(new CustomEvent('kb-library-change', {
                detail: 'material'
              }));
            }} onKeyDown={e => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                localStorage.setItem('kb_library', 'material');
                window.dispatchEvent(new CustomEvent('kb-library-change', {
                  detail: 'material'
                }));
              }
            }} title={t("layout.k37")} role="button" tabIndex={0}>
                <span className={styles.actionBtnIcon}>📁</span>
                <span className={styles.actionBtnLabel}>{t("layout.k37")}</span>
              </div>
              <div className={`${styles.actionBtn} ${localStorage.getItem('kb_library') === 'study' ? styles.actionBtnActive : ''}`} onClick={() => {
              localStorage.setItem('kb_library', 'study');
              window.dispatchEvent(new CustomEvent('kb-library-change', {
                detail: 'study'
              }));
            }} onKeyDown={e => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                localStorage.setItem('kb_library', 'study');
                window.dispatchEvent(new CustomEvent('kb-library-change', {
                  detail: 'study'
                }));
              }
            }} title={t("layout.k38")} role="button" tabIndex={0}>
                <span className={styles.actionBtnIcon}>📚</span>
                <span className={styles.actionBtnLabel}>{t("layout.k38")}</span>
              </div>
            </div>}

          {(sidebarContent as any).actionButtons && (sidebarContent as any).actionButtons.length > 0 && <div className={styles.actionButtons}>
              {(sidebarContent as any).actionButtons.map((btn: {
              id: string;
              label: string;
              icon: string;
            }) => <div key={btn.id} onClick={() => {
              if (btn.id === 'insert-template') {
                window.dispatchEvent(new CustomEvent('kb-action', {
                  detail: 'insert-template'
                }));
              } else if (btn.id === 'relation-graph') {
                window.dispatchEvent(new CustomEvent('kb-action', {
                  detail: 'relation-graph'
                }));
              }
            }} onKeyDown={e => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                window.dispatchEvent(new CustomEvent('kb-action', {
                  detail: btn.id === 'insert-template' ? 'insert-template' : 'relation-graph'
                }));
              }
            }} className={styles.actionBtn} title={btn.label} role="button" tabIndex={0}>
                  <span className={styles.actionBtnIcon}>{btn.icon}</span>
                  <span className={styles.actionBtnLabel}>{btn.label}</span>
                </div>)}
            </div>}

          {/* 底部固定区域 - 扩展模块和回收站 */}
          <div style={{
            marginTop: 'auto',
            paddingTop: '15px'
          }}>
            {/* 扩展模块 */}
            <div style={{
              borderTop: 'var(--nt-border-width) solid rgba(0, 240, 255, 0.4)',
              borderBottom: 'var(--nt-border-width) solid rgba(0, 240, 255, 0.4)',
              marginBottom: '10px'
            }}>
              <div onClick={() => setIsExpanded(!isExpanded)} onKeyDown={e => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  setIsExpanded(!isExpanded);
                }
              }} className={styles.expandSection} role="button" tabIndex={0} aria-expanded={isExpanded}>
                <div style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '6px'
                }}>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#00F0FF" strokeWidth="1.5">
                    <path d="M4 6H20M4 12H20M4 18H20" />
                  </svg>
                  <span style={{
                    fontSize: '13px',
                    color: '#00F0FF'
                  }}>{t("layout.k39")}{extensions.length})</span>
                </div>
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#00F0FF" strokeWidth="1.5" style={{
                  transform: isExpanded ? 'rotate(180deg)' : 'rotate(0deg)',
                  transition: 'transform 0.3s'
                }}>
                  <path d="M6 9L12 15L18 9" />
                </svg>
              </div>

              {/* 扩展内容区域 */}
              {isExpanded && <div className={styles.expandContent}>
                  <div style={{
                  padding: '30px 20px',
                  textAlign: 'center',
                  color: 'rgba(106, 106, 138, 0.65)',
                  fontSize: '13px',
                  fontStyle: 'italic'
                }}>
                    {t("layout.k40")}
                  </div>
                </div>}
            </div>

            {/* 个人中心（从 header 移至此处，全局固定） */}
            <div className={styles.profileIcon} onClick={navigateToProfile} onKeyDown={e => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                navigateToProfile();
              }
            }} role="button" tabIndex={0} aria-label={t('nav.profile')}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#00F0FF" strokeWidth="1.5">
                <path d="M20 21V19C20 17.9391 19.5786 16.9217 18.8284 16.1716C18.0783 15.4214 17.0609 15 16 15H8C6.93913 15 5.92172 15.4214 5.17157 16.1716C4.42143 16.9217 4 17.9391 4 19V21" />
                <circle cx="12" cy="7" r="4" />
              </svg>
              <span style={{ fontSize: '13px' }}>{t('nav.profile')}</span>
            </div>

            {/* 回收站 */}
            <div className={styles.recycleBin}>
              <div onClick={navigateToRecycle} onKeyDown={e => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  navigateToRecycle();
                }
              }} className={styles.recycleBinItem} role="button" tabIndex={0}>
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                  <path d="M3 6H5H21" strokeLinecap="round" strokeLinejoin="round" />
                  <path d="M8 6V4C8 3.46957 8.21071 2.96086 8.58579 2.58579C8.96086 2.21071 9.46957 2 10 2H14C14.5304 2 15.0391 2.21071 15.4142 2.58579C15.7893 2.96086 16 3.46957 16 4V6M19 6V20C19 20.5304 18.7893 21.0391 18.4142 21.4142C18.0391 21.7893 17.5304 22 17 22H7C6.46957 22 5.96086 21.7893 5.58579 21.4142C5.21071 21.0391 5 20.5304 5 20V6H19Z" strokeLinecap="round" strokeLinejoin="round" />
                  <path d="M10 11V17" strokeLinecap="round" />
                  <path d="M14 11V17" strokeLinecap="round" />
                </svg>
                <span style={{
                  fontSize: '13px'
                }}>{t("components.PermissionRestricted.k4")}</span>
              </div>
            </div>
          </div>
        </aside>

        {/* 主内容区域（C4 §2.3.3 skip-link 目标）*/}
        <main id="main-content" className={styles.mainContent} role="main" tabIndex={-1}>
          <Outlet />
        </main>
      </div>
      
      {/* 添加扩展模态框 */}
      <AddExtensionModal isOpen={showAddModal} onClose={() => setShowAddModal(false)} onExtensionAdded={handleExtensionAdded} />

      {/* 智能悬浮球 */}
      <FloatingBall />
      {/* 选中文本快捷 AI 工具栏 */}
      <SelectionToolbar />
      {/* C4 §2.4.3 路由切换通知节点：屏幕阅读器播报路由变化（sr-only 视觉隐藏）*/}
      <div id="route-announcer" role="status" aria-live="polite" className="sr-only"></div>
    </div>
    </RecycleProvider>;
}

// 辅助函数：获取导航路径
function getNavPath(navItem: string): string {
  const pathMap: Record<string, string> = {
    '首页': '/home',
    'AI会话': '/ai',
    '知识库': '/knowledge',
    '终端': '/terminal',
    '小欣': '/xin',
    '游戏': '/game',
    '搜索': '/search'
  };
  return pathMap[navItem] || '';
}