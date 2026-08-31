/**
 * EventsFlowScene - 事件流粒子（P3 升级版，game-future-outlook-completion Task 2.3）
 *
 * 表达：时空粒子流，区分即将到来的待办 vs 已发生的过期事件
 *   - 沿 X 轴 -3 → 3 的时空粒子流（useFrame 推进）
 *   - 待办事项（upcoming_buildings）= 即将抵达亮点（黄色发光，AdditiveBlending）
 *   - 过期事件（recent_events）= 暗淡尾迹（灰色淡出）
 *   - 突破可触发时整个粒子流带青色脉冲
 *
 * 数据来源：`game.getEventsAndTasks(worldId)` API（从 `@/lib/ipc` 导入）
 *   外部可通过 props 注入预加载数据避免重复请求
 *
 * 时空布局：
 *   X = -3  过去 ─────────── 现在 ─────────── 未来  X = +3
 *              ↓                ↓                 ↓
 *         过期事件（灰）    流动粒子（青）    待办亮点（黄）
 *
 * change-id: game-future-outlook-completion
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import * as THREE from 'three';
import { useFrame } from '@react-three/fiber';
import { Points, PointMaterial, Html } from '@react-three/drei';
import { game } from '@/lib/ipc';
import type { EventsAndTasks, RecentEvent, UpcomingBuilding } from '@/types/game';

/** 流动粒子总数（lowPerf 时减半） */
const FLOW_PARTICLE_BASE = 80;
/** 过期事件尾迹最大渲染数 */
const EXPIRED_TAIL_MAX = 8;
/** 待办亮点最大渲染数 */
const UPCOMING_BRIGHT_MAX = 8;

interface Props {
  /** 当前世界 ID；若提供则组件可自行调用 IPC API 加载缺省数据 */
  worldId?: string;
  /** 外部预加载的事件与任务（来自 `game.getEventsAndTasks()`） */
  events?: EventsAndTasks | null;
  /** 性能模式：减少粒子数 + 简化几何 */
  lowPerf?: boolean;
}

