import { t } from "i18next";
/**
 * GamePreview - 游戏数据预览页（/game）
 *
 * 阶段7 Task 7.3：4 卡片数据预览布局
 *   - 境界信息卡 RealmCard
 *   - 建筑统计卡 BuildingStatsCard
 *   - 知识领域积分卡 KnowledgeProgressCard
 *   - 事件与任务卡 EventsCard
 *
 * 数据加载流程：
 *   1. listWorlds() → 取第一个 worldId（最近游玩）
 *   2. Promise.all([getRealmInfo, getBuildings, getKnowledgeDomains,
 *                   getKnowledgeProgress, getEventsAndTasks])
 *   3. 无世界 → 显示"创建世界"CTA
 *   4. 失败 → 显示错误提示 + 重试按钮
 *
 * 顶部 Header：玩家名 + 文明等级 + "进入游戏"按钮（跳转 /game/play）
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useState, useEffect, useCallback, useRef } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { game } from '@/lib/ipc';
import type { RealmInfo, GameBuilding, GameKnowledgeDomain, GameKnowledgeProgress, EventsAndTasks, WorldSummary, GameNpc, PointsTrend } from '@/types/game';
import { ROUTES } from '@/routes/routes';
import RealmCard from './components/RealmCard';
import BuildingStatsCard from './components/BuildingStatsCard';
import KnowledgeProgressCard from './components/KnowledgeProgressCard';
import EventsCard from './components/EventsCard';
import CardError from './components/CardError';
import { Preview3DView } from './components/Preview3DView';
import NpcDialog from './components/NpcDialog';
import GameStoryDialog from './components/GameStoryDialog';
import NaturalLanguageInput from './components/NaturalLanguageInput';
import InsightsPanel from './components/InsightsPanel';
import styles from './GamePreview.module.css';
export default function GamePreview() {
  const navigate = useNavigate();

  // 当前选中世界
  const [currentWorldId, setCurrentWorldId] = useState<string | null>(null);
  const [currentWorld, setCurrentWorld] = useState<WorldSummary | null>(null);

  // 4 卡片数据
  const [realmInfo, setRealmInfo] = useState<RealmInfo | null>(null);
  const [buildings, setBuildings] = useState<GameBuilding[]>([]);
  const [domains, setDomains] = useState<GameKnowledgeDomain[]>([]);
  const [progress, setProgress] = useState<GameKnowledgeProgress[]>([]);
  const [events, setEvents] = useState<EventsAndTasks | null>(null);
  const [pointsTrend, setPointsTrend] = useState<PointsTrend | null>(null);

  // 加载状态
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);

  // 2D/3D 视图模式（localStorage 持久化）
  const [viewMode, setViewMode] = useState<'2d' | '3d'>(() => {
    try {
      const saved = localStorage.getItem('game_preview_mode');
      return saved === '3d' ? '3d' : '2d';
    } catch {
      return '2d';
    }
  });
  const toggleViewMode = useCallback(() => {
    setViewMode(prev => {
      const next = prev === '2d' ? '3d' : '2d';
      try { localStorage.setItem('game_preview_mode', next); } catch { /* ignore */ }
      return next;
    });
  }, []);

  // 4 卡片独立错误态（单卡片失败不影响其他卡片）
  const [cardErrors, setCardErrors] = useState<{
    realm: string | null;
    buildings: string | null;
    knowledge: string | null;
    events: string | null;
  }>({ realm: null, buildings: null, knowledge: null, events: null });

  // D4.2 NPC 对话浮窗
  const [npcList, setNpcList] = useState<GameNpc[]>([]);
  const [activeNpcId, setActiveNpcId] = useState<string | null>(null);
  const [showNpcPanel, setShowNpcPanel] = useState(false);

  // D4.4b 动态剧情浮窗
  const [showStoryDialog, setShowStoryDialog] = useState(false);

  // D4.4 自然语言交互浮窗
  const [showNlInput, setShowNlInput] = useState(false);

  // D4.6 数据分析 AI 洞察浮窗
  const [showInsights, setShowInsights] = useState(false);

  /** 加载境界卡数据（独立错误态） */
  const loadRealmCard = useCallback(async (worldId: string) => {
    try {
      const r = await game.getRealmInfo(worldId);
      if (r.code !== 0) throw new Error(r.message);
      setRealmInfo(r.data ?? null);
      setCardErrors(prev => ({ ...prev, realm: null }));
    } catch (e) {
      setCardErrors(prev => ({ ...prev, realm: e instanceof Error ? e.message : String(e) }));
    }
  }, []);

  /** 加载建筑卡数据（独立错误态） */
  const loadBuildingsCard = useCallback(async (worldId: string) => {
    try {
      const r = await game.getBuildings(worldId);
      if (r.code !== 0) throw new Error(r.message);
      setBuildings(r.data ?? []);
      setCardErrors(prev => ({ ...prev, buildings: null }));
    } catch (e) {
      setCardErrors(prev => ({ ...prev, buildings: e instanceof Error ? e.message : String(e) }));
    }
  }, []);

  /** 加载知识领域卡数据（domains + progress + trend，任一失败则整卡失败） */
  const loadKnowledgeCard = useCallback(async (worldId: string) => {
    try {
      const [domainsRes, progressRes, trendRes] = await Promise.all([
        game.getKnowledgeDomains(),
        game.getKnowledgeProgress(worldId),
        game.getPointsTrend(worldId, 7)
      ]);
      if (domainsRes.code !== 0) throw new Error(domainsRes.message);
      if (progressRes.code !== 0) throw new Error(progressRes.message);
      setDomains(domainsRes.data ?? []);
      setProgress(progressRes.data ?? []);
      // 趋势数据失败不阻断卡片主体（独立降级，仅丢失 sparkline）
      setPointsTrend(trendRes.code === 0 ? trendRes.data ?? null : null);
      setCardErrors(prev => ({ ...prev, knowledge: null }));
    } catch (e) {
      setCardErrors(prev => ({ ...prev, knowledge: e instanceof Error ? e.message : String(e) }));
    }
  }, []);

  /** 加载事件卡数据（独立错误态） */
  const loadEventsCard = useCallback(async (worldId: string) => {
    try {
      const r = await game.getEventsAndTasks(worldId);
      if (r.code !== 0) throw new Error(r.message);
      setEvents(r.data ?? null);
      setCardErrors(prev => ({ ...prev, events: null }));
    } catch (e) {
      setCardErrors(prev => ({ ...prev, events: e instanceof Error ? e.message : String(e) }));
    }
  }, []);

  /** 并行加载 4 卡片数据（Promise.allSettled，单卡片失败不影响其他） */
  const loadData = useCallback(async (worldId: string) => {
    setLoading(true);
    setError(null);
    await Promise.allSettled([
      loadRealmCard(worldId),
      loadBuildingsCard(worldId),
      loadKnowledgeCard(worldId),
      loadEventsCard(worldId)
    ]);
    setLoading(false);
  }, [loadRealmCard, loadBuildingsCard, loadKnowledgeCard, loadEventsCard]);

  /** 单卡片重试 */
  const retryCard = useCallback((cardKey: 'realm' | 'buildings' | 'knowledge' | 'events') => {
    if (!currentWorldId) return;
    switch (cardKey) {
      case 'realm': void loadRealmCard(currentWorldId); break;
      case 'buildings': void loadBuildingsCard(currentWorldId); break;
      case 'knowledge': void loadKnowledgeCard(currentWorldId); break;
      case 'events': void loadEventsCard(currentWorldId); break;
    }
  }, [currentWorldId, loadRealmCard, loadBuildingsCard, loadKnowledgeCard, loadEventsCard]);

  /** 初始化：加载世界列表，选第一个或显示 CTA */
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      setLoading(true);
      setError(null);
      try {
        const res = await game.listWorlds();
        if (cancelled) return;
        if (res.code !== 0) throw new Error(res.message);
        const list = res.data ?? [];
        if (list.length > 0) {
          const first = list[0];
          setCurrentWorldId(first.id);
          setCurrentWorld(first);
          // 加载 4 卡片数据
          void loadData(first.id);
        } else {
          // 无世界，停止加载状态
          setCurrentWorldId(null);
          setCurrentWorld(null);
          setLoading(false);
        }
      } catch (err) {
        if (cancelled) return;
        setError(err instanceof Error ? err.message : String(err));
        setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [loadData]);

  /**
   * 60 秒定时刷新（静默刷新，不切换 loading 状态，避免骨架屏闪烁）
   * 仅当有 worldId 且页面可见时触发
   */
  const worldIdRef = useRef<string | null>(null);
  worldIdRef.current = currentWorldId;
  useEffect(() => {
    const REFRESH_INTERVAL = 60_000; // 60s
    const timer = window.setInterval(() => {
      // 页面不可见时跳过，节省 IPC 调用
      if (document.hidden) return;
      const wid = worldIdRef.current;
      if (!wid) return;
      // 静默刷新：直接调用 4 个独立加载函数，不切换 loading（保留旧数据可见）
      void Promise.allSettled([
        loadRealmCard(wid),
        loadBuildingsCard(wid),
        loadKnowledgeCard(wid),
        loadEventsCard(wid)
      ]);
    }, REFRESH_INTERVAL);
    return () => window.clearInterval(timer);
  }, [loadRealmCard, loadBuildingsCard, loadKnowledgeCard, loadEventsCard]);

  /**
   * 3D 页（/game/play）返回时强制刷新
   * 监听 location 变化：从 /game/play 返回 /game 时刷新数据
   */
  const location = useLocation();
  const prevPathRef = useRef<string>(location.pathname);
  useEffect(() => {
    const prev = prevPathRef.current;
    const cur = location.pathname;
    prevPathRef.current = cur;
    // 从 3D 玩游戏页返回预览页时强制刷新
    if (prev === ROUTES.GAME_PLAY && cur === ROUTES.GAME && currentWorldId) {
      void loadData(currentWorldId);
    }
  }, [location.pathname, currentWorldId, loadData]);

  /** 创建世界 CTA */
  const handleCreateWorld = useCallback(async () => {
    setCreating(true);
    setError(null);
    try {
      const res = await game.initWorld();
      if (res.code !== 0) throw new Error(res.message);
      // 创建成功后刷新世界列表
      const listRes = await game.listWorlds();
      if (listRes.code !== 0) throw new Error(listRes.message);
      const list = listRes.data ?? [];
      if (list.length > 0) {
        const first = list[0];
        setCurrentWorldId(first.id);
        setCurrentWorld(first);
        void loadData(first.id);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setCreating(false);
    }
  }, [loadData]);

  /** 重试 */
  const handleRetry = useCallback(() => {
    if (currentWorldId) {
      void loadData(currentWorldId);
    } else {
      // 重新触发初始化（无 worldId 时重新 listWorlds）
      window.location.reload();
    }
  }, [currentWorldId, loadData]);

  /** D4.2 加载 NPC 列表并打开面板 */
  // Bug 修复 v1.52.19.1：原逻辑 NPC 列表加载失败时调用 setError 覆盖整页，
  // 导致 4 卡片数据被隐藏。改为局部错误处理（控制台告警 + 仍打开面板显示空列表）
  const openNpcPanel = useCallback(async () => {
    try {
      const res = await game.npcList();
      if (res?.data) setNpcList(res.data);
      setShowNpcPanel(true);
    } catch (e) {
      console.warn('[GamePreview] NPC 列表加载失败，打开空面板:', e);
      setNpcList([]);
      setShowNpcPanel(true);
    }
  }, []);

  // ===== 渲染分支 =====

  // 1. 错误提示
  if (error && !loading) {
    return <div className={styles.container}>
        <div className={styles.ctaCard}>
          <h2 className={styles.cardTitle}>{t("game.GamePreview.k1")}</h2>
          <p className={styles.errorText}>{error}</p>
          <button className={styles.retryBtn} onClick={handleRetry}>
            {t("components.ErrorBoundary.k5")}
          </button>
        </div>
      </div>;
  }

  // 2. 无世界 → 显示创建世界 CTA
  if (!currentWorldId && !loading) {
    return <div className={styles.container}>
        <div className={styles.ctaCard}>
          <h2 className={styles.cardTitle}>{t("game.GamePreview.k2")}</h2>
          <p className={styles.ctaDesc}>
            {t("game.GamePreview.k3")}<br />
            {t("game.GamePreview.k4")}
          </p>
          <button className={styles.createWorldBtn} onClick={handleCreateWorld} disabled={creating}>
            {creating ? t("game.GamePreview.k5") : t("game.GamePreview.k6")}
          </button>
        </div>
      </div>;
  }

  // 3. 正常 4 卡片布局
  return <div className={styles.container}>
      <header className={styles.header}>
        <div className={styles.headerInfo}>
          <h1 className={styles.title}>
            {currentWorld?.player_name ?? '—'} {t("game.GamePreview.k7")}
          </h1>
          <p className={styles.subtitle}>
            {t("game.GamePreview.k8")}{currentWorld?.civilization_level ?? 0} {t("game.GamePreview.k9")} {currentWorld?.building_count ?? 0} {t("game.GamePreview.k10")} {currentWorld?.completed_building_count ?? 0}
          </p>
        </div>
        <button
          className={`${styles.npcBtn} ${viewMode === '3d' ? styles.modeBtnActive : ''}`}
          onClick={toggleViewMode}
          disabled={!currentWorldId}
          title={t('game.GamePreview.k13')}
        >
          {viewMode === '2d' ? '🖼 2D' : '🌐 3D'}
        </button>
        <button className={styles.enterGameBtn} onClick={() => navigate(ROUTES.GAME_PLAY)} disabled={!currentWorldId}>
          {t("game.GamePreview.k11")}
        </button>
        <button className={styles.npcBtn} onClick={openNpcPanel} disabled={!currentWorldId} title="NPC 对话">
          🧙 NPC
        </button>
        <button
          className={styles.npcBtn}
          onClick={() => setShowStoryDialog(true)}
          disabled={!currentWorldId}
          title="动态剧情"
        >
          📖 剧情
        </button>
        <button
          className={styles.npcBtn}
          onClick={() => setShowNlInput(true)}
          disabled={!currentWorldId}
          title="自然语言交互"
        >
          💬 指令
        </button>
        <button
          className={styles.npcBtn}
          onClick={() => setShowInsights(true)}
          disabled={!currentWorldId}
          title="AI 行为洞察"
        >
          📊 洞察
        </button>
      </header>

      {error && <div className={styles.errorBanner}>
          {t("game.GamePreview.k12")}{error}
          <button className={styles.retrySmall} onClick={handleRetry}>
            {t("components.ErrorBoundary.k5")}
          </button>
        </div>}

      <main className={styles.grid} style={viewMode === '3d' ? { display: 'block' } : undefined}>
        {viewMode === '3d' ? (
          <Preview3DView
            worldId={currentWorldId ?? undefined}
            realmInfo={realmInfo}
            buildings={buildings}
            domains={domains}
            progress={progress}
            events={events}
            trend7d={pointsTrend}
          />
        ) : (
          <>
            {cardErrors.realm ? (
              <CardError message={cardErrors.realm} onRetry={() => retryCard('realm')} />
            ) : (
              <RealmCard realmInfo={realmInfo} loading={loading} />
            )}
            {cardErrors.buildings ? (
              <CardError message={cardErrors.buildings} onRetry={() => retryCard('buildings')} />
            ) : (
              <BuildingStatsCard buildings={buildings} loading={loading} />
            )}
            {cardErrors.knowledge ? (
              <CardError message={cardErrors.knowledge} onRetry={() => retryCard('knowledge')} />
            ) : (
              <KnowledgeProgressCard domains={domains} progress={progress} loading={loading} trend={pointsTrend} />
            )}
            {cardErrors.events ? (
              <CardError message={cardErrors.events} onRetry={() => retryCard('events')} />
            ) : (
              <EventsCard events={events} loading={loading} onBreakthroughClick={() => navigate(`${ROUTES.GAME_PLAY}?focus=breakthrough`)} />
            )}
          </>
        )}
      </main>

      {showNpcPanel && (
        <div className={styles.npcPanelOverlay} onClick={() => setShowNpcPanel(false)}>
          <div className={styles.npcPanel} onClick={e => e.stopPropagation()}>
            <div className={styles.npcPanelHeader}>
              <h2 className={styles.npcPanelTitle}>🧙 修真界 NPC</h2>
              <button className={styles.npcPanelClose} onClick={() => setShowNpcPanel(false)}>✕</button>
            </div>
            <div className={styles.npcList}>
              {npcList.length === 0 && (
                <div className={styles.npcEmpty}>暂无可对话 NPC</div>
              )}
              {npcList.map(npc => (
                <button
                  key={npc.id}
                  className={styles.npcCard}
                  onClick={() => {
                    setActiveNpcId(npc.id);
                    setShowNpcPanel(false);
                  }}
                >
                  <span className={styles.npcAvatar}>{npc.avatar_emoji}</span>
                  <div className={styles.npcMeta}>
                    <div className={styles.npcName}>{npc.name}</div>
                    <div className={styles.npcRole}>{npc.role} · {npc.location}</div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        </div>
      )}

      {activeNpcId && currentWorldId && (
        <NpcDialog
          npcId={activeNpcId}
          worldId={currentWorldId}
          onClose={() => setActiveNpcId(null)}
        />
      )}

      {showStoryDialog && currentWorldId && (
        <GameStoryDialog
          worldId={currentWorldId}
          playerName={currentWorld?.player_name ?? '修仙者'}
          realm={
            realmInfo
              ? `${realmInfo.realm_major}·${realmInfo.realm_minor}`
              : '未知境界'
          }
          onClose={() => setShowStoryDialog(false)}
        />
      )}

      {showNlInput && currentWorldId && (
        <NaturalLanguageInput
          worldId={currentWorldId}
          playerName={currentWorld?.player_name ?? '修仙者'}
          realm={
            realmInfo
              ? `${realmInfo.realm_major}·${realmInfo.realm_minor}`
              : '未知境界'
          }
          onClose={() => setShowNlInput(false)}
        />
      )}

      {showInsights && currentWorldId && (
        <InsightsPanel
          worldId={currentWorldId}
          playerName={currentWorld?.player_name ?? '修仙者'}
          realm={
            realmInfo
              ? `${realmInfo.realm_major}·${realmInfo.realm_minor}`
              : '未知境界'
          }
          onClose={() => setShowInsights(false)}
        />
      )}
    </div>;
}