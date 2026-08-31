import { t } from "i18next";
import { useState, useEffect, useCallback, useRef } from 'react';
import { ipc, intelligence, knowledge } from '@/lib/ipc';
import { useIntelligence } from '@/hooks/useIntelligence';
import { random } from '@/lib/utils';
import styles from './Search.module.css';
interface GlobalSearchResult {
  module: string;
  id: string;
  title: string;
  preview: string;
  score: number;
  updated_at: string;
}
interface SearchEngine {
  id: string;
  name: string;
  icon: string;
  searchUrl: string;
}
interface Bookmark {
  id: string;
  label: string;
  icon: string;
  iconUrl: string;
  url: string;
}
interface GlobalSearchResult {
  module: string;
  id: string;
  title: string;
  preview: string;
  score: number;
  updated_at: string;
}
const MODULE_META: Record<string, {
  label: string;
  icon: string;
}> = {
  knowledge_base: {
    label: t("components.intelligence.ActivityPanel.k1"),
    icon: '📚'
  },
  conversations: {
    label: t("layout.k17"),
    icon: '💬'
  },
  notes: {
    label: t("Recycle.k3"),
    icon: '📝'
  },
  code_files: {
    label: t("Search.k1"),
    icon: '💻'
  },
  todos: {
    label: t("components.intelligence.ActivityPanel.k2"),
    icon: '✅'
  },
  journals: {
    label: t("components.intelligence.ActivityPanel.k3"),
    icon: '📅'
  }
};
const SEARCH_ENGINES: SearchEngine[] = [{
  id: 'bing',
  name: 'Bing',
  icon: 'B',
  searchUrl: 'https://www.bing.com/search?q='
}, {
  id: 'baidu',
  name: t("Search.k2"),
  icon: t("Search.k3"),
  searchUrl: 'https://www.baidu.com/s?wd='
}, {
  id: 'duckduckgo',
  name: 'DuckDuckGo',
  icon: 'D',
  searchUrl: 'https://duckduckgo.com/?q='
}];
const BOOKMARKS_KEY = 'nexterm_bookmarks';
function loadBookmarks(): Bookmark[] {
  try {
    const raw = localStorage.getItem(BOOKMARKS_KEY);
    if (raw) return JSON.parse(raw);
  } catch {/* ignore */}
  return [];
}
interface BrowserTab {
  id: string;
  url: string;
  displayUrl: string;
  title: string;
  loading: boolean;
  history: string[];
  historyIdx: number;
  childLabel: string | null;
}
function createTab(url?: string): BrowserTab {
  return {
    id: random.uid('tab-'),
    url: url || '',
    displayUrl: url || '',
    title: url ? new URL(url).hostname : t("Search.k4"),
    loading: false,
    history: url ? [url] : [],
    historyIdx: url ? 0 : -1,
    childLabel: null
  };
}
function isUrl(str: string): boolean {
  return /^(https?:\/\/|file:\/\/)/i.test(str.trim());
}
function looksLikeUrl(str: string): boolean {
  const trimmed = str.trim();
  if (isUrl(trimmed)) return true;
  if (/^[a-zA-Z0-9][-a-zA-Z0-9]*\.[a-zA-Z]{2,}/.test(trimmed)) return true;
  return false;
}
function normalizeUrl(input: string): string {
  const trimmed = input.trim();
  if (isUrl(trimmed)) return trimmed;
  if (/^[a-zA-Z0-9][-a-zA-Z0-9]*\.[a-zA-Z]{2,}/.test(trimmed)) return `https://${trimmed}`;
  return trimmed;
}
function getViewportRect(el: HTMLElement) {
  const r = el.getBoundingClientRect();
  return {
    x: r.x,
    y: r.y,
    width: r.width,
    height: r.height
  };
}
export default function Search() {
  const [tabs, setTabs] = useState<BrowserTab[]>([createTab()]);
  const [activeTabId, setActiveTabId] = useState(tabs[0].id);
  const [engine, setEngine] = useState<SearchEngine>(SEARCH_ENGINES[0]);
  const [showEngineMenu, setShowEngineMenu] = useState(false);
  const [showBookmarkDialog, setShowBookmarkDialog] = useState(false);
  const [bookmarkName, setBookmarkName] = useState('');
  const [userBookmarks, setUserBookmarks] = useState<Bookmark[]>(loadBookmarks);
  const [aiSearching, setAiSearching] = useState(false);
  const [aiSearchResult, setAiSearchResult] = useState<{
    query: string;
    correctedQuery: string | null;
    corrections: string[];
    suggestions: string[];
    relatedTerms: string[];
  } | null>(null);
  const {
    aiOn,
    featureOn
  } = useIntelligence();

  // 4.3 搜索结果一键归档知识库
  const [archivingIds, setArchivingIds] = useState<Set<string>>(new Set());
  const [archivedIds, setArchivedIds] = useState<Set<string>>(new Set());
  const handleArchiveToKb = async (r: GlobalSearchResult) => {
    const itemKey = `${r.module}-${r.id}`;
    if (archivedIds.has(itemKey) || archivingIds.has(itemKey)) return;
    setArchivingIds(prev => new Set(prev).add(itemKey));
    try {
      await knowledge.createItem({
        title: r.title,
        content: t("Search.k5", {
          title: r.title,
          module: r.module,
          preview: r.preview,
          arg0: new Date().toLocaleString('zh-CN')
        }),
        category_id: null,
        tags: [t("Search.k6")]
      });
      setArchivedIds(prev => new Set(prev).add(itemKey));
    } catch {
      // 静默失败
    } finally {
      setArchivingIds(prev => {
        const n = new Set(prev);
        n.delete(itemKey);
        return n;
      });
    }
  };

  // 全站搜索状态
  const [searchMode, setSearchMode] = useState<'browser' | 'global'>('browser');
  const [globalSearchQuery, setGlobalSearchQuery] = useState('');
  const [globalSearchResults, setGlobalSearchResults] = useState<GlobalSearchResult[]>([]);
  const [globalSearchLoading, setGlobalSearchLoading] = useState(false);
  const [globalSearchError, setGlobalSearchError] = useState('');
  const [aiSummary, setAiSummary] = useState('');
  const [aiSummaryLoading, setAiSummaryLoading] = useState(false);
  const [collapsedModules, setCollapsedModules] = useState<Set<string>>(new Set());
  const [, forceRender] = useState(0);
  const viewportRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const engineMenuRef = useRef<HTMLDivElement>(null);
  const activeViewsRef = useRef<Set<string>>(new Set());
  const tabsRef = useRef(tabs);
  tabsRef.current = tabs;
  const activeTab = tabs.find(t => t.id === activeTabId) || tabs[0];
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (engineMenuRef.current && !engineMenuRef.current.contains(e.target as Node)) {
        setShowEngineMenu(false);
      }
    }
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, []);
  useEffect(() => {
    function onNavigate(e: Event) {
      const targetUrl = (e as CustomEvent).detail;
      if (targetUrl) navigateTo(targetUrl);
    }
    window.addEventListener('search-navigate', onNavigate);
    return () => window.removeEventListener('search-navigate', onNavigate);
  }, [activeTabId]);
  useEffect(() => {
    function onSwitchEngine(e: Event) {
      const engineId = (e as CustomEvent).detail;
      const target = SEARCH_ENGINES.find(se => se.id === engineId);
      if (target) setEngine(target);
    }
    window.addEventListener('search-switch-engine', onSwitchEngine);
    return () => window.removeEventListener('search-switch-engine', onSwitchEngine);
  }, []);
  useEffect(() => {
    function onBookmarkChange() {
      setUserBookmarks(loadBookmarks());
    }
    window.addEventListener('bookmark-add', onBookmarkChange);
    window.addEventListener('bookmark-remove', onBookmarkChange);
    return () => {
      window.removeEventListener('bookmark-add', onBookmarkChange);
      window.removeEventListener('bookmark-remove', onBookmarkChange);
    };
  }, []);
  useEffect(() => {
    const tab = tabs.find(t => t.id === activeTabId);
    if (showBookmarkDialog && tab?.childLabel) {
      hideView(tab.childLabel);
    } else if (!showBookmarkDialog && tab?.childLabel) {
      showView(tab.childLabel);
    }
  }, [showBookmarkDialog]);
  useEffect(() => {
    ipc.invoke('browser_cleanup').then(res => {
      if (res.data && res.data > 0) {
        console.log(`[Browser] 清理了 ${res.data} 个残留 WebView`);
      }
    }).catch(() => {});
    return () => {
      activeViewsRef.current.forEach(label => {
        ipc.invoke('browser_close_view', {
          label
        }).catch(() => {});
        ipc.invoke('browser_resize_view', {
          label,
          x: -99999,
          y: -99999,
          width: 1,
          height: 1
        }).catch(() => {});
      });
      activeViewsRef.current.clear();
    };
  }, []);
  useEffect(() => {
    const el = viewportRef.current;
    if (!el) return;
    let rafId: number | null = null;
    const observer = new ResizeObserver(() => {
      if (rafId) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        const tab = tabs.find(t => t.id === activeTabId);
        if (!tab?.childLabel) return;
        const rect = getViewportRect(el);
        ipc.invoke('browser_resize_view', {
          label: tab.childLabel,
          x: rect.x,
          y: rect.y,
          width: rect.width,
          height: rect.height
        }).catch(() => {});
      });
    });
    observer.observe(el);
    return () => {
      observer.disconnect();
      if (rafId) cancelAnimationFrame(rafId);
    };
  }, [activeTabId, tabs]);
  const updateTab = useCallback((id: string, patch: Partial<BrowserTab>) => {
    setTabs(prev => prev.map(t => t.id === id ? {
      ...t,
      ...patch
    } : t));
  }, []);
  const closeChildView = async (label: string) => {
    try {
      await ipc.invoke('browser_close_view', {
        label
      });
    } catch {/* already closed */}
  };
  const hideView = async (label: string) => {
    try {
      await ipc.invoke('browser_resize_view', {
        label,
        x: -99999,
        y: -99999,
        width: 1,
        height: 1
      });
    } catch {/* ignore */}
  };
  const showView = async (label: string) => {
    if (!viewportRef.current) return;
    const rect = getViewportRect(viewportRef.current);
    try {
      await ipc.invoke('browser_resize_view', {
        label,
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height
      });
    } catch {/* ignore */}
  };
  const resolveUrl = (input: string): string => {
    const trimmed = input.trim();
    if (looksLikeUrl(trimmed)) return normalizeUrl(trimmed);
    return `${engine.searchUrl}${encodeURIComponent(trimmed)}`;
  };
  const navigateTo = useCallback(async (input: string, targetTabId?: string) => {
    const tid = targetTabId || activeTabId;
    const target = resolveUrl(input);
    const hostname = (() => {
      try {
        return new URL(target).hostname;
      } catch {
        return input;
      }
    })();
    updateTab(tid, {
      url: target,
      displayUrl: input,
      title: hostname,
      loading: true,
      history: [],
      historyIdx: -1
    });
    setTabs(prev => {
      return prev.map(t => {
        if (t.id !== tid) return t;
        const newHistory = [...t.history.slice(0, t.historyIdx + 1), target];
        return {
          ...t,
          history: newHistory,
          historyIdx: newHistory.length - 1
        };
      });
    });
    forceRender(n => n + 1);
    const tab = tabsRef.current.find(t => t.id === tid);
    if (!tab?.childLabel) {
      if (!viewportRef.current) return;
      const rect = getViewportRect(viewportRef.current);
      try {
        const result = await ipc.invoke<string>('browser_create_view', {
          url: target,
          x: rect.x,
          y: rect.y,
          width: rect.width,
          height: rect.height
        });
        if (result.data) {
          activeViewsRef.current.add(result.data);
          updateTab(tid, {
            childLabel: result.data
          });
          setTimeout(() => {
            if (!viewportRef.current) return;
            const r2 = getViewportRect(viewportRef.current);
            ipc.invoke('browser_resize_view', {
              label: result.data,
              x: r2.x,
              y: r2.y,
              width: r2.width,
              height: r2.height
            }).catch(() => {});
          }, 150);
        }
      } catch (e) {
        console.error('浏览器导航失败:', e);
      }
    } else {
      try {
        await showView(tab.childLabel);
        await ipc.invoke('browser_navigate_view', {
          label: tab.childLabel,
          url: target
        });
      } catch (e) {
        console.error('浏览器导航失败:', e);
      }
    }
    updateTab(tid, {
      loading: false
    });
  }, [activeTabId, engine]);
  const switchTab = async (tabId: string) => {
    if (tabId === activeTabId) return;
    const oldTab = tabs.find(t => t.id === activeTabId);
    const newTab = tabs.find(t => t.id === tabId);
    if (!newTab) return;
    if (oldTab?.childLabel) await hideView(oldTab.childLabel);
    if (newTab.childLabel) await showView(newTab.childLabel);
    setActiveTabId(tabId);
  };
  const openNewTab = () => {
    const nt = createTab();
    setTabs(prev => [...prev, nt]);
    setActiveTabId(nt.id);
  };
  const closeTab = async (tabId: string) => {
    const idx = tabs.findIndex(t => t.id === tabId);
    if (idx < 0) return;
    const tab = tabs[idx];
    if (tab.childLabel) {
      try {
        await ipc.invoke('browser_close_view', {
          label: tab.childLabel
        });
      } catch {/* ignore */}
      try {
        await hideView(tab.childLabel);
      } catch {/* fallback */}
      activeViewsRef.current.delete(tab.childLabel);
    }
    const remaining = tabs.filter((_, i) => i !== idx);
    if (remaining.length === 0) {
      const nt = createTab();
      setTabs([nt]);
      setActiveTabId(nt.id);
      return;
    }
    setTabs(remaining);
    if (activeTabId === tabId) {
      const nextIdx = Math.min(idx, remaining.length - 1);
      const nextTab = remaining[nextIdx];
      setActiveTabId(nextTab.id);
      if (nextTab.childLabel) showView(nextTab.childLabel);
    }
  };
  const handleGo = () => {
    const val = inputRef.current?.value || activeTab.displayUrl;
    if (!val.trim()) return;
    if (aiOn && featureOn('search_enhance') && !isUrl(val)) {
      handleAiSearch(val);
      return;
    }
    navigateTo(val);
  };

  // ========== 全站搜索 ==========

  const handleGlobalSearch = async () => {
    const q = globalSearchQuery.trim();
    if (!q) return;
    setGlobalSearchLoading(true);
    setGlobalSearchError('');
    setAiSummary('');
    setCollapsedModules(new Set());
    try {
      const res = await ipc.invoke<GlobalSearchResult[]>('global_search', {
        query: q,
        modules: null,
        limit: 100
      });
      if (res.data) {
        setGlobalSearchResults(res.data);
      } else {
        setGlobalSearchResults([]);
        setGlobalSearchError(res.message || t("Knowledge.k58"));
      }
    } catch (e: any) {
      setGlobalSearchResults([]);
      setGlobalSearchError(e?.message || String(e));
    } finally {
      setGlobalSearchLoading(false);
    }
  };
  const handleGenerateAiSummary = async () => {
    if (globalSearchResults.length === 0) return;
    setAiSummaryLoading(true);
    try {
      const res = await ipc.invoke<{
        summary: string;
      }>('search_ai_summary', {
        query: globalSearchQuery.trim(),
        results: globalSearchResults
      });
      if (res.data) {
        setAiSummary(res.data.summary);
      }
    } catch {
      // fallback handled by backend
    } finally {
      setAiSummaryLoading(false);
    }
  };
  const toggleModuleCollapse = (module: string) => {
    setCollapsedModules(prev => {
      const next = new Set(prev);
      if (next.has(module)) next.delete(module);else next.add(module);
      return next;
    });
  };
  const handleGlobalSearchKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') handleGlobalSearch();
  };

  // 按模块分组
  const groupedResults = (() => {
    const groups: Record<string, GlobalSearchResult[]> = {};
    for (const r of globalSearchResults) {
      if (!groups[r.module]) groups[r.module] = [];
      groups[r.module].push(r);
    }
    for (const g of Object.values(groups)) {
      g.sort((a, b) => b.score - a.score);
    }
    return groups;
  })();
  const handleAiSearch = async (query: string) => {
    if (aiSearching) return;
    setAiSearching(true);
    setAiSearchResult(null);
    try {
      const res = await intelligence.searchAnalyze(query.trim());
      if (res?.data) {
        setAiSearchResult({
          query: query.trim(),
          correctedQuery: res.data.corrected_query,
          corrections: res.data.corrections || [],
          suggestions: res.data.suggestions || [],
          relatedTerms: res.data.related_terms || []
        });
        // 有纠正结果时自动用纠错后的词搜索
        if (res.data.corrected_query) {
          navigateTo(res.data.corrected_query);
        } else {
          navigateTo(query);
        }
      } else {
        setAiSearchResult({
          query: query.trim(),
          correctedQuery: null,
          corrections: [],
          suggestions: [t("Search.k7")],
          relatedTerms: []
        });
        navigateTo(query);
      }
    } catch {
      setAiSearchResult({
        query: query.trim(),
        correctedQuery: null,
        corrections: [],
        suggestions: [t("Search.k7")],
        relatedTerms: []
      });
      navigateTo(query);
    } finally {
      setAiSearching(false);
    }
  };
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') handleGo();
  };
  const handleBack = async () => {
    const tab = tabsRef.current.find(t => t.id === activeTabId);
    if (!tab || tab.historyIdx <= 0) return;
    const newIdx = tab.historyIdx - 1;
    const prevUrl = tab.history[newIdx];
    updateTab(activeTabId, {
      url: prevUrl,
      displayUrl: prevUrl,
      historyIdx: newIdx,
      loading: true
    });
    if (tab.childLabel) {
      try {
        await ipc.invoke('browser_navigate_view', {
          label: tab.childLabel,
          url: prevUrl
        });
      } catch {/* ignore */}
    }
    updateTab(activeTabId, {
      loading: false
    });
  };
  const handleForward = async () => {
    const tab = tabsRef.current.find(t => t.id === activeTabId);
    if (!tab || tab.historyIdx >= tab.history.length - 1) return;
    const newIdx = tab.historyIdx + 1;
    const nextUrl = tab.history[newIdx];
    updateTab(activeTabId, {
      url: nextUrl,
      displayUrl: nextUrl,
      historyIdx: newIdx,
      loading: true
    });
    if (tab.childLabel) {
      try {
        await ipc.invoke('browser_navigate_view', {
          label: tab.childLabel,
          url: nextUrl
        });
      } catch {/* ignore */}
    }
    updateTab(activeTabId, {
      loading: false
    });
  };
  const handleRefresh = async () => {
    const tab = tabsRef.current.find(t => t.id === activeTabId);
    if (!tab || !tab.url) return;
    updateTab(activeTabId, {
      loading: true
    });
    if (tab.childLabel) await closeChildView(tab.childLabel);
    if (!viewportRef.current) {
      updateTab(activeTabId, {
        childLabel: null,
        loading: false
      });
      return;
    }
    const rect = getViewportRect(viewportRef.current);
    try {
      const result = await ipc.invoke<string>('browser_create_view', {
        url: tab.url,
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height
      });
      if (result.data) {
        activeViewsRef.current.add(result.data);
        updateTab(activeTabId, {
          childLabel: result.data
        });
      }
    } catch (e) {
      console.error('浏览器刷新失败:', e);
    }
    updateTab(activeTabId, {
      loading: false
    });
  };
  const handleHome = async () => {
    const tab = tabsRef.current.find(t => t.id === activeTabId);
    if (tab?.childLabel) {
      try {
        await ipc.invoke('browser_close_view', {
          label: tab.childLabel
        });
      } catch {/* ignore */}
      try {
        await hideView(tab.childLabel);
      } catch {/* fallback */}
      activeViewsRef.current.delete(tab.childLabel);
    }
    const nt = createTab();
    setTabs(prev => prev.map(t => t.id === activeTabId ? nt : t));
    setActiveTabId(nt.id);
    setTimeout(() => inputRef.current?.focus(), 0);
  };
  const handleOpenNative = async () => {
    if (!activeTab.url) return;
    try {
      await ipc.invoke('browser_open_window', {
        url: activeTab.url
      });
    } catch (e) {
      console.error('打开原生窗口失败:', e);
    }
  };
  const isBookmarked = userBookmarks.some(b => b.url === activeTab.url);
  const handleBookmarkToggle = () => {
    if (!activeTab.url) return;
    if (isBookmarked) {
      const next = userBookmarks.filter(b => b.url !== activeTab.url);
      localStorage.setItem(BOOKMARKS_KEY, JSON.stringify(next));
      setUserBookmarks(next);
      window.dispatchEvent(new CustomEvent('bookmark-remove'));
      return;
    }
    const hostname = (() => {
      try {
        return new URL(activeTab.url).hostname;
      } catch {
        return activeTab.url;
      }
    })();
    setBookmarkName(hostname);
    setShowBookmarkDialog(true);
  };
  const handleBookmarkConfirm = () => {
    if (!bookmarkName.trim() || !activeTab.url) return;
    const hostname = (() => {
      try {
        return new URL(activeTab.url).hostname;
      } catch {
        return activeTab.url;
      }
    })();
    const bookmark: Bookmark = {
      id: `bkm-${Date.now()}`,
      label: bookmarkName.trim(),
      icon: hostname.charAt(0).toUpperCase(),
      iconUrl: `https://www.google.com/s2/favicons?domain=${hostname}&sz=32`,
      url: activeTab.url
    };
    window.dispatchEvent(new CustomEvent('bookmark-add', {
      detail: bookmark
    }));
    setShowBookmarkDialog(false);
  };
  const canGoBack = activeTab.historyIdx > 0;
  const canGoForward = activeTab.historyIdx < activeTab.history.length - 1;
  return <div className={styles.page}>
      {/* 模式切换 */}
      <div className={styles.modeBar}>
        <button className={`${styles.modeBtn} ${searchMode === 'browser' ? styles.modeBtnActive : ''}`} onClick={() => setSearchMode('browser')}>
          {t("Search.k8")}
        </button>
        <button className={`${styles.modeBtn} ${searchMode === 'global' ? styles.modeBtnActive : ''}`} onClick={() => setSearchMode('global')}>
          {t("Search.k9")}
        </button>
      </div>

      {searchMode === 'browser' ? <>
          <div className={styles.toolbar}>
            <div className={styles.navBtns}>
              <button className={styles.navBtn} onClick={handleBack} disabled={!canGoBack} title={t("Search.k10")}>◀</button>
              <button className={styles.navBtn} onClick={handleForward} disabled={!canGoForward} title={t("Search.k11")}>▶</button>
              <button className={`${styles.navBtn} ${activeTab.loading ? styles.navBtnLoading : ''}`} onClick={handleRefresh} title={t("common.refresh")}>
                {activeTab.loading ? '⟳' : '↻'}
              </button>
              <button className={styles.navBtn} onClick={handleHome} title={t("Search.k12")}>⌂</button>
            </div>

            <div className={styles.urlBar}>
              <div className={styles.engineSelector} ref={engineMenuRef}>
                <button className={styles.engineBtn} onClick={() => setShowEngineMenu(!showEngineMenu)} title={engine.name}>
                  <span className={styles.engineIcon}>{engine.icon}</span>
                </button>
                {showEngineMenu && <div className={styles.engineDropdown}>
                    {SEARCH_ENGINES.map(e => <button key={e.id} className={`${styles.engineOption} ${engine.id === e.id ? styles.engineOptionActive : ''}`} onClick={() => {
                setEngine(e);
                setShowEngineMenu(false);
              }}>
                        <span className={styles.engineOptionIcon}>{e.icon}</span><span>{e.name}</span>
                      </button>)}
                  </div>}
              </div>

              <input ref={inputRef} className={styles.urlInput} value={activeTab.displayUrl} onChange={e => updateTab(activeTabId, {
            displayUrl: e.target.value
          })} onKeyDown={handleKeyDown} placeholder={activeTab.url ? activeTab.url : t("Search.k13")} spellCheck={false} />

              {activeTab.displayUrl && <button className={styles.clearBtn} onClick={() => {
            updateTab(activeTabId, {
              displayUrl: ''
            });
            inputRef.current?.focus();
          }} title={t("Search.k14")}>✕</button>}
              {activeTab.url && <button className={`${styles.clearBtn} ${styles.bookmarkBtn} ${isBookmarked ? styles.bookmarked : ''}`} onClick={handleBookmarkToggle} title={isBookmarked ? t("components.FloatingBall.k56") : t("Search.k15")}>
                  {isBookmarked ? '★' : '☆'}
                </button>}
            </div>

            <div className={styles.actionBtns}>
              <button className={styles.nativeBtn} onClick={handleOpenNative} disabled={!activeTab.url} title={t("Search.k16")}>🗗</button>
            </div>
          </div>

          {aiSearchResult && <div className={styles.aiSearchPanel}>
              <div className={styles.aiSearchPanelHeader}>
                <span>{t("Search.k17")} {aiSearchResult.query}</span>
                <button onClick={() => setAiSearchResult(null)} className={styles.aiSearchPanelClose}>✕</button>
              </div>
              <div className={styles.aiSearchPanelContent}>
                {aiSearchResult.corrections.length > 0 && <div className={styles.aiSection}>
                    <div className={styles.aiSectionTitle}>{t("Search.k18")}</div>
                    {aiSearchResult.corrections.map((c, i) => <div key={i} className={styles.aiCorrectionItem}>{c}</div>)}
                    {aiSearchResult.correctedQuery && <button className={styles.aiActionBtn} onClick={() => {
              inputRef.current!.value = aiSearchResult.correctedQuery!;
              navigateTo(aiSearchResult.correctedQuery!);
            }}>
                        {t("Search.k19")} {aiSearchResult.correctedQuery}
                      </button>}
                  </div>}
                {aiSearchResult.relatedTerms.length > 0 && <div className={styles.aiSection}>
                    <div className={styles.aiSectionTitle}>{t("Search.k20")}</div>
                    <div className={styles.aiRelatedTags}>
                      {aiSearchResult.relatedTerms.map((term, i) => <button key={i} className={styles.aiTag} onClick={() => {
                inputRef.current!.value = term;
                handleGo();
              }}>
                          {term}
                        </button>)}
                    </div>
                  </div>}
                {aiSearchResult.suggestions.length > 0 && <div className={styles.aiSection}>
                    <div className={styles.aiSectionTitle}>{t("components.FloatingBall.k54")}</div>
                    {aiSearchResult.suggestions.map((s, i) => <div key={i} className={styles.aiSuggestionItem}>{s}</div>)}
                  </div>}
              </div>
            </div>}

          {tabs.length > 0 && <div className={styles.tabBar}>
              <div className={styles.tabList}>
                {tabs.map(tab => <div key={tab.id} className={`${styles.tab} ${tab.id === activeTabId ? styles.tabActive : ''}`} onClick={() => switchTab(tab.id)}>
                    <span className={styles.tabTitle}>{tab.title}</span>
                    {tab.loading && <span className={styles.tabSpinner}>⟳</span>}
                    {tabs.length > 1 && <button className={styles.tabClose} onClick={e => {
              e.stopPropagation();
              closeTab(tab.id);
            }} title={t("Search.k21")}>✕</button>}
                  </div>)}
              </div>
              <button className={styles.tabNew} onClick={openNewTab} title={t("Search.k22")}>+</button>
            </div>}

          <div className={styles.viewport} ref={viewportRef}>
            {activeTab.loading && <div className={styles.loadingBar}>
                <div className={styles.loadingTrack}><div className={styles.loadingFill} /></div>
              </div>}

            {!activeTab.url && <div className={styles.startup}>
                <div className={styles.startupLogo}>
                  <span className={styles.startupLogoIcon}>N</span>
                  <span className={styles.startupLogoText}>{t("Search.k23")}</span>
                </div>

                <div className={styles.searchArea}>
                  <div className={styles.searchEngineRow}>
                    {SEARCH_ENGINES.map(e => <button key={e.id} className={`${styles.searchEngineChip} ${engine.id === e.id ? styles.searchEngineChipActive : ''}`} onClick={() => setEngine(e)}>
                        <span className={styles.searchEngineChipIcon}>{e.icon}</span>
                        <span>{e.name}</span>
                      </button>)}
                  </div>
                </div>

                <div className={styles.hint}>{t("Search.k24")}</div>
              </div>}
          </div>

          {activeTab.url && <div className={styles.statusBar}>
              <span className={styles.statusEngine}>{engine.name}</span>
              <span className={styles.statusUrl}>{activeTab.url}</span>
              {activeTab.childLabel && <span className={styles.statusNative}>● WebView2</span>}
            </div>}

          {showBookmarkDialog && <div className={styles.bookmarkOverlay} onClick={() => setShowBookmarkDialog(false)}>
              <div className={styles.bookmarkDialog} onClick={e => e.stopPropagation()}>
                <div className={styles.bookmarkDialogTitle}>{t("Search.k25")}</div>
                <div className={styles.bookmarkUrlHint}>{activeTab.url}</div>
                <input className={styles.bookmarkInput} value={bookmarkName} onChange={e => setBookmarkName(e.target.value)} onKeyDown={e => {
            if (e.key === 'Enter') handleBookmarkConfirm();
          }} placeholder={t("Search.k26")} autoFocus />
                <div className={styles.bookmarkActions}>
                  <button className={styles.bookmarkCancelBtn} onClick={() => setShowBookmarkDialog(false)}>{t("common.cancel")}</button>
                  <button className={styles.bookmarkConfirmBtn} onClick={handleBookmarkConfirm} disabled={!bookmarkName.trim()}>{t("components.FloatingBall.k57")}</button>
                </div>
              </div>
            </div>}
        </> : (/* ========== 全站搜索模式 ========== */
    <div className={styles.globalSearchContainer}>
          <div className={styles.globalSearchBar}>
            <input className={styles.globalSearchInput} value={globalSearchQuery} onChange={e => setGlobalSearchQuery(e.target.value)} onKeyDown={handleGlobalSearchKeyDown} placeholder={t("Search.k27")} spellCheck={false} autoFocus />
            <button className={styles.globalSearchBtn} onClick={handleGlobalSearch} disabled={globalSearchLoading || !globalSearchQuery.trim()}>
              {globalSearchLoading ? '⟳' : t("common.search")}
            </button>
          </div>

          {globalSearchError && <div className={styles.globalSearchError}>{globalSearchError}</div>}

          {/* AI 综合摘要卡片 */}
          {globalSearchResults.length > 0 && <div className={styles.aiSummaryCard}>
              <div className={styles.aiSummaryHeader}>
                <span className={styles.aiSummaryTitle}>{t("Search.k28")}</span>
                {!aiSummary && !aiSummaryLoading && <button className={styles.aiSummaryBtn} onClick={handleGenerateAiSummary}>
                    {t("Search.k29")}
                  </button>}
                {aiSummaryLoading && <span className={styles.aiSummaryLoading}>{t("components.intelligence.DashboardPanel.k67")}</span>}
              </div>
              {aiSummary && <div className={styles.aiSummaryContent}>
                  <span className={styles.aiSummaryPrompt}>$ </span>
                  {aiSummary}
                </div>}
            </div>}

          {/* 搜索结果分组 */}
          {globalSearchLoading && !globalSearchResults.length && <div className={styles.globalSearchLoading}>{t("ai.ConversationList.k4")}</div>}

          <div className={styles.globalSearchResults}>
            {Object.keys(groupedResults).length === 0 && !globalSearchLoading && globalSearchQuery && <div className={styles.globalSearchEmpty}>
                <span className={styles.globalSearchEmptyIcon}>∅</span>
                <span>{t("Search.k30")}</span>
              </div>}

            {Object.entries(groupedResults).map(([module, results]) => {
          const meta = MODULE_META[module] || {
            label: module,
            icon: '📄'
          };
          const isCollapsed = collapsedModules.has(module);
          return <div key={module} className={styles.moduleGroup}>
                  <div className={styles.moduleHeader} onClick={() => toggleModuleCollapse(module)}>
                    <span className={styles.moduleCaret}>{isCollapsed ? '▶' : '▼'}</span>
                    <span className={styles.moduleIcon}>{meta.icon}</span>
                    <span className={styles.moduleLabel}>{meta.label}</span>
                    <span className={styles.moduleCount}>({results.length})</span>
                  </div>
                  {!isCollapsed && <div className={styles.moduleResults}>
                      {results.map((r, idx) => {
                const itemKey = `${r.module}-${r.id}`;
                const isArchived = archivedIds.has(itemKey);
                const isArchiving = archivingIds.has(itemKey);
                return <div key={`${r.module}-${r.id}-${idx}`} className={styles.resultItem}>
                          <div className={styles.resultTitle}>
                            <span className={styles.resultScore}>[{r.score.toFixed(2)}]</span>
                            {r.title}
                          </div>
                          <div className={styles.resultPreview}>{r.preview}</div>
                          <div className={styles.resultFooter}>
                            {r.updated_at && <span className={styles.resultMeta}>{r.updated_at}</span>}
                            {r.module !== 'knowledge_base' && <button className={styles.archiveBtn} onClick={() => handleArchiveToKb(r)} disabled={isArchived || isArchiving} title={isArchived ? t("Search.k31") : t("Search.k32")}>
                                {isArchiving ? '⏳' : isArchived ? t("Search.k33") : t("Search.k34")}
                              </button>}
                          </div>
                        </div>;
              })}
                    </div>}
                </div>;
        })}
          </div>
        </div>)}
    </div>;
}