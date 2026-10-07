// boards.knowledge L1 壳 + 核心数据层。
// 功能域已物理拆分至 features/<dir>/ 各 L2 插件（browse/search/tags/media/editors/import/history/ai），
// 本文件仅保留：路由页骨架（左栏 + 右栏布局）、核心共享状态与数据加载、?tab= 解析分发、以及经 props 组合各功能域渲染。
import { t } from "i18next";
import { useState, useEffect, useCallback } from 'react';
import { useSearchParams } from 'react-router-dom';
import { ipc, intelligence } from '@/lib/ipc';
import { kb } from './ipc';
import { kt } from './features/templates/ipc';
import { systemtools } from '@/plugins/customs/systemtools';
import { useNetworkStatus } from '@/hooks/useNetworkStatus';
import styles from './Knowledge.module.css';
import type { KbCategory, KbEntry, KbDatabase, SortMode, KbTemplate, KbTag, ViewMode, SelectedId, StatusMsg, KnowledgeCore } from './knowledge/types';
import DatabaseModal from './knowledge/DatabaseModal';
import TemplateModals from './features/templates/TemplateModals';
import GraphView from './features/graph/GraphView';
import { useAi } from './features/ai/useAi';
import { useImport } from './features/import/useImport';
import { useMedia } from './features/media/useMedia';
import { useEditors } from './features/editors/useEditors';
import { useTags } from './features/tags/useTags';
import { useHistory } from './features/history/useHistory';
import { useSearch } from './features/search/useSearch';
import { useBrowse } from './features/browse/useBrowse';

