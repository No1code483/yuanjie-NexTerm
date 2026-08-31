import { t } from "i18next";
/**
 * Game3D - 3D 游戏页主入口（Task 8.1）
 *
 * 参考 05_3D场景设计.md §3.1 场景组件树 + §15.4 与 3D 场景协同
 *
 * 职责：
 *   1. 挂载 <Canvas> 容器 + shadows + ACESFilmic 色调映射
 *   2. <Suspense> 包裹异步资源（GLB / HDR）
 *   3. <ErrorBoundary> 捕获 3D 渲染崩溃，提供回退 UI
 *   4. <IntroOverlay> 开场文案动画，掩盖资源加载
 *   5. UI 叠加层：返回按钮（→ /game）+ 视角切换按钮（→ 调用 store）
 *   6. Canvas 淡入：与 IntroOverlay 淡出同步
 *
 * 跳过逻辑：sessionStorage 标记后二次进入直接跳过开场
 *
 * 颜色体系：使用 --game3d-* 变量（与主应用 --nt-* 隔离）
 *
 * change-id: game-3d-rebuild-refactor
 */
import { Suspense, useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Canvas } from '@react-three/fiber';
import { AdaptiveDpr } from '@react-three/drei';
import { ACESFilmicToneMapping } from 'three';
import ErrorBoundary from '@/components/ErrorBoundary';
import { ROUTES } from '@/routes/routes';
import { game } from '@/lib/ipc';
import type { WorldState } from '@/types/game';
import { Scene } from './components/Scene';
import { BuildPanel } from './components/UI/BuildPanel';
import { BuildingDetail } from './components/UI/BuildingDetail';
import { IntroOverlay } from './components/UI/IntroOverlay';
import { ReplayToggleButton } from './components/UI/ReplayToggleButton';
import { ReplayPanel } from './components/UI/ReplayPanel';
import { BreakthroughQuiz } from './components/UI/BreakthroughQuiz';
import { useGame3DStore } from './stores/gameStore';
import { CAMERA_PRESET_LABELS } from './hooks/useCamera';
import type { CameraPreset } from './stores/gameStore';
import './styles/game3d.css';

/** sessionStorage 键：标记已观看开场动画 */
const INTRO_STORAGE_KEY = 'game3d_intro_played';

