/**
 * usePreview3DScene - 3D 模式场景切换 hook（game-future-outlook-completion Task 2.4）
 *
 * 职责：
 *   1. 管理 4 个 3D 场景的当前激活状态（BuildingsIsland / RealmTower / KnowledgeStarMap / EventsFlow）
 *   2. 提供场景切换函数（带过渡状态标记）
 *   3. 提供每个场景的默认相机位置（供 CameraRig lerp 使用）
 *
 * 相机 lerp 系数：0.08（spec 要求）
 * 过渡时长：约 1.5s（lerp 收敛 + 缓冲）
 *
 * 切换流程：
 *   1. setScene(newScene) → 设置 currentScene + isTransitioning=true
 *   2. CameraRig 在 useFrame 中 lerp 相机位置到新场景默认值（系数 0.08）
 *   3. 1.5s 后 isTransitioning=false，OrbitControls 重新接管
 *
 * change-id: game-future-outlook-completion
 */
import { useCallback, useEffect, useRef, useState } from 'react';

/** 场景 ID（4 个 3D 场景） */
export type SceneId =
  | 'buildingsIsland'
  | 'realmTower'
  | 'knowledgeStarMap'
  | 'eventsFlow';

/** 场景默认相机配置 */
export interface SceneCameraConfig {
  /** 相机位置 */
  position: [number, number, number];
  /** 相机看向的目标点 */
  target: [number, number, number];
}

/** 4 个场景的默认相机位置（每个场景最佳观察角度） */
export const SCENE_CAMERAS: Record<SceneId, SceneCameraConfig> = {
  // 建筑鸟瞰岛：俯视角度，看清建筑分布
  buildingsIsland: {
    position: [0, 8, 0.01],
    target: [0, 0, 0],
  },
  // 境界塔：前侧方仰视，看清塔身层次
  realmTower: {
    position: [3, 3, 7],
    target: [0, 1.8, 0],
  },
  // 知识星图：俯视，看清环形分布
  knowledgeStarMap: {
    position: [0, 6, 0.01],
    target: [0, 0, 0],
  },
  // 事件流粒子：侧视，看清 X 轴方向流动
  eventsFlow: {
    position: [0, 1.5, 8],
    target: [0, 0, 0],
  },
};

/** 场景切换过渡时长（ms），略大于 lerp 收敛时间 */
const TRANSITION_DURATION_MS = 1500;

/**
 * 3D 场景切换 hook
 *
 * 用法：
 * ```tsx
 * const { currentScene, isTransitioning, switchScene, cameraConfig } = usePreview3DScene();
 * ```
 */
export function usePreview3DScene() {
  const [currentScene, setCurrentScene] = useState<SceneId>('buildingsIsland');
  const [isTransitioning, setIsTransitioning] = useState(false);
  const transitionTimerRef = useRef<number | null>(null);

  /** 切换到指定场景 */
  const switchScene = useCallback((scene: SceneId) => {
    if (scene === currentScene) return;
    setCurrentScene(scene);
    setIsTransitioning(true);

    // 清除上一个过渡计时器
    if (transitionTimerRef.current !== null) {
      window.clearTimeout(transitionTimerRef.current);
    }
    // 1.5s 后结束过渡，OrbitControls 接管
    transitionTimerRef.current = window.setTimeout(() => {
      setIsTransitioning(false);
      transitionTimerRef.current = null;
    }, TRANSITION_DURATION_MS);
  }, [currentScene]);

  // 卸载时清理计时器
  useEffect(() => {
    return () => {
      if (transitionTimerRef.current !== null) {
        window.clearTimeout(transitionTimerRef.current);
        transitionTimerRef.current = null;
      }
    };
  }, []);

  return {
    /** 当前激活的场景 */
    currentScene,
    /** 是否正在过渡（OrbitControls 应禁用） */
    isTransitioning,
    /** 切换到指定场景 */
    switchScene,
    /** 当前场景的默认相机配置 */
    cameraConfig: SCENE_CAMERAS[currentScene],
  };
}
