import { t } from "i18next";
import { useState, useEffect, useRef, useCallback } from 'react';
import { home, system, intelligence } from '@/lib/ipc';
import { useIntelligence } from '@/hooks/useIntelligence';
import { useNetworkStatus } from '@/hooks/useNetworkStatus';
import { useFloatingOrbStore } from '@/stores/floatingOrbStore';
import { SkeletonList } from '@/components/ui/Skeleton';
import type { NewsItem, RefreshProgress, FetchStage } from './types';
import { getCategoryLabel, isGithubItem } from './utils';
import styles from '../Home.module.css';
const fetchStages: FetchStage[] = [{
  timeRange: [0, 3],
  percent: [0, 8],
  stage: t("home.NewsPanel.k1"),
  detail: t("home.NewsPanel.k2")
}, {
  timeRange: [3, 30],
  percent: [8, 65],
  stage: t("home.NewsPanel.k3"),
  detail: t("home.NewsPanel.k4")
}, {
  timeRange: [30, 35],
  percent: [65, 78],
  stage: t("home.NewsPanel.k5"),
  detail: t("home.NewsPanel.k6")
}, {
  timeRange: [35, 42],
  percent: [78, 90],
  stage: t("home.NewsPanel.k7"),
  detail: t("home.NewsPanel.k8")
}, {
  timeRange: [42, Infinity],
  percent: [90, 90],
  stage: t("home.NewsPanel.k9"),
  detail: t("home.NewsPanel.k10")
}];
function getProgressStage(elapsedSeconds: number): {
  stage: FetchStage;
  percent: number;
} {
  for (let i = fetchStages.length - 1; i >= 0; i--) {
    if (elapsedSeconds >= fetchStages[i].timeRange[0]) {
      const stage = fetchStages[i];
      const [minT, maxT] = stage.timeRange;
      const [minP, maxP] = stage.percent;
      let percent: number;
      if (maxT === Infinity) {
        percent = maxP;
      } else {
        const ratio = Math.min(1, (elapsedSeconds - minT) / (maxT - minT));
        percent = minP + ratio * (maxP - minP);
      }
      return {
        stage,
        percent: Math.round(percent)
      };
    }
  }
  return {
    stage: fetchStages[0],
    percent: 0
  };
}
export default function NewsPanel() {
  const {
    aiOn,
    featureOn,
    llmConfigured
  } = useIntelligence();
  const { isOnline } = useNetworkStatus();
  const [news, setNews] = useState<NewsItem[]>([]);
  const [loadingNews, setLoadingNews] = useState(false);
  const [newsCategory, setNewsCategory] = useState<string>('all');
  const [expandedNewsId, setExpandedNewsId] = useState<number | null>(null);
  const [aiSummarizing, setAiSummarizing] = useState<number | null>(null);
  const [aiSummary, setAiSummary] = useState<Record<number, string>>({});
  const [refreshingNews, setRefreshingNews] = useState(false);
  const [lastRefreshResult, setLastRefreshResult] = useState<string | null>(null);
  const [refreshProgress, setRefreshProgress] = useState<RefreshProgress | null>(null);
  // A5 Phase 3 Task 3: 离线缓存状态（null=在线/未知，number=缓存年龄分钟）
  const [cacheAgeMinutes, setCacheAgeMinutes] = useState<number | null>(null);
  const batchQueueRef = useRef<NewsItem[]>([]);
  const batchProcessingRef = useRef<boolean>(false);
  const prevFeatureOnRef = useRef<boolean>(false);
  const batchInitializedRef = useRef<boolean>(false);

  // 离线缓存场景下，将 NewsCacheItem（id=string）映射为 NewsItem（id=number）
  // 使用字符串哈希生成稳定数字 ID，保证 React key 稳定
  const hashStringToId = (s: string): number => {
    let h = 0;
    for (let i = 0; i < s.length; i++) {
      h = ((h << 5) - h + s.charCodeAt(i)) | 0;
    }
    return Math.abs(h);
  };

  const loadNews = useCallback(async (category?: string) => {
    setLoadingNews(true);
    try {
      if (!isOnline) {
        // 离线模式：从 news_offline_cache 读取缓存快照
        const [cachedRes, statusRes] = await Promise.all([
          home.newsGetCached(),
          home.newsCacheStatus(),
        ]);
        if (cachedRes.code === 0 && cachedRes.data) {
          const cachedData = cachedRes.data as any[];
          setNews(cachedData.map((item: any) => ({
            id: hashStringToId(item.id),
            title: item.title || '',
            url: item.url || undefined,
            source: item.source || undefined,
            summary: item.content || undefined,
            content: item.content || undefined,
            category: item.source || undefined,
            published_at: item.published_at || undefined,
            fetched_at: 0,
            is_read: false,
            is_favorite: false,
            ai_summary: undefined,
          })));
        } else {
          setNews([]);
        }
        if (statusRes.code === 0 && statusRes.data) {
          setCacheAgeMinutes(statusRes.data.age_minutes ?? null);
        } else {
          setCacheAgeMinutes(null);
        }
      } else {
        // 在线模式：正常拉取 + 清除缓存年龄标识
        setCacheAgeMinutes(null);
        const result = category && category !== 'all' ? await home.getNewsByCategory(category) : await home.getNews();
        if (result.code === 0 && result.data) {
          const newsData = result.data as any[];
          setNews(newsData.map((item: any) => ({
            id: item.id,
            title: item.title,
            url: item.url,
            source: item.source,
            summary: item.summary,
            content: item.content,
            category: item.category,
            published_at: item.published_at,
            fetched_at: item.fetched_at,
            is_read: item.is_read || false,
            is_favorite: item.is_favorite || false,
            ai_summary: item.ai_summary || undefined
          })));
        }
      }
    } catch (error) {
      console.error('加载新闻失败:', error);
    } finally {
      setLoadingNews(false);
    }
  }, [isOnline]);
  const handleAiSummarizeNews = useCallback(async (item: NewsItem, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    setAiSummarizing(item.id);
    try {
      const github = isGithubItem(item);
      if (github) {
        const res = await home.generateNewsAiSummary(item.id);
        if (res?.code === 0 && res?.data?.ai_summary) {
          const summary = res.data.ai_summary;
          setAiSummary(prev => ({
            ...prev,
            [item.id]: summary
          }));
          const {
            addOrb
          } = useFloatingOrbStore.getState();
          addOrb({
            type: 'summary',
            title: t("home.NewsPanel.k11", {
              arg0: item.title.slice(0, 20),
              arg1: item.title.length > 20 ? '...' : ''
            }),
            content: summary,
            source: 'home',
            sourceId: item.id,
            icon: '🐙',
            color: '#6e5494'
          });
        } else {
          setAiSummary(prev => ({
            ...prev,
            [item.id]: res?.message || t("home.NewsPanel.k12")
          }));
        }
      } else {
        const res = await intelligence.newsSummarize(item.title, item.summary || item.content || '');
        if (res?.data) {
          const {
            addOrb
          } = useFloatingOrbStore.getState();
          addOrb({
            type: 'summary',
            title: t("home.NewsPanel.k13", {
              arg0: item.title.slice(0, 20),
              arg1: item.title.length > 20 ? '...' : ''
            }),
            content: res.data.summary,
            source: 'home',
            sourceId: item.id,
            icon: '📰',
            color: '#7FD962'
          });
        } else {
          setAiSummary(prev => ({
            ...prev,
            [item.id]: res?.message || t("home.NewsPanel.k14")
          }));
        }
      }
    } catch {
      setAiSummary(prev => ({
        ...prev,
        [item.id]: t("home.NewsPanel.k15")
      }));
    } finally {
      setAiSummarizing(null);
    }
  }, []);
  const processBatchQueue = useCallback(async () => {
    if (batchProcessingRef.current) return;
    if (batchQueueRef.current.length === 0) return;
    batchProcessingRef.current = true;
    while (batchQueueRef.current.length > 0) {
      const item = batchQueueRef.current[0];
      if (item.ai_summary || aiSummary[item.id]) {
        batchQueueRef.current.shift();
        continue;
      }
      setAiSummarizing(item.id);
      try {
        await handleAiSummarizeNews(item);
        batchQueueRef.current.shift();
      } catch {
        batchQueueRef.current.shift();
        setAiSummarizing(null);
      }
      await new Promise(r => setTimeout(r, 300));
    }
    batchProcessingRef.current = false;
  }, [aiSummary, handleAiSummarizeNews]);
  useEffect(() => {
    const isOn = aiOn && featureOn('news_summary');
    const wasOff = !prevFeatureOnRef.current && isOn;
    prevFeatureOnRef.current = isOn;
    if (!isOn || news.length === 0) return;
    const needSummary = news.filter(item => !item.ai_summary && !aiSummary[item.id] && aiSummarizing !== item.id);
    if (needSummary.length === 0) return;
    if (wasOff || !batchInitializedRef.current) {
      const normalNews = needSummary.filter(item => !isGithubItem(item));
      const githubNews = needSummary.filter(item => isGithubItem(item));
      batchQueueRef.current = [...normalNews, ...githubNews];
      batchInitializedRef.current = true;
      processBatchQueue();
    } else {
      for (const item of needSummary) {
        if (!batchQueueRef.current.find(q => q.id === item.id)) {
          if (isGithubItem(item)) {
            const lastNormalIdx = batchQueueRef.current.map(q => !isGithubItem(q)).lastIndexOf(true);
            batchQueueRef.current.splice(lastNormalIdx + 1, 0, item);
          } else {
            batchQueueRef.current.push(item);
          }
        }
      }
      if (!batchProcessingRef.current) {
        processBatchQueue();
      }
    }
  }, [news, aiOn]);
  const refreshNews = async () => {
    if (refreshingNews) return;
    setRefreshingNews(true);
    setLastRefreshResult(null);
    const startTime = Date.now();
    let progressInterval: ReturnType<typeof setInterval> | null = null;
    let isCompleted = false;
    progressInterval = setInterval(() => {
      if (isCompleted) return;
      const elapsed = (Date.now() - startTime) / 1000;
      const {
        stage,
        percent
      } = getProgressStage(elapsed);
      setRefreshProgress({
        stage: stage.stage,
        percent,
        elapsed: Math.round(elapsed),
        isWarning: elapsed > 50
      });
    }, 500);
    try {
      const result = await Promise.race([home.fetchNews(), new Promise<never>((_, reject) => setTimeout(() => reject(new Error(t("home.NewsPanel.k16"))), 90000))]);
      isCompleted = true;
      if (progressInterval) clearInterval(progressInterval);
      const totalElapsed = ((Date.now() - startTime) / 1000).toFixed(1);
      setRefreshProgress({
        stage: t("home.NewsPanel.k17"),
        percent: 98,
        elapsed: Math.round(parseFloat(totalElapsed)),
        isWarning: false
      });
      await loadNews(newsCategory);
      setRefreshProgress({
        stage: t("common.finish"),
        percent: 100,
        elapsed: Math.round(parseFloat(totalElapsed)),
        isWarning: false
      });
      if (result.code === 0) {
        const data = result.data as unknown as {
          deleted: number;
          inserted: number;
          net_change: number;
          total: number;
        };
        const {
          deleted,
          inserted,
          net_change,
          total
        } = data;
        if (deleted > 0 && inserted > 0) {
          setLastRefreshResult(t("home.NewsPanel.k18", {
            deleted: deleted,
            inserted: inserted,
            arg0: net_change >= 0 ? '+' : '',
            net_change: net_change,
            totalElapsed: totalElapsed
          }));
        } else if (deleted > 0) {
          setLastRefreshResult(t("home.NewsPanel.k19", {
            deleted: deleted,
            total: total
          }));
        } else if (inserted > 0) {
          setLastRefreshResult(t("home.NewsPanel.k20", {
            inserted: inserted,
            totalElapsed: totalElapsed
          }));
        } else {
          setLastRefreshResult(t("home.NewsPanel.k21", {
            total: total
          }));
        }
      } else {
        setLastRefreshResult(`❌ ${result.message}`);
      }
    } catch (error) {
      isCompleted = true;
      if (progressInterval) clearInterval(progressInterval);
      const totalElapsed = ((Date.now() - startTime) / 1000).toFixed(1);
      console.error('刷新新闻失败:', error);
      setLastRefreshResult(t("home.NewsPanel.k22", {
        totalElapsed: totalElapsed
      }));
      setRefreshProgress(null);
    } finally {
      setTimeout(() => {
        setRefreshingNews(false);
        setRefreshProgress(null);
      }, 1200);
      setTimeout(() => setLastRefreshResult(null), 5000);
    }
  };
  const toggleFavorite = async (id: number, currentFavorite: boolean, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    try {
      await home.toggleNewsFavorite(id, !currentFavorite);
      setNews(prev => prev.map(item => item.id === id ? {
        ...item,
        is_favorite: !currentFavorite
      } : item));
    } catch (error) {
      console.error('收藏操作失败:', error);
    }
  };
  useEffect(() => {
    loadNews();
  }, [loadNews]);
  return <div className={styles.subpage}>
      <div className={styles.newsToolbar}>
        <div className={styles.newsCategoryTabs}>
          {['all', 'security', 'ai', 'programming', 'github'].map(cat => <button key={cat} className={`${styles.newsCategoryTab} ${newsCategory === cat ? styles.newsCategoryTabActive : ''}`} onClick={() => {
          setNewsCategory(cat);
          loadNews(cat === 'all' ? undefined : cat);
        }}>
              {cat === 'all' ? t("common.all") : cat === 'security' ? t("home.NewsPanel.k23") : cat === 'ai' ? '🤖 AI' : cat === 'programming' ? t("home.NewsPanel.k24") : '🐙 GitHub'}
            </button>)}
        </div>
        <button className={styles.newsRefreshBtn} onClick={refreshNews} disabled={refreshingNews || !isOnline} title={!isOnline ? t("home.NewsPanel.k25") + "（离线不可用）" : t("home.NewsPanel.k25")} style={!isOnline ? { opacity: 0.4, cursor: 'not-allowed' } : undefined}>
          {refreshingNews ? '⏳' : '🔄'}
        </button>
        {!isOnline && <span style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: 4,
          padding: '3px 8px',
          background: 'rgba(255, 165, 0, 0.08)',
          border: '1px solid rgba(255, 165, 0, 0.3)',
          borderRadius: 4,
          fontSize: 11,
          color: '#FFA500',
          verticalAlign: 'middle',
          fontFamily: 'Consolas, monospace',
          whiteSpace: 'nowrap',
        }} title="离线模式 — 显示缓存内容">
          📴 {cacheAgeMinutes !== null ? t("home.NewsPanel.k34", { arg0: cacheAgeMinutes }) : t("home.NewsPanel.k35")}
        </span>}
        {refreshProgress && <div style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 8,
        padding: '4px 10px',
        background: refreshProgress.isWarning ? 'rgba(255, 0, 110, 0.08)' : 'rgba(10, 0, 20, 0.95)',
        border: `1px solid ${refreshProgress.isWarning ? 'rgba(255, 0, 110, 0.3)' : 'rgba(0, 240, 255, 0.25)'}`,
        borderRadius: 4,
        fontSize: 11,
        fontFamily: 'Consolas, monospace',
        color: refreshProgress.isWarning ? '#FF006E' : '#00F0FF',
        verticalAlign: 'middle'
      }}>
            <div style={{
          width: 12,
          height: 12,
          borderRadius: '50%',
          border: `2px solid ${refreshProgress.isWarning ? '#FF006E' : '#00F0FF'}`,
          borderTopColor: 'transparent',
          animation: 'spin 1s linear infinite',
          flexShrink: 0
        }} />
            <span style={{
          fontWeight: 600,
          minWidth: 32
        }}>{refreshProgress.stage}</span>
            <div style={{
          width: 60,
          height: 4,
          background: 'rgba(10, 0, 20, 0.8)',
          borderRadius: 2,
          overflow: 'hidden',
          position: 'relative'
        }}>
              <div style={{
            width: `${refreshProgress.percent}%`,
            height: '100%',
            background: refreshProgress.isWarning ? '#FF006E' : '#00F0FF',
            borderRadius: 2,
            transition: 'width 0.3s ease',
            boxShadow: `0 0 6px ${refreshProgress.isWarning ? 'rgba(255, 0, 110, 0.5)' : 'rgba(0, 240, 255, 0.5)'}`
          }} />
            </div>
            <span style={{
          color: refreshProgress.isWarning ? '#FF6699' : '#00F0FF',
          fontWeight: 700,
          fontFamily: 'Courier New',
          fontSize: 12
        }}>{refreshProgress.percent}%</span>
            <span style={{
          color: '#6a6a8a',
          fontSize: 10
        }}>{refreshProgress.elapsed}s</span>
            {refreshProgress.isWarning && <span style={{
          color: '#FFD700',
          fontSize: 10
        }}>⚠️</span>}
          </div>}
        {!refreshProgress && lastRefreshResult && <span style={{
        fontSize: 11,
        color: lastRefreshResult.includes('✅') ? '#00F0FF' : lastRefreshResult.includes('ℹ️') ? '#B026FF' : '#FF006E',
        marginLeft: 6,
        verticalAlign: 'middle'
      }}>
            {lastRefreshResult}
          </span>}
      </div>
      {loadingNews ? <SkeletonList count={6} /> : news.length === 0 ? <div style={{
      textAlign: 'center',
      padding: '40px',
      color: '#6a6a8a'
    }}>
          <div style={{
        fontSize: 18,
        marginBottom: 10
      }}>📰</div>
          <div style={{
        marginBottom: 8
      }}>{t("home.NewsPanel.k26")}</div>
          <div style={{
        fontSize: 12,
        opacity: 0.7
      }}>{t("home.NewsPanel.k27")}</div>
        </div> : <div className={styles.newsList}>
          {news.map(item => {
        const catInfo = getCategoryLabel(item.category);
        const isExpanded = expandedNewsId === item.id;
        return <div key={item.id} className={`${styles.newsItem} ${isExpanded ? styles.newsItemExpanded : ''}`}>
                <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6
          }}>
                  <div className={styles.newsTitle} style={{
              cursor: 'pointer',
              flex: '1 1 0%',
              minWidth: 0
            }} onClick={() => setExpandedNewsId(isExpanded ? null : item.id)}>
                    {item.title}
                  </div>
                  {item.category && <span style={{
              fontSize: 10,
              color: catInfo.color,
              border: `1px solid ${catInfo.color}`,
              borderRadius: 2,
              padding: '0 3px',
              lineHeight: '14px',
              letterSpacing: 0.5,
              whiteSpace: 'nowrap',
              userSelect: 'none',
              flexShrink: 0
            }}>
                      {catInfo.text}
                    </span>}
                  <span onClick={e => toggleFavorite(item.id, item.is_favorite, e)} style={{
              cursor: 'pointer',
              fontSize: 13,
              color: item.is_favorite ? '#FFD700' : '#555',
              filter: item.is_favorite ? 'drop-shadow(0 0 4px #FFD70080)' : 'none',
              transition: 'all 0.2s ease',
              userSelect: 'none',
              flexShrink: 0
            }} title={item.is_favorite ? t("components.FloatingBall.k56") : t("home.NewsPanel.k28")}>
                    {item.is_favorite ? '★' : '☆'}
                  </span>
                  {item.published_at && <span style={{
              fontSize: 11,
              color: '#6a6a8a',
              flexShrink: 0,
              whiteSpace: 'normal',
              wordBreak: 'break-all'
            }}>{item.published_at}</span>}
                  {aiOn && featureOn('news_summary') && <span onClick={() => {
              const nextId = isExpanded ? null : item.id;
              setExpandedNewsId(nextId);
              if (nextId === item.id && !item.ai_summary && !aiSummary[item.id] && aiSummarizing !== item.id) {
                handleAiSummarizeNews(item);
              }
            }} style={{
              cursor: 'pointer',
              fontSize: 12,
              color: '#00F0FF',
              flexShrink: 0,
              marginLeft: 'auto',
              userSelect: 'none'
            }}>
                      {isExpanded ? '▲' : '▽'}
                    </span>}
                </div>
                {isExpanded && aiOn && featureOn('news_summary') && <div className={styles.newsDetailSection}>
                    {item.ai_summary || aiSummary[item.id] ? <div className={styles.aiSummaryContent}>{item.ai_summary || aiSummary[item.id]}</div> : <div className={styles.aiSummaryAutoHint}>
                        {aiSummarizing === item.id ? <span className={styles.aiPulsing}>{t("home.NewsPanel.k29")}</span> : aiOn && !llmConfigured ? <span className={styles.aiSummaryHint}>{t("home.NewsPanel.k30")}</span> : <span className={styles.aiSummaryHint}>{t("home.NewsPanel.k31")}</span>}
                      </div>}
                  </div>}
                {item.url && <span onClick={e => {
            e.stopPropagation();
            system.openUrl(item.url!);
          }} style={{
            display: 'inline-block',
            marginTop: 6,
            fontSize: 11,
            color: '#00F0FF80',
            textDecoration: 'underline',
            textDecorationColor: '#00F0FF40',
            cursor: 'pointer',
            maxWidth: '100%',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            transition: 'color 0.2s ease'
          }} title={item.url}>
                    {item.url}
                  </span>}
                {aiOn && featureOn('news_summary') && !isExpanded && <button onClick={e => {
            e.stopPropagation();
            handleAiSummarizeNews(item);
          }} disabled={aiSummarizing === item.id} style={{
            display: 'block',
            marginTop: 8,
            padding: '3px 10px',
            fontSize: 11,
            borderRadius: 4,
            border: '1px solid #00FF00',
            background: '#000000',
            color: '#00FF00',
            cursor: 'pointer',
            fontFamily: 'monospace',
            fontWeight: 'bold',
            opacity: aiSummarizing === item.id ? 0.5 : 1
          }}>
                    {aiSummarizing === item.id ? t("home.NewsPanel.k32") : t("home.NewsPanel.k33")}
                  </button>}
                {(item.ai_summary || aiSummary[item.id]) && !isExpanded && <div style={{
            marginTop: 6,
            padding: '6px 10px',
            borderRadius: 4,
            border: '1px solid rgba(0,255,0,0.2)',
            background: 'rgba(0,255,0,0.04)',
            fontSize: 11,
            color: '#00FF00',
            fontFamily: 'monospace',
            lineHeight: 1.5
          }}>
                    <span style={{
              color: '#00FF0080',
              marginRight: 4
            }}>▸</span>
                    {item.ai_summary || aiSummary[item.id]}
                  </div>}
              </div>;
      })}
        </div>}
    </div>;
}