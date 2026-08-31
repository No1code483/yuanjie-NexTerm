import { t } from "i18next";
/**
 * modelPaths - GLB 模型路径映射 + 程序化 fallback 配置
 *
 * Task 6.3：由于网络下载不可靠（GitHub 502、Quaternius 超时、poly.pizza 需交互），
 *          采用程序化几何体 fallback 方案。8 大类各有独特形状 + PBR 材质。
 *          后续用户手动下载 GLB 后，将 useProceduralFallback 改为 false 即可切换。
 *
 * 模型资源来源（CC0，供用户手动下载）：
 *   - Kenney Fantasy Town Kit: https://kenney.nl/assets/fantasy-town-kit （167 模型，GLB）
 *   - Quaternius Medieval Village Pack: https://quaternius.com/packs/medievalvillagepack.html
 *   - poly.pizza 单模型下载: https://poly.pizza/search?q=building
 *
 * 手动下载后放置规则：
 *   前端/public/models/{building_category}/{subtype}.glb
 *
 * change-id: game-3d-rebuild-refactor
 */
import type { GameBuildingCategory } from '@/types/game';

/** 是否使用程序化几何体 fallback（无 GLB 时为 true） */
export const USE_PROCEDURAL_FALLBACK = true;

/** 8 大类的 GLB 模型路径映射（fallback 关闭后使用） */
export const BUILDING_MODEL_PATHS: Record<GameBuildingCategory, string> = {
  house: '/models/house/cottage.glb',
  town: '/models/town/village_house.glb',
  city: '/models/city/city_building.glb',
  kingdom: '/models/kingdom/castle.glb',
  palace: '/models/palace/grand_palace.glb',
  technology: '/models/technology/lab.glb',
  sect: '/models/sect/temple.glb',
  immortal: '/models/immortal/pavilion.glb'
};

/** 8 大类中文名（用于 UI 显示） */
export const CATEGORY_LABELS: Record<GameBuildingCategory, string> = {
  house: t("game3d.components.UI.BuildingDetail.k1"),
  town: t("game.components.BuildingStatsCard.k2"),
  city: t("game.components.BuildingStatsCard.k3"),
  kingdom: t("game.components.BuildingStatsCard.k4"),
  palace: t("game.components.BuildingStatsCard.k5"),
  technology: t("game.components.BuildingStatsCard.k6"),
  sect: t("game.components.BuildingStatsCard.k7"),
  immortal: t("game3d.components.UI.BuildingDetail.k2")
};

/** 8 大类程序化几何体配置（fallback 时使用） */
export interface ProceduralBuildingConfig {
  /** 主体几何体类型 */
  shape: 'box' | 'cylinder' | 'cone' | 'sphere' | 'tower' | 'palace' | 'temple' | 'pavilion';
  /** 主色调（十六进制） */
  color: string;
  /** 金属度 0-1 */
  metalness: number;
  /** 粗糙度 0-1 */
  roughness: number;
  /** 环境反射强度 */
  envMapIntensity: number;
  /** 建筑尺寸（宽、高、深） */
  size: [number, number, number];
}

/** 8 大类程序化配置（参考 05_3D场景设计.md §6.2 材质参数标准） */
export const PROCEDURAL_BUILDING_CONFIGS: Record<GameBuildingCategory, ProceduralBuildingConfig> = {
  house: {
    shape: 'box',
    color: '#8b6f47',
    // 土褐
    metalness: 0.0,
    roughness: 0.85,
    envMapIntensity: 0.4,
    size: [1.6, 1.2, 1.6]
  },
  town: {
    shape: 'box',
    color: '#a67c52',
    // 木棕
    metalness: 0.0,
    roughness: 0.8,
    envMapIntensity: 0.5,
    size: [2.0, 1.8, 2.0]
  },
  city: {
    shape: 'tower',
    color: '#6b7280',
    // 石灰
    metalness: 0.1,
    roughness: 0.7,
    envMapIntensity: 0.6,
    size: [2.4, 3.0, 2.4]
  },
  kingdom: {
    shape: 'tower',
    color: '#7c5e3c',
    // 深木色
    metalness: 0.15,
    roughness: 0.65,
    envMapIntensity: 0.7,
    size: [3.2, 4.0, 3.2]
  },
  palace: {
    shape: 'palace',
    color: '#c9a227',
    // 金黄
    metalness: 0.4,
    roughness: 0.4,
    envMapIntensity: 1.1,
    size: [4.0, 3.5, 4.0]
  },
  technology: {
    shape: 'box',
    color: '#4a9eff',
    // 科技蓝
    metalness: 0.5,
    roughness: 0.3,
    envMapIntensity: 0.9,
    size: [2.8, 3.5, 2.8]
  },
  sect: {
    shape: 'temple',
    color: '#9b59b6',
    // 紫色
    metalness: 0.3,
    roughness: 0.4,
    envMapIntensity: 1.0,
    size: [3.0, 3.2, 3.0]
  },
  immortal: {
    shape: 'pavilion',
    color: '#c084fc',
    // 紫金
    metalness: 0.5,
    roughness: 0.2,
    envMapIntensity: 1.3,
    size: [3.5, 4.5, 3.5]
  }
};

/**
 * 根据建筑大类获取模型路径或 fallback 配置
 * - fallback 开启时返回 null，调用方使用 PROCEDURAL_BUILDING_CONFIGS
 * - fallback 关闭时返回 GLB 路径
 */
export function getBuildingModel(category: GameBuildingCategory): string | null {
  if (USE_PROCEDURAL_FALLBACK) return null;
  return BUILDING_MODEL_PATHS[category];
}