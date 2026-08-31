/**
 * KnowledgeStarMapScene - 知识星图（P3 升级版，game-future-outlook-completion Task 2.2）
 *
 * 表达：12 知识领域成圆环恒星分布，亮度=积分，轨道粒子=近 7 天趋势
 *   - 12 颗恒星圆环分布（半径 2.5），每颗对应一个知识领域
 *     （cs / math / physics / literature / history / art /
 *      engineering / medicine / philosophy / economics / language / other）
 *   - 恒星亮度 = 当前积分（emissiveIntensity ∝ log10(points+1)）
 *   - 恒星大小 = log10(points+1) * 0.18 + 0.10
 *   - 颜色按等级梯度：Lv1-2 灰白 / Lv3-4 青色 / Lv5-6 紫色 / Lv7-8 金色
 *   - 轨道粒子（drei `<Points>`）数量 = 近 7 天该领域每日积分增量的总和（上限 128）
 *   - 使用 drei `<Html>` 标签显示领域名 + 积分 + Lv
 *
 * 数据来源：
 *   - `game.getKnowledgeProgress(worldId)` API（从 `@/lib/ipc` 导入）
 *   - `game.getKnowledgeDomains()` API
 *   - `game.getPointsTrend(worldId, 7)` API（7 天趋势）
 *   - 外部可通过 props 注入预加载数据避免重复请求
 *
 * change-id: game-future-outlook-completion
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import * as THREE from 'three';
import { useFrame } from '@react-three/fiber';
import { Html, Points, PointMaterial } from '@react-three/drei';
import { game } from '@/lib/ipc';
import type {
  GameKnowledgeDomain,
  GameKnowledgeProgress,
  PointsTrend,
} from '@/types/game';
import { GAME_DOMAIN_IDS } from '@/types/game';

/** 等级 → 恒星颜色 */
function levelToColor(level: number): string {
  if (level >= 7) return '#fbbf24'; // 金色
  if (level >= 5) return '#a855f7'; // 紫色
  if (level >= 3) return '#00f0ff'; // 青色
  return '#9aa5b8'; // 灰白
}

/** 由等级计算等级标签 */
function levelToLabel(level: number): string {
  return `Lv${level}`;
}

interface Props {
  /** 当前世界 ID；若提供则组件可自行调用 IPC API 加载缺省数据 */
  worldId?: string;
  /** 外部预加载的领域定义（避免重复请求） */
  domains?: GameKnowledgeDomain[];
  /** 外部预加载的积分进度（来自 `game.getKnowledgeProgress()`） */
  progress?: GameKnowledgeProgress[];
  /** 外部预加载的 7 天趋势（来自 `game.getPointsTrend(worldId, 7)`） */
  trend7d?: PointsTrend | null;
  /** 性能模式：关闭粒子 + 简化几何 */
  lowPerf?: boolean;
}

/** 单颗恒星 + 轨道粒子 + Html 标签 */
function Star({
  position,
  size,
  color,
  emissiveIntensity,
  label,
  points,
  level,
  orbitParticleCount,
  lowPerf,
}: {
  position: [number, number, number];
  size: number;
  color: string;
  emissiveIntensity: number;
  label: string;
  points: number;
  level: number;
  orbitParticleCount: number;
  lowPerf: boolean;
}) {
  const orbitRef = useRef<THREE.Points>(null);

  // 轨道粒子位置（圆环分布，加微噪声）
  const orbitPositions = useMemo(() => {
    const count = lowPerf ? 0 : Math.min(128, orbitParticleCount);
    if (count === 0) return new Float32Array(0);
    const arr = new Float32Array(count * 3);
    const orbitRadius = size + 0.25;
    for (let i = 0; i < count; i++) {
      const angle = (i / count) * Math.PI * 2;
      arr[i * 3] = Math.cos(angle) * orbitRadius;
      arr[i * 3 + 1] = (Math.random() - 0.5) * 0.08;
      arr[i * 3 + 2] = Math.sin(angle) * orbitRadius;
    }
    return arr;
  }, [orbitParticleCount, size, lowPerf]);

  // 旋转轨道粒子
  useFrame((_, delta) => {
    if (orbitRef.current) {
      orbitRef.current.rotation.y += delta * 0.4;
    }
  });

  // Html 标签：根据积分高低决定是否显示（避免低分领域标签拥挤）
  const showLabel = points > 0;

  return (
    <group position={position}>
      {/* 恒星本体（emissive 表达亮度=积分） */}
      <mesh>
        <sphereGeometry args={[size, 16, 16]} />
        <meshStandardMaterial
          color={color}
          emissive={color}
          emissiveIntensity={emissiveIntensity}
          roughness={0.3}
          metalness={0.4}
        />
      </mesh>
      {/* 恒星光晕（外层透明球） */}
      <mesh>
        <sphereGeometry args={[size * 1.7, 16, 16]} />
        <meshBasicMaterial color={color} transparent opacity={0.12} />
      </mesh>

      {/* 轨道粒子（drei <Points>，近 7 天趋势） */}
      {orbitPositions.length > 0 && (
        <Points ref={orbitRef} positions={orbitPositions} stride={3} frustumCulled={false}>
          <PointMaterial
            transparent
            color={color}
            size={0.06}
            sizeAttenuation
            depthWrite={false}
            opacity={0.85}
          />
        </Points>
      )}

      {/* Html 标签：领域名 + 积分 + Lv */}
      {showLabel && (
        <Html
          position={[0, size + 0.35, 0]}
          center
          distanceFactor={8}
          occlude={false}
          style={{
            pointerEvents: 'none',
            userSelect: 'none',
            padding: '2px 6px',
            background: 'rgba(10, 14, 20, 0.78)',
            border: `1px solid ${color}`,
            borderRadius: '4px',
            color: color,
            fontFamily: 'var(--nt-font-mono, monospace)',
            fontSize: '11px',
            lineHeight: 1.4,
            whiteSpace: 'nowrap',
            textShadow: `0 0 4px ${color}`,
            boxShadow: `0 0 8px ${color}40`,
          }}
        >
          <div style={{ fontWeight: 600 }}>{label}</div>
          <div style={{ fontSize: '10px', opacity: 0.85 }}>
            {points.toLocaleString()} · {levelToLabel(level)}
          </div>
        </Html>
      )}
    </group>
  );
}

