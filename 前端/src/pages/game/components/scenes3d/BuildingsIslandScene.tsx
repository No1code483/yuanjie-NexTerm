/**
 * BuildingsIslandScene - 建筑鸟瞰岛（文档06 §4.2.2 P2）
 *
 * 表达：建筑集合的空间分布鸟瞰
 *   - 微型地形（圆形岛 + 微噪声高度）
 *   - InstancedMesh 实例化所有建筑（一实例一建筑），按状态着色
 *   - 建筑位置由 pos_x/pos_z 映射到岛面（缩放至 ±2.5 单位）
 *   - 建筑高度 = level * 0.3 + 0.4
 *
 * 性能：InstancedMesh 一次 draw call，支持百级建筑无压力。
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo, useRef } from 'react';
import * as THREE from 'three';
import type { GameBuilding } from '@/types/game';

/** 建筑状态 → 颜色（终端黑客青/警示红/暖黄/暗灰） */
const STATUS_COLORS: Record<string, THREE.Color> = {
  completed: new THREE.Color('#00f0ff'), // 青色（主色）
  building: new THREE.Color('#fbbf24'), // 暖黄（建造中）
  planning: new THREE.Color('#5a6478'), // 暗灰（规划中）
  ruined: new THREE.Color('#ff006e'), // 警示红（废弃）
};

interface Props {
  buildings: GameBuilding[];
}

export function BuildingsIslandScene({ buildings }: Props) {
  const meshRef = useRef<THREE.InstancedMesh>(null);

  // 实例化矩阵：每建筑一个 4x4 变换矩阵
  const instanceData = useMemo(() => {
    const list: { matrix: THREE.Matrix4; color: THREE.Color }[] = [];
    const dummy = new THREE.Object3D();
    for (const b of buildings) {
      // 坐标映射：游戏世界 0~32 → 岛面 ±2.5
      const x = ((b.pos_x ?? 16) - 16) / 16 * 2.5;
      const z = ((b.pos_z ?? 16) - 16) / 16 * 2.5;
      const height = b.level * 0.3 + 0.4;
      dummy.position.set(x, height / 2, z);
      dummy.rotation.y = b.rotation_y ?? 0;
      dummy.scale.set(0.25, height, 0.25);
      dummy.updateMatrix();
      list.push({
        matrix: dummy.matrix.clone(),
        color: STATUS_COLORS[b.status] ?? STATUS_COLORS.planning,
      });
    }
    return list;
  }, [buildings]);

  // 应用实例矩阵 + 颜色
  useMemo(() => {
    if (!meshRef.current || instanceData.length === 0) return;
    instanceData.forEach((data, i) => {
      meshRef.current!.setMatrixAt(i, data.matrix);
      meshRef.current!.setColorAt(i, data.color);
    });
    meshRef.current.instanceMatrix.needsUpdate = true;
    if (meshRef.current.instanceColor) meshRef.current.instanceColor.needsUpdate = true;
  }, [instanceData]);

  return (
    <group>
      {/* 微型地形：圆形岛 + 微噪声 */}
      <mesh position={[0, -0.05, 0]} receiveShadow>
        <cylinderGeometry args={[3, 3.2, 0.1, 32]} />
        <meshStandardMaterial
          color="#1a2230"
          roughness={0.85}
          metalness={0.15}
          emissive="#0a1018"
          emissiveIntensity={0.3}
        />
      </mesh>
      {/* 岛边缘发光环 */}
      <mesh position={[0, 0.01, 0]} rotation={[-Math.PI / 2, 0, 0]}>
        <ringGeometry args={[3, 3.15, 32]} />
        <meshBasicMaterial color="#00f0ff" transparent opacity={0.4} side={THREE.DoubleSide} />
      </mesh>

      {/* 建筑实例（无建筑时渲染 1 个占位实例避免警告） */}
      <instancedMesh
        ref={meshRef}
        args={[undefined, undefined, Math.max(1, instanceData.length)]}
        castShadow
      >
        <boxGeometry args={[1, 1, 1]} />
        <meshStandardMaterial
          vertexColors
          roughness={0.4}
          metalness={0.6}
          emissiveIntensity={0.4}
          emissive={'#003040'}
        />
      </instancedMesh>

      {/* 空状态提示（无建筑时） */}
      {buildings.length === 0 && (
        <mesh position={[0, 0.3, 0]}>
          <sphereGeometry args={[0.15, 16, 16]} />
          <meshBasicMaterial color="#5a6478" />
        </mesh>
      )}
    </group>
  );
}
