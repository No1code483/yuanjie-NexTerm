import { t } from "i18next";
/**
 * Building - 单建筑渲染（Task 9.1 / 9.2 / 9.4 / 9.6）
 *
 * 按 building.status 分发渲染：
 *   - planning  规划中：线框占位 + 标牌（未动工）
 *   - building  建造中：ProceduralBuilding(status=building) + 进度条标牌
 *                       （建筑组件内部已处理半透明 + 生长动画 + 简易脚手架）
 *   - completed 已建成：完整 ProceduralBuilding
 *   - ruined    破损  ：v1 暂按 completed 显示（后续加灰色半透明覆盖）
 *
 * 点击交互（Task 9.4）：
 *   - 非放置模式下，点击建筑 → selectBuilding(id) → 触发右侧详情面板
 *   - 放置模式下点击建筑被忽略（由 PlacementController 处理地面点击）
 *
 * 选中高亮：selected=true 时显示黄色高亮环
 *
 * 建造进度动画（Task 9.6）：
 *   - useFrame 平滑插值 displayProgress → 目标 buildProgress
 *   - 节流 setState（差距 > 阈值才更新），避免每帧 React 渲染
 *   - status: building → completed 转换时触发完成动画：
 *       <CompletionBurst/> 金光粒子爆发（2 秒）
 *       <ScaffoldFader/> 脚手架淡出（1.5 秒）
 *   - 完成动画播完（2.1 秒）后自动卸载
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useEffect, useRef, useState } from 'react';
import { Detailed, Html } from '@react-three/drei';
import { useFrame, type ThreeEvent } from '@react-three/fiber';
import { ProceduralBuilding } from './ProceduralBuilding';
import { SimpleBuilding } from './SimpleBuilding';
import { CompletionBurst } from './CompletionBurst';
import { ScaffoldFader } from './ScaffoldFader';
import { useGame3DStore } from '../stores/gameStore';
import type { Building3D } from '../stores/gameStore';
import type { GameBuildingStatus } from '@/types/game';
interface BuildingProps {
  building: Building3D;
  /** 是否被选中（用于高亮） */
  selected?: boolean;
}

/** 不同 category 建筑的近似包围盒尺寸 [w, h, d]，用于脚手架淡出几何 */
const SCAFFOLD_SIZE: Record<Building3D['category'], [number, number, number]> = {
  house: [2.0, 1.6, 2.0],
  town: [3.0, 2.5, 3.0],
  city: [4.0, 3.5, 4.0],
  kingdom: [5.0, 4.5, 5.0],
  palace: [4.5, 4.0, 4.5],
  technology: [2.5, 2.2, 2.5],
  sect: [3.5, 3.0, 3.5],
  immortal: [4.0, 4.5, 4.0]
};

/** 显示进度节流阈值：差距 ≥ 此值才 setState，避免每帧渲染 */
const PROGRESS_SETSTATE_THRESHOLD = 0.5;

