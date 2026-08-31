import { t } from "i18next";
/**
 * BreakthroughAltar - 3D 发光祭坛（T2.2，12_AI考验机制.md §3 入口）
 *
 * 触发条件（02_境界系统设计.md §6）：境界圆满（realm_minor='complete'）+
 * 修为达当前大境界 XP 上限 + 无 24h 冷却 → breakthroughPreview.can_breakthrough=true。
 *
 * 显示位置：场景中心 (0, 0, 0) 略上方，独立于建筑层（Buildings.tsx）。
 *
 * 交互：点击祭坛 → 调用 `game.startBreakthrough(worldId)` 拉取会话 →
 *      `setBreakthroughSession(session)` 写入 store，由 Game3D.tsx 顶层 UI 接管渲染答题页。
 *
 * 视觉（终端黑客风：黑底 + 荧光绿 #00FF00）：
 *   - 八角形金属底座（黑色 + 微绿色自发光）
 *   - 中心悬浮绿色发光宝珠（上下浮动）
 *   - 双反向旋转的绿色光环
 *   - 8 个循环上升的能量粒子（环绕祭坛）
 *   - hover 显示"突破祭坛"标牌 + 整体放大
 *
 * 仅在非回放模式 + 数据就绪 + can_breakthrough=true 时渲染（由 Scene.tsx 控制条件挂载）。
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo, useRef, useState } from 'react';
import { Html } from '@react-three/drei';
import { useFrame, type ThreeEvent } from '@react-three/fiber';
import * as THREE from 'three';
import { game } from '@/lib/ipc';
import { useGame3DStore } from '../stores/gameStore';

/** 八角底座半径 */
const BASE_RADIUS = 1.2;
/** 宝珠悬浮高度 */
const ORB_HEIGHT = 2.0;
export function BreakthroughAltar() {
  const preview = useGame3DStore(s => s.breakthroughPreview);
  const world = useGame3DStore(s => s.world);
  const loading = useGame3DStore(s => s.breakthroughLoading);
  const replayMode = useGame3DStore(s => s.replayMode);
  const setSession = useGame3DStore(s => s.setBreakthroughSession);
  const setLoading = useGame3DStore(s => s.setBreakthroughLoading);
  const [hovered, setHovered] = useState(false);
  const groupRef = useRef<THREE.Group>(null);
  const orbRef = useRef<THREE.Mesh>(null);
  const ring1Ref = useRef<THREE.Mesh>(null);
  const ring2Ref = useRef<THREE.Mesh>(null);
  const particlesRef = useRef<THREE.Group>(null);
  const startRef = useRef(performance.now() / 1000);

  // 预生成 8 个粒子的圆周位置 + 错峰相位
  const particles = useMemo(() => Array.from({
    length: 8
  }, (_, i) => {
    const angle = i / 8 * Math.PI * 2;
    return {
      offsetX: Math.cos(angle) * BASE_RADIUS,
      offsetZ: Math.sin(angle) * BASE_RADIUS,
      phase: i / 8 * 2 // 0~2 秒错峰
    };
  }), []);

  // 仅当可突破时显示（preview 由 UI 层预先拉取并写入 store）
  if (!preview?.can_breakthrough) return null;
  // 回放模式下不显示祭坛（避免与历史场景冲突）
  if (replayMode) return null;

  /** 点击祭坛：拉取突破会话 */
  const handleClick = async (e: ThreeEvent<MouseEvent>) => {
    e.stopPropagation();
    if (loading || !world) return;
    setLoading(true);
    try {
      const res = await game.startBreakthrough(world.id);
      if (res.code !== 0 || !res.data) {
        console.error('[BreakthroughAltar] startBreakthrough failed:', res.message);
        return;
      }
      setSession(res.data);
    } catch (err) {
      console.error('[BreakthroughAltar] startBreakthrough error:', err);
    } finally {
      setLoading(false);
    }
  };
  useFrame(() => {
    const now = performance.now() / 1000;
    const t = now - startRef.current;

    // 中心宝珠上下浮动 + 缓慢自转
    if (orbRef.current) {
      orbRef.current.position.y = ORB_HEIGHT + Math.sin(t * 1.5) * 0.3;
      orbRef.current.rotation.y = t * 0.5;
    }
    // 双环反向旋转
    if (ring1Ref.current) ring1Ref.current.rotation.z = t * 0.8;
    if (ring2Ref.current) ring2Ref.current.rotation.z = -t * 0.6;
    // hover 时整体放大插值
    if (groupRef.current) {
      const targetScale = hovered ? 1.15 : 1.0;
      const cur = groupRef.current.scale.x;
      const next = cur + (targetScale - cur) * 0.1;
      groupRef.current.scale.set(next, next, next);
    }
    // 粒子上升循环（2 秒周期，到顶后透明度归零重新开始）
    if (particlesRef.current) {
      particlesRef.current.children.forEach((child, i) => {
        if (i >= particles.length) return;
        const p = particles[i];
        const cycle = (t + p.phase) % 2;
        const mesh = child as THREE.Mesh;
        mesh.position.y = cycle * 2.5;
        const mat = mesh.material as THREE.MeshBasicMaterial;
        // 前 80% 渐显，后 20% 渐隐
        mat.opacity = cycle < 1.6 ? cycle / 1.6 * 0.9 : Math.max(0, (2 - cycle) / 0.4 * 0.9);
      });
    }
  });
  return <group ref={groupRef} position={[0, 0, 0]} onClick={handleClick} onPointerOver={e => {
    e.stopPropagation();
    setHovered(true);
    document.body.style.cursor = 'pointer';
  }} onPointerOut={() => {
    setHovered(false);
    document.body.style.cursor = 'default';
  }}>
      {/* 八角形金属底座（黑色 + 微绿色自发光） */}
      <mesh position={[0, 0.1, 0]} receiveShadow castShadow>
        <cylinderGeometry args={[BASE_RADIUS, BASE_RADIUS + 0.2, 0.2, 8]} />
        <meshStandardMaterial color="#0a0a0a" metalness={0.8} roughness={0.3} emissive="#00FF00" emissiveIntensity={0.15} />
      </mesh>

      {/* 底座顶部内嵌的发光符文圆盘 */}
      <mesh position={[0, 0.21, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[0.4, 0.9, 8]} />
        <meshBasicMaterial color="#00FF00" transparent opacity={0.4} side={THREE.DoubleSide} />
      </mesh>

      {/* 中心悬浮发光宝珠（自发光，无光照影响） */}
      <mesh ref={orbRef} position={[0, ORB_HEIGHT, 0]} castShadow>
        <icosahedronGeometry args={[0.5, 1]} />
        <meshBasicMaterial color="#00FF00" transparent opacity={0.95} />
      </mesh>

      {/* 双反向旋转光环 */}
      <mesh ref={ring1Ref} position={[0, ORB_HEIGHT, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <torusGeometry args={[1.0, 0.04, 8, 32]} />
        <meshBasicMaterial color="#00FF00" transparent opacity={0.7} />
      </mesh>
      <mesh ref={ring2Ref} position={[0, ORB_HEIGHT, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <torusGeometry args={[1.3, 0.03, 8, 32]} />
        <meshBasicMaterial color="#00FF00" transparent opacity={0.5} />
      </mesh>

      {/* 8 个上升能量粒子（独立 group 包裹，避免与其它子物体索引混淆） */}
      <group ref={particlesRef}>
        {particles.map((p, i) => <mesh key={i} position={[p.offsetX, 0, p.offsetZ]}>
            <sphereGeometry args={[0.06, 6, 6]} />
            <meshBasicMaterial color="#00FF00" transparent opacity={0.8} />
          </mesh>)}
      </group>

      {/* hover / loading 标牌 */}
      <Html position={[0, ORB_HEIGHT + 1.5, 0]} center distanceFactor={12} zIndexRange={[10, 0]}>
        <div className={`game3d-altar-tag ${hovered ? 'is-hovered' : ''}`}>
          {loading ? t("game3d.components.BreakthroughAltar.k1") : t("game3d.components.BreakthroughAltar.k2")}
        </div>
      </Html>
    </group>;
}