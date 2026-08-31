import { t } from "i18next";
/**
 * KnowledgeMapping - 知识领域映射设置页（Task 10.3）
 *
 * 参考 09_知识领域映射.md §5.1 三栏布局：
 *   - 左栏：知识库一级分类列表（按 library 分组）
 *   - 中栏：选中分类的映射关系可视化
 *   - 右栏：12 个预定义知识领域选项
 *
 * v1 实现范围（spec §3.1）：
 *   - ✅ 手动映射：点击分类 → 点击领域 → 立即保存
 *   - ✅ 修改已映射分类（弹窗确认）
 *   - ✅ 重置映射（删除映射，回归 other 兜底）
 *   - ❌ 拖拽映射（v2 复杂交互）
 *   - ❌ AI 建议（spec v2 才做）
 *   - ❌ 批量映射（v2 才做）
 *
 * 数据流：
 *   1. Promise.all([game.getKnowledgeDomains(), knowledge.getCategories(), game.getKbCategoryMappings()])
 *   2. 前端 LEFT JOIN：categories（一级）× mappings → 每个分类附带映射信息
 *   3. 用户点击领域 → game.mapKbCategory(categoryId, domainId) → 更新本地状态
 *
 * 路由：/game/play/mapping（从 3D 游戏页设置按钮进入）
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ROUTES } from '@/routes/routes';
import { game, knowledge } from '@/lib/ipc';
import type { GameKnowledgeDomain, GameKnowledgeDomainId, GameKbCategoryMapping } from '@/types/game';
import './styles/game3d.css';

/** 知识库一级分类（前端类型化，对齐后端 KbCategory struct） */
interface KbCategory {
  id: number;
  name: string;
  parent_id: number | null;
  library: string;
  sort_order: number;
  created_at: number;
}

/** 左栏分类项：分类信息 + 当前映射（无映射时为 null） */
interface CategoryWithMapping extends KbCategory {
  mapping: GameKbCategoryMapping | null;
}

/** 加载状态 */
type LoadState = 'loading' | 'ready' | 'error';

