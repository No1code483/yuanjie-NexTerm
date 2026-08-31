import { t } from "i18next";
import { useState, useEffect, useCallback, useMemo } from 'react';
import styles from './Recycle.module.css';
import { useRecycleContext } from '@/contexts/RecycleContext';
import { recycleBin } from '@/lib/ipc';
import { time, file } from '@/lib/utils';
interface RecycleFile {
  id: string;
  name: string;
  type: string;
  rawType: string;
  size: string;
  deleteTime: string;
  expireTime: string;
  originalPath: string;
}
interface RecycleStats {
  total_size: number;
  file_count: number;
  oldest_date: number;
  expiring_count: number;
}
const TYPE_LABELS: Record<string, string> = {
  todo: t("AutoSaveDemo.k13"),
  journal: t("components.intelligence.ActivityPanel.k3"),
  kb_entry: t("profile.StatsDashboard.k6"),
  kb_category: t("Recycle.k1"),
  conversation: t("Recycle.k2"),
  timer: t("components.intelligence.DashboardPanel.k104")
};
const CATEGORIES = [{
  key: 'all',
  label: t("common.all")
}, {
  key: 'kb',
  label: t("components.intelligence.ActivityPanel.k1"),
  types: ['kb_entry', 'kb_category']
}, {
  key: 'conversation',
  label: t("layout.k17"),
  types: ['conversation']
}, {
  key: 'note',
  label: t("Recycle.k3"),
  types: []
}, {
  key: 'todo',
  label: t("components.intelligence.ActivityPanel.k2"),
  types: ['todo']
}, {
  key: 'journal',
  label: t("components.intelligence.ActivityPanel.k3"),
  types: ['journal']
}, {
  key: 'file',
  label: t("knowledge.GraphView.k1"),
  types: []
}, {
  key: 'other',
  label: t("game3d.components.UI.BuildingDetail.k8"),
  types: []
}];
function getCategoryKey(rawType: string): string {
  for (const cat of CATEGORIES) {
    if (cat.types && cat.types.includes(rawType)) return cat.key;
  }
  return 'other';
}
function formatTimestamp(ms: number): string {
  if (!ms || ms === 0) return '-';
  return time.formatUtcToLocal(new Date(ms).toISOString());
}
function formatDate(ms: number): string {
  if (!ms || ms === 0) return '-';
  const d = new Date(ms);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}
