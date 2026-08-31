import { t } from "i18next";
/**
 * BuildPanel - 建造面板（Task 9.3）
 *
 * UI 形态：左下角浮动按钮 + 弹出菜单
 *
 * 交互流程：
 *   1. 左下角"建造"按钮（idle 状态）
 *   2. 点击展开弹出菜单：8 大类 × N 子类的可滚动列表
 *      每项显示：名称 + 所需积分（base_cost）+ 文明等级要求
 *   3. 点击子类项 → 调用 store.startPlacement(item)
 *      → 菜单关闭，进入放置模式
 *   4. 放置模式中按钮变为"取消建造"（点击 cancelPlacement）
 *
 * 数据源：game.getBuildingCatalog() + game.getRealmInfo()（校验文明等级）
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useCallback, useEffect, useMemo, useState } from 'react';
import { game } from '@/lib/ipc';
import type { BuildingCatalog, BuildingCategoryInfo, RealmInfo } from '@/types/game';
import { useGame3DStore, type PlacementItem } from '../../stores/gameStore';
import { CATEGORY_DEFAULT_DOMAIN } from '../../utils/domainMapping';
export function BuildPanel() {
  const world = useGame3DStore(s => s.world);
  const placementMode = useGame3DStore(s => s.placementMode);
  const startPlacement = useGame3DStore(s => s.startPlacement);
  const cancelPlacement = useGame3DStore(s => s.cancelPlacement);
  const setCatalogCache = useGame3DStore(s => s.setCatalogCache);
  const [open, setOpen] = useState(false);
  const [catalog, setCatalog] = useState<BuildingCatalog | null>(null);
  const [realmInfo, setRealmInfo] = useState<RealmInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /** 加载建筑目录 + 境界信息（点击展开时按需加载） */
  const loadCatalog = useCallback(async () => {
    if (!world) return;
    setLoading(true);
    setError(null);
    try {
      const [catRes, realmRes] = await Promise.all([game.getBuildingCatalog(), game.getRealmInfo(world.id)]);
      if (catRes.code !== 0) throw new Error(catRes.message);
      if (realmRes.code !== 0) throw new Error(realmRes.message);
      const catalog = catRes.data ?? null;
      setCatalog(catalog);
      setCatalogCache(catalog); // 缓存到 store，供 BuildingDetail 计算 cost
      setRealmInfo(realmRes.data ?? null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [world]);

  /** 首次展开时加载目录 */
  useEffect(() => {
    if (open && !catalog && !loading && world) {
      loadCatalog();
    }
  }, [open, catalog, loading, world, loadCatalog]);

  /** 选中子类 → 进入放置模式 */
  const handleSelectSubtype = useCallback((cat: BuildingCategoryInfo, subtype: string, name: string, baseCost: number) => {
    const item: PlacementItem = {
      category: cat.category,
      subtype,
      name,
      baseCost,
      knowledgeDomain: CATEGORY_DEFAULT_DOMAIN[cat.category]
    };
    startPlacement(item);
    setOpen(false);
  }, [startPlacement]);

  /** 当前文明等级（用于禁用未解锁大类） */
  const civLevel = useMemo(() => realmInfo?.civilization_level ?? 1, [realmInfo]);

  /** 放置模式中：显示取消按钮 */
  if (placementMode === 'placing' || placementMode === 'moving') {
    return <div className="game3d-build-panel game3d-build-panel--active">
        <button className="game3d-btn game3d-btn--danger" onClick={cancelPlacement} aria-label={t("game3d.components.UI.BuildPanel.k1")}>
          ✕ {placementMode === 'placing' ? t("game3d.components.UI.BuildPanel.k2") : t("game3d.components.UI.BuildPanel.k3")}
        </button>
      </div>;
  }
  return <div className="game3d-build-panel">
      {/* 触发按钮 */}
      <button className={`game3d-btn game3d-btn--build ${open ? 'is-active' : ''}`} onClick={() => setOpen(v => !v)} aria-expanded={open} aria-label={t("game3d.components.UI.BuildPanel.k4")}>
        {open ? t("components.FloatingXin.k28") : t("game3d.components.UI.BuildPanel.k5")}
      </button>

      {/* 弹出菜单 */}
      {open && <div className="game3d-build-menu">
          <div className="game3d-build-menu__header">
            <span>{t("game3d.components.UI.BuildPanel.k6")}</span>
            <span className="game3d-build-menu__civ">{t("game3d.components.UI.BuildPanel.k7")}{civLevel}</span>
          </div>

          {loading && <div className="game3d-build-menu__msg">{t("common.loading")}</div>}
          {error && <div className="game3d-build-menu__msg game3d-build-menu__msg--err">
              {error}
              <button className="game3d-btn" onClick={loadCatalog}>{t("components.ErrorBoundary.k5")}</button>
            </div>}
          {!loading && !error && catalog && <div className="game3d-build-menu__list">
              {catalog.categories.map(cat => {
          const locked = civLevel < cat.civilization_level_required;
          return <div key={cat.category} className={`game3d-build-cat ${locked ? 'is-locked' : ''}`}>
                    <div className="game3d-build-cat__head">
                      <span className="game3d-build-cat__name">{cat.name}</span>
                      <span className="game3d-build-cat__req">
                        {locked ? t("game3d.components.UI.BuildPanel.k8", {
                  civilization_level_required: cat.civilization_level_required
                }) : t("game3d.components.UI.BuildPanel.k9")}
                      </span>
                    </div>
                    {!locked && <div className="game3d-build-cat__subs">
                        {cat.subtypes.map(sub => <button key={sub.subtype} className="game3d-build-sub" onClick={() => handleSelectSubtype(cat, sub.subtype, sub.name, sub.base_cost)}>
                            <span className="game3d-build-sub__name">{sub.name}</span>
                            <span className="game3d-build-sub__cost">{sub.base_cost} {t("game3d.components.UI.BuildingDetail.k20")}</span>
                          </button>)}
                      </div>}
                  </div>;
        })}
            </div>}
        </div>}
    </div>;
}