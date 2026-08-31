/**
 * Buildings - 建筑渲染层（Task 9.1 / 9.2 / 9.4）
 *
 * 遍历 gameStore.buildings，为每栋建筑渲染 <Building />。
 * 传入 selected 标志（用于高亮选中环）。
 * v1 使用程序化建筑（USE_PROCEDURAL_FALLBACK=true），无 GLB 依赖。
 *
 * 参考 05_3D场景设计.md §3.1 场景组件树 <Buildings> 节点
 *
 * T1.6 时间轴回放（11_时间轴回放.md §4.3）：
 *   - replayMode=true 时从 replayCurrentScene.buildings 渲染（BuildingSnapshot[]）
 *   - 回放模式下渲染为只读 <ReplayBuilding>（无点击交互、无放置/建造动画）
 *   - replayMode=false 时维持原 <Building> 渲染逻辑
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useGame3DStore } from '../stores/gameStore'
import { Building } from './Building'
import { ReplayBuilding } from './ReplayBuilding'
import { InstancedBuildings } from './InstancedBuildings'
import type { BuildingSnapshot } from '@/types/game'
import type { Building3D } from '../stores/gameStore'

/** 从 BuildingSnapshot 构造 Building3D（ReplayBuilding 内部需要的字段集）。 */
function snapshotToBuilding3D(s: BuildingSnapshot): Building3D {
  return {
    id: s.id,
    // BuildingSnapshot 未存 category，按 subtype 推断（house/town/...）
    category: inferCategoryFromSubtype(s.building_subtype),
    subtype: s.building_subtype,
    name: s.name,
    level: s.level,
    position: [s.pos_x, s.pos_y, s.pos_z],
    rotationY: s.rotation_y,
    status: s.status,
    buildProgress: s.build_progress,
    knowledgeDomain: s.knowledge_domain,
  }
}

/** 从 building_subtype 字符串推断建筑大类（粗略映射，仅供回放渲染分类用）。 */
function inferCategoryFromSubtype(subtype: string): Building3D['category'] {
  // subtype 命名约定参考 BuildingCatalog：thatch_cottage / wooden_house / village / ...
  if (subtype.includes('cottage') || subtype.includes('house') || subtype.includes('hut')) return 'house'
  if (subtype.includes('village') || subtype.includes('town')) return 'town'
  if (subtype.includes('city') || subtype.includes('fort')) return 'city'
  if (subtype.includes('kingdom') || subtype.includes('castle')) return 'kingdom'
  if (subtype.includes('palace')) return 'palace'
  if (subtype.includes('tech') || subtype.includes('tower')) return 'technology'
  if (subtype.includes('sect') || subtype.includes('temple')) return 'sect'
  return 'immortal'
}

export function Buildings() {
  const buildings = useGame3DStore((s) => s.buildings)
  const selectedBuildingId = useGame3DStore((s) => s.selectedBuildingId)
  const replayMode = useGame3DStore((s) => s.replayMode)
  const replayCurrentScene = useGame3DStore((s) => s.replayCurrentScene)

  // ===== T1.6 回放模式：从 replayCurrentScene.buildings 渲染 =====
  if (replayMode && replayCurrentScene) {
    return (
      <group>
        {replayCurrentScene.buildings.map((s) => (
          <ReplayBuilding
            key={s.id}
            building={snapshotToBuilding3D(s)}
          />
        ))}
      </group>
    )
  }

  // ===== 实时模式：T4.1 高密度场景实例化分支（≥100 建筑启用） =====
  // 已完成建筑（completed）走 InstancedBuildings（drawcall ~16 vs 原 800）
  // 其他状态（building/planning/ruined）数量较少，仍走完整 <Building>
  const INSTANCE_THRESHOLD = 100
  const useInstances = buildings.length >= INSTANCE_THRESHOLD

  if (useInstances) {
    const completed = buildings.filter((b) => b.status === 'completed')
    const nonCompleted = buildings.filter((b) => b.status !== 'completed')
    return (
      <group>
        <InstancedBuildings buildings={completed} />
        {nonCompleted.map((b) => (
          <Building
            key={b.id}
            building={b}
            selected={b.id === selectedBuildingId}
          />
        ))}
      </group>
    )
  }

  // ===== 实时模式：常规渲染（<100 建筑） =====
  return (
    <group>
      {buildings.map((b) => (
        <Building
          key={b.id}
          building={b}
          selected={b.id === selectedBuildingId}
        />
      ))}
    </group>
  )
}
