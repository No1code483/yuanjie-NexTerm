import { t } from "i18next";
/**
 * BuildingDetail - 建筑详情面板（Task 9.4 / 9.5）
 *
 * UI 形态：右侧滑出面板
 *
 * 显示内容：
 *   - 建筑名称 + 类别 + 子类
 *   - 等级 + 状态 + 建造进度
 *   - 关联知识领域
 *   - 3D 坐标
 *   - 升级/拆除/移动按钮
 *
 * IPC 调用：
 *   - 升级：game.upgradeBuilding(id, upgradeCost)
 *     upgradeCost = baseCost * (1 + level * 0.5)（升到 level+1，对齐后端 building_required_points）
 *   - 拆除：game.removeBuilding(id) → 返还 50% 积分
 *   - 移动：store.startMoveMode(id) → 进入移动模式（PlacementController 处理）
 *
 * cost 来源：store.catalogCache（BuildPanel 加载后缓存）
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useCallback, useMemo, useState } from 'react';
import { game } from '@/lib/ipc';
import type { GameBuildingStatus, GameKnowledgeDomainId } from '@/types/game';
import { useGame3DStore } from '../../stores/gameStore';

/** 建筑大类中文名 */
const CATEGORY_LABELS: Record<string, string> = {
  house: t("game3d.components.UI.BuildingDetail.k1"),
  town: t("game.components.BuildingStatsCard.k2"),
  city: t("game.components.BuildingStatsCard.k3"),
  kingdom: t("game.components.BuildingStatsCard.k4"),
  palace: t("game.components.BuildingStatsCard.k5"),
  technology: t("game.components.BuildingStatsCard.k6"),
  sect: t("game.components.BuildingStatsCard.k7"),
  immortal: t("game3d.components.UI.BuildingDetail.k2")
};

/** 建筑状态中文名 + 颜色 */
const STATUS_INFO: Record<GameBuildingStatus, {
  label: string;
  cls: string;
}> = {
  planning: {
    label: t("CommandManual.k216"),
    cls: 'game3d-status--planning'
  },
  building: {
    label: t("game.components.BuildingStatsCard.k12"),
    cls: 'game3d-status--building'
  },
  completed: {
    label: t("game.components.BuildingStatsCard.k11"),
    cls: 'game3d-status--completed'
  },
  ruined: {
    label: t("game3d.components.UI.BuildingDetail.k3"),
    cls: 'game3d-status--ruined'
  }
};

