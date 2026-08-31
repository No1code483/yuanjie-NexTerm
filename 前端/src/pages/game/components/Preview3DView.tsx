/**
 * Preview3DView - 3D 数据可视化视图（P3 升级版，game-future-outlook-completion Task 2.4）
 *
 * 升级点（相对 P2）：
 *   - 取消 2×2 象限并排展示，改为单场景显示 + 4 按钮切换
 *   - 相机位置 lerp 平滑过渡（系数 0.08，spec 要求）
 *   - 场景切换时 OrbitControls 禁用，过渡结束重新接管
 *   - 4 个场景：BuildingsIsland / RealmTower / KnowledgeStarMap / EventsFlow
 *
 * 性能降级链路（保留 P2 基线）：
 *   - AdaptiveDpr：帧率下降时自动降低 DPR
 *   - PerformanceMonitor：3 次抖动后强制 lowPerf 模式（关闭阴影 + DPR=1 + 减少粒子）
 *
 * 错误兜底：ErrorBoundary 捕获 3D 渲染崩溃 → 回退到提示卡片
 *
 * change-id: game-future-outlook-completion
 */
import { Suspense, useState, useCallback, useEffect, useRef } from 'react';
import { Canvas, useThree, useFrame } from '@react-three/fiber';
import { AdaptiveDpr, PerformanceMonitor, OrbitControls } from '@react-three/drei';
import * as THREE from 'three';
import { ACESFilmicToneMapping } from 'three';
import ErrorBoundary from '@/components/ErrorBoundary';
import { RealmTowerScene } from './scenes3d/RealmTowerScene';
import { BuildingsIslandScene } from './scenes3d/BuildingsIslandScene';
import { KnowledgeStarMapScene } from './scenes3d/KnowledgeStarMapScene';
import { EventsFlowScene } from './scenes3d/EventsFlowScene';
import {
  usePreview3DScene,
  SCENE_CAMERAS,
  type SceneId,
} from './hooks/usePreview3DScene';
import type {
  RealmInfo,
  GameBuilding,
  GameKnowledgeDomain,
  GameKnowledgeProgress,
  EventsAndTasks,
  PointsTrend,
} from '@/types/game';
import { t } from 'i18next';

interface Props {
  /** 当前世界 ID（可选；若提供则场景内部可调用 IPC API 加载缺省数据） */
  worldId?: string;
  /** 外部预加载的境界信息（可选，避免重复请求） */
  realmInfo?: RealmInfo | null;
  /** 外部预加载的建筑列表（可选） */
  buildings?: GameBuilding[];
  /** 外部预加载的领域定义（可选） */
  domains?: GameKnowledgeDomain[];
  /** 外部预加载的积分进度（可选） */
  progress?: GameKnowledgeProgress[];
  /** 外部预加载的事件与任务（可选） */
  events?: EventsAndTasks | null;
  /** 外部预加载的 7 天积分趋势（可选） */
  trend7d?: PointsTrend | null;
}

/** 场景按钮配置（顺序与 SCENE_CAMERAS 对应） */
const SCENE_BUTTONS: ReadonlyArray<{ id: SceneId; labelKey: string }> = [
  { id: 'buildingsIsland', labelKey: 'game.Preview3DView.k2' },
  { id: 'realmTower', labelKey: 'game.Preview3DView.k1' },
  { id: 'knowledgeStarMap', labelKey: 'game.Preview3DView.k3' },
  { id: 'eventsFlow', labelKey: 'game.Preview3DView.k4' },
];

/**
 * CameraRig - 相机位置 lerp 控制器
 *
 * 当场景切换时（isTransitioning=true），每帧 lerp 相机位置到目标场景的默认位置（系数 0.08）。
 * 过渡结束后，OrbitControls 接管，CameraRig 不再写入相机位置。
 */