/** 相机预设按钮列表（顺序：俯视 → 45° → 平视） */
const CAMERA_PRESETS: CameraPreset[] = ['top', 'iso', 'side'];
export default function Game3D() {
  const navigate = useNavigate();
  // 二次进入直接跳过开场（05 文档 §15.5）
  const hasPlayedIntro = sessionStorage.getItem(INTRO_STORAGE_KEY) === '1';
  const [introDone, setIntroDone] = useState(hasPlayedIntro);
  const setIntroPlayed = useGame3DStore(s => s.setIntroPlayed);
  const setReady = useGame3DStore(s => s.setReady);
  const setCameraPreset = useGame3DStore(s => s.setCameraPreset);
  const currentPreset = useGame3DStore(s => s.cameraPreset);
  const loadFromBackendWorld = useGame3DStore(s => s.loadFromBackendWorld);
  const loadFromBackendBuildings = useGame3DStore(s => s.loadFromBackendBuildings);
  const setBreakthroughPreview = useGame3DStore(s => s.setBreakthroughPreview);
  const replayMode = useGame3DStore(s => s.replayMode);
  const breakthroughSessionExists = useGame3DStore(s => s.breakthroughSession !== null || s.breakthroughOutcome !== null);
  const resetStore = useGame3DStore(s => s.reset);

  // ===== 数据加载（与开场动画并行） =====
  // dataState: loading → ready / error / noWorld
  const [dataState, setDataState] = useState<'loading' | 'ready' | 'error' | 'noWorld'>('loading');
  const [dataError, setDataError] = useState<string | null>(null);
  const [playerName, setPlayerName] = useState('');

  /** 加载世界数据：listWorlds → 取首个 → getWorldState → 填充 store */
  const loadWorld = useCallback(async () => {
    setDataState('loading');
    setDataError(null);
    try {
      const listRes = await game.listWorlds();
      if (listRes.code !== 0) throw new Error(listRes.message);
      const worlds = listRes.data ?? [];
      if (worlds.length === 0) {
        setDataState('noWorld');
        return;
      }
      const worldId = worlds[0].id;
      const stateRes = await game.getWorldState(worldId);
      if (stateRes.code !== 0) throw new Error(stateRes.message);
      const ws = stateRes.data;
      if (!ws) {
        setDataState('noWorld');
        return;
      }
      loadFromBackendWorld(ws.world);
      loadFromBackendBuildings(ws.buildings);
      setDataState('ready');
      // T2.5 拉取突破预览（决定祭坛是否显示）。失败仅 warn，不阻断主流程
      try {
        const previewRes = await game.getBreakthroughPreview(worldId);
        if (previewRes.code === 0 && previewRes.data) {
          setBreakthroughPreview(previewRes.data);
        }
      } catch (previewErr) {
        console.warn('[Game3D] getBreakthroughPreview failed:', previewErr);
      }
    } catch (err) {
      setDataError(err instanceof Error ? err.message : String(err));
      setDataState('error');
    }
  }, [loadFromBackendWorld, loadFromBackendBuildings, setBreakthroughPreview]);

  /** 创建世界（无世界 CTA 触发） */
  const handleCreateWorld = useCallback(async () => {
    setDataState('loading');
    setDataError(null);
    try {
      const name = playerName.trim() || t("game3d.Game3D.k1");
      const res = await game.initWorld(name);
      if (res.code !== 0) throw new Error(res.message);
      const ws: WorldState | null = res.data ?? null;
      if (!ws) throw new Error(t("game3d.Game3D.k2"));
      loadFromBackendWorld(ws.world);
      loadFromBackendBuildings(ws.buildings);
      setDataState('ready');
      // T2.5 新建世界后拉取突破预览（凡人初始境界，必然 can_breakthrough=false，但保证数据一致）
      try {
        const previewRes = await game.getBreakthroughPreview(ws.world.id);
        if (previewRes.code === 0 && previewRes.data) {
          setBreakthroughPreview(previewRes.data);
        }
      } catch (previewErr) {
        console.warn('[Game3D] getBreakthroughPreview (after create) failed:', previewErr);
      }
    } catch (err) {
      setDataError(err instanceof Error ? err.message : String(err));
      setDataState('error');
    }
  }, [playerName, loadFromBackendWorld, loadFromBackendBuildings, setBreakthroughPreview]);

  // 进入页面立即加载（与 IntroOverlay 并行）
  useEffect(() => {
    loadWorld();
  }, [loadWorld]);

  // 卸载时清理 React 层状态（避免下次进入时残留旧数据 + 释放大对象引用）
  // 依据 01_架构设计.md §2.3.2 资源释放策略：
  //   - React 状态由本 cleanup 清理
  //   - Three.js GLB/Texture/Geometry 资源由 R3F 在 Canvas 卸载时自动 dispose
  useEffect(() => {
    return () => {
      resetStore();
    };
  }, [resetStore]);

  // 开场完成时同步 store 状态
  useEffect(() => {
    if (introDone) {
      setIntroPlayed(true);
      setReady(true);
    }
  }, [introDone, setIntroPlayed, setReady]);

  /** 开场动画完成回调 */
  const handleIntroComplete = () => {
    sessionStorage.setItem(INTRO_STORAGE_KEY, '1');
    setIntroDone(true);
    setIntroPlayed(true);
    setReady(true);
  };

  /** 3D 场景崩溃回退 UI */
  const errorFallback = <div className="game3d-error-fallback">
      <div className="game3d-error-fallback__title">{t("game3d.Game3D.k3")}</div>
      <div className="game3d-error-fallback__hint">
        {t("game3d.Game3D.k4")}
        <br />
        {t("game3d.Game3D.k5")}
      </div>
      <button className="game3d-btn" onClick={() => navigate(ROUTES.GAME)}>
        {t("game3d.Game3D.k6")}
      </button>
    </div>;
  return <div className="game3d-root">
      {/* Canvas 容器（淡入过渡） */}
      <div className={`game3d-canvas-wrapper ${introDone ? 'is-visible' : ''}`}>
        <ErrorBoundary name={t("game3d.Game3D.k7")} fallback={errorFallback}>
          <Canvas shadows camera={{
          fov: 50,
          near: 0.1,
          far: 1000,
          position: [16, 16, 16]
        }} gl={{
          antialias: true,
          toneMapping: ACESFilmicToneMapping,
          toneMappingExposure: 1.0
        }} dpr={[1, 2]}>
            <Suspense fallback={null}>
              <Scene />
              {/* T4.3 AdaptiveDpr：帧率下降时自动降低 DPR，恢复时回调升档 */}
              <AdaptiveDpr pixelated={false} />
            </Suspense>
          </Canvas>
        </ErrorBoundary>
      </div>

      {/* 顶部 UI 层：返回按钮 + 视角切换 + 映射设置入口 + 时间轴回放入口 */}
      {introDone && <div className="game3d-ui-layer game3d-ui-layer--top">
          <div style={{
        display: 'flex',
        gap: 6
      }}>
            <button className="game3d-btn game3d-btn--back" onClick={() => navigate(ROUTES.GAME)} aria-label={t("game3d.Game3D.k8")}>
              {t("CommandManual.k217")}
            </button>
            <button className="game3d-btn" onClick={() => navigate(ROUTES.GAME_PLAY_MAPPING)} aria-label={t("game3d.Game3D.k9")} title={t("game3d.Game3D.k10")}>
              {t("game3d.Game3D.k11")}
            </button>
            {/* 时间轴回放入口：仅在非回放模式且数据就绪时显示 */}
            {dataState === 'ready' && !replayMode && <ReplayToggleButton />}
          </div>
          <div className="game3d-camera-presets">
            {CAMERA_PRESETS.map(preset => <button key={preset} className={`game3d-btn ${currentPreset === preset ? 'is-active' : ''}`} onClick={() => setCameraPreset(preset)} aria-pressed={currentPreset === preset}>
                {CAMERA_PRESET_LABELS[preset]}
              </button>)}
          </div>
        </div>}

      {/* 数据状态覆盖层（开场完成后，数据未就绪时显示） */}
      {introDone && dataState !== 'ready' && <div className="game3d-data-overlay">
          {dataState === 'loading' && <div className="game3d-data-msg">{t("game3d.Game3D.k12")}</div>}
          {dataState === 'error' && <>
              <div className="game3d-data-msg game3d-data-msg--err">{t("game3d.Game3D.k13")}</div>
              <div className="game3d-data-hint">{dataError}</div>
              <button className="game3d-btn" onClick={loadWorld}>{t("components.ErrorBoundary.k5")}</button>
            </>}
          {dataState === 'noWorld' && <>
              <div className="game3d-data-msg">{t("game3d.Game3D.k14")}</div>
              <div className="game3d-data-hint">{t("game3d.Game3D.k15")}</div>
              <input className="game3d-input" value={playerName} onChange={e => setPlayerName(e.target.value)} placeholder={t("game3d.Game3D.k16")} maxLength={20} />
              <button className="game3d-btn game3d-btn--primary" onClick={handleCreateWorld}>
                {t("game3d.Game3D.k17")}
              </button>
            </>}
        </div>}

      {/* 建造面板（左下角浮动按钮+弹出菜单，Task 9.3） */}
      {introDone && dataState === 'ready' && !replayMode && <BuildPanel />}

      {/* 建筑详情面板（右侧滑出，Task 9.4/9.5） */}
      {introDone && dataState === 'ready' && !replayMode && <BuildingDetail />}

      {/* 时间轴回放面板（底部固定，T1.5）；回放模式下隐藏建造/详情面板避免冲突 */}
      {introDone && dataState === 'ready' && replayMode && <ReplayPanel />}

      {/* T2.3 突破考验全屏答题（条件渲染：session 或 outcome 任一存在即显示；内部处理两态切换） */}
      {introDone && dataState === 'ready' && !replayMode && breakthroughSessionExists && <BreakthroughQuiz />}

      {/* 开场文案动画（introDone 后卸载） */}
      {!introDone && <IntroOverlay onComplete={handleIntroComplete} />}
    </div>;
}