/** 知识领域中文名（部分，用于详情显示） */
const DOMAIN_LABELS: Partial<Record<GameKnowledgeDomainId, string>> = {
  cs: t("game3d.components.UI.BuildingDetail.k4"),
  math: t("game3d.components.UI.BreakthroughQuiz.k10"),
  physics: t("game3d.components.UI.BuildingDetail.k5"),
  literature: t("game3d.components.UI.BreakthroughQuiz.k12"),
  history: t("game3d.components.UI.BreakthroughQuiz.k13"),
  art: t("game3d.components.UI.BreakthroughQuiz.k14"),
  engineering: t("game3d.components.UI.BuildingDetail.k6"),
  medicine: t("game3d.components.UI.BreakthroughQuiz.k16"),
  philosophy: t("game3d.components.UI.BreakthroughQuiz.k17"),
  economics: t("game3d.components.UI.BuildingDetail.k7"),
  language: t("game3d.components.UI.BreakthroughQuiz.k19"),
  other: t("game3d.components.UI.BuildingDetail.k8")
};
export function BuildingDetail() {
  const selectedBuildingId = useGame3DStore(s => s.selectedBuildingId);
  const buildings = useGame3DStore(s => s.buildings);
  const selectBuilding = useGame3DStore(s => s.selectBuilding);
  const removeBuilding = useGame3DStore(s => s.removeBuilding);
  const updateBuilding = useGame3DStore(s => s.updateBuilding);
  const startMoveMode = useGame3DStore(s => s.startMoveMode);
  const setActionInProgress = useGame3DStore(s => s.setActionInProgress);
  const getBaseCost = useGame3DStore(s => s.getBaseCost);
  const [actionMsg, setActionMsg] = useState<string | null>(null);
  const [confirmingRemove, setConfirmingRemove] = useState(false);
  const building = useMemo(() => buildings.find(b => b.id === selectedBuildingId) ?? null, [buildings, selectedBuildingId]);

  /** 计算 base_cost / upgrade_cost / move_cost */
  const costs = useMemo(() => {
    if (!building) return null;
    const baseCost = getBaseCost(building.category, building.subtype);
    if (!baseCost) return null;
    // 升级到 level+1：level_factor = 1 + (new_level - 1) * 0.5 = 1 + level * 0.5
    const upgradeCost = Math.round(baseCost * (1 + building.level * 0.5));
    // 移动消耗 10% 建筑成本（04 文档）
    const moveCost = Math.round(baseCost * 0.1);
    return {
      baseCost,
      upgradeCost,
      moveCost
    };
  }, [building, getBaseCost]);

  /** 升级建筑 */
  const handleUpgrade = useCallback(async () => {
    if (!building || !costs) return;
    setActionInProgress(true);
    setActionMsg(null);
    try {
      const res = await game.upgradeBuilding(building.id, costs.upgradeCost);
      if (res.code !== 0) throw new Error(res.message);
      const b = res.data;
      if (b) {
        updateBuilding(building.id, {
          level: b.level,
          status: b.status,
          buildProgress: b.build_progress
        });
        setActionMsg(t("game3d.components.UI.BuildingDetail.k9", {
          level: b.level
        }));
      }
    } catch (err) {
      setActionMsg(`✕ ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setActionInProgress(false);
    }
  }, [building, costs, updateBuilding, setActionInProgress]);

  /** 拆除建筑 */
  const handleRemove = useCallback(async () => {
    if (!building || !confirmingRemove) return;
    setActionInProgress(true);
    setActionMsg(null);
    try {
      const res = await game.removeBuilding(building.id);
      if (res.code !== 0) throw new Error(res.message);
      const refund = res.data ?? 0;
      removeBuilding(building.id);
      selectBuilding(null);
      setActionMsg(t("game3d.components.UI.BuildingDetail.k10", {
        refund: refund
      }));
    } catch (err) {
      setActionMsg(`✕ ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setActionInProgress(false);
      setConfirmingRemove(false);
    }
  }, [building, confirmingRemove, removeBuilding, selectBuilding, setActionInProgress]);

  /** 移动建筑：进入移动模式 */
  const handleMove = useCallback(() => {
    if (!building) return;
    startMoveMode(building.id);
    setActionMsg(t("game3d.components.UI.BuildingDetail.k11"));
  }, [building, startMoveMode]);
  if (!building) return null;
  const statusInfo = STATUS_INFO[building.status];
  const domainLabel = DOMAIN_LABELS[building.knowledgeDomain] ?? building.knowledgeDomain;
  return <div className="game3d-detail-panel">
      <div className="game3d-detail-panel__header">
        <span className="game3d-detail-panel__title">{t("game3d.components.UI.BuildingDetail.k12")}</span>
        <button className="game3d-detail-panel__close" onClick={() => selectBuilding(null)} aria-label={t("common.close")}>
          ✕
        </button>
      </div>

      <div className="game3d-detail-panel__body">
        {/* 名称 */}
        <div className="game3d-detail-row">
          <span className="game3d-detail-row__label">{t("game3d.components.UI.BuildingDetail.k13")}</span>
          <span className="game3d-detail-row__value">{building.name}</span>
        </div>

        {/* 类别 */}
        <div className="game3d-detail-row">
          <span className="game3d-detail-row__label">{t("game3d.components.UI.BuildingDetail.k14")}</span>
          <span className="game3d-detail-row__value">
            {CATEGORY_LABELS[building.category] ?? building.category} · {building.subtype}
          </span>
        </div>

        {/* 等级 */}
        <div className="game3d-detail-row">
          <span className="game3d-detail-row__label">{t("game3d.components.UI.BuildingDetail.k15")}</span>
          <span className="game3d-detail-row__value game3d-detail-row__value--accent">
            Lv.{building.level}
          </span>
        </div>

        {/* 状态 */}
        <div className="game3d-detail-row">
          <span className="game3d-detail-row__label">{t("common.status")}</span>
          <span className={`game3d-detail-status ${statusInfo.cls}`}>{statusInfo.label}</span>
        </div>

        {/* 建造进度（建造中显示） */}
        {building.status === 'building' && <div className="game3d-detail-row game3d-detail-row--col">
            <div className="game3d-detail-row__head">
              <span className="game3d-detail-row__label">{t("game3d.components.UI.BuildingDetail.k16")}</span>
              <span className="game3d-detail-row__value">{building.buildProgress.toFixed(1)}%</span>
            </div>
            <div className="game3d-detail-progress">
              <div className="game3d-detail-progress__bar" style={{
            width: `${Math.min(100, building.buildProgress)}%`
          }} />
            </div>
          </div>}

        {/* 知识领域 */}
        <div className="game3d-detail-row">
          <span className="game3d-detail-row__label">{t("game3d.components.UI.BuildingDetail.k17")}</span>
          <span className="game3d-detail-row__value">{domainLabel}</span>
        </div>

        {/* 3D 坐标 */}
        <div className="game3d-detail-row">
          <span className="game3d-detail-row__label">{t("game3d.components.UI.BuildingDetail.k18")}</span>
          <span className="game3d-detail-row__value game3d-detail-row__value--mono">
            ({building.position[0].toFixed(1)}, {building.position[1].toFixed(1)}, {building.position[2].toFixed(1)})
          </span>
        </div>

        {/* Cost 信息 */}
        {costs ? <div className="game3d-detail-cost">
            <div className="game3d-detail-cost__row">
              <span>{t("game3d.components.UI.BuildingDetail.k19")}</span><span>{costs.baseCost} {t("game3d.components.UI.BuildingDetail.k20")}</span>
            </div>
            <div className="game3d-detail-cost__row">
              <span>{t("game3d.components.UI.BuildingDetail.k21")}</span><span className="game3d-detail-cost__val">{costs.upgradeCost} {t("game3d.components.UI.BuildingDetail.k20")}</span>
            </div>
            <div className="game3d-detail-cost__row">
              <span>{t("game3d.components.UI.BuildingDetail.k22")}</span><span>{costs.moveCost} {t("game3d.components.UI.BuildingDetail.k20")}</span>
            </div>
          </div> : <div className="game3d-detail-cost game3d-detail-cost--loading">
            {t("game3d.components.UI.BuildingDetail.k23")}
          </div>}
      </div>

      {/* 操作按钮 */}
      <div className="game3d-detail-panel__actions">
        {building.status === 'completed' && <>
            <button className="game3d-btn game3d-btn--action game3d-btn--upgrade" onClick={handleUpgrade} disabled={!costs}>
              {t("game3d.components.UI.BuildingDetail.k24")}
            </button>
            <button className="game3d-btn game3d-btn--action game3d-btn--move" onClick={handleMove} disabled={!costs}>
              {t("game3d.components.UI.BuildingDetail.k25")}
            </button>
            {!confirmingRemove ? <button className="game3d-btn game3d-btn--action game3d-btn--remove" onClick={() => setConfirmingRemove(true)}>
                {t("game3d.components.UI.BuildingDetail.k26")}
              </button> : <div className="game3d-detail-confirm">
                <span>{t("game3d.components.UI.BuildingDetail.k27")}</span>
                <div className="game3d-detail-confirm__btns">
                  <button className="game3d-btn game3d-btn--danger" onClick={handleRemove}>
                    {t("game3d.components.UI.BuildingDetail.k28")}
                  </button>
                  <button className="game3d-btn" onClick={() => setConfirmingRemove(false)}>
                    {t("common.cancel")}
                  </button>
                </div>
              </div>}
          </>}
      </div>

      {/* 操作反馈消息 */}
      {actionMsg && <div className="game3d-detail-msg">{actionMsg}</div>}
    </div>;
}