function CameraRig({
  targetScene,
  isTransitioning,
}: {
  targetScene: SceneId;
  isTransitioning: boolean;
}) {
  const { camera } = useThree();
  const targetPos = useRef(new THREE.Vector3());
  const targetLookAt = useRef(new THREE.Vector3());

  // 当目标场景变化时，更新目标位置
  useEffect(() => {
    const cfg = SCENE_CAMERAS[targetScene];
    targetPos.current.set(...cfg.position);
    targetLookAt.current.set(...cfg.target);
  }, [targetScene]);

  // 初始化：首次挂载时直接对齐到默认场景位置（避免开场动画从原点飞入）
  useEffect(() => {
    const cfg = SCENE_CAMERAS[targetScene];
    camera.position.set(...cfg.position);
    camera.lookAt(targetLookAt.current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useFrame(() => {
    if (!isTransitioning) return;
    // 系数 0.08（spec 要求）：每帧逼近目标 8%
    camera.position.lerp(targetPos.current, 0.08);
    // 相机看向目标点（直接 lookAt，避免四元数 lerp 复杂度）
    camera.lookAt(targetLookAt.current);
  });

  return null;
}

/** 单场景渲染器：根据 currentScene 渲染对应场景 */
function SceneRenderer({
  scene,
  worldId,
  realmInfo,
  buildings,
  domains,
  progress,
  events,
  trend7d,
  lowPerf,
}: Props & { scene: SceneId; lowPerf: boolean }) {
  switch (scene) {
    case 'buildingsIsland':
      return <BuildingsIslandScene buildings={buildings ?? []} />;
    case 'realmTower':
      return <RealmTowerScene realmInfo={realmInfo ?? null} />;
    case 'knowledgeStarMap':
      return (
        <KnowledgeStarMapScene
          worldId={worldId}
          domains={domains}
          progress={progress}
          trend7d={trend7d}
          lowPerf={lowPerf}
        />
      );
    case 'eventsFlow':
      return (
        <EventsFlowScene
          worldId={worldId}
          events={events}
          lowPerf={lowPerf}
        />
      );
    default:
      return null;
  }
}

/** 3D 内部场景（含灯光 + 相机 + 当前场景） */
function Preview3DScene({
  currentScene,
  isTransitioning,
  lowPerf,
  ...props
}: Props & {
  currentScene: SceneId;
  isTransitioning: boolean;
  lowPerf: boolean;
}) {
  return (
    <>
      {/* 三光源体系（lowPerf 时关闭阴影） */}
      <ambientLight intensity={0.4} />
      <directionalLight
        position={[6, 10, 6]}
        intensity={1.0}
        castShadow={!lowPerf}
        shadow-mapSize={lowPerf ? 512 : 1024}
      />
      <hemisphereLight args={['#00f0ff', '#1a1a2e', 0.3]} />

      {/* 相机 lerp 控制器（场景切换时生效） */}
      <CameraRig targetScene={currentScene} isTransitioning={isTransitioning} />

      {/* 当前激活场景（仅渲染一个） */}
      <SceneRenderer scene={currentScene} lowPerf={lowPerf} {...props} />

      {/* 相机轨道控制（过渡期间禁用，避免与 CameraRig 冲突） */}
      {/* target 跟随当前场景配置，避免过渡结束后 OrbitControls 接管时视角跳变
          （Bug 修复 v1.52.19.1：原固定 [0,0,0] 与 realmTower 的 target [0,1.8,0] 不一致） */}
      <OrbitControls
        enabled={!isTransitioning}
        enablePan={false}
        minDistance={4}
        maxDistance={18}
        maxPolarAngle={Math.PI / 2.1}
        minPolarAngle={Math.PI / 8}
        target={SCENE_CAMERAS[currentScene].target}
      />

      {/* 性能降级：帧率下降时自动降低 DPR */}
      <AdaptiveDpr pixelated={false} />
    </>
  );
}

export function Preview3DView(props: Props) {
  const [lowPerf, setLowPerf] = useState(false);
  const { currentScene, isTransitioning, switchScene } = usePreview3DScene();

  /** 3D 崩溃回退 UI */
  const errorFallback = (
    <div
      style={{
        padding: '40px',
        textAlign: 'center',
        color: 'var(--nt-error)',
        fontFamily: 'var(--nt-font-mono)',
      }}
    >
      <div style={{ fontSize: '32px', marginBottom: '12px' }}>⚠</div>
      <div>{t('game.Preview3DView.k5')}</div>
      <div
        style={{
          fontSize: '12px',
          marginTop: '8px',
          color: 'var(--nt-text-secondary)',
        }}
      >
        {t('game.Preview3DView.k6')}
      </div>
    </div>
  );

  /** PerformanceMonitor 回调 */
  const handleDecline = useCallback(() => setLowPerf(true), []);
  const handleIncline = useCallback(() => setLowPerf(false), []);

  return (
    <div
      style={{
        width: '100%',
        height: '70vh',
        minHeight: '500px',
        background:
          'radial-gradient(ellipse at center, #0a0e14 0%, #000000 100%)',
        borderRadius: 'var(--nt-radius-lg)',
        border: '1px solid var(--nt-border-color)',
        overflow: 'hidden',
        position: 'relative',
      }}
    >
      <ErrorBoundary name={t('game.Preview3DView.k7')} fallback={errorFallback}>
        {/* 场景切换按钮组（4 个场景，spec 要求） */}
        <div
          style={{
            position: 'absolute',
            top: '12px',
            left: '50%',
            transform: 'translateX(-50%)',
            display: 'flex',
            gap: '6px',
            padding: '6px',
            background: 'rgba(10, 14, 20, 0.78)',
            border: '1px solid var(--nt-border-color)',
            borderRadius: 'var(--nt-radius-md)',
            backdropFilter: 'blur(8px)',
            zIndex: 10,
            boxShadow: '0 4px 16px rgba(0, 0, 0, 0.4)',
          }}
          role="tablist"
          aria-label={t('game.Preview3DView.k10')}
        >
          {SCENE_BUTTONS.map(btn => {
            const active = currentScene === btn.id;
            return (
              <button
                key={btn.id}
                role="tab"
                aria-selected={active}
                onClick={() => switchScene(btn.id)}
                disabled={isTransitioning}
                style={{
                  padding: '6px 12px',
                  background: active
                    ? 'linear-gradient(135deg, rgba(0, 240, 255, 0.25), rgba(176, 38, 255, 0.12))'
                    : 'transparent',
                  border: `1px solid ${active ? 'var(--nt-primary)' : 'var(--nt-border-subtle)'}`,
                  borderRadius: 'var(--nt-radius-sm)',
                  color: active ? 'var(--nt-primary)' : 'var(--nt-text-secondary)',
                  fontFamily: 'var(--nt-font-mono)',
                  fontSize: '12px',
                  fontWeight: 500,
                  cursor: isTransitioning ? 'wait' : 'pointer',
                  transition: 'all var(--nt-transition-normal)',
                  letterSpacing: '0.5px',
                  whiteSpace: 'nowrap',
                  textShadow: active ? '0 0 6px rgba(0, 240, 255, 0.6)' : 'none',
                  boxShadow: active ? '0 0 12px rgba(0, 240, 255, 0.45)' : 'none',
                }}
              >
                {t(btn.labelKey)}
              </button>
            );
          })}
        </div>

        {/* 过渡中指示器 */}
        {isTransitioning && (
          <div
            style={{
              position: 'absolute',
              top: '60px',
              left: '50%',
              transform: 'translateX(-50%)',
              padding: '4px 10px',
              background: 'rgba(10, 14, 20, 0.78)',
              border: '1px solid var(--nt-primary)',
              borderRadius: 'var(--nt-radius-sm)',
              color: 'var(--nt-primary)',
              fontFamily: 'var(--nt-font-mono)',
              fontSize: '11px',
              zIndex: 10,
              pointerEvents: 'none',
              animation: 'transitionPulse 1s ease-in-out infinite',
            }}
          >
            {t('game.Preview3DView.k11')}
          </div>
        )}

        <Canvas
          shadows={!lowPerf}
          camera={{ fov: 50, near: 0.1, far: 100, position: SCENE_CAMERAS.buildingsIsland.position }}
          gl={{
            antialias: !lowPerf,
            toneMapping: ACESFilmicToneMapping,
            toneMappingExposure: 1.0,
          }}
          dpr={lowPerf ? 1 : [1, 2]}
        >
          <Suspense fallback={null}>
            <PerformanceMonitor
              onDecline={handleDecline}
              onIncline={handleIncline}
              flipflops={3}
              onFallback={() => setLowPerf(true)}
            />
            <Preview3DScene
              {...props}
              currentScene={currentScene}
              isTransitioning={isTransitioning}
              lowPerf={lowPerf}
            />
          </Suspense>
        </Canvas>

        {/* 性能降级指示器 */}
        {lowPerf && (
          <div
            style={{
              position: 'absolute',
              top: '8px',
              right: '8px',
              padding: '4px 10px',
              background: 'rgba(251, 191, 36, 0.15)',
              border: '1px solid #fbbf24',
              borderRadius: 'var(--nt-radius-sm)',
              color: '#fbbf24',
              fontSize: '11px',
              fontFamily: 'var(--nt-font-mono)',
            }}
          >
            {t('game.Preview3DView.k8')}
          </div>
        )}
      </ErrorBoundary>
    </div>
  );
}
