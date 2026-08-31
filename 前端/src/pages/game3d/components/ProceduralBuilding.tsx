/**
 * ProceduralBuilding - 程序化建筑分发器
 *
 * 根据 building_category 分发到对应的中式修仙风格建筑组件：
 *   - house       → House3D      凡人茅屋（双坡茅草顶）
 *   - town        → Town3D       集镇铺面（悬山顶+灯笼）
 *   - city        → City3D       城楼（城墙带垛口+歇山顶）
 *   - kingdom     → Kingdom3D    凡人宫殿（重檐庑殿顶+红墙+斗拱）
 *   - palace      → Palace3D     仙家宫殿（金色宝顶+飞檐翘角+灵气）
 *   - technology  → Technology3D 炼丹坊（八卦炉+烟囱+火焰光晕）
 *   - sect        → Sect3D       宗门山门（三间四柱牌楼+飞檐+斗拱）
 *   - immortal    → Immortal3D   七层宝塔（相轮宝顶+莲花座+祥云）
 *
 * 阶段6（v1）：程序化几何体 fallback，无 GLB 依赖
 * 阶段9+：将支持加载真实 GLB 模型（USE_PROCEDURAL_FALLBACK=false 时切换）
 *
 * change-id: game-3d-rebuild-refactor
 */
import type { GameBuildingCategory, GameBuildingStatus } from '@/types/game'
import { USE_PROCEDURAL_FALLBACK } from '../utils/modelPaths'
import { House3D } from './buildings/House3D'
import { Town3D } from './buildings/Town3D'
import { City3D } from './buildings/City3D'
import { Kingdom3D } from './buildings/Kingdom3D'
import { Palace3D } from './buildings/Palace3D'
import { Technology3D } from './buildings/Technology3D'
import { Sect3D } from './buildings/Sect3D'
import { Immortal3D } from './buildings/Immortal3D'

interface ProceduralBuildingProps {
  category: GameBuildingCategory
  position: [number, number, number]
  rotationY?: number
  status?: GameBuildingStatus
  buildProgress?: number
  level?: number
}

/**
 * 程序化建筑分发器
 *
 * 按 category 调用对应的建筑组件，统一对外接口。
 * 后续引入真实 GLB 时，在此处根据 USE_PROCEDURAL_FALLBACK 切换：
 *   - true  → 使用程序化建筑组件（当前实现）
 *   - false → 使用 useGLTF 加载真实模型（阶段9+ 实现）
 */
export function ProceduralBuilding({
  category,
  position,
  rotationY = 0,
  status = 'completed',
  buildProgress = 100,
  level = 1,
}: ProceduralBuildingProps) {
  // USE_PROCEDURAL_FALLBACK 当前恒为 true，预留切换点
  void USE_PROCEDURAL_FALLBACK

  const commonProps = {
    position,
    rotationY,
    status,
    buildProgress,
    level,
  }

  switch (category) {
    case 'house':
      return <House3D {...commonProps} />
    case 'town':
      return <Town3D {...commonProps} />
    case 'city':
      return <City3D {...commonProps} />
    case 'kingdom':
      return <Kingdom3D {...commonProps} />
    case 'palace':
      return <Palace3D {...commonProps} />
    case 'technology':
      return <Technology3D {...commonProps} />
    case 'sect':
      return <Sect3D {...commonProps} />
    case 'immortal':
      return <Immortal3D {...commonProps} />
    default:
      // 兜底：house
      return <House3D {...commonProps} />
  }
}