/** 过期事件尾迹：暗淡灰色粒子（淡出） */
function ExpiredTail({ events }: { events: RecentEvent[] }) {
  const list = events.slice(0, EXPIRED_TAIL_MAX);
  return (
    <group>
      {list.map((e, i) => {
        // 沿 X 轴 -3 → -1 分布（越早越靠左）
        const x = -3 + (i / Math.max(1, list.length - 1)) * 2;
        // 越早越暗淡（透明度递减）
        const opacity = 0.55 - (i / list.length) * 0.45;
        return (
          <group key={e.event_id} position={[x, 0, 0]}>
            {/* 灰色粒子球 */}
            <mesh>
              <sphereGeometry args={[0.08, 8, 8]} />
              <meshBasicMaterial color="#5a6478" transparent opacity={opacity} />
            </mesh>
            {/* 暗淡尾迹（小拖尾） */}
            <mesh position={[-0.1, 0, 0]} scale={[0.6, 0.4, 0.4]}>
              <sphereGeometry args={[0.08, 8, 8]} />
              <meshBasicMaterial color="#3a4250" transparent opacity={opacity * 0.5} />
            </mesh>
            {/* 事件标题 Html 标签（仅显示前 4 个，避免拥挤） */}
            {i < 4 && (
              <Html
                position={[0, 0.3, 0]}
                center
                distanceFactor={10}
                occlude={false}
                style={{
                  pointerEvents: 'none',
                  userSelect: 'none',
                  padding: '2px 5px',
                  background: 'rgba(10, 14, 20, 0.65)',
                  border: '1px solid #5a6478',
                  borderRadius: '3px',
                  color: '#9aa5b8',
                  fontFamily: 'var(--nt-font-mono, monospace)',
                  fontSize: '9px',
                  lineHeight: 1.3,
                  whiteSpace: 'nowrap',
                  opacity: 0.85,
                }}
              >
                <div style={{ maxWidth: '120px', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  {e.title}
                </div>
              </Html>
            )}
          </group>
        );
      })}
    </group>
  );
}

/** 待办事项亮点：黄色发光球（AdditiveBlending） */
function UpcomingBrights({ buildings }: { buildings: UpcomingBuilding[] }) {
  const list = buildings.slice(0, UPCOMING_BRIGHT_MAX);
  return (
    <group>
      {list.map((b, i) => {
        // 沿 X 轴 1 → 3 分布（越近未来越靠左）
        const x = 1 + (i / Math.max(1, list.length - 1)) * 2;
        return (
          <group key={b.building_id} position={[x, 0, 0]}>
            {/* 黄色发光球本体 */}
            <mesh>
              <sphereGeometry args={[0.12, 12, 12]} />
              <meshBasicMaterial
                color="#fbbf24"
                transparent
                opacity={0.95}
                blending={THREE.AdditiveBlending}
                depthWrite={false}
              />
            </mesh>
            {/* 外层光晕 */}
            <mesh>
              <sphereGeometry args={[0.22, 12, 12]} />
              <meshBasicMaterial
                color="#fbbf24"
                transparent
                opacity={0.35}
                blending={THREE.AdditiveBlending}
                depthWrite={false}
              />
            </mesh>
            {/* 待办名称 Html 标签 */}
            <Html
              position={[0, 0.4, 0]}
              center
              distanceFactor={10}
              occlude={false}
              style={{
                pointerEvents: 'none',
                userSelect: 'none',
                padding: '2px 6px',
                background: 'rgba(10, 14, 20, 0.78)',
                border: '1px solid #fbbf24',
                borderRadius: '4px',
                color: '#fbbf24',
                fontFamily: 'var(--nt-font-mono, monospace)',
                fontSize: '10px',
                lineHeight: 1.3,
                whiteSpace: 'nowrap',
                textShadow: '0 0 6px #fbbf24',
                boxShadow: '0 0 8px #fbbf2480',
              }}
            >
              <div style={{ maxWidth: '140px', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                {b.name}
              </div>
              <div style={{ fontSize: '9px', opacity: 0.85 }}>
                {b.progress.toFixed(1)}%
              </div>
            </Html>
          </group>
        );
      })}
    </group>
  );
}

/** 流动粒子流：useFrame 推进，沿 X 轴循环 */
function FlowStream({
  count,
  canBreakthrough,
}: {
  count: number;
  canBreakthrough: boolean;
}) {
  const pointsRef = useRef<THREE.Points>(null);

  // 粒子位置 + 颜色（一次性初始化）
  const { positions, colors } = useMemo(() => {
    const positions = new Float32Array(count * 3);
    const colors = new Float32Array(count * 3);
    // 颜色：可突破=青色脉冲，否则=洋红缓慢
    const baseColor = canBreakthrough
      ? new THREE.Color('#00f0ff')
      : new THREE.Color('#ff006e');
    const dimColor = new THREE.Color('#3a2030');
    for (let i = 0; i < count; i++) {
      // X 初始位置：-3 到 3 均匀分布
      positions[i * 3] = -3 + (i / count) * 6;
      positions[i * 3 + 1] = (Math.random() - 0.5) * 0.25;
      positions[i * 3 + 2] = (Math.random() - 0.5) * 0.25;
      // 颜色：越靠前（X 越大）越亮
      const t = i / count;
      const brightness = canBreakthrough ? 1 - t * 0.4 : 0.5 - t * 0.3;
      const c = dimColor.clone().lerp(baseColor, Math.max(0, brightness));
      colors[i * 3] = c.r;
      colors[i * 3 + 1] = c.g;
      colors[i * 3 + 2] = c.b;
    }
    return { positions, colors };
  }, [count, canBreakthrough]);

  // 帧动画：粒子向前流动
  useFrame((_, delta) => {
    if (!pointsRef.current) return;
    const positions = pointsRef.current.geometry.attributes.position.array as Float32Array;
    const speed = canBreakthrough ? 1.2 : 0.45;
    const max = 3;
    const min = -3;
    for (let i = 0; i < count; i++) {
      positions[i * 3] += delta * speed;
      // 超出右端 → 重置到左端
      if (positions[i * 3] > max) {
        positions[i * 3] = min;
      }
    }
    pointsRef.current.geometry.attributes.position.needsUpdate = true;
  });

  return (
    <Points ref={pointsRef} positions={positions} colors={colors} stride={3} frustumCulled={false}>
      <PointMaterial
        transparent
        vertexColors
        size={0.1}
        sizeAttenuation
        depthWrite={false}
        opacity={0.9}
        blending={canBreakthrough ? THREE.AdditiveBlending : THREE.NormalBlending}
      />
    </Points>
  );
}

export function EventsFlowScene({
  worldId,
  events: eventsProp,
  lowPerf = false,
}: Props) {
  // 内部状态：仅在 props 未提供时通过 IPC API 加载
  const [eventsState, setEventsState] = useState<EventsAndTasks | null>(eventsProp ?? null);

  // props 同步
  useEffect(() => {
    if (eventsProp !== undefined) setEventsState(eventsProp ?? null);
  }, [eventsProp]);

  // 数据加载：调用 `game.getEventsAndTasks()` API（spec 要求）
  useEffect(() => {
    if (!worldId) return;
    if (eventsProp !== undefined) return;
    let cancelled = false;
    const load = async () => {
      try {
        const res = await game.getEventsAndTasks(worldId);
        if (!cancelled && res.code === 0) {
          setEventsState(res.data ?? null);
        }
      } catch {
        // 静默失败
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [worldId, eventsProp]);

  const events = eventsState;
  const canBreakthrough = events?.breakthrough_hint?.can_breakthrough ?? false;
  const recentEvents = events?.recent_events ?? [];
  const upcomingBuildings = events?.upcoming_buildings ?? [];

  // 流动粒子数量
  const flowCount = lowPerf
    ? Math.floor(FLOW_PARTICLE_BASE / 2)
    : Math.min(
        200,
        FLOW_PARTICLE_BASE + (recentEvents.length + upcomingBuildings.length) * 8,
      );

  // 中心轨道颜色
  const trackColor = canBreakthrough ? '#00f0ff' : '#5a6478';

  return (
    <group>
      {/* 中心轨道线（thin cylinder 沿 X 轴，避免 <line> SVG 类型冲突） */}
      <mesh position={[0, 0, 0]} rotation={[0, 0, Math.PI / 2]}>
        <cylinderGeometry args={[0.02, 0.02, 6, 8]} />
        <meshBasicMaterial color={trackColor} transparent opacity={0.35} />
      </mesh>

      {/* 过期事件尾迹（左侧，灰色淡出） */}
      <ExpiredTail events={recentEvents} />

      {/* 流动粒子流（中部） */}
      <FlowStream count={flowCount} canBreakthrough={canBreakthrough} />

      {/* 待办事项亮点（右侧，黄色发光 AdditiveBlending） */}
      <UpcomingBrights buildings={upcomingBuildings} />

      {/* 左端入口标记（事件源） */}
      <mesh position={[-3, 0, 0]}>
        <boxGeometry args={[0.1, 0.5, 0.5]} />
        <meshBasicMaterial color={trackColor} transparent opacity={0.6} />
      </mesh>
      {/* 右端出口标记（事件汇/未来） */}
      <mesh position={[3, 0, 0]}>
        <boxGeometry args={[0.1, 0.5, 0.5]} />
        <meshBasicMaterial
          color="#fbbf24"
          transparent
          opacity={0.85}
          blending={THREE.AdditiveBlending}
          depthWrite={false}
        />
      </mesh>

      {/* 突破可触发时的中心脉冲球 */}
      {canBreakthrough && (
        <mesh position={[0, 0, 0]}>
          <sphereGeometry args={[0.15, 16, 16]} />
          <meshBasicMaterial
            color="#00f0ff"
            transparent
            opacity={0.8}
            blending={THREE.AdditiveBlending}
            depthWrite={false}
          />
        </mesh>
      )}
    </group>
  );
}