/** 完成动画总时长（取 max(粒子 2s, 脚手架 1.5s) + 0.1s 缓冲） */
const COMPLETION_DURATION_MS = 2100;
export function Building({
  building,
  selected = false
}: BuildingProps) {
  const {
    category,
    position,
    rotationY,
    status,
    buildProgress,
    level,
    name,
    id
  } = building;
  const placementMode = useGame3DStore(s => s.placementMode);
  const selectBuilding = useGame3DStore(s => s.selectBuilding);
  const actionInProgress = useGame3DStore(s => s.actionInProgress);

  // ===== Task 9.6 平滑进度插值（所有 hooks 必须在条件分支前） =====
  const [displayProgress, setDisplayProgress] = useState(buildProgress);
  const displayRef = useRef(buildProgress);
  useFrame((_, delta) => {
    if (status !== 'building') return;
    const target = buildProgress;
    const current = displayRef.current;
    if (Math.abs(target - current) < 0.01) return;
    // 指数衰减：每秒靠近目标 50% × 5 = 5/s（约 0.3s 到达 95%）
    const dt = Math.min(delta, 0.05);
    const next = current + (target - current) * Math.min(1, dt * 5);
    displayRef.current = next;
    // 节流 setState：仅当差距 ≥ 阈值才触发 React 渲染
    if (Math.abs(next - displayProgress) >= PROGRESS_SETSTATE_THRESHOLD) {
      setDisplayProgress(next);
    }
  });

  // ===== Task 9.6 完成动画触发（status: building → completed） =====
  const [completionKey, setCompletionKey] = useState(0);
  const [completionVisible, setCompletionVisible] = useState(false);
  const prevStatusRef = useRef<GameBuildingStatus>(status);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);
  useEffect(() => {
    // 仅 building → completed 转换才触发
    if (prevStatusRef.current === 'building' && status === 'completed') {
      setCompletionKey(k => k + 1);
      setCompletionVisible(true);
      const timer = setTimeout(() => {
        if (mountedRef.current) setCompletionVisible(false);
      }, COMPLETION_DURATION_MS);
      return () => clearTimeout(timer);
    }
    prevStatusRef.current = status;
  }, [status]);

  /** 点击建筑：非放置模式下选中（再次点击取消） */
  const handleClick = (e: ThreeEvent<MouseEvent>) => {
    e.stopPropagation();
    if (placementMode !== 'idle' || actionInProgress) return;
    selectBuilding(selected ? null : id);
  };

  // 规划中：仅线框占位 + 标牌（未动工，无实体）
  if (status === 'planning') {
    return <group position={position} rotation={[0, rotationY, 0]} onClick={handleClick}>
        <mesh position={[0, 0.5, 0]}>
          <boxGeometry args={[1.6, 1, 1.6]} />
          <meshBasicMaterial color="#6bb6ff" wireframe transparent opacity={0.4} />
        </mesh>
        <Html position={[0, 1.6, 0]} center distanceFactor={12} zIndexRange={[10, 0]}>
          <div className="game3d-building-tag game3d-building-tag--planning">
            ▓ {name} {t("game3d.components.Building.k1")}
          </div>
        </Html>
        {selected && <mesh position={[0, 0.05, 0]} rotation={[-Math.PI / 2, 0, 0]}>
            <ringGeometry args={[1.8, 2.0, 32]} />
            <meshBasicMaterial color="#fbbf24" transparent opacity={0.8} side={2} />
          </mesh>}
      </group>;
  }
  const isBuilding = status === 'building';
  const scaffoldSize = SCAFFOLD_SIZE[category] ?? [2.5, 2.0, 2.5];
  return <group position={position} rotation={[0, rotationY, 0]} onClick={handleClick}>
      {/* T4.2 LOD：近(0-30m)完整渲染 / 远(30m+)简化 box+cone */}
      <Detailed distances={[0, 30]}>
        {/* LOD0：完整程序化建筑 */}
        <ProceduralBuilding category={category} position={[0, 0, 0]} rotationY={0} status={status as GameBuildingStatus} buildProgress={displayProgress} level={level} />
        {/* LOD1：远距离简化版（box + cone，~24 顶点 vs 完整数百） */}
        <SimpleBuilding category={category} status={status as GameBuildingStatus} buildProgress={displayProgress} />
      </Detailed>

      {/* 建造中：进度条标牌（显示插值后的 displayProgress） */}
      {isBuilding && <Html position={[0, 3, 0]} center distanceFactor={12} zIndexRange={[10, 0]}>
          <div className="game3d-building-tag game3d-building-tag--building">
            ▓ {name} {t("game3d.components.Building.k2")} {displayProgress.toFixed(0)}%
          </div>
        </Html>}

      {/* 选中高亮环（点击建筑时显示） */}
      {selected && <mesh position={[0, 0.05, 0]} rotation={[-Math.PI / 2, 0, 0]}>
          <ringGeometry args={[1.8, 2.0, 32]} />
          <meshBasicMaterial color="#fbbf24" transparent opacity={0.8} side={2} />
        </mesh>}

      {/* Task 9.6 完成动画：金光粒子 + 脚手架淡出 */}
      {completionVisible && <>
          <CompletionBurst key={`burst-${completionKey}`} originY={scaffoldSize[1] / 2} />
          <ScaffoldFader key={`scaffold-${completionKey}`} size={scaffoldSize} />
        </>}
    </group>;
}