import { t } from "i18next";
/**
 * useCamera - 相机预设常量（Task 8.3）
 *
 * 参考 05_3D场景设计.md §4.3 相机预设视角：
 *   - top（俯视）：[0, 40, 0.01] → [0, 0, 0]    便于规划布局
 *   - iso（45°）： [16, 16, 16]   → [0, 0, 0]    默认等距视角
 *   - side（平视）：[0, 3, 25]    → [0, 2, 0]    沉浸感最强
 *
 * 设计说明（D3 决策）：
 *   - hook 不调用 useFrame（hook 不能在 Canvas 外使用）
 *   - 仅导出预设常量供 Camera3D 组件读取
 *   - 实际相机插值在 Camera3D 组件内用 useFrame 实现
 *
 * 切换流程：
 *   1. 用户点击视角按钮 → store.setCameraPreset('top')
 *   2. Camera3D 组件 useEffect 监听 cameraPreset 变化
 *   3. useFrame 中 lerp 插值 camera.position 与 controls.target
 *
 * change-id: game-3d-rebuild-refactor
 */
import * as THREE from 'three';
import type { CameraPreset } from '../stores/gameStore';

/** 单个预设配置：相机位置 + 视线焦点 */
export interface CameraPresetConfig {
  position: THREE.Vector3;
  target: THREE.Vector3;
}

/** 三种相机预设（位置 + 焦点） */
export const CAMERA_PRESETS: Record<CameraPreset, CameraPresetConfig> = {
  top: {
    position: new THREE.Vector3(0, 40, 0.01),
    target: new THREE.Vector3(0, 0, 0)
  },
  iso: {
    position: new THREE.Vector3(16, 16, 16),
    target: new THREE.Vector3(0, 0, 0)
  },
  side: {
    position: new THREE.Vector3(0, 3, 25),
    target: new THREE.Vector3(0, 2, 0)
  }
};

/** 预设中文名（UI 按钮显示用） */
export const CAMERA_PRESET_LABELS: Record<CameraPreset, string> = {
  top: t("game3d.hooks.useCamera.k1"),
  iso: '45°',
  side: t("game3d.hooks.useCamera.k2")
};