import { t } from "i18next";
import { useState, useEffect, useCallback, useRef } from 'react';
import { intelligence } from '@/lib/ipc';
// 阶段3 批次3c：browser 域命令已迁 terminal.yuancode 插件（yc 命名空间）
import { yc } from '@/plugins/boards/terminal/features/yuancode/ipc';
import { useIntelligence } from '@/hooks/useIntelligence';
import { random } from '@/lib/utils';
import styles from '../../routes/Search.module.css';
import { removeBookmark, useBookmarks } from '../bookmarks/bookmarks';
import BookmarkDialog from '../bookmarks/BookmarkDialog';
interface SearchEngine {
  id: string;
  name: string;
  icon: string;
  searchUrl: string;
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
interface Props {
  bookmarksEnabled: boolean;
}
/** search.browser 域：多标签 / WebView 视口 / 前进后退刷新 / 主页 / 引擎选择 / AI 检索增强 */
export default function BrowserSearch({
  bookmarksEnabled
}: Props) {
  const [tabs, setTabs] = useState<BrowserTab[]>([createTab()]);
  const [activeTabId, setActiveTabId] = useState(tabs[0].id);
  const [engine, setEngine] = useState<SearchEngine>(SEARCH_ENGINES[0]);
  const [showEngineMenu, setShowEngineMenu] = useState(false);
  const [showBookmarkDialog, setShowBookmarkDialog] = useState(false);
  const userBookmarks = useBookmarks();
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
    const tab = tabs.find(t => t.id === activeTabId);
    if (showBookmarkDialog && tab?.childLabel) {
      hideView(tab.childLabel);
    } else if (!showBookmarkDialog && tab?.childLabel) {
      showView(tab.childLabel);
    }
  }, [showBookmarkDialog]);
  useEffect(() => {
    yc.browserCleanup().then(res => {
      if (res.data && res.data > 0) {
        console.log(`[Browser] 清理了 ${res.data} 个残留 WebView`);
      }
    }).catch(() => {});
    return () => {
      activeViewsRef.current.forEach(label => {
        yc.browserCloseView({
          label
        }).catch(() => {});
        yc.browserResizeView({
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
        yc.browserResizeView({
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
      await yc.browserCloseView({
        label
      });
    } catch {/* already closed */}
  };
  const hideView = async (label: string) => {
    try {
      await yc.browserResizeView({
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
      await yc.browserResizeView({
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
        const result = await yc.browserCreateView<string>({
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
            if (!viewportRef.current || !result.data) return;
            const r2 = getViewportRect(viewportRef.current);
            yc.browserResizeView({
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
        await yc.browserNavigateView({
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
        await yc.browserCloseView({
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
        await yc.browserNavigateView({
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
        await yc.browserNavigateView({
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
      const result = await yc.browserCreateView<string>({
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
        await yc.browserCloseView({
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
      await yc.browserOpenWindow({
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
      removeBookmark(activeTab.url);
      return;
    }
    setShowBookmarkDialog(true);
  };
  const canGoBack = activeTab.historyIdx > 0;
  const canGoForward = activeTab.historyIdx < activeTab.history.length - 1;
  return <>
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
          {bookmarksEnabled && activeTab.url && <button className={`${styles.clearBtn} ${styles.bookmarkBtn} ${isBookmarked ? styles.bookmarked : ''}`} onClick={handleBookmarkToggle} title={isBookmarked ? t("components.FloatingBall.k56") : t("Search.k15")}>
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

      {bookmarksEnabled && showBookmarkDialog && <BookmarkDialog url={activeTab.url} onClose={() => setShowBookmarkDialog(false)} />}
    </>;
}