export function KnowledgeStarMapScene({
  worldId,
  domains: domainsProp,
  progress: progressProp,
  trend7d: trend7dProp,
  lowPerf = false,
}: Props) {
  // 内部状态：仅在 props 未提供时通过 IPC API 加载
  const [domainsState, setDomainsState] = useState<GameKnowledgeDomain[] | null>(
    domainsProp ?? null,
  );
  const [progressState, setProgressState] = useState<GameKnowledgeProgress[] | null>(
    progressProp ?? null,
  );
  const [trend7dState, setTrend7dState] = useState<PointsTrend | null>(trend7dProp ?? null);

  // 当 props 变化时同步到内部状态
  useEffect(() => {
    if (domainsProp !== undefined) setDomainsState(domainsProp);
  }, [domainsProp]);
  useEffect(() => {
    if (progressProp !== undefined) setProgressState(progressProp);
  }, [progressProp]);
  useEffect(() => {
    if (trend7dProp !== undefined) setTrend7dState(trend7dProp ?? null);
  }, [trend7dProp]);

  // 数据加载：调用 `game.getKnowledgeProgress()` API 等（spec 要求）
  useEffect(() => {
    if (!worldId) return;
    let cancelled = false;

    const load = async () => {
      try {
        // 进度数据：spec 明确要求调用 game.getKnowledgeProgress()
        if (progressProp === undefined) {
          const res = await game.getKnowledgeProgress(worldId);
          if (!cancelled && res.code === 0) {
            setProgressState(res.data ?? []);
          }
        }
        // 领域定义
        if (domainsProp === undefined) {
          const res = await game.getKnowledgeDomains();
          if (!cancelled && res.code === 0) {
            setDomainsState(res.data ?? []);
          }
        }
        // 7 天趋势：spec 要求"轨道粒子近 7 天趋势"
        if (trend7dProp === undefined) {
          const res = await game.getPointsTrend(worldId, 7);
          if (!cancelled && res.code === 0) {
            setTrend7dState(res.data ?? null);
          }
        }
      } catch {
        // 静默失败：保持 null，下游用空数组兜底
      }
    };

    void load();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [worldId]);

  // 兜底：若无领域数据，使用 GAME_DOMAIN_IDS 静态字典生成占位
  const domains = useMemo<GameKnowledgeDomain[]>(() => {
    if (domainsState && domainsState.length > 0) return domainsState;
    return GAME_DOMAIN_IDS.map((id, i) => ({
      id,
      name: id,
      name_en: id,
      description: '',
      building_category: 'house' as const,
      sort_order: i,
    }));
  }, [domainsState]);

  const progress = progressState ?? [];
  const trend7d = trend7dState;

  // 计算 12 颗恒星布局
  const stars = useMemo(() => {
    const progressMap = new Map<string, GameKnowledgeProgress>();
    for (const p of progress) progressMap.set(p.domain_id, p);

    return domains.map((d, i) => {
      const angle = (i / domains.length) * Math.PI * 2;
      const radius = 2.5;
      const p = progressMap.get(d.id);
      const points = p?.points ?? 0;
      const level = p?.level ?? 1;
      const size = Math.log10(points + 1) * 0.18 + 0.10;
      const color = levelToColor(level);
      // 亮度（emissiveIntensity）= 当前积分对数
      const emissiveIntensity = Math.min(2.5, Math.log10(points + 1) * 0.6 + 0.3);
      // 近 7 天该领域每日积分增量总和 → 轨道粒子数
      const trendArr = trend7d?.trend?.[d.id] ?? [];
      const trendSum = trendArr.reduce((a, b) => a + Math.max(0, b), 0);
      const orbitParticleCount = Math.min(128, Math.floor(trendSum * 2));

      return {
        id: d.id,
        name: d.name,
        position: [Math.cos(angle) * radius, 0, Math.sin(angle) * radius] as [
          number,
          number,
          number,
        ],
        size,
        color,
        emissiveIntensity,
        points,
        level,
        orbitParticleCount,
      };
    });
  }, [domains, progress, trend7d]);

  return (
    <group>
      {/* 中心连接线（圆环虚化） */}
      <mesh rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[2.45, 2.55, 64]} />
        <meshBasicMaterial color="#1a2230" transparent opacity={0.6} side={THREE.DoubleSide} />
      </mesh>

      {/* 12 颗恒星 */}
      {stars.map(s => (
        <Star
          key={s.id}
          position={s.position}
          size={s.size}
          color={s.color}
          emissiveIntensity={s.emissiveIntensity}
          label={s.name}
          points={s.points}
          level={s.level}
          orbitParticleCount={s.orbitParticleCount}
          lowPerf={lowPerf}
        />
      ))}

      {/* 中心节点（玩家本体） */}
      <mesh>
        <sphereGeometry args={[0.12, 16, 16]} />
        <meshBasicMaterial color="#00f0ff" />
      </mesh>
      <mesh>
        <sphereGeometry args={[0.18, 16, 16]} />
        <meshBasicMaterial color="#00f0ff" transparent opacity={0.2} />
      </mesh>
    </group>
  );
}
