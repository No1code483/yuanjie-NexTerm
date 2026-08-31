/**
 * RealmTowerScene - 境界塔（文档06 §4.2.2 P2）
 *
 * 表达：10 大境界层数塔 + 当前层高亮 + 小境界进度环
 *   - 10 层圆柱堆叠（凡人→仙），每层高度 0.4，半径递减
 *   - 当前境界层高亮（青色发光）+ 其他层暗灰
 *   - 当前层外环绕进度环（按 realm_minor: early=33% / middle=66% / complete=100%）
 *   - 道基颜色染色塔顶宝石（白/蓝/红/紫/黑）
 *
 * 数据源：RealmInfo
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useMemo } from 'react';
import * as THREE from 'three';
import { Text } from '@react-three/drei';
import type { RealmInfo, GameDaoFoundation } from '@/types/game';

/** 道基 → 顶宝石颜色 */
const DAO_COLORS: Record<GameDaoFoundation, string> = {
  white: '#e8ecf2',
  blue: '#3b82f6',
  red: '#ef4444',
  purple: '#a855f7',
  black: '#1a1a2e',
};

/** 小境界 → 进度百分比 */
const MINOR_PROGRESS: Record<string, number> = {
  early: 0.33,
  middle: 0.66,
  complete: 1.0,
};

interface Props {
  realmInfo: RealmInfo | null;
}

const REALM_NAMES = ['凡人', '练气', '筑基', '金丹', '元婴', '化神', '合体', '大乘', '渡劫', '仙'];

export function RealmTowerScene({ realmInfo }: Props) {
  const currentOrdinal = realmInfo?.realm_ordinal ?? 1; // 1-10
  const minorProgress = realmInfo
    ? MINOR_PROGRESS[realmInfo.realm_minor] ?? 0.33
    : 0;

  // 10 层数据：层序号、是否当前层、是否已通过
  const layers = useMemo(() => {
    return Array.from({ length: 10 }, (_, i) => {
      const ordinal = i + 1;
      return {
        ordinal,
        y: i * 0.42, // 每层 0.42 高度
        radius: 0.6 - i * 0.035, // 半径递减
        isCurrent: ordinal === currentOrdinal,
        isPassed: ordinal < currentOrdinal,
      };
    });
  }, [currentOrdinal]);

  return (
    <group>
      {/* 塔基 */}
      <mesh position={[0, -0.15, 0]} receiveShadow>
        <cylinderGeometry args={[0.8, 1.0, 0.3, 8]} />
        <meshStandardMaterial color="#2a3140" roughness={0.7} metalness={0.3} />
      </mesh>

      {/* 10 层塔身 */}
      {layers.map(layer => (
        <mesh
          key={layer.ordinal}
          position={[0, layer.y, 0]}
          castShadow
        >
          <cylinderGeometry args={[layer.radius, layer.radius + 0.05, 0.4, 8]} />
          <meshStandardMaterial
            color={layer.isCurrent ? '#00f0ff' : layer.isPassed ? '#4a6478' : '#2a3140'}
            emissive={layer.isCurrent ? '#00f0ff' : '#000000'}
            emissiveIntensity={layer.isCurrent ? 0.6 : 0}
            roughness={0.4}
            metalness={0.5}
            transparent
            opacity={layer.isPassed ? 0.7 : 1.0}
          />
        </mesh>
      ))}

      {/* 当前层进度环（围绕当前层旋转） */}
      {realmInfo && (
        <mesh
          position={[0, layers[currentOrdinal - 1]?.y ?? 0, 0]}
          rotation={[-Math.PI / 2, 0, 0]}
        >
          <ringGeometry args={[
            layers[currentOrdinal - 1].radius + 0.08,
            layers[currentOrdinal - 1].radius + 0.18,
            32,
            1,
            0,
            minorProgress * Math.PI * 2
          ]} />
          <meshBasicMaterial color="#00f0ff" transparent opacity={0.8} side={THREE.DoubleSide} />
        </mesh>
      )}

      {/* 塔顶尖晶（道基色） */}
      <mesh position={[0, 10 * 0.42 + 0.2, 0]} castShadow>
        <octahedronGeometry args={[0.18, 0]} />
        <meshStandardMaterial
          color={DAO_COLORS[realmInfo?.dao_foundation ?? 'white']}
          emissive={DAO_COLORS[realmInfo?.dao_foundation ?? 'white']}
          emissiveIntensity={0.5}
          metalness={0.8}
          roughness={0.2}
        />
      </mesh>

      {/* 当前境界文字标签 */}
      {realmInfo && (
        <Text
          position={[0, layers[currentOrdinal - 1].y + 0.4, layers[currentOrdinal - 1].radius + 0.3]}
          fontSize={0.18}
          color="#00f0ff"
          anchorX="center"
          anchorY="middle"
        >
          {REALM_NAMES[currentOrdinal - 1]}
        </Text>
      )}
    </group>
  );
}
