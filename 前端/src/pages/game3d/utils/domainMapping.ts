/**
 * domainMapping - 建筑大类 → 默认知识领域映射（v1 简化）
 *
 * 04 文档 §2.1 / 03 文档 §1 的领域-建筑对应关系：
 *   house       → other       （凡人起居，无特定领域）
 *   town        → economics    （集市/酒馆/学堂等经济活动）
 *   city        → history      （城池是历史载体）
 *   kingdom     → history      （王国是政治历史实体）
 *   palace      → art          （宫殿承载艺术文化）
 *   technology  → engineering  （科技=工程）
 *   sect        → philosophy   （宗门=哲学思辨）
 *   immortal    → philosophy   （修仙=哲学终极）
 *
 * v2 可由用户在预建造面板手动选择领域
 *
 * change-id: game-3d-rebuild-refactor
 */
import type { GameBuildingCategory, GameKnowledgeDomainId } from '@/types/game'

/** 建筑大类 → 默认知识领域 */
export const CATEGORY_DEFAULT_DOMAIN: Record<GameBuildingCategory, GameKnowledgeDomainId> = {
  house: 'other',
  town: 'economics',
  city: 'history',
  kingdom: 'history',
  palace: 'art',
  technology: 'engineering',
  sect: 'philosophy',
  immortal: 'philosophy',
}
