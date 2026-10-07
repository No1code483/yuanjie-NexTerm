import { t } from "i18next";
import { useState } from 'react';
import { ipc } from '@/lib/ipc';
import { kb } from '@/plugins/boards/knowledge/ipc';
import styles from '../../routes/Search.module.css';
interface GlobalSearchResult {
  module: string;
  id: string;
  title: string;
  preview: string;
  score: number;
  updated_at: string;
}
const MODULE_META: Record<string, {
  label: string;
  icon: string;
}> = {
  knowledge_base: {
    label: t("components.intelligence.ActivityPanel.k1"),
    icon: '📚'
  },
  conversations: {
    label: t("layout.k17"),
    icon: '💬'
  },
  notes: {
    label: t("Recycle.k3"),
    icon: '📝'
  },
  code_files: {
    label: t("Search.k1"),
    icon: '💻'
  },
  todos: {
    label: t("components.intelligence.ActivityPanel.k2"),
    icon: '✅'
  },
  journals: {
    label: t("components.intelligence.ActivityPanel.k3"),
    icon: '📅'
  }
};
/** search.global 域：全站搜索（结果按模块分组 / AI 综合摘要 / 一键归档知识库） */
export default function GlobalSearch() {
  const [globalSearchQuery, setGlobalSearchQuery] = useState('');
  const [globalSearchResults, setGlobalSearchResults] = useState<GlobalSearchResult[]>([]);
  const [globalSearchLoading, setGlobalSearchLoading] = useState(false);
  const [globalSearchError, setGlobalSearchError] = useState('');
  const [aiSummary, setAiSummary] = useState('');
  const [aiSummaryLoading, setAiSummaryLoading] = useState(false);
  const [collapsedModules, setCollapsedModules] = useState<Set<string>>(new Set());

  // 4.3 搜索结果一键归档知识库
  const [archivingIds, setArchivingIds] = useState<Set<string>>(new Set());
  const [archivedIds, setArchivedIds] = useState<Set<string>>(new Set());
  const handleArchiveToKb = async (r: GlobalSearchResult) => {
    const itemKey = `${r.module}-${r.id}`;
    if (archivedIds.has(itemKey) || archivingIds.has(itemKey)) return;
    setArchivingIds(prev => new Set(prev).add(itemKey));
    try {
      await kb.addKbEntry({
        request: {
          title: r.title,
          content: t("Search.k5", {
            title: r.title,
            module: r.module,
            preview: r.preview,
            arg0: new Date().toLocaleString('zh-CN')
          }),
          category_id: null,
          tags: [t("Search.k6")]
        }
      });
      setArchivedIds(prev => new Set(prev).add(itemKey));
    } catch {
      // 静默失败
    } finally {
      setArchivingIds(prev => {
        const n = new Set(prev);
        n.delete(itemKey);
        return n;
      });
    }
  };
  const handleGlobalSearch = async () => {
    const q = globalSearchQuery.trim();
    if (!q) return;
    setGlobalSearchLoading(true);
    setGlobalSearchError('');
    setAiSummary('');
    setCollapsedModules(new Set());
    try {
      const res = await ipc.invoke<GlobalSearchResult[]>('global_search', {
        query: q,
        modules: null,
        limit: 100
      });
      if (res.data) {
        setGlobalSearchResults(res.data);
      } else {
        setGlobalSearchResults([]);
        setGlobalSearchError(res.message || t("Knowledge.k58"));
      }
    } catch (e: any) {
      setGlobalSearchResults([]);
      setGlobalSearchError(e?.message || String(e));
    } finally {
      setGlobalSearchLoading(false);
    }
  };
  const handleGenerateAiSummary = async () => {
    if (globalSearchResults.length === 0) return;
    setAiSummaryLoading(true);
    try {
      const res = await ipc.invoke<{
        summary: string;
      }>('search_ai_summary', {
        query: globalSearchQuery.trim(),
        results: globalSearchResults
      });
      if (res.data) {
        setAiSummary(res.data.summary);
      }
    } catch {
      // fallback handled by backend
    } finally {
      setAiSummaryLoading(false);
    }
  };
  const toggleModuleCollapse = (module: string) => {
    setCollapsedModules(prev => {
      const next = new Set(prev);
      if (next.has(module)) next.delete(module);else next.add(module);
      return next;
    });
  };
  const handleGlobalSearchKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') handleGlobalSearch();
  };

  // 按模块分组
  const groupedResults = (() => {
    const groups: Record<string, GlobalSearchResult[]> = {};
    for (const r of globalSearchResults) {
      if (!groups[r.module]) groups[r.module] = [];
      groups[r.module].push(r);
    }
    for (const g of Object.values(groups)) {
      g.sort((a, b) => b.score - a.score);
    }
    return groups;
  })();
  return <div className={styles.globalSearchContainer}>
      <div className={styles.globalSearchBar}>
        <input className={styles.globalSearchInput} value={globalSearchQuery} onChange={e => setGlobalSearchQuery(e.target.value)} onKeyDown={handleGlobalSearchKeyDown} placeholder={t("Search.k27")} spellCheck={false} autoFocus />
        <button className={styles.globalSearchBtn} onClick={handleGlobalSearch} disabled={globalSearchLoading || !globalSearchQuery.trim()}>
          {globalSearchLoading ? '⟳' : t("common.search")}
        </button>
      </div>

      {globalSearchError && <div className={styles.globalSearchError}>{globalSearchError}</div>}

      {/* AI 综合摘要卡片 */}
      {globalSearchResults.length > 0 && <div className={styles.aiSummaryCard}>
          <div className={styles.aiSummaryHeader}>
            <span className={styles.aiSummaryTitle}>{t("Search.k28")}</span>
            {!aiSummary && !aiSummaryLoading && <button className={styles.aiSummaryBtn} onClick={handleGenerateAiSummary}>
                {t("Search.k29")}
              </button>}
            {aiSummaryLoading && <span className={styles.aiSummaryLoading}>{t("components.intelligence.DashboardPanel.k67")}</span>}
          </div>
          {aiSummary && <div className={styles.aiSummaryContent}>
              <span className={styles.aiSummaryPrompt}>$ </span>
              {aiSummary}
            </div>}
        </div>}

      {/* 搜索结果分组 */}
      {globalSearchLoading && !globalSearchResults.length && <div className={styles.globalSearchLoading}>{t("ai.ConversationList.k4")}</div>}

      <div className={styles.globalSearchResults}>
        {Object.keys(groupedResults).length === 0 && !globalSearchLoading && globalSearchQuery && <div className={styles.globalSearchEmpty}>
            <span className={styles.globalSearchEmptyIcon}>∅</span>
            <span>{t("Search.k30")}</span>
          </div>}

        {Object.entries(groupedResults).map(([module, results]) => {
        const meta = MODULE_META[module] || {
          label: module,
          icon: '📄'
        };
        const isCollapsed = collapsedModules.has(module);
        return <div key={module} className={styles.moduleGroup}>
              <div className={styles.moduleHeader} onClick={() => toggleModuleCollapse(module)}>
                <span className={styles.moduleCaret}>{isCollapsed ? '▶' : '▼'}</span>
                <span className={styles.moduleIcon}>{meta.icon}</span>
                <span className={styles.moduleLabel}>{meta.label}</span>
                <span className={styles.moduleCount}>({results.length})</span>
              </div>
              {!isCollapsed && <div className={styles.moduleResults}>
                  {results.map((r, idx) => {
              const itemKey = `${r.module}-${r.id}`;
              const isArchived = archivedIds.has(itemKey);
              const isArchiving = archivingIds.has(itemKey);
              return <div key={`${r.module}-${r.id}-${idx}`} className={styles.resultItem}>
                        <div className={styles.resultTitle}>
                          <span className={styles.resultScore}>[{r.score.toFixed(2)}]</span>
                          {r.title}
                        </div>
                        <div className={styles.resultPreview}>{r.preview}</div>
                        <div className={styles.resultFooter}>
                          {r.updated_at && <span className={styles.resultMeta}>{r.updated_at}</span>}
                          {r.module !== 'knowledge_base' && <button className={styles.archiveBtn} onClick={() => handleArchiveToKb(r)} disabled={isArchived || isArchiving} title={isArchived ? t("Search.k31") : t("Search.k32")}>
                              {isArchiving ? '⏳' : isArchived ? t("Search.k33") : t("Search.k34")}
                            </button>}
                        </div>
                      </div>;
            })}
                </div>}
            </div>;
      })}
      </div>
    </div>;
}