export default function Knowledge() {
  const [searchParams] = useSearchParams();
  const tab = searchParams.get('tab');

  // ===== 核心共享状态 =====
  const [categories, setCategories] = useState<KbCategory[]>([]);
  const [allEntries, setAllEntries] = useState<KbEntry[]>([]);
  const [missingFiles, setMissingFiles] = useState<Set<number>>(new Set());
  // A5 Phase 3 Task 4: 已标记为「常用」的附件 entry_id 集合（离线可访问）
  const [pinnedEntries, setPinnedEntries] = useState<Set<number>>(new Set());
  const { isOnline } = useNetworkStatus();
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set());
  const [selectedId, setSelectedId] = useState<SelectedId | null>(null);
  const [statusMsg, setStatusMsg] = useState<StatusMsg | null>(null);
  const [sortMode, setSortMode] = useState<SortMode>(() => {
    const saved = typeof localStorage !== 'undefined' ? localStorage.getItem('kb_sort_mode') : null;
    return saved as SortMode || 'time_desc';
  });
  const [isLoading, setIsLoading] = useState(true);
  const [databases, setDatabases] = useState<KbDatabase[]>(() => {
    try {
      const raw = localStorage.getItem('kb_databases');
      return raw ? JSON.parse(raw) : [{ id: 'default', name: t("Knowledge.k1"), created_at: Date.now() }];
    } catch {
      return [{ id: 'default', name: t("Knowledge.k1"), created_at: Date.now() }];
    }
  });
  const [currentDbId, setCurrentDbId] = useState<string>(() => {
    return localStorage.getItem('kb_current_db') || 'default';
  });
  const [tagStats, setTagStats] = useState<Array<{ tag_id: number; tag_name: string; tag_color: string; entry_count: number }>>([]);
  const [categoryCounts, setCategoryCounts] = useState<Map<number, number>>(new Map());
  const [currentLibrary, setCurrentLibrary] = useState<string>(() => {
    return typeof localStorage !== 'undefined' ? localStorage.getItem('kb_library') || 'material' : 'material';
  });
  const [nameConflict, setNameConflict] = useState<KnowledgeCore['nameConflict']>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('all');
  const [viewFilterId, setViewFilterId] = useState<number | null>(null);
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const [tags, setTags] = useState<KbTag[]>([]);
  const [entryTags, setEntryTags] = useState<Map<number, KbTag[]>>(new Map());
  const [templates, setTemplates] = useState<KbTemplate[]>([]);
  const [showTemplateModal, setShowTemplateModal] = useState(false);
  const [showTemplateManager, setShowTemplateManager] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<KbTemplate | null>(null);
  const [newTemplate, setNewTemplate] = useState({ name: '', icon: '📄', description: '', entry_type: 'text', content: '' });
  const [pendingTemplate, setPendingTemplate] = useState<KbTemplate | null>(null);
  const [showGraph, setShowGraph] = useState(false);
  const [showDbModal, setShowDbModal] = useState(false);
  const [newDbName, setNewDbName] = useState('');

  const showStatus = useCallback((type: 'success' | 'error', text: string) => {
    setStatusMsg({ type, text });
    setTimeout(() => setStatusMsg(null), 3000);
  }, []);

  // ===== 核心数据加载 =====
  const loadCategories = useCallback(async (library?: string) => {
    try {
      const lib = library || currentLibrary;
      const res = await kb.getKbCategories({ library: lib });
      if (res.code === 0 && res.data) {
        setCategories(res.data);
        const rootIds = new Set(res.data.filter(c => !c.parent_id).map(c => c.id));
        setExpandedIds(prev => {
          const s = new Set(prev);
          rootIds.forEach(id => s.add(id));
          return s;
        });
      }
    } catch {
      showStatus('error', t("Knowledge.k2"));
    }
  }, [showStatus, currentLibrary]);

  const loadEntries = useCallback(async () => {
    try {
      const res = await kb.getAllKbEntries();
      if (res.code === 0 && res.data) {
        setAllEntries(res.data);
      }
    } catch {
      showStatus('error', t("Knowledge.k3"));
    }
  }, [showStatus]);

  const loadCategoryCounts = useCallback(async () => {
    try {
      const res = await kb.getKbCategoryCounts();
      if (res.code === 0 && res.data) {
        const map = new Map<number, number>();
        res.data.forEach(c => map.set(c.category_id, c.count));
        setCategoryCounts(map);
      }
    } catch {/* silent */}
  }, []);

  // A5 Phase 3 Task 4: 加载已标记为「常用」的附件列表
  const loadPinnedEntries = useCallback(async () => {
    try {
      const res = await kb.kbAttachmentListPinned();
      if (res.code === 0 && res.data) {
        setPinnedEntries(new Set(res.data.map((item: any) => item.entry_id as number)));
      }
    } catch {/* silent */}
  }, []);

  const loadTags = useCallback(async () => {
    try {
      const res = await kb.getKbTags();
      if (res.code === 0 && res.data) setTags(res.data);
    } catch {/* silent */}
  }, []);

  const loadTagStats = useCallback(async () => {
    try {
      const res = await kb.getKbTagStats();
      if (res.code === 0 && res.data) setTagStats(res.data);
    } catch {/* silent */}
  }, []);

  const loadTemplates = useCallback(async () => {
    try {
      const res = await kt.kbGetTemplates();
      if (res.code === 0 && res.data) setTemplates(res.data);
    } catch {/* silent */}
  }, []);

  const loadEntryTags = useCallback(async (forEntryId?: number) => {
    try {
      if (forEntryId) {
        const r = await kb.getKbEntryTags({ entryId: forEntryId });
        if (r.code === 0 && r.data) {
          setEntryTags(prev => {
            const m = new Map(prev);
            m.set(forEntryId, r.data!);
            return m;
          });
        }
      }
    } catch {/* silent */}
  }, []);

  const loadAllEntryTags = useCallback(async () => {
    try {
      const res = await kb.getKbAllEntryTags();
      if (res.code === 0 && res.data) {
        const m = new Map<number, KbTag[]>();
        for (const item of res.data) {
          if (item.tags && item.tags.length > 0) {
            m.set(item.entry_id, item.tags);
          }
        }
        setEntryTags(m);
      }
    } catch {/* silent */}
  }, []);

  const loadAll = useCallback(async () => {
    setIsLoading(true);
    const lib = currentLibrary;
    await Promise.all([loadCategories(lib), loadEntries(), loadCategoryCounts(), loadTags(), loadEntryTags(), loadTagStats(), loadTemplates(), loadPinnedEntries()]);
    setIsLoading(false);
  }, [loadCategories, loadEntries, loadCategoryCounts, loadPinnedEntries, currentLibrary, loadTags, loadTagStats, loadTemplates, loadEntryTags]);

  // ===== 核心副作用 =====
  useEffect(() => {
    loadAll();
  }, [loadAll]);

  // 独立检测：当 allEntries 加载完成后，批量检查文件存在性
  useEffect(() => {
    if (allEntries.length === 0) return;
    let cancelled = false;
    (async () => {
      const pathsToCheck: { id: number; name: string; checkPath: string }[] = [];
      for (const e of allEntries) {
        const p = e.source_path || e.path_url || '';
        if (p && !p.startsWith('kb://')) {
          pathsToCheck.push({ id: e.id, name: e.name, checkPath: p });
        }
      }
      if (pathsToCheck.length === 0) {
        if (!cancelled) setMissingFiles(new Set());
        return;
      }
      try {
        const checkRes = await kb.kbCheckFilesExistence({ paths: pathsToCheck.map(p => p.checkPath) });
        if (checkRes?.data) {
          const d = checkRes.data as Record<string, boolean>;
          const missing = new Set<number>();
          for (const item of pathsToCheck) {
            if (d[item.checkPath] === false) missing.add(item.id);
          }
          if (!cancelled) {
            setMissingFiles(missing);
            console.log(`[KB] 文件存在性检测完成：${missing.size}/${pathsToCheck.length} 个文件已失效`);
          }
        }
      } catch (e) {
        console.warn('[KB] 文件存在性检测失败:', e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [allEntries]);

  // 打开图谱时预加载标签数据
  useEffect(() => {
    if (showGraph) {
      loadAllEntryTags();
    }
  }, [showGraph, loadAllEntryTags]);

  // 选中条目时加载其标签
  useEffect(() => {
    if (selectedId?.type === 'entry') loadEntryTags(selectedId.id);
  }, [selectedId, loadEntryTags]);

  useEffect(() => {
    localStorage.setItem('kb_databases', JSON.stringify(databases));
    localStorage.setItem('kb_current_db', currentDbId);
  }, [databases, currentDbId]);

  // ===== 核心工具 =====
  const getCategoryDepth = (id: number, depth: number = 0): number => {
    const cat = categories.find(c => c.id === id);
    if (!cat || !cat.parent_id) return depth;
    return getCategoryDepth(cat.parent_id, depth + 1);
  };

  const getEntriesInCategory = (categoryId: number): KbEntry[] => {
    return allEntries.filter(e => e.category_id === categoryId);
  };

  const findNameConflict = (name: string, categoryId: number): KbEntry | null => {
    const siblings = getEntriesInCategory(categoryId);
    return siblings.find(e => e.name.toLowerCase() === name.toLowerCase()) || null;
  };

  const resolveAutoRename = (name: string, categoryId: number): string => {
    let base = name;
    let ext = '';
    const lastDot = base.lastIndexOf('.');
    if (lastDot > 0) {
      ext = base.slice(lastDot);
      base = base.slice(0, lastDot);
    }
    const siblings = getEntriesInCategory(categoryId).map(e => e.name.toLowerCase());
    let n = 1;
    while (true) {
      const candidate = `${base}（${n}）${ext}`;
      if (!siblings.includes(candidate.toLowerCase())) return candidate;
      n++;
      if (n > 999) return `${base}（${Date.now()}）${ext}`;
    }
  };

  const isExternalEntry = (entry: KbEntry): boolean => {
    const url = entry.path_url || '';
    return /^[a-zA-Z]:[\\/]/.test(url) || url.startsWith('/') || url.startsWith('\\\\');
  };

  const moveEntryToRecycle = async (entryId: number): Promise<boolean> => {
    try {
      const res = await ipc.invoke('recycle_move_to', { itemType: 'kb_entry', itemIds: [entryId] });
      return res.code === 0;
    } catch {
      return false;
    }
  };

  const getAllDescendantIds = (cats: KbCategory[], rootId: number): number[] => {
    const result: number[] = [];
    const queue = [rootId];
    while (queue.length > 0) {
      const current = queue.shift()!;
      const children = cats.filter(c => c.parent_id === current);
      for (const child of children) {
        result.push(child.id);
        queue.push(child.id);
      }
    }
    return result;
  };

  // A5 Phase 3 Task 4: 判断条目是否可标记为常用（需为本地外部文件，非链接/非内部条目）
  const isPinnable = useCallback((entry: KbEntry): boolean => {
    if (entry.entry_type === 'link') return false;
    const p = entry.source_path || entry.path_url || '';
    return p !== '' && !p.startsWith('kb://') && !p.startsWith('http');
  }, []);

  const handleToggleFavorite = async (entryId: number) => {
    try {
      const res = await kb.toggleKbFavorite({ entryId: entryId });
      if (res.code === 0) loadAll();else showStatus('error', res.message || t("common.failed"));
    } catch {
      showStatus('error', t("common.failed"));
    }
  };

  // A5 Phase 3 Task 4: 切换附件「常用」标记（pin/unpin）
  const handleTogglePin = useCallback(async (entry: KbEntry, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    const isPinned = pinnedEntries.has(entry.id);
    try {
      if (isPinned) {
        const res = await kb.kbAttachmentUnpin({ entryId: entry.id });
        if (res.code === 0) {
          setPinnedEntries(prev => {
            const next = new Set(prev);
            next.delete(entry.id);
            return next;
          });
          showStatus('success', t("Knowledge.k295"));
        } else {
          showStatus('error', res.message || t("common.failed"));
        }
      } else {
        const filePath = entry.source_path || entry.path_url || '';
        if (!filePath) {
          showStatus('error', t("Knowledge.k296"));
          return;
        }
        const res = await kb.kbAttachmentPin({ entryId: entry.id, filePath });
        if (res.code === 0) {
          setPinnedEntries(prev => new Set(prev).add(entry.id));
          showStatus('success', t("Knowledge.k297"));
        } else {
          showStatus('error', res.message || t("common.failed"));
        }
      }
    } catch {
      showStatus('error', t("common.failed"));
    }
  }, [pinnedEntries, showStatus]);

  const formatTime = (ts: number) => {
    const d = new Date(ts);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  const getTypeIcon = (type?: string) => {
    switch (type) {
      case 'link': return '🔗';
      case 'file': return '📄';
      case 'text': return '📝';
      case 'video': return '🎬';
      case 'image': return '🖼️';
      case 'audio': return '🎵';
      case 'document': return '📑';
      default: return '📄';
    }
  };

  const getTypeLabel = (type?: string) => {
    switch (type) {
      case 'link': return t("components.TextEditor.k9");
      case 'file': return t("knowledge.GraphView.k1");
      case 'text': return t("knowledge.GraphView.k2");
      case 'video': return t("knowledge.GraphView.k3");
      case 'image': return t("knowledge.GraphView.k4");
      case 'audio': return t("knowledge.GraphView.k5");
      case 'document': return t("knowledge.GraphView.k6");
      default: return t("components.GroupChatOrchestrationPanel.k35");
    }
  };

  // ===== 数据库（左侧多库，仅 UI 状态层） =====
  const handleCreateDatabase = () => {
    if (!newDbName.trim()) {
      showStatus('error', t("Knowledge.k4"));
      return;
    }
    const id = `db_${Date.now()}`;
    const db: KbDatabase = { id, name: newDbName.trim(), created_at: Date.now() };
    setDatabases(prev => [...prev, db]);
    setCurrentDbId(id);
    setShowDbModal(false);
    setNewDbName('');
    showStatus('success', t("Knowledge.k5", { name: db.name }));
  };

  // ===== 模版（knowledge.templates，本轮为配合 browse 新建条目保留于壳） =====
  const handleCreateTemplate = async () => {
    if (!newTemplate.name.trim()) {
      showStatus('error', t("Knowledge.k84"));
      return;
    }
    try {
      const res = await kt.kbCreateTemplate({
        request: { name: newTemplate.name.trim(), icon: newTemplate.icon, description: newTemplate.description.trim(), entry_type: newTemplate.entry_type, content: newTemplate.content }
      });
      if (res.code === 0) {
        showStatus('success', t("Knowledge.k85", { name: newTemplate.name }));
        setNewTemplate({ name: '', icon: '📄', description: '', entry_type: 'text', content: '' });
        setShowTemplateManager(false);
        loadTemplates();
      } else {
        showStatus('error', res.message || t("Knowledge.k10"));
      }
    } catch {
      showStatus('error', t("Knowledge.k86"));
    }
  };

  const handleUpdateTemplate = async () => {
    if (!editingTemplate) return;
    try {
      const res = await kt.kbUpdateTemplate({
        request: { id: editingTemplate.id, name: editingTemplate.name, icon: editingTemplate.icon, description: editingTemplate.description, entry_type: editingTemplate.entry_type, content: editingTemplate.content }
      });
      if (res.code === 0) {
        showStatus('success', t("Knowledge.k87"));
        setEditingTemplate(null);
        loadTemplates();
      } else {
        showStatus('error', res.message || t("Knowledge.k40"));
      }
    } catch {
      showStatus('error', t("Knowledge.k88"));
    }
  };

  const handleDeleteTemplate = async (id: number) => {
    try {
      const res = await kt.kbDeleteTemplate({ id });
      if (res.code === 0) {
        showStatus('success', t("Knowledge.k89"));
        loadTemplates();
      } else {
        showStatus('error', res.message || t("errors.deleteFailed"));
      }
    } catch {
      showStatus('error', t("Knowledge.k90"));
    }
  };

  // ===== 核心数据层对象（注入各功能域 hook） =====
  const core: KnowledgeCore = {
    categories,
    allEntries,
    missingFiles,
    pinnedEntries,
    selectedId,
    currentLibrary,
    isOnline,
    sortMode,
    typeFilter,
    viewMode,
    viewFilterId,
    tags,
    tagStats,
    entryTags,
    categoryCounts,
    expandedIds,
    setExpandedIds,
    setSelectedId: (value) => setSelectedId(value),
    setPinnedEntries: (updater) => setPinnedEntries(updater),
    setSortMode,
    setTypeFilter,
    setViewMode,
    setViewFilterId,
    showStatus,
    reload: loadAll,
    loadTags,
    loadTagStats,
    loadAllEntryTags,
    loadEntryTags,
    nameConflict,
    setNameConflict,
    findNameConflict,
    resolveAutoRename,
    isExternalEntry,
    moveEntryToRecycle,
    getCategoryDepth,
    getEntriesInCategory,
    getAllDescendantIds,
    isPinnable,
    handleToggleFavorite,
    handleTogglePin,
    formatTime,
    formatFileSize,
    getTypeIcon,
    getTypeLabel
  };

  // ===== 功能域 hook（按依赖顺序调用） =====
  const ai = useAi(core);
  const importApi = useImport(core, ai);
  const media = useMedia(core);
  const editors = useEditors(core, media.close);
  const tagsApi = useTags(core);

  const handleOpenEntry = (entry: KbEntry) => {
    kb.recordKbAccess({ entryId: entry.id }).catch(() => {});
    // 记录活动日志：打开知识库条目
    intelligence.logActivity('1', new Date().toISOString().replace('T', ' ').slice(0, 19), 'knowledge', entry.entry_type === 'link' ? t("components.intelligence.ActivityPanel.k14") : t("components.intelligence.ActivityPanel.k15"), `name:${entry.name},path_url:${entry.path_url},source_path:${entry.source_path || ''}`).catch(() => {});
    if (entry.entry_type === 'link') {
      systemtools.openUrl(entry.path_url);
    } else {
      media.open(entry);
    }
  };

  const handleWikiLinkClick = (entryName: string) => {
    const entry = allEntries.find(e => e.name === entryName);
    if (entry) {
      handleOpenEntry(entry);
    } else {
      showStatus('error', t("Knowledge.k83", { entryName: entryName }));
    }
  };

  const historyApi = useHistory(core, handleOpenEntry);
  const search = useSearch(core, handleOpenEntry, importApi.setImporting);

  const selectedEntry = selectedId?.type === 'entry' ? allEntries.find(e => e.id === selectedId.id) ?? null : null;

  const browse = useBrowse(core, {
    handleOpenEntry,
    openFileViewer: media.open,
    handleWikiLinkClick,
    mediaTextColor: media.textColor,
    mediaClose: media.close,
    editors,
    searchIsSearching: search.isSearching,
    resetSearch: search.resetSearch,
    tagsApi,
    historyApi,
    aiClassifyMenuItem: ai.aiClassifyMenuItem,
    isLoading,
    pendingTemplate,
    setPendingTemplate,
    setShowTemplateModal
  });

  // 快捷切换器（Ctrl+P）+ Escape 关闭弹层
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'p') {
        e.preventDefault();
        browse.openQuickSwitcher();
      }
      if (e.key === 'Escape') {
        browse.closeQuickSwitcher();
        importApi.closeDirScanModal();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // ===== ?tab= 分发（侧边栏选项激活对应功能域；无 tab 保持默认浏览布局） =====
  useEffect(() => {
    if (!tab) return;
    switch (tab) {
      case 'material':
        localStorage.setItem('kb_library', 'material');
        setCurrentLibrary('material');
        break;
      case 'learning':
        localStorage.setItem('kb_library', 'study');
        setCurrentLibrary('study');
        break;
      case 'search':
        search.focusSearch();
        break;
      case 'import':
        importApi.openDirScan();
        break;
      case 'media':
        if (selectedEntry) media.open(selectedEntry);
        break;
      case 'editors':
        if (selectedEntry) editors.handleStartFileEdit(selectedEntry);
        break;
      case 'templates':
        setShowTemplateModal(true);
        break;
      case 'graph':
        setShowGraph(true);
        break;
      default:
        break;
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab]);

  return <div className={styles.page}>
      {statusMsg && <div className={`${styles.toast} ${statusMsg.type === 'error' ? styles.toastErr : styles.toastOk}`}>
          {statusMsg.text}
        </div>}

      {showDbModal && <DatabaseModal newDbName={newDbName} onNameChange={setNewDbName} onCreate={handleCreateDatabase} onCancel={() => setShowDbModal(false)} />}

      {(showTemplateModal || showTemplateManager) && <TemplateModals showTemplateModal={showTemplateModal} showTemplateManager={showTemplateManager} editingTemplate={editingTemplate} newTemplate={newTemplate} templates={templates} onClosePicker={() => setShowTemplateModal(false)} onOpenManager={() => {
      setShowTemplateModal(false);
      setShowTemplateManager(true);
    }} onCloseManager={() => {
      setShowTemplateManager(false);
      setEditingTemplate(null);
      setNewTemplate({ name: '', icon: '📄', description: '', entry_type: 'text', content: '' });
    }} onNewTemplateChange={updates => setNewTemplate(prev => ({ ...prev, ...updates }))} onEditingTemplateChange={setEditingTemplate} onEditingTemplateField={updates => setEditingTemplate(prev => prev ? { ...prev, ...updates } : null)} onInsertTemplate={browse.handleInsertTemplate} onCreateTemplate={handleCreateTemplate} onUpdateTemplate={handleUpdateTemplate} onDeleteTemplate={handleDeleteTemplate} />}

      {showGraph && <GraphView allEntries={allEntries} categories={categories} entryTags={entryTags} selectedId={selectedId} onClose={() => setShowGraph(false)} onOpenFileViewer={media.open} onSelectedIdChange={setSelectedId} onShowStatus={showStatus} />}

      {browse.renderModals()}

      {/* ===== 中间：目录树栏 ===== */}
      <aside className={styles.sidebar}>
        <header className={styles.sidebarToolbar}>
          {browse.renderSidebarButtons()}
          {importApi.renderImportToolbarButtons()}
          {ai.renderSidebarBadge()}
        </header>

        {importApi.renderTrackedPanel()}
        {browse.renderNewFolderForm()}
        {browse.renderViewTabs()}
        {tagsApi.renderTagBar()}
        {browse.renderFileTree()}
        {browse.renderBatchBar()}
      </aside>

      {browse.renderBatchModals()}

      {/* ===== 右侧：主内容区 ===== */}
      <main className={styles.main}>
        <header className={styles.toolbar}>
          <div className={styles.toolbarLeft}>
            {search.renderToolbarLeft()}
          </div>
          <div className={styles.toolbarRight}>
            {search.renderToolbarCount()}
            {browse.renderDisplayToggle(!search.isSearching && !selectedEntry)}
          </div>
        </header>

        {browse.renderNewEntryForm()}

        {selectedId && <div className={styles.breadcrumb}>
            {(() => {
          const parts: string[] = [];
          if (selectedId.type === 'entry') {
            let catId: number | null = selectedEntry?.category_id ?? null;
            while (catId) {
              const cat = categories.find(c => c.id === catId);
              if (cat) {
                parts.unshift(cat.name);
                catId = cat.parent_id;
              } else break;
            }
          } else {
            // 文件夹选择时，显示该文件夹的祖先路径
            let catId: number | null | undefined = selectedId.id;
            while (catId) {
              const cat = categories.find(c => c.id === catId);
              if (cat) {
                parts.unshift(cat.name);
                catId = cat.parent_id;
              } else break;
            }
          }
          return parts.length > 0 ? parts.join(' / ') : t("Knowledge.k212");
        })()} / {selectedEntry?.name ?? categories.find(c => c.id === selectedId.id)?.name}
            <button className={styles.breadcrumbClose} onClick={() => setSelectedId(null)} title={t("Knowledge.k213")}>×</button>
          </div>}

        {/* AI 智能体工具栏 */}
        {ai.renderAiBar(selectedEntry)}

        {/* AI 摘要已通过悬浮球展示 */}
        {ai.renderSummaryLoading()}

        {/* AI 标签建议 */}
        {ai.renderTagsPanel()}

        {tagsApi.renderFilterChips()}

        <section className={styles.contentArea}>
          {search.isSearching ? search.renderResults() : selectedEntry ? <article className={`${styles.previewPanel} ${media.hasEntry ? styles.previewPanelFull : ''}`}>
              {editors.renderActiveEditor(selectedEntry, allEntries.map(e => e.name), handleWikiLinkClick) || (media.hasEntry ? media.renderInline(editors) : browse.renderPreviewDefault(selectedEntry))}
            </article> : selectedId?.type === 'category' ? browse.renderFolderContent() : browse.renderGlobalContent()}
        </section>

        {importApi.renderDirScanModal()}

        {media.renderFullscreen(editors)}

        {browse.renderQuickSwitcher()}

      </main>
    </div>;
}