function HighlightText({
  text,
  query
}: {
  text: string;
  query: string;
}) {
  if (!query) return <>{text}</>;
  const idx = text.toLowerCase().indexOf(query.toLowerCase());
  if (idx === -1) return <>{text}</>;
  return <>
      {text.slice(0, idx)}
      <span className={styles.highlight}>{text.slice(idx, idx + query.length)}</span>
      {text.slice(idx + query.length)}
    </>;
}
export default function Recycle() {
  const [showConfirm, setShowConfirm] = useState(false);
  const [confirmAction, setConfirmAction] = useState<'restore' | 'delete' | 'clear'>('delete');
  const [files, setFiles] = useState<RecycleFile[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [stats, setStats] = useState<RecycleStats | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [activeCategory, setActiveCategory] = useState('all');
  const {
    selectedFiles,
    toggleFileSelection,
    selectAll,
    clearSelection,
    totalFiles,
    setTotalFiles
  } = useRecycleContext();
  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [res, statsRes] = await Promise.all([recycleBin.getItems(), recycleBin.stats()]);
      if (res.code === 0 && Array.isArray(res.data)) {
        const mapped: RecycleFile[] = res.data.map(item => ({
          id: String(item.id),
          name: item.title || item.original_path || `#${item.id}`,
          type: TYPE_LABELS[item.item_type] || item.item_type,
          rawType: item.item_type,
          size: file.formatSize(item.file_size),
          deleteTime: formatTimestamp(item.deleted_at),
          expireTime: formatTimestamp(item.auto_delete_at),
          originalPath: item.original_path
        }));
        setFiles(mapped);
        setTotalFiles(mapped.length);
      } else {
        setError(res.message || t("Recycle.k4"));
      }
      if (statsRes.code === 0 && statsRes.data) {
        setStats(statsRes.data);
      }
    } catch (e) {
      console.error('[Recycle] 加载失败:', e);
      setError(t("Recycle.k5"));
    } finally {
      setLoading(false);
    }
  }, [setTotalFiles]);
  useEffect(() => {
    loadData();
  }, [loadData]);
  useEffect(() => {
    const handleRestore = () => {
      if (selectedFiles.length === 0) return;
      setConfirmAction('restore');
      setShowConfirm(true);
    };
    const handleDelete = () => {
      if (selectedFiles.length === 0) return;
      setConfirmAction('delete');
      setShowConfirm(true);
    };
    const handleClear = () => {
      setConfirmAction('clear');
      setShowConfirm(true);
    };
    const handleSelectAll = () => {
      selectAll(files.map(f => f.id));
    };
    window.addEventListener('recycle-restore', handleRestore);
    window.addEventListener('recycle-delete', handleDelete);
    window.addEventListener('recycle-clear', handleClear);
    window.addEventListener('recycle-select-all', handleSelectAll);
    return () => {
      window.removeEventListener('recycle-restore', handleRestore);
      window.removeEventListener('recycle-delete', handleDelete);
      window.removeEventListener('recycle-clear', handleClear);
      window.removeEventListener('recycle-select-all', handleSelectAll);
    };
  }, [selectedFiles.length, selectAll, files]);
  const executeAction = async () => {
    try {
      switch (confirmAction) {
        case 'restore':
          {
            const ids = selectedFiles.map(id => Number(id));
            const res = await recycleBin.restore(ids);
            if (res.code !== 0) throw new Error(res.message || t("Recycle.k6"));
            break;
          }
        case 'delete':
          {
            const ids = selectedFiles.map(id => Number(id));
            const res = await recycleBin.permanentDelete(ids);
            if (res.code !== 0) throw new Error(res.message || t("errors.deleteFailed"));
            break;
          }
        case 'clear':
          {
            const res = await recycleBin.empty();
            if (res.code !== 0) throw new Error(res.message || t("Recycle.k7"));
            break;
          }
      }
      clearSelection();
      setShowConfirm(false);
      await loadData();
    } catch (e) {
      console.error('[Recycle] 操作失败:', e);
      alert(e instanceof Error ? e.message : t("Recycle.k8"));
    }
  };
  const categoryCounts = useMemo(() => {
    const counts: Record<string, number> = {
      all: files.length
    };
    for (const cat of CATEGORIES) {
      if (cat.key === 'all') continue;
      if (cat.types && cat.types.length > 0) {
        counts[cat.key] = files.filter(f => cat.types!.includes(f.rawType)).length;
      } else {
        counts[cat.key] = files.filter(f => getCategoryKey(f.rawType) === cat.key).length;
      }
    }
    return counts;
  }, [files]);
  const filteredFiles = useMemo(() => {
    let result = files;
    if (activeCategory !== 'all') {
      const cat = CATEGORIES.find(c => c.key === activeCategory);
      if (cat && cat.types && cat.types.length > 0) {
        result = result.filter(f => cat.types!.includes(f.rawType));
      } else {
        result = result.filter(f => getCategoryKey(f.rawType) === activeCategory);
      }
    }
    if (searchQuery.trim()) {
      const q = searchQuery.trim().toLowerCase();
      result = result.filter(f => f.name.toLowerCase().includes(q) || f.originalPath.toLowerCase().includes(q));
    }
    return result;
  }, [files, activeCategory, searchQuery]);
  const getConfirmMessage = () => {
    switch (confirmAction) {
      case 'restore':
        return t("Recycle.k9", {
          length: selectedFiles.length
        });
      case 'delete':
        return t("Recycle.k10", {
          length: selectedFiles.length
        });
      case 'clear':
        return t("Recycle.k11");
      default:
        return '';
    }
  };
  const getConfirmTitle = () => {
    switch (confirmAction) {
      case 'restore':
        return t("Recycle.k12");
      case 'delete':
        return t("Recycle.k13");
      case 'clear':
        return t("Recycle.k14");
      default:
        return t("Recycle.k15");
    }
  };
  return <div className={styles.container}>
      <h2 className={styles.header}>{t("components.PermissionRestricted.k4")}</h2>

      {error && <div className={styles.errorBanner}>
          <span>{error}</span>
          <button onClick={loadData}>{t("components.ErrorBoundary.k5")}</button>
        </div>}

      {/* Stats Bar */}
      {stats && !loading && <div className={styles.statsBar}>
          <span>{t("Recycle.k16")} <span className={styles.statsValue}>{file.formatSize(stats.total_size)}</span></span>
          <span className={styles.statsSep}>|</span>
          <span>{t("Recycle.k17")} <span className={styles.statsValue}>{stats.file_count} {t("Knowledge.k283")}</span></span>
          <span className={styles.statsSep}>|</span>
          <span>{t("Recycle.k18")} <span className={styles.statsValue}>{formatDate(stats.oldest_date)}</span></span>
          <span className={styles.statsSep}>|</span>
          <span>{t("Recycle.k19")} <span className={styles.statsWarning}>{t("Recycle.k20")} {stats.expiring_count} {t("Knowledge.k283")}</span></span>
        </div>}

      {/* Search + Filter */}
      {!loading && files.length > 0 && <div className={styles.toolbar}>
          <div className={styles.searchWrap}>
            <input className={styles.searchInput} type="text" placeholder={t("Recycle.k21")} value={searchQuery} onChange={e => setSearchQuery(e.target.value)} />
            {searchQuery && <button className={styles.searchClear} onClick={() => setSearchQuery('')}>✕</button>}
          </div>
          <div className={styles.filterBar}>
            {CATEGORIES.map(cat => <button key={cat.key} className={`${styles.filterBtn} ${activeCategory === cat.key ? styles.filterBtnActive : ''}`} onClick={() => setActiveCategory(cat.key)}>
                {cat.label} ({categoryCounts[cat.key] ?? 0})
              </button>)}
          </div>
        </div>}

      {loading ? <div className={styles.loadingState}>{t("common.loading")}</div> : files.length === 0 ? <div className={styles.emptyState}>{t("Recycle.k22")}</div> : <>
          <div className={styles.fileList}>
            <table className={styles.fileTable}>
              <thead className={styles.tableHeader}>
                <tr>
                  <th style={{
                width: '40px'
              }}></th>
                  <th>{t("game3d.components.UI.BuildingDetail.k13")}</th>
                  <th style={{
                width: '120px'
              }}>{t("common.type")}</th>
                  <th style={{
                width: '100px'
              }}>{t("common.size")}</th>
                  <th style={{
                width: '180px'
              }}>{t("Recycle.k23")}</th>
                  <th style={{
                width: '120px'
              }}>{t("Recycle.k24")}</th>
                  <th style={{
                width: '200px'
              }}>{t("Recycle.k25")}</th>
                </tr>
              </thead>
              <tbody>
                {filteredFiles.length === 0 ? <tr>
                    <td colSpan={7} className={styles.noResults}>{t("Recycle.k26")}</td>
                  </tr> : filteredFiles.map(f => <tr key={f.id} className={`${styles.fileRow} ${selectedFiles.includes(f.id) ? styles.selected : ''}`} onClick={() => toggleFileSelection(f.id)}>
                      <td className={styles.fileCell}>
                        <div className={`${styles.checkbox} ${selectedFiles.includes(f.id) ? styles.checked : ''}`} onClick={e => {
                  e.stopPropagation();
                  toggleFileSelection(f.id);
                }} />
                      </td>
                      <td className={styles.fileCell}>
                        <span className={styles.fileName}>
                          <HighlightText text={f.name} query={searchQuery} />
                        </span>
                      </td>
                      <td className={styles.fileCell}>
                        <span className={styles.fileType}>{f.type}</span>
                      </td>
                      <td className={styles.fileCell}>
                        <span className={styles.fileSize}>{f.size}</span>
                      </td>
                      <td className={styles.fileCell}>
                        <span className={styles.deleteTime}>{f.deleteTime}</span>
                      </td>
                      <td className={styles.fileCell}>
                        <span className={styles.expireTime}>{f.expireTime}</span>
                      </td>
                      <td className={styles.fileCell}>
                        <HighlightText text={f.originalPath} query={searchQuery} />
                      </td>
                    </tr>)}
              </tbody>
            </table>
          </div>

          {selectedFiles.length > 0 && <div className={styles.batchInfo}>
              {t("ai.ConversationModal.k12")} {selectedFiles.length} / {totalFiles} {t("Recycle.k27")}
            </div>}
        </>}

      {showConfirm && <div className={styles.confirmDialog}>
          <div className={styles.confirmTitle}>{getConfirmTitle()}</div>
          <div className={styles.confirmMessage}>{getConfirmMessage()}</div>
          <div className={styles.confirmButtons}>
            <button className={styles.primaryButton} onClick={() => setShowConfirm(false)}>
              {t("common.cancel")}
            </button>
            <button className={styles.dangerButton} onClick={executeAction}>
              {t("common.confirm")}
            </button>
          </div>
        </div>}
    </div>;
}