/** 12 领域主题色（与 game3d.css --game3d-civ-* 协调） */
const DOMAIN_COLORS: Record<GameKnowledgeDomainId, string> = {
  cs: '#4a9eff',
  math: '#9b59b6',
  physics: '#c084fc',
  literature: '#fbbf24',
  history: '#a67c52',
  art: '#ec4899',
  engineering: '#8b6f47',
  medicine: '#4ade80',
  philosophy: '#c084fc',
  economics: '#fbbf24',
  language: '#6bb6ff',
  other: '#5a6478'
};
export default function KnowledgeMapping() {
  const navigate = useNavigate();
  const [loadState, setLoadState] = useState<LoadState>('loading');
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [domains, setDomains] = useState<GameKnowledgeDomain[]>([]);
  const [categories, setCategories] = useState<CategoryWithMapping[]>([]);
  const [selectedCategoryId, setSelectedCategoryId] = useState<number | null>(null);
  const [pendingDomain, setPendingDomain] = useState<GameKnowledgeDomainId | null>(null);
  const [saving, setSaving] = useState(false);
  const [toast, setToast] = useState<string | null>(null);

  /** 加载数据：12 领域 + 所有分类 + 所有映射，前端 LEFT JOIN */
  const loadData = useCallback(async () => {
    setLoadState('loading');
    setErrorMsg(null);
    try {
      const [domainsRes, categoriesRes, mappingsRes] = await Promise.all([game.getKnowledgeDomains(), knowledge.getCategories(), game.getKbCategoryMappings()]);
      if (domainsRes.code !== 0) throw new Error(domainsRes.message);
      if (categoriesRes.code !== 0) throw new Error(categoriesRes.message);
      if (mappingsRes.code !== 0) throw new Error(mappingsRes.message);
      const domainList = domainsRes.data ?? [];
      const categoryList = (categoriesRes.data ?? []) as KbCategory[];
      const mappingList = mappingsRes.data ?? [];

      // 只保留一级分类（parent_id IS NULL），与映射做 LEFT JOIN
      const rootCategories = categoryList.filter(c => c.parent_id === null);
      const mappingMap = new Map<number, GameKbCategoryMapping>();
      for (const m of mappingList) {
        mappingMap.set(m.category_id, m);
      }
      const joined: CategoryWithMapping[] = rootCategories.map(c => ({
        ...c,
        mapping: mappingMap.get(c.id) ?? null
      }));
      // 按 library 分组后保持原顺序
      joined.sort((a, b) => {
        if (a.library !== b.library) return a.library.localeCompare(b.library);
        return a.sort_order - b.sort_order;
      });
      setDomains(domainList);
      setCategories(joined);
      setLoadState('ready');
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setLoadState('error');
    }
  }, []);
  useEffect(() => {
    loadData();
  }, [loadData]);

  /** toast 自动消失 */
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(null), 3000);
    return () => clearTimeout(timer);
  }, [toast]);
  const selectedCategory = useMemo(() => categories.find(c => c.id === selectedCategoryId) ?? null, [categories, selectedCategoryId]);

  /** 统计 */
  const stats = useMemo(() => {
    const mapped = categories.filter(c => c.mapping !== null).length;
    return {
      mapped,
      unmapped: categories.length - mapped,
      total: categories.length
    };
  }, [categories]);

  /** 按 library 分组（左栏展示用） */
  const groupedCategories = useMemo(() => {
    const groups: Record<string, CategoryWithMapping[]> = {};
    for (const c of categories) {
      if (!groups[c.library]) groups[c.library] = [];
      groups[c.library].push(c);
    }
    return groups;
  }, [categories]);

  /** 点击领域：若该分类已映射到同领域，无操作；否则进入"待确认"状态 */
  const handleDomainClick = (domainId: GameKnowledgeDomainId) => {
    if (!selectedCategory || saving) return;
    if (selectedCategory.mapping?.domain_id === domainId) return;
    setPendingDomain(domainId);
  };

  /** 确认保存映射 */
  const handleConfirmSave = async () => {
    if (!selectedCategory || !pendingDomain) return;
    setSaving(true);
    try {
      const res = await game.mapKbCategory(selectedCategory.id, pendingDomain);
      if (res.code !== 0) throw new Error(res.message);
      // 更新本地状态
      setCategories(prev => prev.map(c => c.id === selectedCategory.id ? {
        ...c,
        mapping: {
          category_id: c.id,
          domain_id: pendingDomain,
          mapped_at: Date.now(),
          mapped_by: 'manual',
          confidence: null
        }
      } : c));
      setToast(t("game3d.KnowledgeMapping.k1", {
        name: selectedCategory.name,
        pendingDomain: pendingDomain
      }));
      setPendingDomain(null);
    } catch (err) {
      setToast(t("game3d.KnowledgeMapping.k2", {
        arg0: err instanceof Error ? err.message : String(err)
      }));
    } finally {
      setSaving(false);
    }
  };

  /** 重置映射（删除映射，回归 other 兜底） */
  const handleResetMapping = async () => {
    if (!selectedCategory || !selectedCategory.mapping || saving) return;
    if (!window.confirm(t("game3d.KnowledgeMapping.k3", {
      name: selectedCategory.name
    }))) {
      return;
    }
    setSaving(true);
    try {
      // v1 简化：调用 mapKbCategory 映射到 other（后端无独立 delete 命令暴露给前端）
      const res = await game.mapKbCategory(selectedCategory.id, 'other');
      if (res.code !== 0) throw new Error(res.message);
      setCategories(prev => prev.map(c => c.id === selectedCategory.id ? {
        ...c,
        mapping: {
          category_id: c.id,
          domain_id: 'other',
          mapped_at: Date.now(),
          mapped_by: 'manual',
          confidence: null
        }
      } : c));
      setToast(t("game3d.KnowledgeMapping.k4", {
        name: selectedCategory.name
      }));
    } catch (err) {
      setToast(t("game3d.KnowledgeMapping.k5", {
        arg0: err instanceof Error ? err.message : String(err)
      }));
    } finally {
      setSaving(false);
    }
  };

  /** 取消待确认状态 */
  const handleCancelPending = () => setPendingDomain(null);
  return <div className="km-root">
      {/* ===== Header ===== */}
      <header className="km-header">
        <button className="km-back-btn" onClick={() => navigate(ROUTES.GAME_PLAY)} aria-label={t("game3d.KnowledgeMapping.k6")}>
          {t("game3d.KnowledgeMapping.k7")}
        </button>
        <h1 className="km-title">{t("game3d.Game3D.k10")}</h1>
        <div className="km-stats">
          <span className="km-stats__item km-stats__item--mapped">{t("game3d.KnowledgeMapping.k8")} {stats.mapped}</span>
          <span className="km-stats__item km-stats__item--unmapped">{t("game3d.KnowledgeMapping.k9")} {stats.unmapped}</span>
          <span className="km-stats__item km-stats__item--total">{t("components.GroupChatOrchestrationPanel.k26")} {stats.total}</span>
        </div>
      </header>

      {/* ===== 主体三栏 ===== */}
      <main className="km-main">
        {loadState === 'loading' && <div className="km-loading">{t("game3d.KnowledgeMapping.k10")}</div>}
        {loadState === 'error' && <div className="km-error">
            <div className="km-error__msg">{t("game3d.Game3D.k13")}</div>
            <div className="km-error__hint">{errorMsg}</div>
            <button className="km-retry-btn" onClick={loadData}>{t("components.ErrorBoundary.k5")}</button>
          </div>}
        {loadState === 'ready' && <>
            {/* 左栏：知识库分类列表 */}
            <section className="km-col km-col--categories">
              <div className="km-col__header">
                <span className="km-col__title">{t("game3d.KnowledgeMapping.k11")}</span>
                <span className="km-col__hint">{t("game3d.KnowledgeMapping.k12")}</span>
              </div>
              <div className="km-col__body">
                {categories.length === 0 && <div className="km-empty">{t("game3d.KnowledgeMapping.k13")}</div>}
                {Object.entries(groupedCategories).map(([library, items]) => <div key={library} className="km-group">
                    <div className="km-group__header">▼ {library}</div>
                    {items.map(c => {
                const isSelected = c.id === selectedCategoryId;
                return <button key={c.id} className={`km-cat-item ${isSelected ? 'is-selected' : ''}`} onClick={() => {
                  setSelectedCategoryId(c.id);
                  setPendingDomain(null);
                }}>
                          <span className="km-cat-item__name">{c.name}</span>
                          {c.mapping ? <span className="km-cat-item__badge" style={{
                    backgroundColor: DOMAIN_COLORS[c.mapping.domain_id] ?? '#5a6478'
                  }}>
                              {c.mapping.domain_id}
                            </span> : <span className="km-cat-item__badge km-cat-item__badge--unmapped">
                              ?
                            </span>}
                        </button>;
              })}
                  </div>)}
              </div>
            </section>

            {/* 中栏：映射关系可视化 */}
            <section className="km-col km-col--relation">
              <div className="km-col__header">
                <span className="km-col__title">{t("game3d.KnowledgeMapping.k14")}</span>
              </div>
              <div className="km-col__body km-relation">
                {!selectedCategory && <div className="km-relation__empty">
                    {t("game3d.KnowledgeMapping.k15")}
                  </div>}
                {selectedCategory && <>
                    <div className="km-relation__row">
                      <div className="km-relation__node km-relation__node--cat">
                        <div className="km-relation__label">{t("CommandManual.k223")}</div>
                        <div className="km-relation__name">{selectedCategory.name}</div>
                        <div className="km-relation__sub">{t("game3d.KnowledgeMapping.k16")}{selectedCategory.library}</div>
                      </div>
                      <div className="km-relation__arrow">
                        {selectedCategory.mapping ? '─────►' : '──?──►'}
                      </div>
                      <div className="km-relation__node km-relation__node--domain">
                        <div className="km-relation__label">{t("game3d.components.UI.BuildingDetail.k17")}</div>
                        {pendingDomain ? <>
                            <div className="km-relation__name" style={{
                      color: DOMAIN_COLORS[pendingDomain]
                    }}>
                              {pendingDomain}
                            </div>
                            <div className="km-relation__sub">{t("game3d.KnowledgeMapping.k17")}</div>
                          </> : selectedCategory.mapping ? <>
                            <div className="km-relation__name" style={{
                      color: DOMAIN_COLORS[selectedCategory.mapping.domain_id]
                    }}>
                              {selectedCategory.mapping.domain_id}
                            </div>
                            <div className="km-relation__sub">
                              {domains.find(d => d.id === selectedCategory.mapping?.domain_id)?.name ?? '—'}
                            </div>
                          </> : <>
                            <div className="km-relation__name km-relation__name--unmapped">???</div>
                            <div className="km-relation__sub">{t("game3d.KnowledgeMapping.k18")}</div>
                          </>}
                      </div>
                    </div>

                    {/* 操作区 */}
                    <div className="km-relation__actions">
                      {pendingDomain ? <>
                          <div className="km-relation__pending-hint">
                            {t("game3d.KnowledgeMapping.k19")}{selectedCategory.name} → {pendingDomain}
                            {selectedCategory.mapping && <span className="km-relation__pending-warn">
                                {t("game3d.KnowledgeMapping.k20")} {selectedCategory.mapping.domain_id}{t("game3d.KnowledgeMapping.k21")}
                              </span>}
                          </div>
                          <div className="km-relation__btns">
                            <button className="km-btn km-btn--primary" onClick={handleConfirmSave} disabled={saving}>
                              {saving ? t("components.AudioEditor.k6") : t("game3d.KnowledgeMapping.k22")}
                            </button>
                            <button className="km-btn km-btn--secondary" onClick={handleCancelPending} disabled={saving}>
                              {t("common.cancel")}
                            </button>
                          </div>
                        </> : selectedCategory.mapping && <button className="km-btn km-btn--danger" onClick={handleResetMapping} disabled={saving}>
                            {t("game3d.KnowledgeMapping.k23")}
                          </button>}
                    </div>

                    <div className="km-relation__tip">
                      {t("game3d.KnowledgeMapping.k24")}
                    </div>
                  </>}
              </div>
            </section>

            {/* 右栏：12 个领域选项 */}
            <section className="km-col km-col--domains">
              <div className="km-col__header">
                <span className="km-col__title">{t("game3d.KnowledgeMapping.k25")}</span>
                <span className="km-col__hint">{t("game3d.KnowledgeMapping.k26")}</span>
              </div>
              <div className="km-col__body">
                {domains.map(d => {
              const isCurrent = selectedCategory?.mapping?.domain_id === d.id;
              const isPending = pendingDomain === d.id;
              return <button key={d.id} className={`km-domain-item ${isCurrent ? 'is-current' : ''} ${isPending ? 'is-pending' : ''}`} onClick={() => handleDomainClick(d.id as GameKnowledgeDomainId)} disabled={!selectedCategory || saving} style={{
                borderLeftColor: DOMAIN_COLORS[d.id as GameKnowledgeDomainId] ?? '#5a6478'
              }}>
                      <div className="km-domain-item__head">
                        <span className="km-domain-item__id" style={{
                    color: DOMAIN_COLORS[d.id as GameKnowledgeDomainId] ?? '#5a6478'
                  }}>
                          {d.id}
                        </span>
                        {isCurrent && <span className="km-domain-item__tag km-domain-item__tag--current">{t("game3d.KnowledgeMapping.k27")}</span>}
                        {isPending && <span className="km-domain-item__tag km-domain-item__tag--pending">{t("game3d.KnowledgeMapping.k28")}</span>}
                      </div>
                      <div className="km-domain-item__name">{d.name}</div>
                      <div className="km-domain-item__building">→ {d.building_category}</div>
                    </button>;
            })}
              </div>
            </section>
          </>}
      </main>

      {/* ===== Toast ===== */}
      {toast && <div className="km-toast">{toast}</div>}
    </div>;
}