import { t } from "i18next";
import { useTranslation } from 'react-i18next';
import { useState, useEffect, useCallback, useRef } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { listen } from '@tauri-apps/api/event';
import type { KernelEvent } from '@/kernel/types';
import { useTimer } from '@/plugins/boards/home/features/timer/context';
import { useNavigation } from '@/routes/useNavigation';
import { ROUTES } from '@/routes/routes';
import { RecycleProvider } from '@/plugins/customs/recycle/context';
import RecycleActions from '@/plugins/customs/recycle/features/actions/RecycleActions';
import { useNavTree } from '@/kernel/registry/context';
import PluginManagerNav from '@/kernel/plugin-manager/PluginManagerNav';
import FloatingBall from '../components/FloatingBall';
import SelectionToolbar from '../components/SelectionToolbar';
import { useActivityTracker } from '@/hooks/useActivityTracker';
import { useRouteAnnouncement } from '@/hooks/useRouteAnnouncement';
import { useModuleTheme } from '@/hooks/useModuleTheme'; // C1.4 模块级主题
import { useAuthStore } from '@/kernel/state/authStore';
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
  // 顶部导航随内核启用清单联动（手稿 20260926：板块插件停用 → 导航项消失、右侧自动补位；
  // null = 内核未就绪/查询失败，保持全量显示兜底）
  const [enabledIds, setEnabledIds] = useState<string[] | null>(null);
  // 注册表导航树（K3/K4：侧边栏改为注册表驱动；标题=父插件名，选项=已启用子插件）
  const navTree = useNavTree();

  // 订阅内核启用清单（独立 listen，不与 App/管理页叠加；启停板块 → 导航即时重建）
  useEffect(() => {
    const query = async () => {
      try {
        const invokeMod = await import('@tauri-apps/api/core');
        const list = await invokeMod.invoke<Array<{ id: string; state: string }>>(
          'plugin:kernel|kernel_dispatch',
          { cmd: 'kernel:plugin:get_enabled', args: {} }
        );
        setEnabledIds(list.filter((p) => p.state === 'enabled').map((p) => p.id));
      } catch { setEnabledIds(null); }
    };
    void query();
    const unlisten = listen<KernelEvent>('k://event', (ev) => {
      if (ev.payload.name === 'kernel:plugin.state-changed') void query();
    });
    return () => { void unlisten.then((fn) => fn()); };
  }, []);
  const [bookmarks, setBookmarks] = useState<Bookmark[]>(loadBookmarks);

  // 窄屏响应式：导航栏溢出处理
  const navContainerRef = useRef<HTMLDivElement>(null);
  const [visibleNavCount, setVisibleNavCount] = useState(7);
  const [showMoreMenu, setShowMoreMenu] = useState(false);
  // 顶部导航项 = 板块插件清单（手稿 20260926 ①：板块插件显示在顶部导航，停用/卸载即消失、右侧自动补位。
  // id 为插件 id（含 Hello/_hello 与 搜索/customs.search），可见性由 enabledIds 过滤）
  const navItems = [
    { id: 'boards.home', label: t("components.intelligence.DashboardPanel.k106"), path: '/home' },
    { id: 'boards.ai', label: t("components.PermissionRestricted.k2"), path: '/ai' },
    { id: 'boards.knowledge', label: t("components.intelligence.ActivityPanel.k1"), path: '/knowledge' },
    { id: 'boards.terminal', label: t("components.intelligence.ActivityPanel.k8"), path: '/terminal' },
    { id: 'boards.xin', label: t("components.FloatingXin.k26"), path: '/xin' },
    { id: 'boards.game', label: t("components.AddExtensionModal.k11"), path: '/game' },
    { id: 'customs.search', label: t("common.search"), path: '/search' },
    { id: '_hello', label: 'Hello', path: '/hello' }
  ];
  // 启用过滤（null = 内核未就绪 → 全量显示兜底；首页为必备插件恒在）
  const visibleNavItems = enabledIds === null
    ? navItems
    : navItems.filter((item) => enabledIds.includes(item.id));

  useEffect(() => {
    const calculateVisibleNav = () => {
      if (!navContainerRef.current) return;
      const containerWidth = navContainerRef.current.offsetWidth;
      // 估算每个导航项的宽度（约 60px，已减小间距）
      const itemWidth = 60;
      const maxItems = Math.floor(containerWidth / itemWidth);
      setVisibleNavCount(Math.max(2, Math.min(maxItems, navItems.length)));
    };

    calculateVisibleNav();
    window.addEventListener('resize', calculateVisibleNav);
    return () => window.removeEventListener('resize', calculateVisibleNav);
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
    navigateToProfile,
    navigateToRecycle
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

  /** 侧边栏注册表驱动（K4）：命中当前路径的主干节点 → 标题=父插件名，选项=已启用子插件
   *  navItems（`labelKey→t()`、`routePath→navigate`）；无子插件或未匹配时返回 null，回退 legacy 硬编码分支。 */
  const registrySidebar = (): {
    title: string;
    registryDriven: true;
    items: Array<{ id: string; label: string; icon?: string; route: string; registryDriven: true; active: boolean }>;
  } | null => {
    const path = location.pathname;
    const match = navTree
      .filter((n) => path === n.routePath || path.startsWith(`${n.routePath}/`))
      .sort((a, b) => b.routePath.length - a.routePath.length)[0];
    // 仅采纳「tab 型板块内选项」（routePath 含 `?tab=`）——排除 recycle 等非 tab 型历史 navItem，
    // 未采纳时回退 legacy 硬编码分支（其他板块本轮不迁移）
    const children = (match?.children ?? []).filter((c) => c.routePath.includes('?tab='));
    if (!match || children.length === 0) return null;
    const tabOf = (routePath: string) =>
      new URLSearchParams(routePath.split('?')[1] ?? '').get('tab');
    const currentTab = new URLSearchParams(location.search).get('tab');
    // 无 tab 参数时以首个选项为默认激活项（如知识库首个选项=条目浏览）
    const defaultTab = tabOf(children[0].routePath);
    return {
      title: t(match.labelKey),
      registryDriven: true,
      items: children.map((c) => ({
        id: c.id,
        label: t(c.labelKey),
        icon: c.icon,
        route: c.routePath,
        registryDriven: true,
        active: (currentTab ?? defaultTab) === tabOf(c.routePath),
      })),
    };
  };

  // 根据当前路由获取侧边栏内容
  const getSidebarContent = () => {
    // 注册表驱动优先（知识库等已迁移板块）；否则走下方 legacy 硬编码分支
    const regSidebar = registrySidebar();
    if (regSidebar) return regSidebar;
    const path = location.pathname;
    const searchParams = new URLSearchParams(location.search);
    const currentTab = searchParams.get('tab');
    // 个人中心 / 回收站（首页子插件，手稿 20260926 ②）：须先于 /home 前缀判定（二者路由为 /home/profile、/home/recycle）
    if (path.startsWith(ROUTES.PROFILE)) {
      // 个人中心「一切皆插件」（2026-10-05）：各选项按子插件 pluginId 门控（停用即隐藏）；
      // boards.profile 为 boards.home 的 L2（三级嵌套），其子插件不采用 ?tab= 注册表驱动
      return {
        title: t("components.intelligence.DashboardPanel.k105"),
        items: [{
          id: 'account',
          pluginId: 'profile.account',
          label: t("layout.k9"),
          icon: '👤',
          route: ROUTES.PROFILE,
          active: !currentTab || currentTab === 'account'
        }, {
          id: 'resume',
          pluginId: 'profile.resume',
          label: t("components.intelligence.ActivityPanel.k5"),
          icon: '📄',
          route: ROUTES.PROFILE,
          active: currentTab === 'resume'
        }, {
          id: 'quote',
          pluginId: 'profile.quote',
          label: t("components.intelligence.ActivityPanel.k6"),
          icon: '💬',
          route: ROUTES.PROFILE,
          active: currentTab === 'quote'
        }, {
          id: 'setting',
          pluginId: 'profile.settings',
          label: t("common.settings"),
          icon: '⚙️',
          route: ROUTES.PROFILE,
          active: currentTab === 'setting'
        }, {
          id: 'logout',
          label: t("layout.k10"),
          icon: '🚪',
          isDanger: true
        }]
      };
    }
    if (path.startsWith(ROUTES.RECYCLE)) {
      return {
        title: t("components.PermissionRestricted.k4"),
        isRecyclePage: true,
        items: []
      };
    }
    if (path.startsWith('/home')) {
      return {
        title: t("layout.k1"),
        items: [{
          id: 'news',
          pluginId: 'home.news',
          label: t("components.intelligence.ActivityPanel.k9"),
          icon: '📰',
          route: '/home',
          active: currentTab === 'news'
        }, {
          id: 'todo',
          pluginId: 'home.todo',
          label: t("components.intelligence.ActivityPanel.k2"),
          icon: '✅',
          route: '/home',
          active: currentTab === 'todo'
        }, {
          id: 'log',
          pluginId: 'home.journal',
          label: t("components.intelligence.ActivityPanel.k3"),
          icon: '📝',
          route: '/home',
          active: currentTab === 'log'
        }, {
          id: 'timer',
          pluginId: 'home.timer',
          label: t("components.intelligence.ActivityPanel.k4"),
          icon: '⏱️',
          route: '/home',
          active: currentTab === 'timer'
        }]
      };
    }
    if (path.startsWith('/ai')) {
      // AI 会话：侧边栏选项由注册表驱动（上方 registrySidebar 已处理，AI 为 /ai?tab= 单页模型）；
      // 此处仅为内核未就绪/子插件全停用时的标题兜底
      return {
        title: t("components.PermissionRestricted.k2"),
        items: []
      };
    }
    if (path.startsWith('/knowledge')) {
      // 知识库：侧边栏选项由注册表驱动（上方 registrySidebar 已处理）；
      // 此处仅为内核未就绪/子插件全停用时的标题兜底（无 actionButtons、无私有事件通道）
      return {
        title: t("components.intelligence.ActivityPanel.k1"),
        items: []
      };
    }
    if (path.startsWith('/terminal')) {
      const isManualPage = path.startsWith('/terminal/manual');
      return {
        title: t("layout.k8"),
        items: [{
          id: 'terminal',
          pluginId: 'boards.terminal',
          label: t("components.intelligence.ActivityPanel.k8"),
          icon: '🖥️',
          route: '/terminal',
          active: !isManualPage && (!currentTab || currentTab === 'terminal')
        }, {
          id: 'yuancode',
          pluginId: 'terminal.yuancode',
          label: 'Yuan Code',
          icon: '◈',
          route: '/terminal/yuancode',
          active: currentTab === 'yuancode'
        }, {
          id: 'linux',
          pluginId: 'terminal.linux',
          label: 'Linux',
          icon: '🐧',
          route: '/terminal/linux',
          active: currentTab === 'linux'
        }, {
          id: 'manual',
          // 修正：命令手册归 terminal.manual L2（原误标 boards.terminal，致停用后选项不消失）
          pluginId: 'terminal.manual',
          label: t("components.PermissionRestricted.k9"),
          icon: '📖',
          route: '/terminal/manual',
          active: isManualPage
        }]
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
      // 小欣：侧边栏选项由注册表驱动（上方 registrySidebar 已处理，/xin?tab= 单页模型）；
      // 此处仅为内核未就绪/子插件全停用时的标题兜底
      return {
        title: t("layout.k16"),
        items: []
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
      // 底层智能：侧边栏选项由注册表驱动（上方 registrySidebar 已处理，/spyglass?tab= 单页模型）；
      // 此处仅为内核未就绪/子插件全停用时的标题兜底
      return {
        title: t("components.intelligence.DashboardPanel.k103"),
        items: []
      };
    }
    if (path.startsWith('/sync')) {
      // 同步：侧边栏选项由注册表驱动（上方 registrySidebar 已处理，/sync?tab= 单页模型）；
      // 此处仅为内核未就绪/子插件全停用时的标题兜底
      return {
        title: t("components.Sync.k1"),
        items: []
      };
    }

    if (path.startsWith('/plugin-manager')) {
      // 插件管理页：分组目录由 PluginManagerNav 渲染于全局侧边栏（手稿 20260926 ③，不占主内容区）
      return {
        title: '插件管理',
        isPluginManagerPage: true,
        items: []
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
  // 板块内菜单随插件启停联动（手稿 20260926：子插件停用 → 对应菜单项同步隐藏，消除僵菜单）
  const sidebarItems = (sidebarContent.items as unknown[]).filter((item) => {
    const pluginId = (item as { pluginId?: string }).pluginId;
    return !pluginId || enabledIds === null || enabledIds.includes(pluginId);
  }) as typeof sidebarContent.items;

  // 侧边栏操作区（recycle.actions 可选子插件）随启停门控（Layout 已持有 enabledIds）
  const recycleActionsEnabled = enabledIds === null || enabledIds.includes('recycle.actions');

  // 处理侧边栏项目点击
  const handleSidebarItemClick = (item: any) => {
    // 注册表驱动的选项：routePath 已含 tab（如 /knowledge?tab=search），直接导航
    if (item.registryDriven) {
      if (item.route) {
        // 临时账号：AI 模型/Agent 只读（等价于 legacy item.id==='model'|'agent' 的 &readonly=true）
        const isTemp = localStorage.getItem('nt_temp_account') === 'true';
        let target = item.route as string;
        if (isTemp && /[?&]tab=(model|agent)\b/.test(target)) target += '&readonly=true';
        navigate(target);
      }
      return;
    }
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

  // 处理导航栏点击：按导航项路径跳转（板块停用后导航项消失，不可达）
  const handleNavClick = (path: string) => {
    navigate(path);
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

        {/* 中间：主导航（板块插件动态渲染：停用即消失、右侧自动补位，手稿 20260926 ①） */}
        <nav className={styles.navContainer} ref={navContainerRef} role="navigation" aria-label={t('a11y.mainNav')}>
          {visibleNavItems.slice(0, visibleNavCount).map(item => <div key={item.id} onClick={() => handleNavClick(item.path)} onKeyDown={e => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              handleNavClick(item.path);
            }
          }} className={`${styles.navItem} ${location.pathname.startsWith(item.path) ? styles.navItemActive : ''}`} role="button" tabIndex={0} aria-current={location.pathname.startsWith(item.path) ? 'page' : undefined} style={item.id === '_hello' ? { color: '#00ff9d' } : undefined}>
              {item.label}
            </div>)}
        </nav>

        {/* 窄屏响应式："…"按钮在导航区与时间区之间 */}
        {visibleNavCount < visibleNavItems.length && <div style={{ position: 'relative' }}>
            <div className={styles.navMoreButton} onClick={() => setShowMoreMenu(!showMoreMenu)} role="button" tabIndex={0} aria-label={t("common.more")}>
              ⋯
            </div>
            {showMoreMenu && <div className={styles.navDropdown}>
                {visibleNavItems.slice(visibleNavCount).map(item => <div key={item.id} className={`${styles.navDropdownItem} ${location.pathname.startsWith(item.path) ? styles.navDropdownItemActive : ''}`} onClick={() => {
                handleNavClick(item.path);
                setShowMoreMenu(false);
              }} role="button" tabIndex={0}>
                    {item.label}
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
            flex: sidebarItems.length > 0 || (sidebarContent as any).isRecyclePage || (sidebarContent as any).isPluginManagerPage ? 1 : undefined,
            padding: '0 10px',
            overflow: 'auto'
          }}>
            <div className={styles.sidebarTitle}>
              {sidebarContent.title}
            </div>

            {(sidebarContent as any).isPluginManagerPage ? <PluginManagerNav /> : (sidebarContent as any).isRecyclePage ? (recycleActionsEnabled ? <RecycleActions /> : null) : <div className={styles.sidebarItems}>
                {sidebarItems.map(item => {
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

          {/* 底部固定区域 - 插件管理和回收站 */}
          <div style={{
            marginTop: 'auto',
            paddingTop: '15px'
          }}>
            {/* 插件管理（首页必备子插件；手稿 20260926 ②：置于原「扩展」位） */}
            <div style={{
              borderTop: 'var(--nt-border-width) solid rgba(0, 240, 255, 0.4)',
              borderBottom: 'var(--nt-border-width) solid rgba(0, 240, 255, 0.4)',
              marginBottom: '10px'
            }}>
              <div onClick={() => handleNavClick('/plugin-manager')} onKeyDown={e => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  handleNavClick('/plugin-manager');
                }
              }} className={`${styles.profileIcon} ${location.pathname.startsWith('/plugin-manager') ? styles.navItemActive : ''}`} role="button" tabIndex={0} aria-label="插件管理">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#00F0FF" strokeWidth="1.5">
                  <path d="M12 2L2 7L12 12L22 7L12 2Z" strokeLinecap="round" strokeLinejoin="round" />
                  <path d="M2 17L12 22L22 17" strokeLinecap="round" strokeLinejoin="round" />
                  <path d="M2 12L12 17L22 12" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
                <span style={{ fontSize: '13px' }}>插件管理</span>
              </div>
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
      
      {/* 智能悬浮球 */}
      <FloatingBall />
      {/* 选中文本快捷 AI 工具栏 */}
      <SelectionToolbar />
      {/* C4 §2.4.3 路由切换通知节点：屏幕阅读器播报路由变化（sr-only 视觉隐藏）*/}
      <div id="route-announcer" role="status" aria-live="polite" className="sr-only"></div>
    </div>
    </RecycleProvider>;
}