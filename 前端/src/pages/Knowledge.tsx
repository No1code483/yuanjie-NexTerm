import { t } from "i18next";
import { useState, useEffect, useCallback, useRef } from 'react';
import { ipc, knowledge, intelligence } from '@/lib/ipc';
import { time } from '@/lib/utils';
import { useIntelligence } from '@/hooks/useIntelligence';
import { useNetworkStatus } from '@/hooks/useNetworkStatus';
import { useFloatingOrbStore } from '@/stores/floatingOrbStore';
import { convertFileSrc } from '@tauri-apps/api/core';
import { PptxViewer } from '@aiden0z/pptx-renderer';
import mammoth from 'mammoth';
import TextEditor from '@/components/TextEditor';
import TableEditor from '@/components/TableEditor';
import PdfViewer from '@/components/PdfViewer';
import CodePreview from '@/components/CodePreview';
import ImageViewer from '@/components/ImageViewer';
import MediaPlayer from '@/components/MediaPlayer';
import PptEditor from '@/components/PptEditor';
import PdfEditor from '@/components/PdfEditor';
import ImageEditor from '@/components/ImageEditor';
import AudioEditor from '@/components/AudioEditor';
import styles from './Knowledge.module.css';
import { SkeletonList } from '@/components/ui/Skeleton';
import type { KbCategory, KbEntry, TreeNode, KbDatabase, SortMode, CategoryCount, KbTrackedPath, KbTemplate, ConfirmDelete, KbTag, ViewMode, ScanDirFile } from './knowledge/types';
import { excelColName } from './knowledge/utils';
import DeleteConfirmModal from './knowledge/DeleteConfirmModal';
import NameConflictModal from './knowledge/NameConflictModal';
import TemplateModals from './knowledge/TemplateModals';
import DatabaseModal from './knowledge/DatabaseModal';
import ContextMenu from './knowledge/ContextMenu';
import MoveTargetModal from './knowledge/MoveTargetModal';
import TagBar from './knowledge/TagBar';
// T2.12 XSS 防护：用户上传的 text/html 文件需经 DOMPurify 消毒后再渲染
import { sanitizeHtml } from '@/utils/sanitize';
import ViewModeList from './knowledge/ViewModeList';
import GraphView from './knowledge/GraphView';
export default function Knowledge() {
  // DEBUG: 如果 Console 看不到这行，说明 HMR 没生效，需要重启 dev server
  console.log('[KB-DEBUG] Knowledge 组件渲染, 时间:', new Date().toLocaleTimeString());
  const [categories, setCategories] = useState<KbCategory[]>([]);
  const [allEntries, setAllEntries] = useState<KbEntry[]>([]);
  const [missingFiles, setMissingFiles] = useState<Set<number>>(new Set());
  // A5 Phase 3 Task 4: 已标记为「常用」的附件 entry_id 集合（离线可访问）
  const [pinnedEntries, setPinnedEntries] = useState<Set<number>>(new Set());
  const { isOnline } = useNetworkStatus();
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set());
  const [selectedId, setSelectedId] = useState<{
    type: 'category' | 'entry';
    id: number;
  } | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [isSearching, setIsSearching] = useState(false);
  const [searchResults, setSearchResults] = useState<KbEntry[]>([]);
  const [semanticSearch, setSemanticSearch] = useState(false);
  const [semanticSearchResults, setSemanticSearchResults] = useState<Array<{
    entry_id: number;
    title: string;
    content_preview: string;
    score: number;
    category_name: string;
    entry_type: string;
  }>>([]);
  const [showNewFolder, setShowNewFolder] = useState(false);
  const [newFolderName, setNewFolderName] = useState('');
  const [newFolderParentId, setNewFolderParentId] = useState<number | null>(null);
  const [showNewEntry, setShowNewEntry] = useState(false);
  const [newEntry, setNewEntry] = useState({
    name: '',
    entry_type: 'text',
    path_url: ''
  });
  const [statusMsg, setStatusMsg] = useState<{
    type: 'success' | 'error';
    text: string;
  } | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [sortMode, setSortMode] = useState<SortMode>(() => {
    const saved = typeof localStorage !== 'undefined' ? localStorage.getItem('kb_sort_mode') : null;
    return saved as SortMode || 'time_desc';
  });
  const [isLoading, setIsLoading] = useState(true);
  const [databases, setDatabases] = useState<KbDatabase[]>(() => {
    try {
      const raw = localStorage.getItem('kb_databases');
      return raw ? JSON.parse(raw) : [{
        id: 'default',
        name: t("Knowledge.k1"),
        created_at: Date.now()
      }];
    } catch {
      return [{
        id: 'default',
        name: t("Knowledge.k1"),
        created_at: Date.now()
      }];
    }
  });
  const [currentDbId, setCurrentDbId] = useState<string>(() => {
    return localStorage.getItem('kb_current_db') || 'default';
  });
  const [tagStats, setTagStats] = useState<Array<{
    tag_id: number;
    tag_name: string;
    tag_color: string;
    entry_count: number;
  }>>([]);
  const [editingTagId, setEditingTagId] = useState<number | null>(null);
  const [editingTagName, setEditingTagName] = useState('');
  const [editingTagColor, setEditingTagColor] = useState('#00F0FF');
  const [displayMode, setDisplayMode] = useState<'list' | 'card' | 'timeline'>('list');
  const [showDbModal, setShowDbModal] = useState(false);
  const [newDbName, setNewDbName] = useState('');
  const [confirmDelete, setConfirmDelete] = useState<ConfirmDelete | null>(null);
  const [editingId, setEditingId] = useState<{
    type: 'category' | 'entry';
    id: number;
  } | null>(null);
  const [editName, setEditName] = useState('');
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    node: TreeNode;
  } | null>(null);
  const [categoryCounts, setCategoryCounts] = useState<Map<number, number>>(new Map());
  const [moveTarget, setMoveTarget] = useState<{
    type: 'entry' | 'category';
    id: number;
  } | null>(null);
  const [currentLibrary, setCurrentLibrary] = useState<string>(() => {
    return typeof localStorage !== 'undefined' ? localStorage.getItem('kb_library') || 'material' : 'material';
  });
  const [dragOverId, setDragOverId] = useState<{
    type: 'entry' | 'category';
    id: number;
  } | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [dragOtherCategories, setDragOtherCategories] = useState<KbCategory[]>([]);
  const [dragOverLibraryRoot, setDragOverLibraryRoot] = useState<string | null>(null);
  const [nameConflict, setNameConflict] = useState<{
    newName: string;
    existingEntry: KbEntry;
    categoryId: number;
    onReplace: () => void;
    onRename: (renamed: string) => void;
  } | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('all');
  const [viewFilterId, setViewFilterId] = useState<number | null>(null);
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const [tags, setTags] = useState<KbTag[]>([]);
  const [entryTags, setEntryTags] = useState<Map<number, KbTag[]>>(new Map());
  const [selectedEntryIds, setSelectedEntryIds] = useState<Set<number>>(new Set());
  const [isMultiMode, setIsMultiMode] = useState(false);
  const [showBatchMove, setShowBatchMove] = useState(false);
  const [showBatchTagPicker, setShowBatchTagPicker] = useState(false);
  const [batchTagAction, setBatchTagAction] = useState<'add' | 'remove'>('add');
  const [editContent, setEditContent] = useState('');
  const [templates, setTemplates] = useState<KbTemplate[]>([]);
  const [showTemplateModal, setShowTemplateModal] = useState(false);
  const [showTemplateManager, setShowTemplateManager] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<KbTemplate | null>(null);
  const [newTemplate, setNewTemplate] = useState({
    name: '',
    icon: '📄',
    description: '',
    entry_type: 'text',
    content: ''
  });
  const [pendingTemplate, setPendingTemplate] = useState<KbTemplate | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);
  const [highlightTokens, setHighlightTokens] = useState<Array<{
    text: string;
    scope: string;
  }>>([]);
  const [csvPreviewData, setCsvPreviewData] = useState<{
    headers: string[];
    rows: string[][];
  } | null>(null);
  const [pdfPreviewUrl, setPdfPreviewUrl] = useState<string | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [showGraph, setShowGraph] = useState(false);
  const [aiCategoryLoading, setAiCategoryLoading] = useState(false);
  const [aiCategorySuggestion, setAiCategorySuggestion] = useState('');
  const {
    aiOn,
    featureOn,
    llmConfigured,
    llmConfig
  } = useIntelligence();
  const aiCategoryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [fileEditMode, setFileEditMode] = useState(false);
  const [fileEditContent, setFileEditContent] = useState('');
  const [fileEditFormatType, setFileEditFormatType] = useState('');
  const [tableEditMode, setTableEditMode] = useState(false);
  const [tableEditSheets, setTableEditSheets] = useState<Array<{
    name: string;
    rows: string[][];
    row_count: number;
    col_count: number;
  }>>([]);
  const [tableEditExt, setTableEditExt] = useState('');
  const [pptEditMode, setPptEditMode] = useState(false);
  const [pptEditSlides, setPptEditSlides] = useState<Array<{
    index: number;
    name: string;
    text_content: string;
    raw_xml: string;
  }>>([]);
  const [pdfEditMode, setPdfEditMode] = useState(false);
  const [imageEditMode, setImageEditMode] = useState(false);
  const [audioEditMode, setAudioEditMode] = useState(false);
  const [backlinks, setBacklinks] = useState<Array<{
    entry: KbEntry;
    snippet: string;
  }>>([]);
  const [backlinksLoading, setBacklinksLoading] = useState(false);
  const [snapshots, setSnapshots] = useState<Array<{
    id: number;
    entry_id: number;
    entry_name: string;
    content: string;
    content_length: number;
    created_at: number;
  }>>([]);
  const [showSnapshots, setShowSnapshots] = useState(false);
  const [snapshotsLoading, setSnapshotsLoading] = useState(false);
  const [snapshotPreview, setSnapshotPreview] = useState<{
    id: number;
    content: string;
  } | null>(null);
  const [aiClassifyResults, setAiClassifyResults] = useState<Array<{
    category_name: string;
    confidence: number;
    reason: string;
    description: string;
  }>>([]);
  const [aiClassifyLoading, setAiClassifyLoading] = useState(false);
  const [aiSummaryLoading, setAiSummaryLoading] = useState(false);
  const [aiTagsLoading, setAiTagsLoading] = useState(false);
  const [aiTags, setAiTags] = useState<string[]>([]);
  const [showDirScanModal, setShowDirScanModal] = useState(false);
  const [dirScanFiles, setDirScanFiles] = useState<ScanDirFile[]>([]);
  const [dirScanPath, setDirScanPath] = useState('');
  const [dirScanFilter, setDirScanFilter] = useState<'all' | 'text' | 'image' | 'video' | 'audio' | 'document' | 'other'>('all');
  const [dirScanSelected, setDirScanSelected] = useState<Set<number>>(new Set());
  const [dirScanLoading, setDirScanLoading] = useState(false);
  const [dirImporting, setDirImporting] = useState(false);
  const [showTrackedPaths, setShowTrackedPaths] = useState(false);
  const [trackedPaths, setTrackedPaths] = useState<KbTrackedPath[]>([]);
  const [trackedPathsLoading, setTrackedPathsLoading] = useState(false);
  const [pathCheckResult, setPathCheckResult] = useState<{
    invalid_tracked: number[];
    invalid_external_entries: number[];
  } | null>(null);
  const [pathChecking, setPathChecking] = useState(false);
  const [showQuickSwitcher, setShowQuickSwitcher] = useState(false);
  const [quickSwitcherQuery, setQuickSwitcherQuery] = useState('');
  const [quickSwitcherIndex, setQuickSwitcherIndex] = useState(0);
  const [mediaViewerEntry, setMediaViewerEntry] = useState<KbEntry | null>(null);
  const [mediaViewerBase64, setMediaViewerBase64] = useState('');
  const [mediaViewerMime, setMediaViewerMime] = useState('');
  const [mediaViewerTextContent, setMediaViewerTextContent] = useState('');
  const [mediaViewerHtmlContent, setMediaViewerHtmlContent] = useState('');
  const [mediaViewerLoading, setMediaViewerLoading] = useState(false);
  const [mediaViewerError, setMediaViewerError] = useState<string | null>(null);
  const [mediaViewerFullscreen, setMediaViewerFullscreen] = useState(false);
  const [mediaViewerTextColor, setMediaViewerTextColor] = useState('#E0E0E0');
  const [mediaViewerFileUrl, setMediaViewerFileUrl] = useState('');
  const [tableData, setTableData] = useState<{
    headers: string[];
    rows: string[][];
    totalCols: number;
  } | null>(null);
  const pptxViewerRef = useRef<any>(null);
  const [pptxSlideCount, setPptxSlideCount] = useState(0);
  const [pptxCurrentSlide, setPptxCurrentSlide] = useState(0);
  const pptxSlideRef = useRef<HTMLDivElement>(null);
  const pptxFullscreenSlideRef = useRef<HTMLDivElement>(null);
  const showStatus = useCallback((type: 'success' | 'error', text: string) => {
    setStatusMsg({
      type,
      text
    });
    setTimeout(() => setStatusMsg(null), 3000);
  }, []);
  const loadCategories = useCallback(async (library?: string) => {
    try {
      const lib = library || currentLibrary;
      const res = await ipc.invoke<KbCategory[]>('get_kb_categories', {
        library: lib
      });
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
      const res = await ipc.invoke<KbEntry[]>('get_all_kb_entries');
      if (res.code === 0 && res.data) {
        setAllEntries(res.data);
      }
    } catch {
      showStatus('error', t("Knowledge.k3"));
    }
  }, [showStatus]);

  // 独立检测：当 allEntries 加载完成后，批量检查文件存在性
  useEffect(() => {
    if (allEntries.length === 0) return;
    let cancelled = false;
    (async () => {
      const pathsToCheck: {
        id: number;
        name: string;
        checkPath: string;
      }[] = [];
      for (const e of allEntries) {
        const p = e.source_path || e.path_url || '';
        if (p && !p.startsWith('kb://')) {
          pathsToCheck.push({
            id: e.id,
            name: e.name,
            checkPath: p
          });
        }
      }
      if (pathsToCheck.length === 0) {
        if (!cancelled) setMissingFiles(new Set());
        return;
      }
      try {
        const checkRes = await knowledge.checkFilesExistence(pathsToCheck.map(p => p.checkPath));
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
  const loadCategoryCounts = useCallback(async () => {
    try {
      const res = await ipc.invoke<CategoryCount[]>('get_kb_category_counts');
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
      const res = await knowledge.attachmentListPinned();
      if (res.code === 0 && res.data) {
        setPinnedEntries(new Set(res.data.map((item: any) => item.entry_id as number)));
      }
    } catch {/* silent */}
  }, []);
  const loadAll = useCallback(async () => {
    setIsLoading(true);
    const lib = currentLibrary;
    await Promise.all([loadCategories(lib), loadEntries(), loadCategoryCounts(), loadTags(), loadEntryTags(), loadTagStats(), loadTemplates(), loadPinnedEntries()]);
    setIsLoading(false);
  }, [loadCategories, loadEntries, loadCategoryCounts, loadPinnedEntries, currentLibrary]);
  const loadTags = useCallback(async () => {
    try {
      const res = await ipc.invoke<KbTag[]>('get_kb_tags');
      if (res.code === 0 && res.data) setTags(res.data);
    } catch {/* silent */}
  }, []);
  const loadTagStats = useCallback(async () => {
    try {
      const res = await ipc.invoke<Array<{
        tag_id: number;
        tag_name: string;
        tag_color: string;
        entry_count: number;
      }>>('get_kb_tag_stats');
      if (res.code === 0 && res.data) setTagStats(res.data);
    } catch {/* silent */}
  }, []);
  const loadTemplates = useCallback(async () => {
    try {
      const res = await ipc.invoke<KbTemplate[]>('kb_get_templates');
      if (res.code === 0 && res.data) setTemplates(res.data);
    } catch {/* silent */}
  }, []);
  const loadEntryTags = useCallback(async (forEntryId?: number) => {
    try {
      if (forEntryId) {
        const r = await ipc.invoke<KbTag[]>('get_kb_entry_tags', {
          entry_id: forEntryId
        });
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
      const res = await ipc.invoke<Array<{
        entry_id: number;
        tags: KbTag[];
      }>>('get_kb_all_entry_tags');
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
  useEffect(() => {
    loadAll();
  }, [loadAll]);

  // 打开图谱时预加载标签数据
  useEffect(() => {
    if (showGraph) {
      loadAllEntryTags();
    }
  }, [showGraph, loadAllEntryTags]);

  // 全局 dragend 清理状态（防止拖放到树外时状态残留）
  useEffect(() => {
    const handleDragEnd = () => {
      setIsDragging(false);
      setDragOtherCategories([]);
      setDragOverId(null);
      setDragOverLibraryRoot(null);
    };
    window.addEventListener('dragend', handleDragEnd);
    return () => window.removeEventListener('dragend', handleDragEnd);
  }, []);
  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent).detail as string;
      if (detail === 'material' || detail === 'study') {
        localStorage.setItem('kb_library', detail);
        setCurrentLibrary(detail);
      }
    };
    window.addEventListener('kb-library-change', handler);
    return () => window.removeEventListener('kb-library-change', handler);
  }, []);
  useEffect(() => {
    if (selectedId?.type === 'entry') loadEntryTags(selectedId.id);
  }, [selectedId, loadEntryTags]);
  useEffect(() => {
    const handleKbAction = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail === 'new-db') {
        setShowDbModal(true);
        setNewDbName('');
      } else if (detail === 'insert-template') {
        setShowTemplateModal(true);
      } else if (detail === 'relation-graph') {
        setShowGraph(true);
      }
    };
    window.addEventListener('kb-action', handleKbAction);
    return () => window.removeEventListener('kb-action', handleKbAction);
  }, []);
  useEffect(() => {
    localStorage.setItem('kb_databases', JSON.stringify(databases));
    localStorage.setItem('kb_current_db', currentDbId);
  }, [databases, currentDbId]);
  const handleCreateDatabase = () => {
    if (!newDbName.trim()) {
      showStatus('error', t("Knowledge.k4"));
      return;
    }
    const id = `db_${Date.now()}`;
    const db: KbDatabase = {
      id,
      name: newDbName.trim(),
      created_at: Date.now()
    };
    setDatabases(prev => [...prev, db]);
    setCurrentDbId(id);
    setShowDbModal(false);
    setNewDbName('');
    showStatus('success', t("Knowledge.k5", {
      name: db.name
    }));
  };
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);
  useEffect(() => {
    if (!searchQuery.trim()) {
      if (isSearching) {
        setIsSearching(false);
        setSearchResults([]);
      }
      return;
    }
    const timer = setTimeout(() => {
      handleSearch();
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery]);
  const buildTree = useCallback((): TreeNode[] => {
    const getChildrenOfCategory = (catId: number): TreeNode[] => {
      let childCats = categories.filter(c => c.parent_id === catId).sort((a, b) => a.sort_order - b.sort_order);
      let entriesInCat = allEntries.filter(e => e.category_id === catId && (typeFilter === 'all' || e.entry_type === typeFilter));
      switch (sortMode) {
        case 'name_asc':
          childCats.sort((a, b) => a.name.localeCompare(b.name));
          entriesInCat.sort((a, b) => a.name.localeCompare(b.name));
          break;
        case 'name_desc':
          childCats.sort((a, b) => b.name.localeCompare(a.name));
          entriesInCat.sort((a, b) => b.name.localeCompare(a.name));
          break;
        case 'time_desc':
          entriesInCat.sort((a, b) => b.updated_at - a.updated_at);
          break;
        case 'time_asc':
          entriesInCat.sort((a, b) => a.updated_at - b.updated_at);
          break;
      }
      const nodes: TreeNode[] = [];
      for (const cat of childCats) {
        nodes.push({
          type: 'category',
          id: cat.id,
          name: cat.name,
          categoryId: cat.id,
          depth: getCategoryDepth(cat.id),
          children: getChildrenOfCategory(cat.id)
        });
      }
      for (const e of entriesInCat) {
        nodes.push({
          type: 'entry',
          id: e.id,
          name: e.name,
          categoryId: e.category_id,
          pathUrl: e.path_url,
          sourcePath: e.source_path || undefined,
          entryType: e.entry_type,
          updatedAt: e.updated_at,
          depth: getCategoryDepth(catId) + 1,
          children: [] as TreeNode[]
        });
      }
      return nodes;
    };
    const rootCats = categories.filter(c => !c.parent_id).sort((a, b) => a.sort_order - b.sort_order);
    return rootCats.map(cat => ({
      type: 'category' as const,
      id: cat.id,
      name: cat.name,
      categoryId: cat.id,
      depth: 0,
      children: getChildrenOfCategory(cat.id)
    }));
  }, [categories, allEntries, sortMode, typeFilter]);
  const getCategoryDepth = (id: number, depth: number = 0): number => {
    const cat = categories.find(c => c.id === id);
    if (!cat || !cat.parent_id) return depth;
    return getCategoryDepth(cat.parent_id, depth + 1);
  };
  const toggleExpand = (id: number) => {
    setExpandedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);else next.add(id);
      return next;
    });
  };
  const handleSelect = (type: 'category' | 'entry', id: number) => {
    setSelectedId({
      type,
      id
    });
    setIsSearching(false);
    setSearchQuery('');
    // 切换选择时，重置所有查看器/编辑状态，使预览区立即切换到新选中的条目
    setMediaViewerEntry(null);
    setFileEditMode(false);
    setTableEditMode(false);
    setPptEditMode(false);
  };
  const selectedEntry = selectedId?.type === 'entry' ? allEntries.find(e => e.id === selectedId.id) : null;
  const selectedCategoryEntries = selectedId?.type === 'category' ? allEntries.filter(e => e.category_id === selectedId.id).sort((a, b) => b.updated_at - a.updated_at) : [];
  // 当前选中分类的直接子文件夹
  const selectedSubCategories = selectedId?.type === 'category' ? categories.filter(c => c.parent_id === selectedId.id).sort((a, b) => a.name.localeCompare(b.name)) : [];
  useEffect(() => {
    if (selectedId?.type === 'entry' && selectedEntry?.entry_type === 'text') {
      setEditContent(selectedEntry.content || '');
    }
  }, [selectedId, selectedEntry]);
  useEffect(() => {
    const entry = selectedEntry;
    if (!entry) {
      setThumbnailUrl(null);
      setHighlightTokens([]);
      setCsvPreviewData(null);
      setPdfPreviewUrl(null);
      setPreviewError(null);
      return;
    }
    const url = (entry.path_url || '').toLowerCase();
    const extMatch = url.match(/\.([a-zA-Z0-9]+)$/);
    const ext = extMatch ? extMatch[1] : '';
    const imageExts = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg', 'ico'];
    const codeExts = ['rs', 'ts', 'tsx', 'js', 'jsx', 'py', 'go', 'java', 'c', 'cpp', 'h', 'hpp', 'cs', 'rb', 'php', 'swift', 'kt', 'scala', 'lua', 'sh', 'bash', 'zsh', 'yaml', 'yml', 'toml', 'json', 'xml', 'css', 'scss', 'less', 'html', 'htm', 'sql'];
    const languageMap: Record<string, string> = {
      rs: 'rust',
      ts: 'typescript',
      tsx: 'typescriptreact',
      js: 'javascript',
      jsx: 'javascriptreact',
      py: 'python',
      go: 'go',
      java: 'java',
      c: 'c',
      cpp: 'cpp',
      h: 'c',
      hpp: 'cpp',
      cs: 'csharp',
      rb: 'ruby',
      php: 'php',
      swift: 'swift',
      kt: 'kotlin',
      scala: 'scala',
      lua: 'lua',
      sh: 'bash',
      bash: 'bash',
      zsh: 'bash',
      yaml: 'yaml',
      yml: 'yaml',
      toml: 'toml',
      json: 'json',
      xml: 'xml',
      css: 'css',
      scss: 'scss',
      less: 'less',
      html: 'html',
      htm: 'html',
      sql: 'sql'
    };
    const loadPreview = async () => {
      setPreviewLoading(true);
      setPreviewError(null);
      setThumbnailUrl(null);
      setHighlightTokens([]);
      setCsvPreviewData(null);
      setPdfPreviewUrl(null);
      const isExternal = entry.path_url && !entry.path_url.startsWith('kb://') && !entry.path_url.startsWith('http');
      let fileContent = entry.content || '';
      if (isExternal && !fileContent && !imageExts.includes(ext) && ext !== 'pdf') {
        try {
          const readRes = await ipc.invoke<any>('kb_read_external_file', {
            path: entry.path_url
          });
          if (readRes.code === 0 && readRes.data) {
            fileContent = readRes.data.content;
          }
        } catch {/* 读不到就算了 */}
      }
      try {
        if (imageExts.includes(ext)) {
          if (isExternal) {
            const readRes = await ipc.invoke<any>('kb_read_file_base64', {
              path: entry.path_url
            });
            if (readRes.code === 0 && readRes.data?.base64) {
              setThumbnailUrl(`data:${readRes.data.mime_type};base64,${readRes.data.base64}`);
            } else {
              setPreviewError(t("Knowledge.k6"));
            }
          } else {
            const res = await ipc.invoke<any>('editor_generate_thumbnail', {
              request: {
                doc_uuid: `kb-entry-${entry.id}`,
                max_width: 400,
                max_height: 300
              }
            });
            if (res.code === 0 && res.data?.base64_png) {
              setThumbnailUrl(`data:image/png;base64,${res.data.base64_png}`);
            }
          }
        } else if (ext === 'pdf') {
          if (isExternal) {
            const readRes = await ipc.invoke<any>('kb_read_file_base64', {
              path: entry.path_url
            });
            if (readRes.code === 0 && readRes.data?.base64) {
              setPdfPreviewUrl(`data:application/pdf;base64,${readRes.data.base64}`);
            } else {
              setPreviewError(t("Knowledge.k7"));
            }
          } else {
            const res = await ipc.invoke<any>('editor_decrypted_preview', {
              docUuid: `kb-entry-${entry.id}`,
              contentType: 'application/pdf'
            });
            if (res.code === 0 && res.data?.base64_content) {
              setPdfPreviewUrl(`data:application/pdf;base64,${res.data.base64_content}`);
            }
          }
        } else if (ext === 'csv') {
          if (fileContent) {
            const lines = fileContent.trim().split('\n');
            const headers = lines[0] ? lines[0].split(',').map(h => h.trim()) : [];
            const rows = lines.slice(1, 21).map(l => l.split(',').map(c => c.trim()));
            setCsvPreviewData({
              headers,
              rows
            });
          }
        } else if (codeExts.includes(ext) && fileContent) {
          const lang = languageMap[ext] || ext;
          const res = await ipc.invoke<any>('editor_highlight', {
            request: {
              content: fileContent,
              language: lang
            }
          });
          if (res.code === 0 && res.data?.tokens) {
            setHighlightTokens(res.data.tokens);
          }
        }
      } catch {
        setPreviewError(t("Knowledge.k8"));
      } finally {
        setPreviewLoading(false);
      }
    };
    loadPreview();
  }, [selectedEntry]);
  const handleAddCategory = async () => {
    if (!newFolderName.trim()) return;
    try {
      const res = await ipc.invoke<KbCategory>('add_kb_category', {
        name: newFolderName.trim(),
        parent_id: newFolderParentId,
        library: currentLibrary,
        sort_order: 0
      });
      if (res.code === 0) {
        showStatus('success', t("Knowledge.k9", {
          newFolderName: newFolderName
        }));
        setNewFolderName('');
        setShowNewFolder(false);
        loadAll();
      } else showStatus('error', res.message || t("Knowledge.k10"));
    } catch {
      showStatus('error', t("Knowledge.k11"));
    }
  };
  const isExternalEntry = (entry: KbEntry): boolean => {
    const url = entry.path_url || '';
    return /^[a-zA-Z]:[\\/]/.test(url) || url.startsWith('/') || url.startsWith('\\\\');
  };
  const moveEntryToRecycle = async (entryId: number): Promise<boolean> => {
    try {
      const res = await ipc.invoke('recycle_move_to', {
        itemType: 'kb_entry',
        itemIds: [entryId]
      });
      return res.code === 0;
    } catch {
      return false;
    }
  };
  const handleDeleteCategory = async (id: number) => {
    const cat = categories.find(c => c.id === id);
    if (!cat) return;
    const subCount = categories.filter(c => c.parent_id === id).length;
    const entryCount = categoryCounts.get(id) || 0;
    setConfirmDelete({
      type: 'category',
      id,
      name: cat.name,
      subCount: subCount + entryCount
    });
  };
  const confirmDeleteCategory = async () => {
    if (!confirmDelete) return;
    const id = confirmDelete.id;
    try {
      const res = await ipc.invoke<{
        deleted_subfolders: number;
        deleted_entries: number;
      }>('move_kb_category_to_recycle', {
        id
      });
      if (res.code === 0) {
        const sub = res.data?.deleted_subfolders || 0;
        const ent = res.data?.deleted_entries || 0;
        showStatus('success', t("Knowledge.k12", {
          sub: sub,
          ent: ent
        }));
        if (selectedId?.id === id) setSelectedId(null);
        loadAll();
      } else showStatus('error', res.message || t("Knowledge.k13"));
    } catch (e) {
      showStatus('error', t("Knowledge.k14", {
        e: e
      }));
    }
    setConfirmDelete(null);
  };
  const getEntriesInCategory = useCallback((categoryId: number): KbEntry[] => {
    return allEntries.filter(e => e.category_id === categoryId);
  }, [allEntries]);
  const findNameConflict = useCallback((name: string, categoryId: number): KbEntry | null => {
    const siblings = getEntriesInCategory(categoryId);
    return siblings.find(e => e.name.toLowerCase() === name.toLowerCase()) || null;
  }, [getEntriesInCategory]);
  const resolveAutoRename = useCallback((name: string, categoryId: number): string => {
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
  }, [getEntriesInCategory]);
  const handleAddEntry = async () => {
    if (!newEntry.name.trim()) {
      showStatus('error', t("Knowledge.k15"));
      return;
    }
    const targetCategoryId = selectedId?.type === 'category' ? selectedId.id : categories[0]?.id;
    if (!targetCategoryId) {
      showStatus('error', t("Knowledge.k16"));
      return;
    }
    const finalName = findNameConflict(newEntry.name.trim(), targetCategoryId) ? resolveAutoRename(newEntry.name.trim(), targetCategoryId) : newEntry.name.trim();
    try {
      const res = await ipc.invoke<KbEntry>('add_kb_entry', {
        request: {
          category_id: targetCategoryId,
          name: finalName,
          path_url: newEntry.path_url.trim() || `kb://${finalName}`,
          entry_type: newEntry.entry_type
        }
      });
      if (res.code === 0) {
        const tmpl = pendingTemplate;
        if (tmpl?.content && res.data) {
          await ipc.invoke<KbEntry>('update_kb_entry', {
            request: {
              id: res.data.id,
              content: tmpl.content
            }
          });
        }
        showStatus('success', t("Knowledge.k17", {
          finalName: finalName,
          arg0: finalName !== newEntry.name.trim() ? t("Knowledge.k18") : ''
        }));
        setNewEntry({
          name: '',
          entry_type: 'text',
          path_url: ''
        });
        setShowNewEntry(false);
        setPendingTemplate(null);
        loadAll();
        intelligence.logActivity('user', new Date().toISOString(), 'knowledge', 'add_entry', finalName).catch(() => {});
      } else {
        showStatus('error', res.message || t("components.PptEditor.k4"));
        setPendingTemplate(null);
      }
    } catch {
      showStatus('error', t("Knowledge.k19"));
      setPendingTemplate(null);
    }
  };
  const handleDeleteEntry = async (id: number) => {
    const entry = allEntries.find(e => e.id === id);
    if (!entry) return;
    setConfirmDelete({
      type: 'entry',
      id,
      name: entry.name
    });
  };
  const confirmDeleteEntry = async () => {
    if (!confirmDelete || confirmDelete.type !== 'entry') return;
    const id = confirmDelete.id!;
    const entry = allEntries.find(e => e.id === id);
    try {
      if (entry && isExternalEntry(entry)) {
        const res = await ipc.invoke('delete_kb_entry', {
          id
        });
        if (res.code === 0) {
          showStatus('success', t("Knowledge.k20"));
          if (selectedId?.id === id) setSelectedId(null);
          loadAll();
          intelligence.logActivity('user', new Date().toISOString(), 'knowledge', 'delete_entry', entry?.name).catch(() => {});
        } else showStatus('error', res.message || t("errors.deleteFailed"));
      } else {
        const success = await moveEntryToRecycle(id);
        if (success) {
          showStatus('success', t("Knowledge.k21"));
          if (selectedId?.id === id) setSelectedId(null);
          loadAll();
          intelligence.logActivity('user', new Date().toISOString(), 'knowledge', 'delete_entry', entry?.name || String(id)).catch(() => {});
        } else showStatus('error', t("Knowledge.k13"));
      }
    } catch {
      showStatus('error', t("errors.deleteFailed"));
    }
    setConfirmDelete(null);
  };
  const startRename = (type: 'category' | 'entry', id: number, name: string) => {
    setEditingId({
      type,
      id
    });
    setEditName(name);
    setContextMenu(null);
  };
  const submitRename = async () => {
    if (!editingId || !editName.trim()) {
      setEditingId(null);
      return;
    }
    const {
      type,
      id
    } = editingId;
    try {
      if (type === 'category') {
        const res = await ipc.invoke<KbCategory>('update_kb_category', {
          id,
          name: editName.trim()
        });
        if (res.code === 0) loadAll();else showStatus('error', res.message || t("Knowledge.k22"));
      } else {
        const res = await ipc.invoke<KbEntry>('update_kb_entry', {
          request: {
            id,
            name: editName.trim()
          }
        });
        if (res.code === 0) loadAll();else showStatus('error', res.message || t("Knowledge.k22"));
      }
    } catch {
      showStatus('error', t("Knowledge.k22"));
    }
    setEditingId(null);
    setEditName('');
  };
  const handleMoveEntry = async (entryId: number, targetCategoryId: number) => {
    const entry = allEntries.find(e => e.id === entryId);
    if (!entry) {
      setContextMenu(null);
      setMoveTarget(null);
      return;
    }
    if (entry.category_id === targetCategoryId) {
      showStatus('error', t("Knowledge.k23"));
      setContextMenu(null);
      setMoveTarget(null);
      return;
    }
    const existing = findNameConflict(entry.name, targetCategoryId);
    if (existing) {
      setNameConflict({
        newName: entry.name,
        existingEntry: existing,
        categoryId: targetCategoryId,
        onReplace: async () => {
          setNameConflict(null);
          try {
            if (isExternalEntry(existing)) {
              await ipc.invoke('delete_kb_entry', {
                id: existing.id
              });
            } else {
              await moveEntryToRecycle(existing.id);
            }
            const res = await ipc.invoke<KbEntry>('move_kb_entry', {
              request: {
                id: entryId,
                target_category_id: targetCategoryId
              }
            });
            if (res.code === 0) {
              showStatus('success', t("Knowledge.k24", {
                name: entry.name
              }));
              loadAll();
            } else showStatus('error', res.message || t("Knowledge.k25"));
          } catch (e) {
            showStatus('error', t("Knowledge.k26", {
              e: e
            }));
          }
          setContextMenu(null);
          setMoveTarget(null);
        },
        onRename: async renamed => {
          setNameConflict(null);
          try {
            const _updateRes = await ipc.invoke<KbEntry>('update_kb_entry', {
              request: {
                id: entryId,
                name: renamed
              }
            });
            if (_updateRes.code === 0) {
              const res = await ipc.invoke<KbEntry>('move_kb_entry', {
                request: {
                  id: entryId,
                  target_category_id: targetCategoryId
                }
              });
              if (res.code === 0) {
                showStatus('success', t("Knowledge.k27", {
                  renamed: renamed
                }));
                loadAll();
              } else showStatus('error', res.message || t("Knowledge.k25"));
            } else showStatus('error', _updateRes.message || t("Knowledge.k22"));
          } catch (e) {
            showStatus('error', t("Knowledge.k26", {
              e: e
            }));
          }
          setContextMenu(null);
          setMoveTarget(null);
        }
      });
      return;
    }
    try {
      const res = await ipc.invoke<KbEntry>('move_kb_entry', {
        request: {
          id: entryId,
          target_category_id: targetCategoryId
        }
      });
      if (res.code === 0) {
        showStatus('success', t("Knowledge.k28"));
        loadAll();
      } else showStatus('error', res.message || t("Knowledge.k25"));
    } catch {
      showStatus('error', t("Knowledge.k25"));
    }
    setContextMenu(null);
    setMoveTarget(null);
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
  const handleMoveCategory = async (categoryId: number, targetParentId: number | null, targetLibrary?: string) => {
    setContextMenu(null);
    setMoveTarget(null);
    try {
      const res = await ipc.invoke<KbCategory>('move_kb_category', {
        request: {
          id: categoryId,
          target_parent_id: targetParentId,
          target_library: targetLibrary ?? null
        }
      });
      if (res.code === 0) {
        showStatus('success', t("Knowledge.k29"));
        loadAll();
      } else showStatus('error', res.message || t("Knowledge.k25"));
    } catch {
      showStatus('error', t("Knowledge.k30"));
    }
  };
  const handleRightClick = (e: React.MouseEvent, node: TreeNode) => {
    e.preventDefault();
    setContextMenu({
      x: e.clientX,
      y: e.clientY,
      node
    });
  };
  const handleViewModeChange = (mode: ViewMode) => {
    setViewMode(mode);
    setViewFilterId(null);
  };
  const handleTagFilter = (tagId: number) => {
    setViewMode('tag');
    setViewFilterId(tagId);
  };
  const handleToggleFavorite = async (entryId: number) => {
    try {
      const res = await ipc.invoke('toggle_kb_favorite', {
        entry_id: entryId
      });
      if (res.code === 0) loadAll();else showStatus('error', res.message || t("common.failed"));
    } catch {
      showStatus('error', t("common.failed"));
    }
  };
  // A5 Phase 3 Task 4: 判断条目是否可标记为常用（需为本地外部文件，非链接/非内部条目）
  const isPinnable = useCallback((entry: KbEntry): boolean => {
    if (entry.entry_type === 'link') return false;
    const p = entry.source_path || entry.path_url || '';
    return p !== '' && !p.startsWith('kb://') && !p.startsWith('http');
  }, []);
  // A5 Phase 3 Task 4: 切换附件「常用」标记（pin/unpin）
  const handleTogglePin = useCallback(async (entry: KbEntry, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    const isPinned = pinnedEntries.has(entry.id);
    try {
      if (isPinned) {
        const res = await knowledge.attachmentUnpin(entry.id);
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
        const res = await knowledge.attachmentPin(entry.id, filePath);
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
  const handleAddTagToEntry = async (entryId: number, tagId: number) => {
    try {
      const existing = entryTags.get(entryId)?.map(t => t.id) || [];
      const ids = [...new Set([...existing, tagId])];
      const res = await ipc.invoke('set_kb_entry_tags', {
        entry_id: entryId,
        tag_ids: ids
      });
      if (res.code === 0) {
        loadEntryTags(entryId);
        showStatus('success', t("Knowledge.k31"));
      } else showStatus('error', res.message || t("Knowledge.k32"));
    } catch {
      showStatus('error', t("Knowledge.k32"));
    }
  };
  const handleRemoveTagFromEntry = async (entryId: number, tagId: number) => {
    try {
      const existing = entryTags.get(entryId)?.map(t => t.id) || [];
      const ids = existing.filter(id => id !== tagId);
      const res = await ipc.invoke('set_kb_entry_tags', {
        entry_id: entryId,
        tag_ids: ids
      });
      if (res.code === 0) {
        loadEntryTags(entryId);
        showStatus('success', t("Knowledge.k33"));
      } else showStatus('error', res.message || t("Knowledge.k34"));
    } catch {
      showStatus('error', t("Knowledge.k34"));
    }
  };
  const handleAddGlobalTag = async (name: string) => {
    if (!name.trim()) return;
    try {
      const hues = [280, 200, 340, 160, 40, 100, 10, 50, 190, 320];
      const h = hues[Math.floor(Math.random() * hues.length)];
      const color = `hsl(${h}, 65%, 55%)`;
      const res = await ipc.invoke('add_kb_tag', {
        name: name.trim(),
        color
      });
      if (res.code === 0) {
        await loadTags();
        await loadTagStats();
        showStatus('success', t("Knowledge.k35", {
          arg0: name.trim()
        }));
      } else showStatus('error', res.message || t("Knowledge.k10"));
    } catch {
      showStatus('error', t("Knowledge.k36"));
    }
  };
  const handleDeleteGlobalTag = async (tagId: number) => {
    try {
      const res = await ipc.invoke('delete_kb_tag', {
        id: tagId
      });
      if (res.code === 0) {
        await loadTags();
        await loadTagStats();
        showStatus('success', t("Knowledge.k37"));
      } else showStatus('error', res.message || t("errors.deleteFailed"));
    } catch {
      showStatus('error', t("Knowledge.k38"));
    }
  };
  const handleUpdateGlobalTag = async () => {
    if (!editingTagId || !editingTagName.trim()) return;
    try {
      const res = await ipc.invoke('update_kb_tag', {
        id: editingTagId,
        name: editingTagName.trim(),
        color: editingTagColor
      });
      if (res.code === 0) {
        await loadTags();
        await loadTagStats();
        await loadAllEntryTags();
        showStatus('success', t("Knowledge.k39"));
        setEditingTagId(null);
      } else showStatus('error', res.message || t("Knowledge.k40"));
    } catch {
      showStatus('error', t("Knowledge.k41"));
    }
  };
  const startEditTag = (tag: KbTag) => {
    setEditingTagId(tag.id);
    setEditingTagName(tag.name);
    setEditingTagColor(tag.color);
  };
  const toggleEntrySelection = (entryId: number) => {
    setSelectedEntryIds(prev => {
      const next = new Set(prev);
      if (next.has(entryId)) next.delete(entryId);else next.add(entryId);
      return next;
    });
  };
  const handleBatchDelete = () => {
    if (selectedEntryIds.size === 0) return;
    setConfirmDelete({
      type: 'batch',
      batchCount: selectedEntryIds.size
    });
  };
  const confirmBatchDelete = async () => {
    if (!confirmDelete || confirmDelete.type !== 'batch') return;
    try {
      const ids = Array.from(selectedEntryIds);
      const entries = ids.map(id => allEntries.find(e => e.id === id)).filter(Boolean) as KbEntry[];
      const externalIds = entries.filter(isExternalEntry).map(e => e.id);
      const internalIds = entries.filter(e => !isExternalEntry(e)).map(e => e.id);
      let deletedCount = 0;
      let recycledCount = 0;
      if (externalIds.length > 0) {
        const res = await ipc.invoke('batch_delete_kb_entries', {
          request: {
            ids: externalIds
          }
        });
        if (res.code === 0) deletedCount = externalIds.length;
      }
      for (const eid of internalIds) {
        const ok = await moveEntryToRecycle(eid);
        if (ok) recycledCount++;
      }
      const parts: string[] = [];
      if (deletedCount > 0) parts.push(t("Knowledge.k42", {
        deletedCount: deletedCount
      }));
      if (recycledCount > 0) parts.push(t("Knowledge.k43", {
        recycledCount: recycledCount
      }));
      if (parts.length > 0) showStatus('success', parts.join('，'));else showStatus('error', t("common.failed"));
      setSelectedEntryIds(new Set());
      loadAll();
    } catch {
      showStatus('error', t("Knowledge.k44"));
    }
    setConfirmDelete(null);
  };
  const handleBatchMove = async (targetCategoryId: number) => {
    if (selectedEntryIds.size === 0) return;
    try {
      const ids = Array.from(selectedEntryIds);
      const conflicts: {
        entryId: number;
        name: string;
        existingId: number;
      }[] = [];
      for (const eid of ids) {
        const entry = allEntries.find(e => e.id === eid);
        if (!entry) continue;
        const existing = findNameConflict(entry.name, targetCategoryId);
        if (existing && !ids.includes(existing.id)) {
          conflicts.push({
            entryId: eid,
            name: entry.name,
            existingId: existing.id
          });
        }
      }
      if (conflicts.length > 0) {
        const firstConflictEntry = allEntries.find(e => e.id === conflicts[0].existingId);
        if (!firstConflictEntry) {
          showStatus('error', t("Knowledge.k45"));
          setShowBatchMove(false);
          return;
        }
        setNameConflict({
          newName: t("Knowledge.k46", {
            length: conflicts.length
          }),
          existingEntry: firstConflictEntry,
          categoryId: targetCategoryId,
          onReplace: async () => {
            setNameConflict(null);
            try {
              for (const c of conflicts) {
                const existEntry = allEntries.find(e => e.id === c.existingId);
                if (existEntry && isExternalEntry(existEntry)) {
                  await ipc.invoke('delete_kb_entry', {
                    id: c.existingId
                  });
                } else {
                  await moveEntryToRecycle(c.existingId);
                }
              }
              const res = await ipc.invoke('batch_move_kb_entries', {
                request: {
                  ids,
                  category_id: targetCategoryId
                }
              });
              if (res.code === 0) {
                showStatus('success', t("Knowledge.k47", {
                  length: ids.length,
                  arg0: conflicts.length
                }));
                setSelectedEntryIds(new Set());
                setShowBatchMove(false);
                loadAll();
              } else showStatus('error', res.message || t("Knowledge.k25"));
            } catch (e) {
              showStatus('error', t("Knowledge.k48", {
                e: e
              }));
            }
            setShowBatchMove(false);
          },
          onRename: async () => {
            setNameConflict(null);
            try {
              const res = await ipc.invoke('batch_move_kb_entries', {
                request: {
                  ids,
                  category_id: targetCategoryId
                }
              });
              if (res.code === 0) {
                let renamedCount = 0;
                for (const c of conflicts) {
                  const renamed = resolveAutoRename(c.name, targetCategoryId);
                  try {
                    await ipc.invoke('update_kb_entry', {
                      request: {
                        id: c.entryId,
                        name: renamed
                      }
                    });
                    renamedCount++;
                  } catch {/* skip */}
                }
                await loadAll();
                showStatus('success', t("Knowledge.k49", {
                  length: ids.length,
                  renamedCount: renamedCount
                }));
                setSelectedEntryIds(new Set());
                setShowBatchMove(false);
              } else showStatus('error', res.message || t("Knowledge.k25"));
            } catch (e) {
              showStatus('error', t("Knowledge.k48", {
                e: e
              }));
            }
            setShowBatchMove(false);
          }
        });
        return;
      }
      const res = await ipc.invoke('batch_move_kb_entries', {
        request: {
          ids,
          category_id: targetCategoryId
        }
      });
      if (res.code === 0) {
        showStatus('success', t("Knowledge.k50", {
          length: ids.length
        }));
        setSelectedEntryIds(new Set());
        setShowBatchMove(false);
        loadAll();
      } else showStatus('error', res.message || t("Knowledge.k25"));
    } catch {
      showStatus('error', t("Knowledge.k51"));
    }
    setShowBatchMove(false);
  };
  const toggleMultiMode = () => {
    setIsMultiMode(prev => !prev);
    if (isMultiMode) setSelectedEntryIds(new Set());
  };
  const handleBatchAddTag = async (tagId: number) => {
    if (selectedEntryIds.size === 0) return;
    try {
      const ids = Array.from(selectedEntryIds);
      const res = await ipc.invoke('batch_add_kb_tag', {
        entry_ids: ids,
        tag_id: tagId
      });
      if (res.code === 0) {
        showStatus('success', t("Knowledge.k52", {
          data: res.data
        }));
        await loadAllEntryTags();
        await loadTagStats();
        setShowBatchTagPicker(false);
      } else showStatus('error', res.message || t("Knowledge.k53"));
    } catch {
      showStatus('error', t("Knowledge.k54"));
    }
  };
  const handleBatchRemoveTag = async (tagId: number) => {
    if (selectedEntryIds.size === 0) return;
    try {
      const ids = Array.from(selectedEntryIds);
      const res = await ipc.invoke('batch_remove_kb_tag', {
        entry_ids: ids,
        tag_id: tagId
      });
      if (res.code === 0) {
        showStatus('success', t("Knowledge.k55", {
          data: res.data
        }));
        await loadAllEntryTags();
        await loadTagStats();
        setShowBatchTagPicker(false);
      } else showStatus('error', res.message || t("Knowledge.k56"));
    } catch {
      showStatus('error', t("Knowledge.k57"));
    }
  };
  const handleSearch = async () => {
    if (!searchQuery.trim()) return;
    setIsSearching(true);
    setIsImporting(true);
    try {
      if (semanticSearch) {
        const res = await ipc.invoke('kb_semantic_search', {
          request: {
            query: searchQuery.trim(),
            limit: 50
          }
        });
        if (res.code === 0 && res.data) {
          setSemanticSearchResults(res.data);
          setSearchResults([]);
        } else {
          setSemanticSearchResults([]);
        }
      } else {
        const res = await ipc.invoke<KbEntry[]>('search_kb_entries', {
          query: searchQuery.trim()
        });
        if (res.code === 0 && res.data) {
          setSearchResults(res.data);
          setSemanticSearchResults([]);
        } else setSearchResults([]);
      }
    } catch {
      showStatus('error', t("Knowledge.k58"));
    } finally {
      setIsImporting(false);
    }
  };
  const handleClearSearch = () => {
    setSearchQuery('');
    setIsSearching(false);
    setSearchResults([]);
    setSemanticSearchResults([]);
  };
  const editSupportedExts = new Set(['md', 'markdown', 'txt', 'text', 'json', 'jsonc', 'json5', 'xml', 'xaml', 'xsl', 'xslt', 'xsd', 'yaml', 'yml', 'toml', 'cfg', 'conf', 'ini', 'inf', 'cnf', 'env', 'envrc', 'properties', 'prop', 'csv', 'tsv', 'html', 'htm', 'xhtml', 'shtml', 'css', 'scss', 'sass', 'less', 'styl', 'js', 'jsx', 'mjs', 'cjs', 'ts', 'tsx', 'vue', 'svelte', 'astro', 'py', 'pyw', 'pyx', 'rs', 'rlib', 'go', 'java', 'kt', 'kts', 'scala', 'sc', 'groovy', 'gradle', 'php', 'phtml', 'phps', 'phpt', 'rb', 'rbw', 'rake', 'gemspec', 'pl', 'pm', 'pod', 'swift', 'dart', 'jl', 'lua', 'r', 'rprofile', 'rmd', 'rnw', 'erl', 'hrl', 'ex', 'exs', 'eex', 'heex', 'hs', 'lhs', 'ml', 'mli', 'clj', 'cljs', 'cljc', 'edn', 'elm', 'fs', 'fsx', 'fsi', 'fsscript', 'nim', 'nims', 'zig', 'sh', 'bash', 'zsh', 'fish', 'bat', 'cmd', 'ps1', 'psm1', 'psd1', 'makefile', 'mk', 'dockerfile', 'containerfile', 'cmake', 'sql', 'psql', 'mysql', 'hql', 'prql', 'sparql', 'rq', 'graphql', 'gql', 'ttl', 'nt', 'n3', 'rdf', 'owl', 'cypher', 'cql', 'proto', 'protobuf', 'thrift', 'avsc', 'avdl', 'wsdl', 'wadl', 'raml', 'oas', 'openapi', 'grpc', 'cue', 'dhall', 'nix', 'tf', 'tfvars', 'hcl', 'nomad', 'sentinel', 'smithy', 'handlebars', 'hbs', 'hbrs', 'mustache', 'ejs', 'ect', 'pug', 'jade', 'twig', 'jinja', 'jinja2', 'j2', 'liquid', 'njk', 'nunjucks', 'dust', 'haml', 'slim', 'erb', 'rhtml', 'volt', 'latte', 'blade', 'mjml', 'rst', 'rest', 'restructuredtext', 'asciidoc', 'adoc', 'textile', 'org', 'wiki', 'mediawiki', 'creole', 'typ', 'tex', 'latex', 'ltx', 'sty', 'cls', 'bib', 'bibtex', 'nfo', 'diz', 'log', 'srt', 'vtt', 'ass', 'ssa', 'sub', 'smi', 'lrc', 'patch', 'diff', 'rej', 'ron', 'eml', 'mbox', 'vcard', 'vcf', 'ics', 'ical', 'ifb', 'prisma', 'coffee', 'cson', 'iced', 'litcoffee', 'peg', 'pegjs', 'ohm', 'rnc', 'rng', 'dtd', 'sgml', 'sgm', 'ent', 'g4', 'ebnf', 'bnf', 'abnf', 'wast', 'wat', 'asm', 'sage', 'sagews', 'm', 'mat', 'octave', 'matlab', 'mma', 'nb', 'wl', 'wls', 'cirru', 'idr', 'lidr', 'agda', 'lagda', 'v', 'vhdl', 'vhd', 'sv', 'svh', 'glsl', 'vert', 'frag', 'tesc', 'tese', 'geom', 'comp', 'hlsl', 'fx', 'fxh', 'vsh', 'psh', 'wgsl', 'metal', 'opencl', 'cuh', 'ispc', 'plist', 'strings', 'dic', 'aff', 'po', 'pot', 'mo', 'lang', 'resx', 'resw', 'resjson', 'xliff', 'xlf', 'arb', 'ftl', 'fxml', 'mxml', 'xib', 'storyboard', 'abc', 'ly', 'ily', 'schemas', 'mod', 'pest', 'pomsky', 'nearley', 'a51', 'bsv', 'semgrep', 'smel', 'editorconfig', 'gitignore', 'gitattributes', 'gitmodules', 'dockerignore', 'npmrc', 'yarnrc', 'lock', 'psv', 'h', 'c', 'cpp', 'cc', 'cxx', 'hpp', 'hh', 'hxx']);
  const handleStartFileEdit = async (entry: KbEntry) => {
    const url = entry.path_url || '';
    const ext = url.split('.').pop()?.toLowerCase() || '';
    if (!editSupportedExts.has(ext)) {
      showStatus('error', t("Knowledge.k59", {
        ext: ext
      }));
      return;
    }
    try {
      const res = await ipc.invoke<any>('fileedit_read', {
        request: {
          path: url,
          ext
        }
      });
      if (res.code === 0 && res.data) {
        setFileEditContent(res.data.content);
        setFileEditFormatType(res.data.format_type);
        setFileEditMode(true);
      } else {
        showStatus('error', res.message || t("Knowledge.k60"));
      }
    } catch (e: any) {
      showStatus('error', e?.toString() || t("Knowledge.k61"));
    }
  };
  const handleStopFileEdit = () => {
    setFileEditMode(false);
    setFileEditContent('');
    setFileEditFormatType('');
  };
  const handleFileEditSaved = () => {
    setMediaViewerTextContent('');
    setMediaViewerHtmlContent('');
    setMediaViewerBase64('');
    setMediaViewerMime('');
    setMediaViewerEntry(null);
    setTableData(null);
  };
  const tableSupportedExts = new Set(['xlsx', 'xls', 'ods', 'csv']);

  // @ts-ignore -- reserved for future table editing feature
  const handleStartTableEdit = async (entry: KbEntry) => {
    const url = entry.path_url || '';
    const ext = url.split('.').pop()?.toLowerCase() || '';
    if (!tableSupportedExts.has(ext)) {
      showStatus('error', t("Knowledge.k62", {
        ext: ext
      }));
      return;
    }
    try {
      const res = await ipc.invoke<any>('tableedit_read', {
        path: url,
        ext
      });
      if (res.code === 0 && res.data) {
        setTableEditSheets(res.data.sheets || []);
        setTableEditExt(res.data.extension || ext);
        setTableEditMode(true);
      } else {
        showStatus('error', res.message || t("Knowledge.k63"));
      }
    } catch (e: any) {
      showStatus('error', e?.toString() || t("Knowledge.k64"));
    }
  };
  const handleStopTableEdit = () => {
    setTableEditMode(false);
    setTableEditSheets([]);
    setTableEditExt('');
  };
  const handleTableEditSaved = () => {
    setMediaViewerTextContent('');
    setMediaViewerHtmlContent('');
    setMediaViewerBase64('');
    setMediaViewerMime('');
    setMediaViewerEntry(null);
    setTableData(null);
  };

  // @ts-ignore -- reserved for future ppt edit feature
  const handleStartPptEdit = async (entry: KbEntry) => {
    const url = entry.path_url || '';
    const ext = url.split('.').pop()?.toLowerCase() || '';
    if (ext !== 'pptx') {
      showStatus('error', t("Knowledge.k59", {
        ext: ext
      }));
      return;
    }
    try {
      const res = await ipc.invoke<any>('pptedit_get_slides', {
        path: url
      });
      if (res.code === 0 && res.data && res.data.slides) {
        setPptEditSlides(res.data.slides);
        setPptEditMode(true);
      } else {
        showStatus('error', res.message || t("Knowledge.k65"));
      }
    } catch (e: any) {
      showStatus('error', e?.toString() || t("Knowledge.k66"));
    }
  };
  const handleStopPptEdit = () => {
    setPptEditMode(false);
    setPptEditSlides([]);
  };

  // @ts-expect-error -- reserved for future pdf edit feature
  const handleStartPdfEdit = () => {
    setPdfEditMode(true);
  };
  const handleStopPdfEdit = () => {
    setPdfEditMode(false);
  };

  // @ts-expect-error -- reserved for future image edit feature
  const handleStartImageEdit = () => {
    setImageEditMode(true);
  };
  const handleStopImageEdit = () => {
    setImageEditMode(false);
  };

  // @ts-expect-error -- reserved for future audio edit feature
  const handleStartAudioEdit = () => {
    setAudioEditMode(true);
  };
  const handleStopAudioEdit = () => {
    setAudioEditMode(false);
  };
  const handleOpenFileViewer = async (entry: KbEntry) => {
    ipc.invoke('record_kb_access', {
      entry_id: entry.id
    }).catch(() => {});
    // 记录活动日志：打开知识库文件
    intelligence.logActivity('1', new Date().toISOString().replace('T', ' ').slice(0, 19), 'knowledge', t("components.intelligence.ActivityPanel.k13"), `name:${entry.name},path_url:${entry.path_url},source_path:${entry.source_path || ''}`).catch(() => {});
    setMediaViewerEntry(entry);
    setMediaViewerBase64('');
    setMediaViewerMime('');
    setMediaViewerTextContent('');
    setMediaViewerHtmlContent('');
    setMediaViewerError(null);
    setMediaViewerLoading(true);
    setMediaViewerFullscreen(false);
    setTableData(null);
    let url = entry.path_url || '';
    // A5 Phase 3 Task 4.5: 离线时使用本地缓存路径（仅对已标记为「常用」的附件）
    if (!isOnline && pinnedEntries.has(entry.id)) {
      try {
        const cachedPathRes = await knowledge.attachmentGetCachedPath(entry.id);
        if (cachedPathRes.code === 0 && cachedPathRes.data) {
          url = cachedPathRes.data;
        }
      } catch {/* 缓存路径获取失败，使用原始路径 */}
    }
    const ext = url.split('.').pop()?.toLowerCase() || '';
    const isInternal = !url || url.startsWith('kb://');
    if (['f4v', 'flv'].includes(ext)) {
      setMediaViewerError(t("Knowledge.k67"));
      setMediaViewerLoading(false);
      return;
    }
    const docxExts = ['docx', 'odt'];
    const epubExts = ['epub'];
    const rtfExts = ['rtf'];
    const legacyDocExts = ['doc'];
    const pptExts = ['pptx', 'ppt'];
    const odpExts = ['odp'];
    const archiveExts = ['zip', 'rar'];
    const designExts = ['psd', 'ai'];
    const tableExts = ['xlsx', 'xls', 'ods'];
    const textExts = [
    // 标记语言
    'md', 'markdown', 'mdown', 'mkd', 'mkdn', 'rst', 'rest', 'restructuredtext', 'asciidoc', 'adoc', 'textile', 'pod', 'org', 'wiki', 'mediawiki', 'creole', 'typ', 'tex', 'latex', 'ltx', 'sty', 'cls', 'bib', 'bibtex', 'nfo', 'diz',
    // 纯文本
    'txt', 'text', 'log', 'csv', 'tsv', 'psv', 'srt', 'vtt', 'ass', 'ssa', 'sub', 'smi', 'lrc',
    // 数据格式
    'json', 'jsonc', 'json5', 'xml', 'xaml', 'xsl', 'xslt', 'xsd', 'yaml', 'yml', 'toml', 'cfg', 'conf', 'ini', 'inf', 'cnf', 'env', 'envrc', 'properties', 'prop', 'lock', 'editorconfig', 'gitignore', 'gitattributes', 'gitmodules', 'dockerignore', 'npmrc', 'yarnrc',
    // 前端
    'html', 'htm', 'xhtml', 'shtml', 'css', 'scss', 'sass', 'less', 'styl', 'js', 'jsx', 'mjs', 'cjs', 'ts', 'tsx', 'vue', 'svelte', 'astro', 'jsx', 'tsx',
    // 后端
    'py', 'pyw', 'pyx', 'rs', 'rlib', 'go', 'java', 'kt', 'kts', 'scala', 'sc', 'groovy', 'gvy', 'gy', 'gradle', 'php', 'phtml', 'php3', 'php4', 'php5', 'phps', 'phpt', 'rb', 'rbw', 'rake', 'gemspec', 'pl', 'pm', 'pod', 't', 'swift', 'dart', 'jl', 'lua', 'r', 'R', 'rprofile', 'rmd', 'rnw',
    // 函数式
    'erl', 'hrl', 'ex', 'exs', 'eex', 'leex', 'heex', 'hs', 'lhs', 'ml', 'mli', 'clj', 'cljs', 'cljc', 'edn', 'elm', 'fs', 'fsx', 'fsi', 'fsscript', 'nim', 'nims', 'zig',
    // 系统/脚本
    'sh', 'bash', 'zsh', 'fish', 'bat', 'cmd', 'ps1', 'psm1', 'psd1', 'makefile', 'mk', 'dockerfile', 'containerfile', 'cmake', 'cmake.in',
    // 数据库/查询
    'sql', 'psql', 'mysql', 'hql', 'prql', 'sparql', 'rq', 'graphql', 'gql', 'ttl', 'nt', 'n3', 'rdf', 'owl', 'cypher', 'cql',
    // 协议/接口
    'proto', 'protobuf', 'thrift', 'avsc', 'avdl', 'wsdl', 'wadl', 'raml', 'oas', 'openapi', 'grpc', 'cue', 'dhall', 'nix', 'tf', 'tfvars', 'hcl', 'nomad', 'sentinel', 'smel', 'smithy',
    // 模板
    'handlebars', 'hbs', 'hbrs', 'mustache', 'ejs', 'ect', 'pug', 'jade', 'twig', 'jinja', 'jinja2', 'j2', 'liquid', 'njk', 'nunjucks', 'dust', 'haml', 'slim', 'erb', 'rhtml', 'volt', 'latte', 'blade', 'blade.php', 'mjml',
    // 其他文本
    'patch', 'diff', 'rej', 'ron', 'eml', 'mbox', 'vcard', 'vcf', 'ics', 'ical', 'ifb', 'icalendar', 'prisma', 'coffee', 'cson', 'iced', 'litcoffee', 'peg', 'pegjs', 'ohm', 'arvo', 'abc', 'ly', 'ily', 'rnc', 'rng', 'schemas', 'dtd', 'sgml', 'sgm', 'ent', 'mod', 'g4', 'ebnf', 'bnf', 'abnf', 'pest', 'pomsky', 'nearley', 'wast', 'wat', 'asm', 's', 'S', 'sage', 'sagews', 'm', 'mat', 'octave', 'matlab', 'mma', 'nb', 'wl', 'wls', 'cirru', 'idr', 'lidr', 'agda', 'lagda', 'v', 'vhdl', 'vhd', 'sv', 'svh', 'bsv', 'a51', 'ispc', 'opencl', 'cl', 'cuh', 'metal', 'wgsl', 'glsl', 'vert', 'frag', 'tesc', 'tese', 'geom', 'comp', 'hlsl', 'fx', 'fxh', 'vsh', 'psh', 'semgrep', 'ftl', 'fxml', 'mxml', 'xib', 'storyboard', 'plist', 'strings', 'dic', 'aff', 'po', 'pot', 'mo', 'lang', 'resx', 'resw', 'resjson', 'xliff', 'xlf', 'arb'];
    const mimeMap: Record<string, string> = {
      pdf: 'application/pdf',
      mp4: 'video/mp4',
      webm: 'video/webm',
      ogg: 'video/ogg',
      f4v: 'video/mp4',
      flv: 'video/x-flv',
      mp3: 'audio/mpeg',
      wav: 'audio/wav',
      oga: 'audio/ogg',
      png: 'image/png',
      jpg: 'image/jpeg',
      jpeg: 'image/jpeg',
      gif: 'image/gif',
      webp: 'image/webp',
      svg: 'image/svg+xml',
      bmp: 'image/bmp',
      ico: 'image/x-icon',
      tiff: 'image/tiff',
      tif: 'image/tiff',
      avif: 'image/avif',
      heic: 'image/heic',
      heif: 'image/heif',
      avi: 'video/x-msvideo',
      mkv: 'video/x-matroska',
      mov: 'video/quicktime',
      wmv: 'video/x-ms-wmv',
      m4v: 'video/mp4',
      '3gp': 'video/3gpp',
      flac: 'audio/flac',
      aac: 'audio/aac',
      wma: 'audio/x-ms-wma',
      m4a: 'audio/mp4',
      opus: 'audio/opus',
      aiff: 'audio/aiff'
    };
    try {
      if (isInternal || entry.entry_type === 'text' || textExts.includes(ext)) {
        let text = entry.content || '';
        if (!text && !isInternal) {
          const readRes = await ipc.invoke<any>('kb_read_external_file', {
            path: url
          });
          if (readRes.code === 0 && readRes.data) {
            text = readRes.data.content;
          } else {
            setMediaViewerError(readRes.message || t("Knowledge.k60"));
            return;
          }
        }
        if (!text.trim()) {
          setMediaViewerTextContent('');
          setMediaViewerMime('text/plain');
          setMediaViewerLoading(false);
          return;
        }
        setMediaViewerTextContent(text);
        setMediaViewerMime('text/plain');
      } else if (tableExts.includes(ext)) {
        const res = await ipc.invoke<any>('kb_extract_table_data', {
          path: url
        });
        if (res.code === 0 && res.data) {
          setTableData({
            headers: res.data.headers,
            rows: res.data.rows,
            totalCols: res.data.total_cols
          });
          setMediaViewerMime('table');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k68"));
        }
      } else if (pptExts.includes(ext)) {
        const readRes = await ipc.invoke<any>('kb_read_file_base64', {
          path: url
        });
        if (readRes.code === 0 && readRes.data?.base64) {
          const binaryStr = atob(readRes.data.base64);
          const bytes = new Uint8Array(binaryStr.length);
          for (let i = 0; i < binaryStr.length; i++) {
            bytes[i] = binaryStr.charCodeAt(i);
          }
          const arrayBuffer = bytes.buffer;
          pptxArrayBufRef.current = arrayBuffer;
          if (pptxViewerRef.current) {
            try {
              pptxViewerRef.current.destroy();
            } catch (_) {}
            pptxViewerRef.current = null;
          }
          setPptxSlideCount(0);
          setPptxCurrentSlide(0);
          setMediaViewerMime('pptx');
          setMediaViewerLoading(true);
          requestAnimationFrame(() => {
            const container = mediaViewerFullscreen ? pptxFullscreenSlideRef.current : pptxSlideRef.current;
            if (!container) {
              setMediaViewerError(t("Knowledge.k69"));
              return;
            }
            container.replaceChildren();
            PptxViewer.open(arrayBuffer, container, {
              renderMode: 'slide',
              fitMode: 'contain',
              onSlideChange: (index: number) => setPptxCurrentSlide(index)
            }).then((viewer: any) => {
              pptxViewerRef.current = viewer;
              setPptxSlideCount(viewer.slideCount || 1);
              setPptxCurrentSlide(0);
              setMediaViewerLoading(false);
            }).catch((err: Error) => {
              console.error('[pptx] PptxViewer.open error:', err);
              setMediaViewerError(t("Knowledge.k70", {
                message: err.message
              }));
              setMediaViewerLoading(false);
            });
          });
        } else {
          setMediaViewerError(readRes.message || t("Knowledge.k71"));
        }
      } else if (odpExts.includes(ext)) {
        const res = await ipc.invoke<any>('kb_extract_odp_text', {
          path: url
        });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k72"));
        }
      } else if (docxExts.includes(ext)) {
        // 使用 mammoth.js 将 .docx 转为 HTML 渲染（保留图片、表格、格式）
        try {
          const res = await ipc.invoke<any>('kb_read_file_base64', {
            path: url
          });
          if (res.code === 0 && res.data && res.data.base64) {
            const binaryStr = atob(res.data.base64);
            const bytes = new Uint8Array(binaryStr.length);
            for (let i = 0; i < binaryStr.length; i++) {
              bytes[i] = binaryStr.charCodeAt(i);
            }
            const result = await mammoth.convertToHtml({
              arrayBuffer: bytes.buffer
            });
            const htmlContent = result.value;
            // 添加基础样式
            const styledHtml = `
              <style>
                .mammoth-doc { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; color: #E0E0E0; line-height: 1.8; padding: 8px; }
                .mammoth-doc h1, .mammoth-doc h2, .mammoth-doc h3 { margin: 16px 0 8px; }
                .mammoth-doc p { margin: 8px 0; }
                .mammoth-doc table { border-collapse: collapse; width: 100%; margin: 12px 0; }
                .mammoth-doc td, .mammoth-doc th { border: 1px solid #555; padding: 6px 10px; text-align: left; }
                .mammoth-doc img { max-width: 100%; height: auto; margin: 8px 0; }
                .mammoth-doc ul, .mammoth-doc ol { padding-left: 24px; }
                .mammoth-doc a { color: #4FC3F7; }
                .mammoth-doc blockquote { border-left: 3px solid #666; padding-left: 12px; margin: 8px 0; color: #AAA; }
              </style>
              <div class="mammoth-doc">${htmlContent}</div>
            `;
            setMediaViewerHtmlContent(styledHtml);
            setMediaViewerMime('text/html');
          } else {
            setMediaViewerError(res.message || t("Knowledge.k73"));
          }
        } catch (e: any) {
          console.error('mammoth conversion error:', e);
          // 降级：使用纯文本提取
          const textRes = await ipc.invoke<any>('kb_extract_docx_text', {
            path: url
          });
          if (textRes.code === 0 && textRes.data && textRes.data.content) {
            setMediaViewerTextContent(textRes.data.content);
            setMediaViewerMime('text/plain');
          } else {
            setMediaViewerError(textRes.message || t("Knowledge.k74"));
          }
        }
      } else if (epubExts.includes(ext)) {
        const res = await ipc.invoke<any>('kb_extract_epub_text', {
          path: url
        });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k75"));
        }
      } else if (legacyDocExts.includes(ext)) {
        const res = await ipc.invoke<any>('kb_extract_doc_text', {
          path: url
        });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k76"));
        }
      } else if (rtfExts.includes(ext)) {
        const res = await ipc.invoke<any>('kb_extract_rtf_text', {
          path: url
        });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k77"));
        }
      } else if (archiveExts.includes(ext)) {
        const res = await ipc.invoke<any>('kb_list_zip_contents', {
          path: url
        });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k78"));
        }
      } else if (designExts.includes(ext)) {
        const cmd = ext === 'psd' ? 'kb_extract_psd_info' : 'kb_extract_ai_info';
        const res = await ipc.invoke<any>(cmd, {
          path: url
        });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k79"));
        }
      } else if (ext === 'pdf') {
        const readRes = await ipc.invoke<any>('kb_read_file_base64', {
          path: url
        });
        if (readRes.code === 0 && readRes.data?.base64) {
          setMediaViewerBase64(readRes.data.base64);
          setMediaViewerMime('application/pdf');
        } else {
          setMediaViewerError(readRes.message || t("Knowledge.k80"));
        }
      } else {
        const mime = mimeMap[ext] || 'application/octet-stream';
        if (mime.startsWith('video/') || mime.startsWith('audio/') || mime.startsWith('image/')) {
          const readRes = await ipc.invoke<any>('kb_read_file_base64', {
            path: url
          });
          if (readRes.code === 0 && readRes.data?.base64) {
            setMediaViewerFileUrl(`data:${mime};base64,${readRes.data.base64}`);
            setMediaViewerMime(mime);
          } else {
            setMediaViewerError(readRes.message || t("Knowledge.k81"));
          }
        } else {
          const fileUrl = convertFileSrc(url);
          setMediaViewerFileUrl(fileUrl);
          setMediaViewerMime(mime);
        }
      }
    } catch (err) {
      setMediaViewerError(t("Knowledge.k82", {
        arg0: err instanceof Error ? err.message : String(err)
      }));
    } finally {
      setMediaViewerLoading(false);
    }
  };
  const handleCloseMediaViewer = () => {
    if (pptxViewerRef.current) {
      try {
        pptxViewerRef.current.destroy();
      } catch (_) {}
      pptxViewerRef.current = null;
    }
    pptxArrayBufRef.current = null;
    setMediaViewerEntry(null);
    setMediaViewerBase64('');
    setMediaViewerMime('');
    setMediaViewerTextContent('');
    setMediaViewerHtmlContent('');
    setMediaViewerFileUrl('');
    setMediaViewerError(null);
    setMediaViewerFullscreen(false);
    setTableData(null);
    setPptxSlideCount(0);
    setPptxCurrentSlide(0);
  };
  const handleOpenEntry = (entry: KbEntry) => {
    ipc.invoke('record_kb_access', {
      entry_id: entry.id
    }).catch(() => {});
    // 记录活动日志：打开知识库条目
    intelligence.logActivity('1', new Date().toISOString().replace('T', ' ').slice(0, 19), 'knowledge', entry.entry_type === 'link' ? t("components.intelligence.ActivityPanel.k14") : t("components.intelligence.ActivityPanel.k15"), `name:${entry.name},path_url:${entry.path_url},source_path:${entry.source_path || ''}`).catch(() => {});
    if (entry.entry_type === 'link') {
      ipc.invoke('system_open_url', {
        url: entry.path_url
      });
    } else {
      handleOpenFileViewer(entry);
    }
  };
  const handleWikiLinkClick = (entryName: string) => {
    const entry = allEntries.find(e => e.name === entryName);
    if (entry) {
      handleOpenEntry(entry);
    } else {
      showStatus('error', t("Knowledge.k83", {
        entryName: entryName
      }));
    }
  };
  const handleInsertTemplate = async (template: KbTemplate) => {
    setShowTemplateModal(false);
    setPendingTemplate(template);
    setNewEntry({
      name: template.name,
      entry_type: template.entry_type,
      path_url: ''
    });
    setShowNewEntry(true);
  };
  const handleCreateTemplate = async () => {
    if (!newTemplate.name.trim()) {
      showStatus('error', t("Knowledge.k84"));
      return;
    }
    try {
      const res = await ipc.invoke<KbTemplate>('kb_create_template', {
        request: {
          name: newTemplate.name.trim(),
          icon: newTemplate.icon,
          description: newTemplate.description.trim(),
          entry_type: newTemplate.entry_type,
          content: newTemplate.content
        }
      });
      if (res.code === 0) {
        showStatus('success', t("Knowledge.k85", {
          name: newTemplate.name
        }));
        setNewTemplate({
          name: '',
          icon: '📄',
          description: '',
          entry_type: 'text',
          content: ''
        });
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
      const res = await ipc.invoke<KbTemplate>('kb_update_template', {
        request: {
          id: editingTemplate.id,
          name: editingTemplate.name,
          icon: editingTemplate.icon,
          description: editingTemplate.description,
          entry_type: editingTemplate.entry_type,
          content: editingTemplate.content
        }
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
      const res = await ipc.invoke('kb_delete_template', {
        id
      });
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
  const handleImportFile = async () => {
    try {
      const {
        open
      } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        title: t("Knowledge.k91"),
        directory: false,
        filters: [{
          name: t("Knowledge.k92"),
          extensions: ['*']
        }]
      });
      if (!selected) return;
      let targetCategoryId = selectedId?.type === 'category' ? selectedId.id : categories[0]?.id ?? null;
      if (!targetCategoryId) {
        showStatus('error', t("Knowledge.k93"));
        return;
      }
      const pathStr = String(selected);
      const fileName = pathStr.split(/[\\/]/).pop() || t("Knowledge.k94");
      const ext = fileName.split('.').pop()?.toLowerCase() || '';
      const entryType = ['pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx'].includes(ext) ? 'file' : ['md', 'txt', 'rst'].includes(ext) ? 'text' : ['mp4', 'avi', 'mkv', 'mov', 'f4v', 'flv'].includes(ext) ? 'video' : ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg'].includes(ext) ? 'image' : ['mp3', 'wav', 'oga', 'ogg'].includes(ext) ? 'audio' : 'file';
      const existing = findNameConflict(fileName, targetCategoryId);
      if (existing) {
        setNameConflict({
          newName: fileName,
          existingEntry: existing,
          categoryId: targetCategoryId,
          onReplace: async () => {
            setNameConflict(null);
            try {
              if (isExternalEntry(existing)) {
                await ipc.invoke('delete_kb_entry', {
                  id: existing.id
                });
              } else {
                await moveEntryToRecycle(existing.id);
              }
              const res = await ipc.invoke<KbEntry>('add_kb_entry', {
                request: {
                  category_id: targetCategoryId,
                  name: fileName,
                  path_url: pathStr,
                  entry_type: entryType
                }
              });
              if (res.code === 0) {
                showStatus('success', t("Knowledge.k95", {
                  fileName: fileName
                }));
                loadAll();
              } else showStatus('error', res.message || t("Knowledge.k96"));
            } catch (e) {
              showStatus('error', t("Knowledge.k97", {
                e: e
              }));
            }
          },
          onRename: async renamed => {
            setNameConflict(null);
            try {
              const res = await ipc.invoke<KbEntry>('add_kb_entry', {
                request: {
                  category_id: targetCategoryId,
                  name: renamed,
                  path_url: pathStr,
                  entry_type: entryType
                }
              });
              if (res.code === 0) {
                showStatus('success', t("Knowledge.k98", {
                  renamed: renamed
                }));
                loadAll();
              } else showStatus('error', res.message || t("Knowledge.k99"));
            } catch (e) {
              showStatus('error', t("Knowledge.k100", {
                e: e
              }));
            }
          }
        });
        return;
      }
      const res = await ipc.invoke<KbEntry>('add_kb_entry', {
        request: {
          category_id: targetCategoryId,
          name: fileName,
          path_url: pathStr,
          entry_type: entryType
        }
      });
      if (res.code === 0) {
        showStatus('success', t("Knowledge.k101", {
          fileName: fileName
        }));
        loadAll();
      } else showStatus('error', res.message || t("Knowledge.k99"));
    } catch (err) {
      showStatus('error', t("Knowledge.k102", {
        arg0: err instanceof Error ? err.message : String(err)
      }));
    }
  };
  const handleImportFolder = async () => {
    try {
      const {
        open
      } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: true,
        title: t("Knowledge.k103"),
        directory: true
      });
      if (!selected) return;
      const paths: string[] = Array.isArray(selected) ? selected.map(String) : [String(selected)];
      let targetCategoryId = selectedId?.type === 'category' ? selectedId.id : categories[0]?.id ?? null;
      if (!targetCategoryId) {
        showStatus('error', t("Knowledge.k104"));
        return;
      }
      setIsImporting(true);
      showStatus('success', t("Knowledge.k105", {
        length: paths.length
      }));
      const res = await ipc.invoke<{
        categories: KbCategory[];
        entries: KbEntry[];
        success_count: number;
        fail_count: number;
      }>('kb_import_multi_folders', {
        folderPaths: paths,
        categoryId: targetCategoryId,
        library: currentLibrary
      });
      if (res.code === 0 && res.data) {
        await loadAll();
        const importedEntries = res.data.entries || [];
        const siblingsBeforeImport = getEntriesInCategory(targetCategoryId).filter(e => !importedEntries.some((ne: KbEntry) => ne.name === e.name));
        let renamedCount = 0;
        for (const newEntry of importedEntries) {
          const conflict = siblingsBeforeImport.find(e => e.name.toLowerCase() === newEntry.name.toLowerCase());
          if (conflict) {
            const renamed = resolveAutoRename(newEntry.name, targetCategoryId);
            try {
              await ipc.invoke('update_kb_entry', {
                request: {
                  id: newEntry.id,
                  name: renamed
                }
              });
              renamedCount++;
            } catch {/* skip */}
          }
        }
        if (renamedCount > 0) {
          await loadAll();
        }
        const {
          categories: newCats,
          entries: newEntries,
          fail_count
        } = res.data;
        let msg = t("Knowledge.k106", {
          length: newCats.length,
          arg0: newEntries.length
        });
        if (renamedCount > 0) msg += t("Knowledge.k107", {
          renamedCount: renamedCount
        });
        if (fail_count > 0) msg += t("Knowledge.k108", {
          fail_count: fail_count
        });
        showStatus(fail_count > 0 ? 'error' : 'success', msg);
      } else showStatus('error', res.message || t("Knowledge.k99"));
    } catch (err) {
      showStatus('error', t("Knowledge.k109", {
        arg0: err instanceof Error ? err.message : String(err)
      }));
    } finally {
      setIsImporting(false);
    }
  };
  const loadTrackedPaths = async () => {
    setTrackedPathsLoading(true);
    try {
      const res = await ipc.invoke<KbTrackedPath[]>('kb_get_tracked_paths', {
        library: currentLibrary
      });
      if (res.code === 0 && res.data) setTrackedPaths(res.data);
    } catch {/* ignore */} finally {
      setTrackedPathsLoading(false);
    }
  };
  const handleRemoveTrackedPath = async (id: number) => {
    try {
      const res = await ipc.invoke('kb_remove_tracked_path', {
        id
      });
      if (res.code === 0) {
        setTrackedPaths(prev => prev.filter(p => p.id !== id));
        showStatus('success', t("Knowledge.k110"));
      } else showStatus('error', res.message || t("Knowledge.k111"));
    } catch {
      showStatus('error', t("Knowledge.k112"));
    }
  };
  const handleReimportTracked = async (tp: KbTrackedPath) => {
    setIsImporting(true);
    try {
      const res = await ipc.invoke<{
        categories: KbCategory[];
        entries: KbEntry[];
        success_count: number;
        fail_count: number;
      }>('kb_import_multi_folders', {
        folderPaths: [tp.path],
        categoryId: tp.category_id,
        library: currentLibrary
      });
      if (res.code === 0 && res.data) {
        await loadAll();
        showStatus('success', t("Knowledge.k113", {
          length: res.data.entries.length
        }));
      } else showStatus('error', res.message || t("Knowledge.k114"));
    } catch (err) {
      showStatus('error', t("Knowledge.k115", {
        arg0: err instanceof Error ? err.message : String(err)
      }));
    } finally {
      setIsImporting(false);
      loadTrackedPaths();
    }
  };
  const toggleTrackedPaths = () => {
    const next = !showTrackedPaths;
    setShowTrackedPaths(next);
    if (next) loadTrackedPaths();
  };
  const handleCheckPaths = async () => {
    setPathChecking(true);
    try {
      const res = await ipc.invoke<any>('kb_check_paths', {});
      if (res.code === 0 && res.data) {
        setPathCheckResult(res.data);
        const invT = res.data.invalid_tracked.length;
        const invE = res.data.invalid_external_entries.length;
        if (invT + invE === 0) {
          showStatus('success', t("Knowledge.k116"));
        } else {
          const parts: string[] = [];
          if (invT > 0) parts.push(t("Knowledge.k117", {
            invT: invT
          }));
          if (invE > 0) parts.push(t("Knowledge.k118", {
            invE: invE
          }));
          showStatus('error', parts.join('，'));
        }
      }
    } catch {
      showStatus('error', t("Knowledge.k119"));
    } finally {
      setPathChecking(false);
    }
  };
  const handleDirectoryScan = async () => {
    try {
      const {
        open
      } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        title: t("Knowledge.k120"),
        directory: true
      });
      if (!selected) return;
      const pathStr = String(selected);
      setDirScanPath(pathStr);
      setDirScanLoading(true);
      setDirScanFilter('all');
      setDirScanSelected(new Set());
      setShowDirScanModal(true);
      const res = await ipc.invoke<ScanDirFile[]>('kb_scan_directory', {
        folderPath: pathStr
      });
      if (res.code === 0 && res.data) {
        setDirScanFiles(res.data);
      } else {
        showStatus('error', res.message || t("Knowledge.k121"));
        setShowDirScanModal(false);
      }
    } catch (err) {
      showStatus('error', t("Knowledge.k122", {
        arg0: err instanceof Error ? err.message : String(err)
      }));
      setShowDirScanModal(false);
    } finally {
      setDirScanLoading(false);
    }
  };
  const filteredDirScanFiles = dirScanFiles.filter(f => {
    if (dirScanFilter === 'all') return true;
    return f.file_type === dirScanFilter;
  });
  const dirScanTypeCounts = {
    all: dirScanFiles.length,
    text: dirScanFiles.filter(f => f.file_type === 'text').length,
    image: dirScanFiles.filter(f => f.file_type === 'image').length,
    video: dirScanFiles.filter(f => f.file_type === 'video').length,
    audio: dirScanFiles.filter(f => f.file_type === 'audio').length,
    document: dirScanFiles.filter(f => f.file_type === 'document').length,
    other: dirScanFiles.filter(f => f.file_type === 'other').length
  };
  const toggleDirScanSelect = (idx: number) => {
    setDirScanSelected(prev => {
      const next = new Set(prev);
      if (next.has(idx)) next.delete(idx);else next.add(idx);
      return next;
    });
  };
  const handleDirScanSelectAll = () => {
    if (dirScanSelected.size === filteredDirScanFiles.length) {
      setDirScanSelected(new Set());
    } else {
      setDirScanSelected(new Set(filteredDirScanFiles.map((_, i) => i)));
    }
  };
  const handleBatchImportSelected = async () => {
    if (dirScanSelected.size === 0) {
      showStatus('error', t("Knowledge.k123"));
      return;
    }
    const targetCategoryId = selectedId?.type === 'category' ? selectedId.id : categories[0]?.id ?? null;
    if (!targetCategoryId) {
      showStatus('error', t("Knowledge.k124"));
      return;
    }
    setDirImporting(true);
    const selectedIndices = Array.from(dirScanSelected);
    const selectedFiles = selectedIndices.map(i => filteredDirScanFiles[i]);
    const filePaths = selectedFiles.map(f => f.path);
    try {
      const res = await ipc.invoke<{
        added: KbEntry[];
        total: number;
      }>('kb_add_scanned_files', {
        filePaths,
        categoryId: targetCategoryId,
        sourcePath: dirScanPath
      });
      if (res.code === 0 && res.data) {
        const imported = res.data.added.length;
        const skipped = res.data.total - imported;
        showStatus('success', t("Knowledge.k125", {
          imported: imported,
          arg0: skipped > 0 ? t("Knowledge.k126", {
            skipped: skipped
          }) : ''
        }));
      } else {
        showStatus('error', res.message || t("Knowledge.k99"));
      }
    } catch (err: any) {
      showStatus('error', t("Knowledge.k127", {
        arg0: err.message || err
      }));
    }
    setShowDirScanModal(false);
    loadAll();
    setDirImporting(false);
    intelligence.logActivity('user', new Date().toISOString(), 'knowledge', 'import_files', `${selectedFiles.length} files from ${dirScanPath}`).catch(() => {});
  };
  const DIR_SCAN_TYPE_LABELS: Record<string, string> = {
    all: t("common.all"),
    text: t("knowledge.GraphView.k2"),
    image: t("knowledge.GraphView.k4"),
    video: t("knowledge.GraphView.k3"),
    audio: t("knowledge.GraphView.k5"),
    document: t("knowledge.GraphView.k6"),
    other: t("game3d.components.UI.BreakthroughQuiz.k20")
  };
  const DIR_SCAN_TYPE_ICONS: Record<string, string> = {
    all: '📋',
    text: '📝',
    image: '🖼️',
    video: '🎬',
    audio: '🎵',
    document: '📑',
    other: '📎'
  };
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'p') {
        e.preventDefault();
        setShowQuickSwitcher(true);
        setQuickSwitcherQuery('');
        setQuickSwitcherIndex(0);
      }
      if (e.key === 'Escape') {
        setShowQuickSwitcher(false);
        setShowDirScanModal(false);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);
  useEffect(() => {
    if (!mediaViewerEntry || pptxSlideCount === 0) return;
    const handleSlideKeys = (e: KeyboardEvent) => {
      if (mediaViewerFullscreen && e.key === 'Escape') return;
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
        e.preventDefault();
        setPptxCurrentSlide(prev => Math.max(0, prev - 1));
      } else if (e.key === 'ArrowRight' || e.key === 'ArrowDown' || e.key === ' ') {
        e.preventDefault();
        setPptxCurrentSlide(prev => Math.min(pptxSlideCount - 1, prev + 1));
      }
    };
    window.addEventListener('keydown', handleSlideKeys);
    return () => window.removeEventListener('keydown', handleSlideKeys);
  }, [mediaViewerEntry, pptxSlideCount, mediaViewerFullscreen]);
  useEffect(() => {
    if (mediaViewerMime === 'pptx' && pptxViewerRef.current) {
      try {
        const v = pptxViewerRef.current as any;
        if (typeof v.goToSlide === 'function') {
          v.goToSlide(pptxCurrentSlide);
        }
      } catch (_) {}
    }
  }, [pptxCurrentSlide, mediaViewerMime, pptxSlideCount, mediaViewerFullscreen]);
  const pptxArrayBufRef = useRef<ArrayBuffer | null>(null);
  useEffect(() => {
    if (mediaViewerMime !== 'pptx' || !pptxArrayBufRef.current) return;
    const buf = pptxArrayBufRef.current;
    async function rebuild() {
      if (pptxViewerRef.current) {
        try {
          pptxViewerRef.current.destroy();
        } catch (_) {}
        pptxViewerRef.current = null;
      }
      const container = mediaViewerFullscreen ? pptxFullscreenSlideRef.current : pptxSlideRef.current;
      if (!container) return;
      container.replaceChildren();
      try {
        const viewer = await PptxViewer.open(buf, container, {
          renderMode: 'slide',
          fitMode: 'contain'
        });
        pptxViewerRef.current = viewer;
        const v = viewer as any;
        setPptxSlideCount(v.slideCount || 1);
        if (typeof v.goToSlide === 'function') {
          v.goToSlide(pptxCurrentSlide);
        }
      } catch (_) {}
    }
    const timer = setTimeout(rebuild, 100);
    return () => clearTimeout(timer);
  }, [mediaViewerFullscreen]);
  const quickSwitcherResults = quickSwitcherQuery.trim() === '' ? allEntries.slice(0, 20) : allEntries.filter(e => e.name.toLowerCase().includes(quickSwitcherQuery.toLowerCase())).slice(0, 20);
  const handleQuickSelect = (entry: KbEntry) => {
    setShowQuickSwitcher(false);
    setQuickSwitcherQuery('');
    handleOpenEntry(entry);
  };
  const handleQuickSwitcherKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setQuickSwitcherIndex(prev => Math.min(prev + 1, quickSwitcherResults.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setQuickSwitcherIndex(prev => Math.max(prev - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const target = quickSwitcherResults[quickSwitcherIndex];
      if (target) handleQuickSelect(target);
    }
  };
  const loadBacklinks = useCallback(async (entryId: number) => {
    if (!entryId) {
      setBacklinks([]);
      return;
    }
    setBacklinksLoading(true);
    try {
      const res = await ipc.invoke<Array<{
        entry: KbEntry;
        snippet: string;
      }>>('kb_get_backlinks', {
        entryId
      });
      if (res.code === 0 && res.data) {
        setBacklinks(res.data);
      } else {
        setBacklinks([]);
      }
    } catch {
      setBacklinks([]);
    }
    setBacklinksLoading(false);
  }, []);
  useEffect(() => {
    if (selectedEntry) {
      loadBacklinks(selectedEntry.id);
    } else {
      setBacklinks([]);
    }
  }, [selectedEntry, loadBacklinks]);
  const loadSnapshots = useCallback(async (entryId: number) => {
    if (!entryId) {
      setSnapshots([]);
      return;
    }
    setSnapshotsLoading(true);
    try {
      const res = await ipc.invoke<Array<{
        id: number;
        entry_id: number;
        entry_name: string;
        content: string;
        content_length: number;
        created_at: number;
      }>>('kb_get_snapshots', {
        entryId
      });
      if (res.code === 0 && res.data) {
        setSnapshots(res.data);
      } else {
        setSnapshots([]);
      }
    } catch {
      setSnapshots([]);
    }
    setSnapshotsLoading(false);
  }, []);
  const handleRestoreSnapshot = async (snapshotId: number) => {
    if (!confirm(t("Knowledge.k128"))) return;
    try {
      const res = await ipc.invoke<KbEntry>('kb_restore_snapshot', {
        snapshotId
      });
      if (res.code === 0 && res.data) {
        setAllEntries(prev => prev.map(e => e.id === res.data!.id ? res.data! : e));
        setShowSnapshots(false);
        setSnapshotPreview(null);
        await loadSnapshots(res.data.id);
        showStatus('success', t("Knowledge.k129"));
      } else {
        showStatus('error', res.message || t("Knowledge.k130"));
      }
    } catch (e: any) {
      showStatus('error', t("Knowledge.k131", {
        e: e
      }));
    }
  };
  const handleAiClassify = async () => {
    setAiClassifyLoading(true);
    setAiClassifyResults([]);
    try {
      const fileNames = filteredDirScanFiles.map(f => f.name).join(' ');
      const existingCats = categories.map(c => c.name);
      const res = await intelligence.classifyKbEntry(fileNames.slice(0, 200), '', existingCats);
      if (res?.data) {
        setAiClassifyResults(res.data.slice(0, 5));
      } else {
        showStatus('error', res?.message || t("Knowledge.k132"));
      }
    } catch (e: any) {
      showStatus('error', t("Knowledge.k133", {
        e: e
      }));
    }
    setAiClassifyLoading(false);
  };

  // Task 5.5: AI 智能分类推荐（右键菜单）
  const handleAiClassifyEntry = async (entryId: number, entryName: string) => {
    setAiClassifyLoading(true);
    try {
      const entry = allEntries.find(e => e.id === entryId);
      const content = entry?.content || entryName;
      const res = await intelligence.kbClassify(entryId, content);
      if (res?.data) {
        const {
          addOrb
        } = useFloatingOrbStore.getState();
        addOrb({
          type: 'summary',
          title: t("Knowledge.k134", {
            entryName: entryName
          }),
          content: t("Knowledge.k135", {
            category_name: res.data.category_name,
            arg0: (res.data.confidence * 100).toFixed(0),
            reason: res.data.reason
          }),
          source: 'knowledge',
          sourceId: entryId,
          icon: '🧠',
          color: '#00FF00'
        });
        showStatus('success', t("Knowledge.k136", {
          category_name: res.data.category_name,
          arg0: (res.data.confidence * 100).toFixed(0)
        }));
      } else {
        showStatus('error', res?.message || t("Knowledge.k137"));
      }
    } catch (e: any) {
      showStatus('error', t("Knowledge.k138", {
        e: e
      }));
    }
    setAiClassifyLoading(false);
  };
  const handleAiSummarizeEntry = async () => {
    if (!selectedEntry) return;
    if (!llmConfigured) {
      showStatus('error', t("Knowledge.k139"));
      return;
    }
    setAiSummaryLoading(true);
    try {
      const res = await intelligence.kbSummarize(selectedEntry.name, selectedEntry.content || '', {
        provider: llmConfig.provider,
        endpoint: llmConfig.endpoint,
        api_key: llmConfig.apiKey || undefined,
        model: llmConfig.model || undefined
      }, String(selectedEntry.id));
      if (res?.data) {
        // dispatch 到全局悬浮球
        const {
          addOrb
        } = useFloatingOrbStore.getState();
        addOrb({
          type: 'summary',
          title: t("Knowledge.k140", {
            name: selectedEntry.name
          }),
          content: res.data.summary,
          keyPoints: res.data.key_points,
          source: 'knowledge',
          sourceId: selectedEntry.id,
          icon: '🧠',
          color: '#00F0FF'
        });
        showStatus('success', t("Knowledge.k141", {
          original_length: res.data.original_length,
          summary_length: res.data.summary_length
        }));
      } else {
        showStatus('error', res?.message || t("Knowledge.k142"));
      }
    } catch (e: any) {
      showStatus('error', t("Knowledge.k143", {
        e: e
      }));
    }
    setAiSummaryLoading(false);
  };
  const handleAiGenerateTags = async () => {
    if (!selectedEntry) return;
    if (!llmConfigured) {
      showStatus('error', t("Knowledge.k139"));
      return;
    }
    setAiTagsLoading(true);
    setAiTags([]);
    try {
      const existing = (entryTags.get(selectedEntry.id) || []).map(t => t.name);
      const res = await intelligence.kbTags(selectedEntry.content || '', existing, {
        provider: llmConfig.provider,
        endpoint: llmConfig.endpoint,
        api_key: llmConfig.apiKey || undefined,
        model: llmConfig.model || undefined
      });
      if (res?.data) {
        setAiTags(res.data.suggested_tags);
        showStatus('success', t("Knowledge.k144", {
          length: res.data.suggested_tags.length
        }));
      } else {
        showStatus('error', res?.message || t("Knowledge.k145"));
      }
    } catch (e: any) {
      showStatus('error', t("Knowledge.k146", {
        e: e
      }));
    }
    setAiTagsLoading(false);
  };
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
  const fetchCategoryRecommendation = useCallback(async (entryId: number, entryName: string, entryContent: string) => {
    setAiCategoryLoading(true);
    try {
      const res = await ipc.invoke<{
        suggestion: string;
      }>('kb_ai_recommend_category', {
        entry_id: entryId,
        entry_name: entryName,
        entry_content: entryContent
      });
      if (res.code === 0 && res.data) {
        setAiCategorySuggestion(res.data.suggestion);
      } else {
        setAiCategorySuggestion('');
      }
    } catch {
      setAiCategorySuggestion('');
    } finally {
      setAiCategoryLoading(false);
    }
  }, []);

  // === Passive AI: auto-recommend category when selecting an entry (debounced 1s) ===
  useEffect(() => {
    if (!aiOn || !featureOn('auto_classify') || !selectedId || selectedId.type !== 'entry') {
      setAiCategorySuggestion('');
      return;
    }
    const entry = allEntries.find(e => e.id === selectedId.id);
    if (!entry) {
      setAiCategorySuggestion('');
      return;
    }
    if (aiCategoryTimerRef.current) clearTimeout(aiCategoryTimerRef.current);
    setAiCategoryLoading(true);
    setAiCategorySuggestion('');
    aiCategoryTimerRef.current = setTimeout(() => {
      fetchCategoryRecommendation(entry.id, entry.name, entry.content || '');
    }, 1000);
    return () => {
      if (aiCategoryTimerRef.current) clearTimeout(aiCategoryTimerRef.current);
    };
  }, [selectedId, aiOn, featureOn, fetchCategoryRecommendation]);
  const getTypeIcon = (type?: string) => {
    switch (type) {
      case 'link':
        return '🔗';
      case 'file':
        return '📄';
      case 'text':
        return '📝';
      case 'video':
        return '🎬';
      case 'image':
        return '🖼️';
      case 'audio':
        return '🎵';
      case 'document':
        return '📑';
      default:
        return '📄';
    }
  };
  const getTypeLabel = (type?: string) => {
    switch (type) {
      case 'link':
        return t("components.TextEditor.k9");
      case 'file':
        return t("knowledge.GraphView.k1");
      case 'text':
        return t("knowledge.GraphView.k2");
      case 'video':
        return t("knowledge.GraphView.k3");
      case 'image':
        return t("knowledge.GraphView.k4");
      case 'audio':
        return t("knowledge.GraphView.k5");
      case 'document':
        return t("knowledge.GraphView.k6");
      default:
        return t("components.GroupChatOrchestrationPanel.k35");
    }
  };
  const cycleSort = () => {
    const modes: SortMode[] = ['time_desc', 'time_asc', 'name_asc', 'name_desc'];
    const idx = modes.indexOf(sortMode);
    const next = modes[(idx + 1) % modes.length];
    setSortMode(next);
    localStorage.setItem('kb_sort_mode', next);
  };
  const tree = buildTree();
  const renderNode = (node: TreeNode) => {
    const isExpanded = node.type === 'category' && expandedIds.has(node.id);
    const isSelected = selectedId?.id === node.id && selectedId?.type === node.type;
    const hasChildren = node.children.length > 0;
    const isEditing = editingId?.id === node.id && editingId?.type === node.type;
    const entryCount = node.type === 'category' ? categoryCounts.get(node.id) || 0 : 0;
    const isEntryMissing = node.type === 'entry' && missingFiles.has(node.id);
    return <div key={`${node.type}-${node.id}`}>
        <div className={`${styles.treeItem} ${isSelected ? styles.treeItemSelected : ''} ${dragOverId?.type === node.type && dragOverId?.id === node.id ? styles.treeItemDragOver : ''}`} style={{
        paddingLeft: `${8 + node.depth * 16}px`
      }} draggable={!isEditing} onClick={() => handleSelect(node.type, node.id)} onDoubleClick={() => {
        if (node.type === 'entry' && selectedEntry) handleOpenEntry(selectedEntry);else if (node.type === 'category' || node.type === 'entry') startRename(node.type, node.id, node.name);
      }} onContextMenu={e => handleRightClick(e, node)} onDragStart={e => {
        e.dataTransfer.setData('text/plain', JSON.stringify({
          type: node.type,
          id: node.id,
          name: node.name
        }));
        e.dataTransfer.effectAllowed = 'move';
        setIsDragging(true);
        // 加载另一库的分类，以便跨库拖放
        const otherLib = currentLibrary === 'material' ? 'study' : 'material';
        ipc.invoke<KbCategory[]>('get_kb_categories', {
          library: otherLib
        }).then(res => {
          if (res.code === 0 && res.data) {
            setDragOtherCategories(res.data);
          }
        }).catch(() => {});
      }} onDragEnd={() => {
        setIsDragging(false);
        setDragOtherCategories([]);
        setDragOverId(null);
        setDragOverLibraryRoot(null);
      }} onDragOver={e => {
        e.preventDefault();
        e.dataTransfer.dropEffect = 'move';
        setDragOverId({
          type: node.type,
          id: node.id
        });
      }} onDragLeave={() => setDragOverId(null)} onDrop={e => {
        e.preventDefault();
        setDragOverId(null);
        const data = JSON.parse(e.dataTransfer.getData('text/plain'));
        if (data.type === 'entry') {
          const targetCatId = node.type === 'category' ? node.id : allEntries.find(en => en.id === node.id)?.category_id ?? node.categoryId ?? 0;
          if (data.id !== node.id) {
            handleMoveEntry(data.id, targetCatId);
          }
        } else if (data.type === 'category' && node.type === 'category') {
          if (data.id !== node.id && !getAllDescendantIds(categories, data.id).includes(node.id)) {
            handleMoveCategory(data.id, node.id);
          }
        }
      }}>
          {isMultiMode && node.type === 'entry' && <input type="checkbox" className={styles.treeCheckbox} checked={selectedEntryIds.has(node.id)} onChange={() => toggleEntrySelection(node.id)} onClick={e => e.stopPropagation()} />}
          <span className={styles.treeArrow} onClick={e => {
          e.stopPropagation();
          if (hasChildren) toggleExpand(node.id);
        }}>
            {node.type === 'category' ? hasChildren ? isExpanded ? '▼' : '▶' : '▸' : ''}
          </span>
          <span className={styles.treeIcon}>
            {node.type === 'category' ? isExpanded ? '📂' : '📁' : getTypeIcon(node.entryType)}
          </span>
          {isEditing ? <input className={styles.inlineEdit} value={editName} onChange={e => setEditName(e.target.value)} onBlur={submitRename} onKeyDown={e => {
          if (e.key === 'Enter') submitRename();
          if (e.key === 'Escape') setEditingId(null);
        }} autoFocus onClick={e => e.stopPropagation()} /> : <span className={styles.treeLabel} style={isEntryMissing ? {
          color: '#FF4444',
          fontWeight: 'bold',
          textDecoration: 'underline'
        } : undefined}>
              {node.name}
              {entryCount > 0 && <span className={styles.entryCount}>{entryCount}</span>}
            </span>}
          {!isEditing && <button className={styles.treeDelBtn} onClick={e => {
          e.stopPropagation();
          node.type === 'category' ? handleDeleteCategory(node.id) : handleDeleteEntry(node.id);
        }}>✕</button>}
        </div>
        {isExpanded && node.children.map(child => renderNode(child))}
      </div>;
  };
  const sortLabels: Record<SortMode, string> = {
    time_desc: t("Knowledge.k147"),
    time_asc: t("Knowledge.k148"),
    name_asc: t("Knowledge.k149"),
    name_desc: t("Knowledge.k150")
  };
  return <div className={styles.page}>
      {statusMsg && <div className={`${styles.toast} ${statusMsg.type === 'error' ? styles.toastErr : styles.toastOk}`}>
          {statusMsg.text}
        </div>}

      {confirmDelete && <DeleteConfirmModal confirmDelete={confirmDelete} allEntries={allEntries} isExternalEntry={isExternalEntry} onCancel={() => setConfirmDelete(null)} onConfirmCategory={confirmDeleteCategory} onConfirmEntry={confirmDeleteEntry} onConfirmBatch={confirmBatchDelete} />}

      {nameConflict && <NameConflictModal conflict={nameConflict} resolveAutoRename={resolveAutoRename} onCancel={() => setNameConflict(null)} />}

      {(showTemplateModal || showTemplateManager) && <TemplateModals showTemplateModal={showTemplateModal} showTemplateManager={showTemplateManager} editingTemplate={editingTemplate} newTemplate={newTemplate} templates={templates} onClosePicker={() => setShowTemplateModal(false)} onOpenManager={() => {
      setShowTemplateModal(false);
      setShowTemplateManager(true);
    }} onCloseManager={() => {
      setShowTemplateManager(false);
      setEditingTemplate(null);
      setNewTemplate({
        name: '',
        icon: '📄',
        description: '',
        entry_type: 'text',
        content: ''
      });
    }} onNewTemplateChange={updates => setNewTemplate(prev => ({
      ...prev,
      ...updates
    }))} onEditingTemplateChange={setEditingTemplate} onEditingTemplateField={updates => setEditingTemplate(prev => prev ? {
      ...prev,
      ...updates
    } : null)} onInsertTemplate={handleInsertTemplate} onCreateTemplate={handleCreateTemplate} onUpdateTemplate={handleUpdateTemplate} onDeleteTemplate={handleDeleteTemplate} />}

      {showDbModal && <DatabaseModal newDbName={newDbName} onNameChange={setNewDbName} onCreate={handleCreateDatabase} onCancel={() => setShowDbModal(false)} />}

      {showGraph && <GraphView allEntries={allEntries} categories={categories} entryTags={entryTags} selectedId={selectedId} onClose={() => setShowGraph(false)} onOpenFileViewer={handleOpenFileViewer} onSelectedIdChange={setSelectedId} onShowStatus={showStatus} />}

      {contextMenu && <ContextMenu menu={contextMenu} onClose={() => setContextMenu(null)} onRename={startRename} onMove={(type, id) => setMoveTarget({
      type,
      id
    })} onDelete={(type, id) => type === 'category' ? handleDeleteCategory(id) : handleDeleteEntry(id)} onAiClassify={aiOn && featureOn('auto_classify') ? handleAiClassifyEntry : undefined} />}

      {moveTarget !== null && <MoveTargetModal target={moveTarget} categories={categories} getCategoryDepth={getCategoryDepth} getAllDescendantIds={getAllDescendantIds} onMoveEntry={handleMoveEntry} onMoveCategory={handleMoveCategory} onCancel={() => setMoveTarget(null)} />}

      {/* ===== 中间：目录树栏 ===== */}
      <aside className={styles.sidebar}>
        <header className={styles.sidebarToolbar}>
          <button className={styles.toolIconBtn} onClick={() => setShowNewEntry(!showNewEntry)} title={t("Knowledge.k151")}>
            ✏️
          </button>
          <button className={styles.toolIconBtn} onClick={() => setShowNewFolder(!showNewFolder)} title={t("Knowledge.k152")}>
            📁
          </button>
          <button className={styles.toolIconBtn} onClick={cycleSort} title={t("Knowledge.k153", {
          sortMode: sortLabels[sortMode]
        })}>
            ↕️
          </button>
          <button className={styles.toolIconBtn} onClick={handleImportFile} title={t("components.intelligence.ActivityPanel.k16")}>
            📄
          </button>
          <button className={styles.toolIconBtn} onClick={handleDirectoryScan} title={t("Knowledge.k154")}>
            🔍
          </button>
          <button className={styles.toolIconBtnPrimary} onClick={handleImportFolder} disabled={isImporting} title={t("Knowledge.k155")}>
            📥
          </button>
          <button className={styles.toolIconBtn} onClick={toggleTrackedPaths} title={t("Knowledge.k156")} style={showTrackedPaths ? {
          color: '#FFC107',
          borderColor: '#FFC107'
        } : undefined}>
            📌 {trackedPaths.length > 0 && <span style={{
            fontSize: 9,
            background: '#FFC107',
            color: '#000',
            borderRadius: '50%',
            padding: '0 3px',
            marginLeft: 2
          }}>{trackedPaths.length}</span>}
          </button>
          {aiOn && featureOn('auto_classify') ? aiCategoryLoading ? <span className={styles.aiPulsingBadge}>{t("Knowledge.k157")}</span> : null : aiOn && !llmConfigured ? <span className={styles.aiPulsingBadge} style={{
          opacity: 0.5,
          background: '#1A1A1F'
        }}>{t("Knowledge.k158")}</span> : null}
        </header>

        {showTrackedPaths && <div className={styles.aiCategoryPanel}>
            <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          marginBottom: 6,
          alignItems: 'center'
        }}>
              <span style={{
            fontSize: 11,
            color: '#FFC107',
            fontFamily: 'var(--nt-font-mono)'
          }}>{t("Knowledge.k159")}</span>
              <div style={{
            display: 'flex',
            gap: 6,
            alignItems: 'center'
          }}>
                <button onClick={handleCheckPaths} disabled={pathChecking} style={{
              background: 'none',
              border: '1px solid rgba(0,240,255,0.3)',
              color: '#00F0FF',
              cursor: pathChecking ? 'default' : 'pointer',
              fontSize: 10,
              padding: '1px 6px',
              borderRadius: 3
            }} title={t("Knowledge.k160")}>{pathChecking ? '⏳' : t("Knowledge.k161")}</button>
                <button onClick={() => {
              setShowTrackedPaths(false);
              setPathCheckResult(null);
            }} style={{
              background: 'none',
              border: 'none',
              color: '#FF5050',
              cursor: 'pointer',
              fontSize: 11
            }}>✕</button>
              </div>
            </div>
            {pathCheckResult && (pathCheckResult.invalid_tracked.length > 0 || pathCheckResult.invalid_external_entries.length > 0) && <div style={{
          fontSize: 10,
          color: '#FF5050',
          marginBottom: 6,
          padding: '2px 6px',
          background: 'rgba(255,80,80,0.1)',
          borderRadius: 3
        }}>
                ⚠ {pathCheckResult.invalid_tracked.length > 0 && t("Knowledge.k162", {
            length: pathCheckResult.invalid_tracked.length
          })}
                {pathCheckResult.invalid_external_entries.length > 0 && t("Knowledge.k163", {
            length: pathCheckResult.invalid_external_entries.length
          })}
              </div>}
            {trackedPathsLoading ? <div style={{
          fontSize: 11,
          color: '#888'
        }}>{t("common.loading")}</div> : trackedPaths.length === 0 ? <div style={{
          fontSize: 11,
          color: '#888'
        }}>{t("Knowledge.k164")}</div> : <div style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 4,
          maxHeight: 200,
          overflowY: 'auto'
        }}>
                {trackedPaths.map(tp => {
            const isInvalid = pathCheckResult?.invalid_tracked.includes(tp.id);
            return <div key={tp.id} style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              padding: '4px 6px',
              background: isInvalid ? 'rgba(255,80,80,0.15)' : 'rgba(255,193,7,0.08)',
              borderRadius: 4,
              fontSize: 11,
              border: isInvalid ? '1px solid rgba(255,80,80,0.3)' : 'none'
            }}>
                    <div style={{
                flex: 1,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                color: isInvalid ? '#FF6B6B' : 'rgba(200,200,220,0.85)'
              }}>
                      {isInvalid && <span style={{
                  marginRight: 4
                }}>⚠️</span>}
                      <span style={{
                  color: isInvalid ? '#FF6B6B' : 'rgba(255,255,255,0.4)',
                  marginRight: 6
                }}>
                        {time.formatCompact(tp.last_imported_at)}
                      </span>
                      {tp.path}
                    </div>
                    <div style={{
                display: 'flex',
                gap: 4,
                flexShrink: 0,
                marginLeft: 8
              }}>
                      {isInvalid && <span style={{
                  fontSize: 9,
                  color: '#FF5050'
                }}>{t("Knowledge.k165")}</span>}
                      <button onClick={() => handleReimportTracked(tp)} disabled={isImporting} style={{
                  background: 'none',
                  border: isInvalid ? '1px solid rgba(255,80,80,0.4)' : '1px solid rgba(0,240,255,0.3)',
                  color: isInvalid ? '#FF6B6B' : '#00F0FF',
                  cursor: isImporting ? 'default' : 'pointer',
                  fontSize: 10,
                  padding: '1px 6px',
                  borderRadius: 3
                }} title={t("Knowledge.k166")}>🔄</button>
                      <button onClick={() => handleRemoveTrackedPath(tp.id)} style={{
                  background: 'none',
                  border: '1px solid rgba(255,80,80,0.3)',
                  color: '#FF5050',
                  cursor: 'pointer',
                  fontSize: 10,
                  padding: '1px 6px',
                  borderRadius: 3
                }} title={t("Knowledge.k167")}>✕</button>
                    </div>
                  </div>;
          })}
              </div>}
          </div>}

        {aiCategorySuggestion && <div className={styles.aiCategoryPanel}>
            <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          marginBottom: 6
        }}>
              <span style={{
            fontSize: 11,
            color: '#00F0FF',
            fontFamily: 'var(--nt-font-mono)'
          }}>{t("Knowledge.k168")}</span>
              <button onClick={() => setAiCategorySuggestion('')} style={{
            background: 'none',
            border: 'none',
            color: '#FF5050',
            cursor: 'pointer',
            fontSize: 11
          }}>✕</button>
            </div>
            <pre style={{
          fontSize: 11,
          color: 'rgba(200,200,220,0.85)',
          lineHeight: 1.6,
          whiteSpace: 'pre-wrap',
          margin: 0
        }}>{aiCategorySuggestion}</pre>
          </div>}

        {showNewFolder && <div className={styles.quickForm}>
            <input value={newFolderName} onChange={e => setNewFolderName(e.target.value)} placeholder={t("Knowledge.k169")} className={styles.inputSmall} onKeyDown={e => e.key === 'Enter' && handleAddCategory()} autoFocus />
            <select value={newFolderParentId ?? ''} onChange={e => setNewFolderParentId(e.target.value ? Number(e.target.value) : null)} className={styles.inputSmall}>
              <option value="">{t("Knowledge.k170")}</option>
              {categories.map(c => <option key={c.id} value={c.id}>{'　'.repeat(getCategoryDepth(c.id))}{c.name}</option>)}
            </select>
            <button onClick={handleAddCategory} className={styles.btnSm}>{t("common.confirm")}</button>
          </div>}

        <div className={styles.viewTabs}>
          <button className={`${styles.viewTab} ${viewMode === 'all' ? styles.viewTabActive : ''}`} onClick={() => handleViewModeChange('all')}>
            {t("Knowledge.k171")}
          </button>
          <button className={`${styles.viewTab} ${viewMode === 'favorite' ? styles.viewTabActive : ''}`} onClick={() => handleViewModeChange('favorite')}>
            {t("Knowledge.k172")}
          </button>
          <button className={`${styles.viewTab} ${viewMode === 'recent' ? styles.viewTabActive : ''}`} onClick={() => handleViewModeChange('recent')}>
            {t("Knowledge.k173")}
          </button>
          <button className={`${styles.viewTabMini} ${isMultiMode ? styles.viewTabMiniActive : ''}`} onClick={toggleMultiMode} title={t("Knowledge.k174")}>
            ☑
          </button>
        </div>

       <TagBar tags={tags} tagStats={tagStats} activeTagId={viewMode === 'tag' ? viewFilterId : null} editingTagId={editingTagId} editingTagName={editingTagName} editingTagColor={editingTagColor} onTagFilter={handleTagFilter} onStartEdit={startEditTag} onDeleteTag={handleDeleteGlobalTag} onUpdateTag={handleUpdateGlobalTag} onCancelEdit={() => setEditingTagId(null)} onEditingNameChange={setEditingTagName} onEditingColorChange={setEditingTagColor} onAddTag={handleAddGlobalTag} />

        <nav className={styles.fileTree}>
          {isLoading ? <div style={{
          padding: '12px'
        }}>
              <SkeletonList count={8} />
            </div> : viewMode === 'all' ? <>
              {/* 拖放时显示跨库移动目标区域 */}
              {isDragging && dragOtherCategories.length > 0 && <div className={styles.crossLibSection}>
                  <div className={styles.crossLibHeader}>
                    {currentLibrary === 'material' ? t("Knowledge.k175") : t("Knowledge.k176")}
                  </div>
                  {/* 移动到另一库根目录 */}
                  <div className={`${styles.crossLibRoot} ${dragOverLibraryRoot === 'other_root' ? styles.crossLibRootActive : ''}`} onDragOver={e => {
              e.preventDefault();
              e.dataTransfer.dropEffect = 'move';
              setDragOverLibraryRoot('other_root');
            }} onDragLeave={() => setDragOverLibraryRoot(null)} onDrop={e => {
              e.preventDefault();
              setDragOverLibraryRoot(null);
              const data = JSON.parse(e.dataTransfer.getData('text/plain'));
              const otherLib = currentLibrary === 'material' ? 'study' : 'material';
              if (data.type === 'entry') {
                // 移动到另一库的根目录（无父分类）
                const rootCat = dragOtherCategories.find(c => !c.parent_id);
                if (rootCat) {
                  handleMoveEntry(data.id, rootCat.id);
                } else {
                  showStatus('error', t("Knowledge.k177"));
                }
              } else if (data.type === 'category') {
                handleMoveCategory(data.id, null, otherLib);
              }
            }}>
                    {t("Knowledge.k178")}
                  </div>
                  {/* 另一库的根分类 */}
                  {dragOtherCategories.filter(c => !c.parent_id).map(otherCat => <div key={`other-${otherCat.id}`} className={`${styles.treeItem} ${dragOverId?.type === 'category' && dragOverId?.id === otherCat.id ? styles.treeItemDragOver : ''}`} style={{
              paddingLeft: '24px'
            }} onDragOver={e => {
              e.preventDefault();
              e.dataTransfer.dropEffect = 'move';
              setDragOverId({
                type: 'category',
                id: otherCat.id
              });
            }} onDragLeave={() => setDragOverId(null)} onDrop={e => {
              e.preventDefault();
              setDragOverId(null);
              const data = JSON.parse(e.dataTransfer.getData('text/plain'));
              const otherLib = currentLibrary === 'material' ? 'study' : 'material';
              if (data.type === 'entry') {
                handleMoveEntry(data.id, otherCat.id);
              } else if (data.type === 'category') {
                handleMoveCategory(data.id, otherCat.id, otherLib);
              }
            }}>
                        <span className={styles.treeIcon}>📁</span>
                        <span className={styles.treeLabel}>{otherCat.name}</span>
                      </div>)}
                </div>}
              {tree.map(n => renderNode(n))}
              {tree.length === 0 && <div className={styles.emptyTipSmall}>{t("knowledge.ViewModeList.k1")}</div>}
            </> : <ViewModeList mode={viewMode} filterId={viewFilterId} sortMode={sortMode} typeFilter={typeFilter} renderNode={renderNode} />}
        </nav>

        {isMultiMode && selectedEntryIds.size > 0 && <div className={styles.batchBar}>
            <span className={styles.batchInfo}>{t("ai.ChatPanel.k18")} {selectedEntryIds.size} {t("Knowledge.k179")}</span>
            <button className={styles.btnSm} onClick={() => setShowBatchMove(true)}>{t("Knowledge.k180")}</button>
            <button className={styles.btnSm} onClick={() => {
          setBatchTagAction('add');
          setShowBatchTagPicker(true);
        }}>{t("Knowledge.k181")}</button>
            <button className={styles.btnSm} onClick={() => {
          setBatchTagAction('remove');
          setShowBatchTagPicker(true);
        }}>{t("Knowledge.k182")}</button>
            <button className={styles.btnDel} onClick={handleBatchDelete}>{t("Knowledge.k183")}</button>
            <button className={styles.btnSm} onClick={() => setSelectedEntryIds(new Set())}>{t("common.deselect")}</button>
          </div>}
      </aside>

      {showBatchMove && <div className={styles.modalOverlay} onClick={() => setShowBatchMove(false)}>
          <div className={styles.modal} onClick={e => e.stopPropagation()}>
            <h3 className={styles.modalTitle}>{t("Knowledge.k184")}</h3>
            <div className={styles.moveList}>
              {categories.map(cat => <button key={cat.id} className={styles.moveItem} onClick={() => handleBatchMove(cat.id)}>
                  {'　'.repeat(getCategoryDepth(cat.id))}📁 {cat.name}
                </button>)}
            </div>
            <div className={styles.modalActions}>
              <button className={styles.btnSm} onClick={() => setShowBatchMove(false)}>{t("common.cancel")}</button>
            </div>
          </div>
        </div>}

      {showBatchTagPicker && <div className={styles.modalOverlay} onClick={() => setShowBatchTagPicker(false)}>
          <div className={styles.modal} onClick={e => e.stopPropagation()} style={{
        maxWidth: 320
      }}>
            <h3 className={styles.modalTitle}>{batchTagAction === 'add' ? t("Knowledge.k185") : t("Knowledge.k186")}（{selectedEntryIds.size} {t("Knowledge.k187")}</h3>
            <div className={styles.moveList}>
              {tags.length === 0 ? <div style={{
            padding: 12,
            color: '#666',
            fontSize: 12
          }}>{t("Knowledge.k188")}</div> : tags.map(t => <button key={t.id} className={styles.moveItem} style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6
          }} onClick={() => batchTagAction === 'add' ? handleBatchAddTag(t.id) : handleBatchRemoveTag(t.id)}>
                    <span className={styles.tagColorDot} style={{
              background: t.color
            }} />
                    {t.name}
                  </button>)}
            </div>
            <div className={styles.modalActions}>
              <button className={styles.btnSm} onClick={() => setShowBatchTagPicker(false)}>{t("common.cancel")}</button>
            </div>
          </div>
        </div>}

      {/* ===== 右侧：主内容区 ===== */}
      <main className={styles.main}>
        <header className={styles.toolbar}>
          <div className={styles.toolbarLeft}>
            {!isSearching && <>
                <input value={searchQuery} onChange={e => setSearchQuery(e.target.value)} ref={searchRef} placeholder={t("Knowledge.k189")} className={styles.searchInput} />
                <button onClick={handleSearch} className={styles.btnSm}>{t("common.search")}</button>
                <label className={styles.semanticToggle} title={t("Knowledge.k190")}>
                  <span className={styles.semanticToggleLabel} style={{
                color: semanticSearch ? '#0F0' : '#555'
              }}>{t("Knowledge.k191")}</span>
                  <div className={`${styles.semanticSwitch} ${semanticSearch ? styles.semanticSwitchOn : ''}`} onClick={() => setSemanticSearch(!semanticSearch)}>
                    <div className={styles.semanticSwitchKnob} />
                  </div>
                </label>
                <select value={typeFilter} onChange={e => setTypeFilter(e.target.value)} className={styles.typeFilterSelect}>
                  <option value="all">{t("Knowledge.k192")}</option>
                  <option value="text">{t("Knowledge.k193")}</option>
                  <option value="file">{t("Knowledge.k194")}</option>
                  <option value="document">{t("Knowledge.k195")}</option>
                  <option value="image">{t("Knowledge.k196")}</option>
                  <option value="video">{t("Knowledge.k197")}</option>
                  <option value="audio">{t("Knowledge.k198")}</option>
                  <option value="link">{t("Knowledge.k199")}</option>
                </select>
              </>}
            {isSearching && <>
                <button onClick={handleClearSearch} className={styles.btnWarn}>{t("Knowledge.k200")}</button>
                <select value={typeFilter} onChange={e => setTypeFilter(e.target.value)} className={styles.typeFilterSelect}>
                  <option value="all">{t("Knowledge.k192")}</option>
                  <option value="text">{t("Knowledge.k193")}</option>
                  <option value="file">{t("Knowledge.k194")}</option>
                  <option value="document">{t("Knowledge.k195")}</option>
                  <option value="image">{t("Knowledge.k196")}</option>
                  <option value="video">{t("Knowledge.k197")}</option>
                  <option value="audio">{t("Knowledge.k198")}</option>
                  <option value="link">{t("Knowledge.k199")}</option>
                </select>
              </>}
          </div>
          <div className={styles.toolbarRight}>
            {isSearching && <span style={{
            color: '#FFD700',
            fontSize: 12
          }}>
                「{searchQuery}」({semanticSearch ? semanticSearchResults.length : searchResults.filter(e => typeFilter === 'all' || e.entry_type === typeFilter).length})
              </span>}
            {!isSearching && !selectedEntry && <div className={styles.viewToggle}>
                <button className={`${styles.viewToggleBtn} ${displayMode === 'list' ? styles.viewToggleBtnActive : ''}`} onClick={() => setDisplayMode('list')} title={t("Knowledge.k201")}>☰</button>
                <button className={`${styles.viewToggleBtn} ${displayMode === 'card' ? styles.viewToggleBtnActive : ''}`} onClick={() => setDisplayMode('card')} title={t("Knowledge.k202")}>▦</button>
                <button className={`${styles.viewToggleBtn} ${displayMode === 'timeline' ? styles.viewToggleBtnActive : ''}`} onClick={() => setDisplayMode('timeline')} title={t("Knowledge.k203")}>⏱</button>
              </div>}
          </div>
        </header>

        {showNewEntry && <section className={styles.formPanel}>
            {pendingTemplate && <div className={styles.formRow} style={{
          marginBottom: 6
        }}>
                <span style={{
            fontSize: 11,
            color: '#FFD700'
          }}>{t("Knowledge.k204")}{pendingTemplate.icon} {pendingTemplate.name}</span>
              </div>}
            <div className={styles.formRow}>
              <input value={newEntry.name} onChange={e => setNewEntry(p => ({
            ...p,
            name: e.target.value
          }))} placeholder={t("Knowledge.k205")} className={styles.input} autoFocus />
              <select value={newEntry.entry_type} onChange={e => setNewEntry(p => ({
            ...p,
            entry_type: e.target.value
          }))} className={styles.input}>
                <option value="text">{t("Knowledge.k206")}</option>
                <option value="link">{t("components.TextEditor.k9")}</option>
                <option value="file">{t("Knowledge.k207")}</option>
              </select>
            </div>
            <div className={styles.formRow}>
              <input value={newEntry.path_url} onChange={e => setNewEntry(p => ({
            ...p,
            path_url: e.target.value
          }))} placeholder={newEntry.entry_type === 'link' ? t("Knowledge.k208") : t("Knowledge.k209")} className={styles.input} onKeyDown={e => e.key === 'Enter' && handleAddEntry()} />
              <button onClick={handleAddEntry} className={styles.btnPrimary}>{pendingTemplate ? t("Knowledge.k210") : t("Knowledge.k211")}</button>
            </div>
          </section>}

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
        {selectedId?.type === 'entry' && selectedEntry && aiOn && <div className={styles.aiAgentBar}>
            <button className={styles.aiAgentBtn} onClick={handleAiSummarizeEntry} disabled={aiSummaryLoading || !llmConfigured} title={llmConfigured ? t("Knowledge.k214") : t("Knowledge.k215")}>
              {aiSummaryLoading ? '⏳' : '🧠'} {t("components.intelligence.SuggestionsPanel.k5")}
            </button>
            <button className={styles.aiAgentBtn} onClick={handleAiGenerateTags} disabled={aiTagsLoading || !llmConfigured} title={llmConfigured ? t("Knowledge.k216") : t("Knowledge.k215")}>
              {aiTagsLoading ? '⏳' : '🏷️'} {t("Knowledge.k217")}
            </button>
          </div>}

        {/* AI 摘要已通过悬浮球展示 */}
        {aiSummaryLoading && <div className={styles.aiAgentResult} style={{
        opacity: 0.6,
        pointerEvents: 'none'
      }}>
            <div className={styles.aiAgentResultHeader}>
              <span>{t("Knowledge.k218")}</span>
            </div>
            <p className={styles.aiAgentSummary} style={{
          color: '#6B7280'
        }}>{t("Knowledge.k219")}</p>
          </div>}

        {/* AI 标签建议 */}
        {aiTags.length > 0 && <div className={styles.aiAgentResult}>
            <div className={styles.aiAgentResultHeader}>
              <span>{t("Knowledge.k220")}</span>
              <button onClick={() => setAiTags([])} style={{
            background: 'none',
            border: 'none',
            color: '#FF5050',
            cursor: 'pointer'
          }}>✕</button>
            </div>
            <div style={{
          display: 'flex',
          gap: 6,
          flexWrap: 'wrap'
        }}>
              {aiTags.map((t, i) => <span key={i} className={styles.aiTagBadge}>{t}</span>)}
            </div>
          </div>}

        {(viewMode === 'tag' || typeFilter !== 'all') && <div className={styles.filterChips}>
            {viewMode === 'tag' && viewFilterId && <span className={styles.filterChip}>
                🏷️ {tags.find(t => t.id === viewFilterId)?.name || t("Knowledge.k221")}
                <button className={styles.filterChipDel} onClick={() => handleViewModeChange('all')}>×</button>
              </span>}
            {typeFilter !== 'all' && <span className={styles.filterChip}>
                {getTypeIcon(typeFilter)} {getTypeLabel(typeFilter)}
                <button className={styles.filterChipDel} onClick={() => setTypeFilter('all')}>×</button>
              </span>}
          </div>}

        <section className={styles.contentArea}>
          {isSearching ? semanticSearch ? semanticSearchResults.length === 0 ? <div className={styles.emptyTip}>{t("Knowledge.k222")}</div> : <div className={styles.resultList}>
                  {semanticSearchResults.map(r => <div key={r.entry_id} className={styles.resultCard} onClick={() => {
            const entry = allEntries.find(e => e.id === r.entry_id);
            if (entry) handleOpenEntry(entry);
          }}>
                      <span>{getTypeIcon(r.entry_type)}</span>
                      <span className={styles.resultName}>{r.title}</span>
                      <span className={styles.semanticScore}>
                        {r.score >= 0.7 ? '🟢' : r.score >= 0.4 ? '🟡' : '🔴'} {(r.score * 100).toFixed(0)}%
                      </span>
                      <span className={styles.resultType}>{r.category_name}</span>
                      <span className={styles.resultPath} style={{
              fontSize: 10,
              color: '#666'
            }}>{r.content_preview.slice(0, 80)}</span>
                    </div>)}
                </div> : searchResults.filter(e => typeFilter === 'all' || e.entry_type === typeFilter).length === 0 ? <div className={styles.emptyTip}>{t("Knowledge.k222")}</div> : <div className={styles.resultList}>
                  {searchResults.filter(e => typeFilter === 'all' || e.entry_type === typeFilter).map(entry => <div key={entry.id} className={`${styles.resultCard}${missingFiles.has(entry.id) ? ` ${styles.entryMissing}` : ''}`} onClick={() => handleOpenEntry(entry)}>
                      <span>{getTypeIcon(entry.entry_type)}</span>
                      <span className={styles.resultName}>{entry.name}</span>
                      <span className={styles.resultPath}>{entry.path_url}</span>
                      {entry.source_path && <span className={styles.resultPath} style={{
              color: '#FFC107',
              fontSize: 10
            }}>📌 {entry.source_path}</span>}
                      <span className={styles.resultType}>{getTypeLabel(entry.entry_type)}</span>
                      <span className={styles.resultTime}>{formatTime(entry.updated_at)}</span>
                    </div>)}
                </div> : selectedEntry ? <article className={`${styles.previewPanel} ${mediaViewerEntry ? styles.previewPanelFull : ''}`}>
              {fileEditMode ? <TextEditor filePath={selectedEntry.path_url || ''} fileExt={(selectedEntry.path_url || '').split('.').pop()?.toLowerCase() || 'txt'} initialContent={fileEditContent} formatType={fileEditFormatType} fileName={selectedEntry.name} onSave={handleFileEditSaved} onClose={handleStopFileEdit} showStatus={showStatus} wikiEntries={allEntries.map(e => e.name)} onWikiLinkClick={handleWikiLinkClick} /> : tableEditMode ? <TableEditor filePath={selectedEntry.path_url || ''} fileExt={tableEditExt} initialSheets={tableEditSheets} fileName={selectedEntry.name} onSave={handleTableEditSaved} onClose={handleStopTableEdit} showStatus={showStatus} /> : pptEditMode ? <PptEditor filePath={selectedEntry.path_url || ''} initialSlides={pptEditSlides} fileName={selectedEntry.name} onSave={() => {}} onClose={handleStopPptEdit} showStatus={showStatus} /> : pdfEditMode ? <PdfEditor filePath={selectedEntry.path_url || ''} fileName={selectedEntry.name} onClose={handleStopPdfEdit} showStatus={showStatus} /> : imageEditMode ? <ImageEditor filePath={selectedEntry.path_url || ''} fileName={selectedEntry.name} onClose={handleStopImageEdit} showStatus={showStatus} /> : audioEditMode ? <AudioEditor filePath={selectedEntry.path_url || ''} fileName={selectedEntry.name} onClose={handleStopAudioEdit} showStatus={showStatus} /> : mediaViewerEntry ? <div className={styles.mediaInlineViewer}>
                  <div className={styles.mediaInlineHeader}>
                    <div className={styles.mediaInlineHeaderLeft}>
                      <span className={styles.mediaInlineIcon}>{getTypeIcon(mediaViewerEntry.entry_type)}</span>
                      <span className={styles.mediaInlineTitle} style={{
                  color: mediaViewerTextColor
                }}>{mediaViewerEntry.name}</span>
                    </div>
                    <div className={styles.mediaInlineActions}>
                      {(() => {
                  const url = mediaViewerEntry.path_url || '';
                  const ext = url.split('.').pop()?.toLowerCase() || '';
                  const isExternal = mediaViewerEntry.path_url && !mediaViewerEntry.path_url.startsWith('kb://') && !mediaViewerEntry.path_url.startsWith('http');
                  const canEdit = isExternal && editSupportedExts.has(ext);
                  const isEditing = fileEditMode;
                  if (!canEdit) return null;
                  return <button className={styles.modeToggleIcon} onClick={() => {
                    if (isEditing) {
                      handleStopFileEdit();
                    } else {
                      handleStartFileEdit(mediaViewerEntry);
                    }
                  }} title={isEditing ? t("Knowledge.k223") : t("Knowledge.k224")}>
                            {isEditing ? '✏️' : '📖'}
                          </button>;
                })()}
                      <div className={styles.colorPickerInline}>
                        {[{
                    color: '#E0E0E0',
                    label: t("game.components.RealmCard.k14")
                  }, {
                    color: '#00FF00',
                    label: t("Knowledge.k225")
                  }, {
                    color: '#FFD700',
                    label: t("Knowledge.k226")
                  }, {
                    color: '#00F0FF',
                    label: t("Knowledge.k227")
                  }, {
                    color: '#B026FF',
                    label: t("game.components.RealmCard.k17")
                  }, {
                    color: '#FF6B6B',
                    label: t("game.components.RealmCard.k16")
                  }].map(c => <button key={c.color} className={`${styles.colorDot} ${mediaViewerTextColor === c.color ? styles.colorDotActive : ''}`} style={{
                    backgroundColor: c.color
                  }} onClick={() => setMediaViewerTextColor(c.color)} title={c.label} />)}
                      </div>
                      <button className={styles.modeToggleBtn} onClick={handleCloseMediaViewer} title={t("Knowledge.k228")}>
                        ✕
                      </button>
                    </div>
                  </div>
                  <div className={styles.mediaInlineBody}>
                    {mediaViewerLoading ? <div className={styles.mediaLoadingWrap}>
                        <div className={styles.spinner} />
                        <span>{t("common.loading")}</span>
                      </div> : mediaViewerError ? <div className={styles.mediaErrorWrap}>
                        <span>⚠ {mediaViewerError}</span>
                        <button className={styles.mediaExternalBtn} onClick={() => ipc.invoke('system_open_file', {
                  path: mediaViewerEntry.path_url
                })}>
                          {t("Knowledge.k229")}
                        </button>
                      </div> : mediaViewerMime.startsWith('video/') || mediaViewerMime.startsWith('audio/') ? <MediaPlayer src={mediaViewerFileUrl} mimeType={mediaViewerMime} fileName={mediaViewerEntry.name} className={mediaViewerMime.startsWith('video/') ? styles.mediaInlineVideo : styles.mediaInlineAudioWrap} /> : mediaViewerMime.startsWith('image/') ? <img src={mediaViewerFileUrl} alt={mediaViewerEntry.name} className={styles.mediaInlineImage} /> : mediaViewerMime === 'application/pdf' ? <embed src={`data:application/pdf;base64,${mediaViewerBase64}`} type="application/pdf" className={styles.mediaInlinePdf} /> : mediaViewerMime === 'table' && tableData ? <div className={styles.mediaInlineTable}>
                        <table className={styles.dataTable}>
                          <thead>
                            <tr>
                              <th className={styles.dataTableTh}>#</th>
                              {Array.from({
                        length: tableData.totalCols || 0
                      }).map((_, i) => <th key={i} className={styles.dataTableTh}>{excelColName(i)}</th>)}
                            </tr>
                          </thead>
                          <tbody>
                            {tableData.rows.map((row, ri) => <tr key={ri}>
                                <td className={styles.dataTableTdRowNum}>{ri + 1}</td>
                                {row.map((cell, ci) => <td key={ci} className={styles.dataTableTd} style={{
                        color: mediaViewerTextColor
                      }}>{cell}</td>)}
                              </tr>)}
                          </tbody>
                        </table>
                      </div> : mediaViewerMime === 'pptx' ? <div className={styles.pptxViewerInline}>
                        <div className={styles.pptxNav}>
                          <button className={styles.pptxNavBtn} disabled={pptxCurrentSlide === 0} onClick={() => setPptxCurrentSlide(prev => Math.max(0, prev - 1))}>
                            {t("Knowledge.k230")}
                          </button>
                          <span className={styles.pptxCounter}>
                            {pptxCurrentSlide + 1} / {pptxSlideCount}
                          </span>
                          <button className={styles.pptxNavBtn} disabled={pptxCurrentSlide >= pptxSlideCount - 1} onClick={() => setPptxCurrentSlide(prev => Math.min(pptxSlideCount - 1, prev + 1))}>
                            {t("Knowledge.k231")}
                          </button>
                          <button className={styles.pptxNavBtn} onClick={() => setMediaViewerFullscreen(true)}>
                            {t("Knowledge.k232")}
                          </button>
                        </div>
                        <div className={styles.pptxSlideContent} ref={pptxSlideRef} />
                      </div> : mediaViewerMime === 'text/plain' ? <pre className={styles.mediaInlineText} style={{
                color: mediaViewerTextColor
              }}>{mediaViewerTextContent}</pre> : mediaViewerMime === 'text/html' ? <div className={styles.mediaInlineHtml} dangerouslySetInnerHTML={{
                __html: sanitizeHtml(mediaViewerHtmlContent)
              }} /> : null}
                  </div>
                </div> : <>
              {selectedEntry && missingFiles.has(selectedEntry.id) && <div style={{
              margin: '12px 16px',
              padding: '12px 16px',
              borderRadius: '8px',
              background: 'rgba(244,67,54,0.12)',
              border: '1.5px solid #F44336',
              color: '#FF5252',
              fontSize: '13.5px',
              lineHeight: '1.6'
            }}>
                  <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                fontWeight: 600,
                marginBottom: '4px'
              }}>
                    <span style={{
                  fontSize: '16px'
                }}>⚠️</span>
                    {t("Knowledge.k233")}
                  </div>
                  <div style={{
                opacity: 0.85,
                fontSize: '12.5px'
              }}>
                    {t("Knowledge.k234")}{selectedEntry.source_path || selectedEntry.path_url}
                  </div>
                </div>}
              <div className={styles.previewHeader}>
                <span className={styles.previewIcon}>{getTypeIcon(selectedEntry.entry_type)}</span>
                <div className={styles.previewMeta}>
                  <h4 className={styles.previewTitle}>{selectedEntry.name}</h4>
                  <div className={styles.previewInfo}>
                    <span className={styles.tag}>{getTypeLabel(selectedEntry.entry_type)}</span>
                    <span>{t("Knowledge.k235")} {formatTime(selectedEntry.updated_at)}</span>
                    <span>{t("Knowledge.k236")} {formatTime(selectedEntry.created_at)}</span>
                  </div>
                  {backlinksLoading && <div className={styles.backlinkHint}>{t("Knowledge.k237")}</div>}
                  {!backlinksLoading && backlinks.length > 0 && <div className={styles.backlinkPanel}>
                      <div className={styles.backlinkTitle}>{t("Knowledge.k238")} {backlinks.length} {t("Knowledge.k239")}</div>
                      <div className={styles.backlinkList}>
                        {backlinks.map((bl, i) => <div key={i} className={styles.backlinkItem} onClick={() => {
                      handleOpenEntry(bl.entry);
                    }} title={bl.snippet}>
                            <span className={styles.backlinkItemIcon}>{getTypeIcon(bl.entry.entry_type)}</span>
                            <span className={styles.backlinkItemName}>{bl.entry.name}</span>
                            <span className={styles.backlinkItemSnippet}>{bl.snippet.slice(0, 60)}{bl.snippet.length > 60 ? '...' : ''}</span>
                          </div>)}
                      </div>
                    </div>}
                  <div className={styles.snapshotToggle}>
                    <button className={styles.btnSm} onClick={() => {
                    if (!showSnapshots) {
                      loadSnapshots(selectedEntry.id);
                    }
                    setShowSnapshots(!showSnapshots);
                    setSnapshotPreview(null);
                  }}>
                      {showSnapshots ? t("Knowledge.k240") : t("Knowledge.k241", {
                      arg0: snapshots.length > 0 ? `(${snapshots.length})` : ''
                    })}
                    </button>
                  </div>
                  {showSnapshots && <div className={styles.snapshotPanel}>
                      {snapshotsLoading ? <div className={styles.snapshotHint}>{t("Knowledge.k242")}</div> : snapshots.length === 0 ? <div className={styles.snapshotHint}>{t("Knowledge.k243")}</div> : <>
                          <div className={styles.snapshotTitle}>{t("Knowledge.k244")} {snapshots.length} {t("components.Linux.k18")}</div>
                          <div className={styles.snapshotList}>
                            {snapshots.map(snap => <div key={snap.id} className={`${styles.snapshotItem} ${snapshotPreview?.id === snap.id ? styles.snapshotItemActive : ''}`}>
                                <div className={styles.snapshotMeta}>
                                  <span className={styles.snapshotTime}>{formatTime(snap.created_at)}</span>
                                  <span className={styles.snapshotSize}>{formatFileSize(snap.content_length)}</span>
                                </div>
                                <div className={styles.snapshotActions}>
                                  <button className={styles.btnSmXs} onClick={() => setSnapshotPreview(snapshotPreview?.id === snap.id ? null : {
                            id: snap.id,
                            content: snap.content
                          })}>
                                    {snapshotPreview?.id === snap.id ? t("common.collapse") : t("common.preview")}
                                  </button>
                                  <button className={styles.btnSmXs} onClick={() => handleRestoreSnapshot(snap.id)}>
                                    {t("Knowledge.k245")}
                                  </button>
                                </div>
                              </div>)}
                          </div>
                        </>}
                      {snapshotPreview && <div className={styles.snapshotPreview}>
                          <div className={styles.snapshotPreviewHeader}>
                            {t("Knowledge.k246")} {formatTime(snapshotPreview.id > 0 ? snapshots.find(s => s.id === snapshotPreview.id)?.created_at || 0 : 0)}
                          </div>
                          <pre className={styles.snapshotPreviewContent}>{snapshotPreview.content}</pre>
                        </div>}
                    </div>}
                </div>
                <div className={styles.previewActions}>
                  {!missingFiles.has(selectedEntry.id) && <button className={styles.btnOpen} onClick={() => handleOpenEntry(selectedEntry)}>
                    {t("common.open")}{selectedEntry.entry_type === 'link' ? t("components.TextEditor.k9") : t("knowledge.GraphView.k1")}
                  </button>}
                  <button className={`${styles.btnSm} ${(selectedEntry as any).is_favorited === 1 ? styles.btnFavoriteActive : ''}`} onClick={() => handleToggleFavorite(selectedEntry.id)} title={t("Knowledge.k247")}>
                    ⭐
                  </button>
                  {isPinnable(selectedEntry) && <button className={`${styles.btnSm} ${pinnedEntries.has(selectedEntry.id) ? styles.btnFavoriteActive : ''}`} onClick={e => handleTogglePin(selectedEntry, e)} title={pinnedEntries.has(selectedEntry.id) ? t("Knowledge.k258") : t("Knowledge.k259")}>
                    {pinnedEntries.has(selectedEntry.id) ? '📌' : '📍'}
                  </button>}
                  <button className={styles.btnDel} onClick={() => handleDeleteEntry(selectedEntry.id)}>{t("common.delete")}</button>
                </div>
              </div>
              <div className={styles.previewBody}>
                <div className={styles.tagRow}>
                  <span className={styles.tagRowLabel}>{t("Knowledge.k248")}</span>
                  {entryTags.get(selectedEntry.id)?.map(t => <span key={t.id} className={styles.tagChipSm} style={{
                  borderColor: t.color + '55',
                  background: t.color + '15'
                }}>
                      <span className={styles.tagColorDot} style={{
                    background: t.color
                  }} />
                      {t.name}
                      <button className={styles.tagDelBtn} onClick={() => handleRemoveTagFromEntry(selectedEntry.id, t.id)}>×</button>
                    </span>)}
                  <select className={styles.tagAddSelect} value="" onChange={e => {
                  if (e.target.value) {
                    handleAddTagToEntry(selectedEntry.id, Number(e.target.value));
                    e.target.value = '';
                  }
                }}>
                    <option value="">{t("Knowledge.k249")}</option>
                    {tags.filter(t => !entryTags.get(selectedEntry.id)?.some(et => et.id === t.id)).map(t => <option key={t.id} value={t.id}>{t.name}</option>)}
                  </select>
                </div>
                <div className={styles.pathDisplay}>
                  <span className={styles.pathLabel}>{t("Knowledge.k250")}</span>
                  <code className={styles.pathValue}>{selectedEntry.path_url}</code>
                </div>
                {selectedEntry.source_path && <div className={styles.pathDisplay} style={{
                color: '#FFC107'
              }}>
                    <span className={styles.pathLabel}>{t("Knowledge.k251")}</span>
                    <code className={styles.pathValue} style={{
                  color: 'rgba(255,193,7,0.85)'
                }}>{selectedEntry.source_path}</code>
                  </div>}
                {(() => {
                const url = (selectedEntry.path_url || '').toLowerCase();
                const extMatch = url.match(/\.([a-zA-Z0-9]+)$/);
                const ext = extMatch ? extMatch[1] : '';
                const imageExts = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg', 'ico'];
                const codeExts = ['rs', 'ts', 'tsx', 'js', 'jsx', 'py', 'go', 'java', 'c', 'cpp', 'h', 'hpp', 'cs', 'rb', 'php', 'swift', 'kt', 'scala', 'lua', 'sh', 'bash', 'zsh', 'yaml', 'yml', 'toml', 'json', 'xml', 'css', 'scss', 'less', 'html', 'htm', 'sql'];
                const languageMapR: Record<string, string> = {
                  rs: 'rust',
                  ts: 'typescript',
                  tsx: 'typescriptreact',
                  js: 'javascript',
                  jsx: 'javascriptreact',
                  py: 'python',
                  go: 'go',
                  java: 'java',
                  c: 'c',
                  cpp: 'cpp',
                  h: 'c',
                  hpp: 'cpp',
                  cs: 'csharp',
                  rb: 'ruby',
                  php: 'php',
                  swift: 'swift',
                  kt: 'kotlin',
                  scala: 'scala',
                  lua: 'lua',
                  sh: 'bash',
                  bash: 'bash',
                  zsh: 'bash',
                  yaml: 'yaml',
                  yml: 'yaml',
                  toml: 'toml',
                  json: 'json',
                  xml: 'xml',
                  css: 'css',
                  scss: 'scss',
                  less: 'less',
                  html: 'html',
                  htm: 'html',
                  sql: 'sql'
                };

                // 文件已删除：直接提示，不尝试加载
                if (missingFiles.has(selectedEntry.id) && selectedEntry.source_path) {
                  return <div className={styles.previewMissing}>
                        <div style={{
                      fontSize: 40,
                      marginBottom: 16
                    }}>⚠️</div>
                        <div style={{
                      color: '#E57373',
                      fontSize: 15,
                      fontWeight: 600,
                      marginBottom: 8
                    }}>{t("Knowledge.k252")}</div>
                        <div style={{
                      color: '#888',
                      fontSize: 12,
                      marginBottom: 4
                    }}>{t("Knowledge.k253")}</div>
                        <code style={{
                      color: '#A55',
                      fontSize: 11,
                      background: 'rgba(229,115,115,0.08)',
                      padding: '4px 8px',
                      borderRadius: 4,
                      display: 'inline-block',
                      marginTop: 8,
                      wordBreak: 'break-all'
                    }}>{selectedEntry.source_path}</code>
                        <div style={{
                      marginTop: 16,
                      display: 'flex',
                      gap: 8,
                      justifyContent: 'center'
                    }}>
                          <button className={styles.btnDel} onClick={() => {
                        if (confirm(t("Knowledge.k254"))) handleDeleteEntry(selectedEntry.id);
                      }}>{t("Knowledge.k255")}</button>
                        </div>
                      </div>;
                }
                if (previewLoading) {
                  return <div className={styles.previewLoading}>
                        <span className={styles.previewSpinner}></span>
                        <span>{t("Knowledge.k256")}</span>
                      </div>;
                }
                if (previewError) {
                  return <div className={styles.previewError}>{previewError}</div>;
                }
                if (imageExts.includes(ext) && thumbnailUrl) {
                  return <div className={styles.previewImageWrap}>
                        <div className={styles.previewImageContainer} style={{
                      cursor: 'pointer',
                      overflow: 'hidden',
                      position: 'relative',
                      borderRadius: 6
                    }} onClick={() => handleOpenEntry(selectedEntry)}>
                          <img src={thumbnailUrl} alt={selectedEntry.name} className={styles.previewImage} />
                          <div style={{
                        position: 'absolute',
                        inset: 0,
                        background: 'rgba(0,0,0,0)',
                        transition: 'background 0.2s',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center'
                      }} onMouseEnter={e => e.currentTarget.style.background = 'rgba(0,0,0,0.4)'} onMouseLeave={e => e.currentTarget.style.background = 'rgba(0,0,0,0)'}>
                            <span style={{
                          color: '#fff',
                          fontSize: 14,
                          opacity: 0,
                          transition: 'opacity 0.2s',
                          pointerEvents: 'none'
                        }} onMouseEnter={e => {
                          const p = e.currentTarget.parentElement;
                          if (p) {
                            (e.currentTarget as HTMLElement).style.opacity = '1';
                            p.style.background = 'rgba(0,0,0,0.4)';
                          }
                        }}>{t("Knowledge.k257")}</span>
                          </div>
                        </div>
                        <div className={styles.previewImageInfo}>
                          <span>{t("Knowledge.k258")}</span>
                          <button className={styles.btnOpen} onClick={() => handleOpenEntry(selectedEntry)}>{t("Knowledge.k257")}</button>
                        </div>
                      </div>;
                }
                if (ext === 'pdf' && pdfPreviewUrl) {
                  return <div className={styles.previewPdfWrap}>
                        <PdfViewer base64Data={pdfPreviewUrl} fileName={selectedEntry.name} />
                      </div>;
                }
                if (ext === 'csv' && csvPreviewData) {
                  return <div className={styles.previewCsvWrap}>
                        <div className={styles.previewCsvToolbar}>
                          <span>{t("Knowledge.k259")}{csvPreviewData.rows.length} {t("components.TableEditor.k6")} {csvPreviewData.headers.length} {t("Knowledge.k260")}</span>
                        </div>
                        <div className={styles.previewCsvTable}>
                          <table>
                            <thead>
                              <tr>
                                {csvPreviewData.headers.map((h, i) => <th key={i}>{h}</th>)}
                              </tr>
                            </thead>
                            <tbody>
                              {csvPreviewData.rows.map((row, ri) => <tr key={ri}>
                                  {row.map((cell, ci) => <td key={ci}>{cell}</td>)}
                                </tr>)}
                            </tbody>
                          </table>
                        </div>
                      </div>;
                }
                if (codeExts.includes(ext) && highlightTokens.length > 0) {
                  return <div className={styles.previewCodeWrap}>
                        <CodePreview tokens={highlightTokens} language={languageMapR[ext] || ext} />
                      </div>;
                }
                if (selectedEntry.entry_type === 'text' && (!selectedEntry.path_url || selectedEntry.path_url.startsWith('kb://'))) {
                  return <div className={styles.contentEditor}>
                        <div className={styles.contentEditorHeader}>
                          <span style={{
                        color: mediaViewerTextColor,
                        fontSize: '13px'
                      }}>{selectedEntry.name}</span>
                          <button className={styles.modeToggleIcon} onClick={() => {
                        if (fileEditMode) {
                          handleStopFileEdit();
                        } else {
                          setFileEditContent(selectedEntry.content || editContent || '');
                          setFileEditFormatType('markdown');
                          setFileEditMode(true);
                        }
                      }} title={fileEditMode ? t("Knowledge.k223") : t("Knowledge.k224")}>
                            {fileEditMode ? '✏️' : '📖'}
                          </button>
                        </div>
                        {fileEditMode ? <TextEditor filePath="" fileExt="md" initialContent={fileEditContent} formatType="markdown" fileName={selectedEntry.name} onSave={() => {
                      handleStopFileEdit();
                    }} onClose={handleStopFileEdit} showStatus={showStatus} wikiEntries={allEntries.map(e => e.name)} onWikiLinkClick={handleWikiLinkClick} /> : <div className={styles.contentPreview}>
                            {(() => {
                        const raw = (editContent || selectedEntry.content || '').slice(0, 2000);
                        const suffix = (editContent || selectedEntry.content || '').length > 2000 ? '...' : '';
                        const parts = raw.split(/(\[\[[^\]]+\]\])/g);
                        return <span>
                                  {parts.map((part, i) => {
                            const m = part.match(/^\[\[([^\]]+)\]\]$/);
                            if (m) {
                              return <span key={i} className={styles.wikiLinkInline} onClick={() => handleWikiLinkClick(m[1])} title={t("Knowledge.k261", {
                                arg0: m[1]
                              })}>[[{m[1]}]]</span>;
                            }
                            return <span key={i}>{part}</span>;
                          })}
                                  {suffix}
                                </span>;
                      })()}
                          </div>}
                      </div>;
                }
                const isExternalFile = selectedEntry.path_url && !selectedEntry.path_url.startsWith('kb://') && !selectedEntry.path_url.startsWith('http');
                const videoExtsP = ['mp4', 'webm', 'avi', 'mkv', 'mov', 'wmv', 'flv'];
                const audioExtsP = ['mp3', 'wav', 'flac', 'aac', 'ogg', 'opus', 'wma'];
                const docExtsP = ['docx', 'doc', 'xlsx', 'xls', 'pptx', 'ppt', 'odt', 'ods', 'odp', 'rtf'];
                if (isExternalFile) {
                  if (videoExtsP.includes(ext)) {
                    return <div className={styles.previewMediaWrap}>
                          <div className={styles.previewMediaIcon}>🎬</div>
                          <span className={styles.previewMediaLabel}>{t("Knowledge.k262")} {ext.toUpperCase()}</span>
                          <button className={styles.btnOpen} onClick={() => handleOpenEntry(selectedEntry)}>{t("Knowledge.k263")}</button>
                        </div>;
                  }
                  if (audioExtsP.includes(ext)) {
                    return <div className={styles.previewMediaWrap}>
                          <div className={styles.previewMediaIcon}>🎵</div>
                          <span className={styles.previewMediaLabel}>{t("Knowledge.k264")} {ext.toUpperCase()}</span>
                          <button className={styles.btnOpen} onClick={() => handleOpenEntry(selectedEntry)}>{t("Knowledge.k263")}</button>
                        </div>;
                  }
                  if (docExtsP.includes(ext)) {
                    return <div className={styles.previewMediaWrap}>
                          <div className={styles.previewMediaIcon}>📄</div>
                          <span className={styles.previewMediaLabel}>{t("Knowledge.k265")} {ext.toUpperCase()}</span>
                          <button className={styles.btnOpen} onClick={() => handleOpenEntry(selectedEntry)}>{t("Knowledge.k266")}</button>
                        </div>;
                  }
                }
                return <>
                      <div className={styles.divider}></div>
                      <div className={styles.previewNote}>
                        <p>{t("Knowledge.k267")}</p>
                        <p>{t("Knowledge.k268")}</p>
                      </div>
                    </>;
              })()}
              </div>
              </>}
            </article> : selectedId?.type === 'category' ? <div className={styles.folderContentView}>
                <div className={styles.folderContentHeader}>
                  <span className={styles.folderContentIcon}>📂</span>
                  <span className={styles.folderContentName}>{categories.find(c => c.id === selectedId.id)?.name || t("knowledge.GraphView.k8")}</span>
                  <span className={styles.folderContentCount}>{selectedSubCategories.length} {t("Knowledge.k269")} {selectedCategoryEntries.length} {t("knowledge.GraphView.k1")}</span>
                </div>
                {selectedSubCategories.length === 0 && selectedCategoryEntries.length === 0 ? <div className={styles.folderEmpty}>
                    <span>{t("Knowledge.k270")}</span>
                    <button className={styles.btnPrimary} onClick={handleAddEntry} style={{
              marginTop: 12,
              fontSize: 12,
              padding: '6px 16px'
            }}>{t("Knowledge.k211")}</button>
                  </div> : displayMode === 'card' ? <div className={styles.cardGrid}>
                    {/* 子文件夹 */}
                    {selectedSubCategories.map(subCat => <div key={`cat-${subCat.id}`} className={styles.cardItem} onClick={() => setSelectedId({
              type: 'category',
              id: subCat.id
            })}>
                        <div className={styles.cardIcon}>📁</div>
                        <div className={styles.cardBody}>
                          <div className={styles.cardName}>{subCat.name}</div>
                          <div className={styles.cardMeta}>{t("Knowledge.k271")} {allEntries.filter(e => e.category_id === subCat.id).length} {t("Knowledge.k179")}</div>
                        </div>
                      </div>)}
                    {/* 文件 */}
                    {selectedCategoryEntries.map(entry => <div key={entry.id} className={`${styles.cardItem}${missingFiles.has(entry.id) ? ` ${styles.entryMissing}` : ''}`} onClick={() => handleOpenEntry(entry)} onDoubleClick={() => handleOpenFileViewer(entry)}>
                        <div className={styles.cardIcon}>{getTypeIcon(entry.entry_type)}</div>
                        <div className={styles.cardBody}>
                          <div className={styles.cardName}>{entry.name}</div>
                          <div className={styles.cardMeta}>
                            {getTypeLabel(entry.entry_type)} · {formatTime(entry.updated_at)}
                            {(entry as any).is_favorited === 1 && ' ⭐'}{pinnedEntries.has(entry.id) && ' 📶'}
                          </div>
                          {entry.source_path && <div className={styles.cardSource}>📌 {entry.source_path}</div>}
                        </div>
                      </div>)}
                  </div> : displayMode === 'timeline' ? (() => {
            const sorted = [...selectedCategoryEntries].sort((a, b) => b.created_at - a.created_at);
            const grouped: Record<string, typeof sorted> = {};
            sorted.forEach(e => {
              const d = new Date(e.created_at);
              const k = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
              if (!grouped[k]) grouped[k] = [];
              grouped[k].push(e);
            });
            return <div className={styles.timelineList}>
                        {/* 子文件夹（时间线模式放在顶部） */}
                        {selectedSubCategories.length > 0 && <div className={styles.timelineGroup}>
                            <div className={styles.timelineDate}>{t("Knowledge.k272")}</div>
                            <div className={styles.timelineItems}>
                              {selectedSubCategories.map(subCat => <div key={`cat-${subCat.id}`} className={styles.timelineItem} onClick={() => setSelectedId({
                    type: 'category',
                    id: subCat.id
                  })}>
                                  <div className={styles.timelineDot} style={{
                      background: '#00F0FF'
                    }} />
                                  <div className={styles.timelineContent}>
                                    <div className={styles.timelineItemName}>📁 {subCat.name}</div>
                                    <div className={styles.timelineItemMeta}>{t("Knowledge.k271")} {allEntries.filter(e => e.category_id === subCat.id).length} {t("Knowledge.k179")}</div>
                                  </div>
                                </div>)}
                            </div>
                          </div>}
                        {/* 文件按日期分组 */}
                        {Object.entries(grouped).map(([date, entries]) => <div key={date} className={styles.timelineGroup}>
                            <div className={styles.timelineDate}>{date}</div>
                            <div className={styles.timelineItems}>
                              {entries.map(entry => <div key={entry.id} className={`${styles.timelineItem}${missingFiles.has(entry.id) ? ` ${styles.entryMissing}` : ''}`} onClick={() => handleOpenEntry(entry)} onDoubleClick={() => handleOpenFileViewer(entry)}>
                                  <div className={styles.timelineDot} />
                                  <div className={styles.timelineContent}>
                                    <div className={styles.timelineItemName}>{getTypeIcon(entry.entry_type)} {entry.name}</div>
                                    <div className={styles.timelineItemMeta}>
                                      {getTypeLabel(entry.entry_type)} · {formatTime(entry.created_at)}
                                      {(entry as any).is_favorited === 1 && ' ⭐'}{pinnedEntries.has(entry.id) && ' 📶'}
                                    </div>
                                  </div>
                                </div>)}
                            </div>
                          </div>)}
                      </div>;
          })() : <div className={styles.folderEntryList}>
                    {/* 子文件夹 */}
                    {selectedSubCategories.map(subCat => <div key={`cat-${subCat.id}`} className={styles.folderEntryItem} onClick={() => setSelectedId({
              type: 'category',
              id: subCat.id
            })}>
                        <span className={styles.folderEntryIcon}>📁</span>
                        <div className={styles.folderEntryInfo}>
                          <span className={styles.folderEntryName}>{subCat.name}</span>
                          <span className={styles.folderEntryMeta}>{t("Knowledge.k271")} {allEntries.filter(e => e.category_id === subCat.id).length} {t("Knowledge.k179")}</span>
                        </div>
                      </div>)}
                    {/* 文件 */}
                    {selectedCategoryEntries.map(entry => {
              const isMissing = missingFiles.has(entry.id);
              return <div key={entry.id} className={`${styles.folderEntryItem}${isMissing ? ` ${styles.entryMissing}` : ''}`} onClick={() => handleOpenEntry(entry)} onDoubleClick={() => handleOpenFileViewer(entry)}>
                        <span className={styles.folderEntryIcon}>{getTypeIcon(entry.entry_type)}</span>
                        <div className={styles.folderEntryInfo}>
                          <span className={styles.folderEntryName} style={isMissing ? {
                    color: '#FF4444',
                    fontWeight: 'bold',
                    textDecoration: 'underline'
                  } : undefined}>{entry.name}</span>
                          <span className={styles.folderEntryMeta}>
                            {getTypeLabel(entry.entry_type)} · {formatTime(entry.updated_at)}
                            {(entry as any).is_favorited === 1 && ' ⭐'}{pinnedEntries.has(entry.id) && ' 📶'}
                          </span>
                        </div>
                        <div className={styles.folderEntryActions}>
                          <button className={`${styles.btnSm} ${(entry as any).is_favorited === 1 ? styles.btnFavoriteActive : ''}`} onClick={e => {
                    e.stopPropagation();
                    handleToggleFavorite(entry.id);
                  }} title={t("components.FloatingBall.k57")}>⭐</button>
                          {isPinnable(entry) && <button className={`${styles.btnSm} ${pinnedEntries.has(entry.id) ? styles.btnFavoriteActive : ''}`} onClick={e => handleTogglePin(entry, e)} title={pinnedEntries.has(entry.id) ? t("Knowledge.k298") : t("Knowledge.k299")}>{pinnedEntries.has(entry.id) ? '📌' : '📍'}</button>}
                          <button className={styles.btnDel} onClick={e => {
                    e.stopPropagation();
                    handleDeleteEntry(entry.id);
                  }} title={t("common.delete")}>✕</button>
                        </div>
                      </div>;
            })}
                  </div>}
              </div> : displayMode === 'card' ? <div className={styles.cardView}>
                <div className={styles.cardViewHeader}>
                  <span className={styles.cardViewTitle}>{t("Knowledge.k273")}</span>
                  <span className={styles.cardViewCount}>{allEntries.filter(e => typeFilter === 'all' || e.entry_type === typeFilter).length} {t("Knowledge.k179")}</span>
                </div>
                {allEntries.filter(e => typeFilter === 'all' || e.entry_type === typeFilter).length === 0 ? <div className={styles.emptyTip}>{t("Knowledge.k274")}</div> : <div className={styles.cardGrid}>
                    {allEntries.filter(e => typeFilter === 'all' || e.entry_type === typeFilter).map(entry => <div key={entry.id} className={`${styles.cardItem}${missingFiles.has(entry.id) ? ` ${styles.entryMissing}` : ''}`} onClick={() => handleOpenEntry(entry)} onDoubleClick={() => handleOpenFileViewer(entry)}>
                        <div className={styles.cardIcon}>{getTypeIcon(entry.entry_type)}</div>
                        <div className={styles.cardBody}>
                          <div className={styles.cardName}>{entry.name}</div>
                          <div className={styles.cardMeta}>
                            {getTypeLabel(entry.entry_type)} · {formatTime(entry.updated_at)}
                            {(entry as any).is_favorited === 1 && ' ⭐'}{pinnedEntries.has(entry.id) && ' 📶'}
                          </div>
                          {entry.source_path && <div className={styles.cardSource}>📌 {entry.source_path}</div>}
                        </div>
                      </div>)}
                  </div>}
              </div> : displayMode === 'timeline' ? <div className={styles.timelineView}>
                <div className={styles.timelineHeader}>
                  <span className={styles.timelineTitle}>{t("Knowledge.k275")}</span>
                  <span className={styles.timelineCount}>{allEntries.filter(e => typeFilter === 'all' || e.entry_type === typeFilter).length} {t("Knowledge.k179")}</span>
                </div>
                {(() => {
            const filtered = allEntries.filter(e => typeFilter === 'all' || e.entry_type === typeFilter);
            const sorted = [...filtered].sort((a, b) => b.created_at - a.created_at);
            if (sorted.length === 0) return <div className={styles.emptyTip}>{t("Knowledge.k274")}</div>;
            const grouped: Record<string, typeof sorted> = {};
            sorted.forEach(e => {
              const date = new Date(e.created_at);
              const key = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
              if (!grouped[key]) grouped[key] = [];
              grouped[key].push(e);
            });
            return <div className={styles.timelineList}>
                      {Object.entries(grouped).map(([date, entries]) => <div key={date} className={styles.timelineGroup}>
                          <div className={styles.timelineDate}>{date}</div>
                          <div className={styles.timelineItems}>
                            {entries.map(entry => <div key={entry.id} className={`${styles.timelineItem}${missingFiles.has(entry.id) ? ` ${styles.entryMissing}` : ''}`} onClick={() => handleOpenEntry(entry)} onDoubleClick={() => handleOpenFileViewer(entry)}>
                                <div className={styles.timelineDot} />
                                <div className={styles.timelineContent}>
                                  <div className={styles.timelineItemName}>{getTypeIcon(entry.entry_type)} {entry.name}</div>
                                  <div className={styles.timelineItemMeta}>
                                    {getTypeLabel(entry.entry_type)} · {formatTime(entry.created_at)}
                                    {(entry as any).is_favorited === 1 && ' ⭐'}{pinnedEntries.has(entry.id) && ' 📶'}
                                  </div>
                                </div>
                              </div>)}
                          </div>
                        </div>)}
                    </div>;
          })()}
              </div> : <div className={styles.emptyTip}>
              <div style={{
            fontSize: 36,
            marginBottom: 16
          }}>📚</div>
              <strong style={{
            fontSize: 15,
            color: '#aaa'
          }}>{t("components.intelligence.ActivityPanel.k1")}</strong><br /><br />
              <span style={{
            opacity: 0.4,
            lineHeight: 2
          }}>
                {t("Knowledge.k276")}<br /><br />
                {t("Knowledge.k277")}<br />
                {t("Knowledge.k278")}<br />
                {t("Knowledge.k279")}<br />
                {t("Knowledge.k280")}
              </span>
            </div>}
        </section>

      {showDirScanModal && <div className={styles.modalOverlay} onClick={() => setShowDirScanModal(false)}>
          <div className={styles.scanModal} onClick={e => e.stopPropagation()}>
            <div className={styles.scanModalHeader}>
              <h2 className={styles.modalTitle}>{t("Knowledge.k281")}</h2>
              <button className={styles.scanCloseBtn} onClick={() => setShowDirScanModal(false)}>✕</button>
            </div>
            <div className={styles.scanPathInfo}>
              <span className={styles.scanPathLabel}>{t("Knowledge.k282")}</span>
              <span className={styles.scanPathValue}>{dirScanPath}</span>
              <span className={styles.scanFileCount}>{t("components.GroupChatOrchestrationPanel.k26")} {dirScanFiles.length} {t("Knowledge.k283")}</span>
            </div>

            {dirScanLoading ? <div className={styles.scanLoading}>
                <span className={styles.scanSpinner}></span>
                <span>{t("Knowledge.k284")}</span>
              </div> : <>
                <div className={styles.scanTypeFilter}>
                  {(['all', 'text', 'image', 'video', 'audio', 'document', 'other'] as const).map(type => <button key={type} className={`${styles.scanTypeBtn} ${dirScanFilter === type ? styles.scanTypeBtnActive : ''}`} onClick={() => {
                setDirScanFilter(type);
                setDirScanSelected(new Set());
                setQuickSwitcherIndex(0);
              }}>
                      {DIR_SCAN_TYPE_ICONS[type]} {DIR_SCAN_TYPE_LABELS[type]}
                      <span className={styles.scanTypeCount}>({dirScanTypeCounts[type]})</span>
                    </button>)}
                </div>

                <div className={styles.scanToolbar}>
                  <label className={styles.scanSelectAll}>
                    <input type="checkbox" checked={dirScanSelected.size === filteredDirScanFiles.length && filteredDirScanFiles.length > 0} onChange={handleDirScanSelectAll} />
                    <span>{t("common.selectAll")} {filteredDirScanFiles.length > 0 ? `(${dirScanSelected.size}/${filteredDirScanFiles.length})` : ''}</span>
                  </label>
                  <span className={styles.scanTotalSize}>
                    {(() => {
                  const totalBytes = Array.from(dirScanSelected).map(i => filteredDirScanFiles[i]?.size_bytes || 0).reduce((a, b) => a + b, 0);
                  return totalBytes > 0 ? formatFileSize(totalBytes) : '';
                })()}
                  </span>
                </div>

                <div className={styles.aiClassifyToolbar}>
                  <button className={styles.btnSm} onClick={handleAiClassify} disabled={aiClassifyLoading || filteredDirScanFiles.length === 0}>
                    {aiClassifyLoading ? t("home.TimerPanel.k4") : t("Knowledge.k285")}
                  </button>
                  {aiClassifyResults.length > 0 && <span className={styles.aiClassifyHint}>
                      {t("Knowledge.k286")}
                      {aiClassifyResults.map((r, i) => <span key={i} className={styles.aiClassifyTag}>
                          {r.category_name} ({Math.round(r.confidence)}%)
                        </span>)}
                    </span>}
                </div>

                <div className={styles.scanFileList}>
                  {filteredDirScanFiles.length === 0 ? <div className={styles.scanEmpty}>{t("Knowledge.k287")}</div> : filteredDirScanFiles.map((file, idx) => {
                const isSelected = dirScanSelected.has(idx);
                return <label key={idx} className={`${styles.scanFileItem} ${isSelected ? styles.scanFileItemSel : ''}`}>
                          <input type="checkbox" checked={isSelected} onChange={() => toggleDirScanSelect(idx)} />
                          <span className={styles.scanFileIcon}>
                            {file.file_type === 'image' ? '🖼' : file.file_type === 'video' ? '🎬' : file.file_type === 'audio' ? '🎵' : file.file_type === 'text' ? '📝' : file.file_type === 'document' ? '📑' : '📄'}
                          </span>
                          <span className={styles.scanFileName}>{file.name}</span>
                          <span className={styles.scanFileSize}>{formatFileSize(file.size_bytes)}</span>
                        </label>;
              })}
                </div>

                <div className={styles.modalActions} style={{
              marginTop: 12
            }}>
                  <button className={styles.btnCancel} onClick={() => setShowDirScanModal(false)}>{t("common.cancel")}</button>
                  <button className={styles.btnPrimary} onClick={handleBatchImportSelected} disabled={dirScanSelected.size === 0 || dirImporting}>
                    {dirImporting ? t("Knowledge.k288") : t("Knowledge.k289", {
                  size: dirScanSelected.size
                })}
                  </button>
                </div>
              </>}
          </div>
        </div>}

      {mediaViewerFullscreen && mediaViewerEntry && <div className={styles.mediaFullscreenOverlay} onClick={() => setMediaViewerFullscreen(false)}>
          <div className={styles.mediaFullscreenContainer} onClick={e => e.stopPropagation()}>
            <div className={styles.mediaFullscreenHeader}>
              <span className={styles.mediaFullscreenTitle} style={{
              color: mediaViewerTextColor
            }}>{mediaViewerEntry.name}</span>
              <div className={styles.mediaFullscreenActions}>
                {(() => {
                const url = mediaViewerEntry.path_url || '';
                const ext = url.split('.').pop()?.toLowerCase() || '';
                const isExternal = mediaViewerEntry.path_url && !mediaViewerEntry.path_url.startsWith('kb://') && !mediaViewerEntry.path_url.startsWith('http');
                const canEdit = isExternal && editSupportedExts.has(ext);
                const isEditing = fileEditMode;
                if (!canEdit) return null;
                return <button className={styles.modeToggleIcon} onClick={() => {
                  if (isEditing) {
                    handleStopFileEdit();
                  } else {
                    handleStartFileEdit(mediaViewerEntry);
                  }
                }} title={isEditing ? t("Knowledge.k223") : t("Knowledge.k224")}>
                      {isEditing ? '✏️' : '📖'}
                    </button>;
              })()}
                <div className={styles.colorPickerInline}>
                  {[{
                  color: '#E0E0E0',
                  label: t("game.components.RealmCard.k14")
                }, {
                  color: '#00FF00',
                  label: t("Knowledge.k225")
                }, {
                  color: '#FFD700',
                  label: t("Knowledge.k226")
                }, {
                  color: '#00F0FF',
                  label: t("Knowledge.k227")
                }, {
                  color: '#B026FF',
                  label: t("game.components.RealmCard.k17")
                }, {
                  color: '#FF6B6B',
                  label: t("game.components.RealmCard.k16")
                }].map(c => <button key={c.color} className={`${styles.colorDot} ${mediaViewerTextColor === c.color ? styles.colorDotActive : ''}`} style={{
                  backgroundColor: c.color
                }} onClick={() => setMediaViewerTextColor(c.color)} title={c.label} />)}
                </div>
                <button className={styles.mediaFullscreenClose} onClick={() => setMediaViewerFullscreen(false)}>✕</button>
              </div>
            </div>
            <div className={styles.mediaFullscreenBody}>
              {mediaViewerMime.startsWith('video/') || mediaViewerMime.startsWith('audio/') ? <MediaPlayer src={mediaViewerFileUrl} mimeType={mediaViewerMime} fileName={mediaViewerEntry.name} className={mediaViewerMime.startsWith('video/') ? styles.mediaFullscreenVideo : styles.mediaFullscreenAudioWrap} /> : mediaViewerMime.startsWith('image/') ? <ImageViewer src={mediaViewerFileUrl} alt={mediaViewerEntry.name} className={styles.mediaFullscreenImage} /> : mediaViewerMime === 'application/pdf' ? <PdfViewer base64Data={`data:application/pdf;base64,${mediaViewerBase64}`} fileName={mediaViewerEntry.name} className={styles.mediaFullscreenPdf} /> : mediaViewerMime === 'table' && tableData ? <div className={styles.mediaFullscreenTable}>
                  <table className={styles.dataTable}>
                    <thead>
                      <tr>
                        <th className={styles.dataTableTh}>#</th>
                        {Array.from({
                      length: tableData.totalCols || 0
                    }).map((_, i) => <th key={i} className={styles.dataTableTh}>{excelColName(i)}</th>)}
                      </tr>
                    </thead>
                    <tbody>
                      {tableData.rows.map((row, ri) => <tr key={ri}>
                          <td className={styles.dataTableTdRowNum}>{ri + 1}</td>
                          {row.map((cell, ci) => <td key={ci} className={styles.dataTableTd} style={{
                      color: mediaViewerTextColor
                    }}>{cell}</td>)}
                        </tr>)}
                    </tbody>
                  </table>
                </div> : mediaViewerMime === 'pptx' && pptxSlideCount > 0 ? <div className={styles.pptxViewerFullscreen}>
                  <div className={styles.pptxNav}>
                    <button className={styles.pptxNavBtn} disabled={pptxCurrentSlide === 0} onClick={() => setPptxCurrentSlide(prev => Math.max(0, prev - 1))}>
                      {t("Knowledge.k230")}
                    </button>
                    <span className={styles.pptxCounter}>
                      {pptxCurrentSlide + 1} / {pptxSlideCount}
                    </span>
                    <button className={styles.pptxNavBtn} disabled={pptxCurrentSlide >= pptxSlideCount - 1} onClick={() => setPptxCurrentSlide(prev => Math.min(pptxSlideCount - 1, prev + 1))}>
                      {t("Knowledge.k231")}
                    </button>
                    <button className={styles.pptxNavBtn} onClick={() => setMediaViewerFullscreen(false)}>
                      {t("Knowledge.k290")}
                    </button>
                  </div>
                  <div className={styles.pptxSlideContentFullscreen} ref={pptxFullscreenSlideRef} />
                </div> : mediaViewerMime === 'text/plain' ? <pre className={styles.mediaFullscreenText} style={{
              color: mediaViewerTextColor
            }}>{mediaViewerTextContent}</pre> : mediaViewerMime === 'text/html' ? <div className={styles.mediaFullscreenHtml} dangerouslySetInnerHTML={{
              __html: sanitizeHtml(mediaViewerHtmlContent)
            }} /> : null}
            </div>
          </div>
        </div>}

      {showQuickSwitcher && <div className={styles.modalOverlay} onClick={() => setShowQuickSwitcher(false)}>
          <div className={styles.quickSwitcher} onClick={e => e.stopPropagation()}>
            <div className={styles.quickSwitcherInput}>
              <span className={styles.quickSwitcherIcon}>🔎</span>
              <input value={quickSwitcherQuery} onChange={e => {
              setQuickSwitcherQuery(e.target.value);
              setQuickSwitcherIndex(0);
            }} onKeyDown={handleQuickSwitcherKeyDown} placeholder={t("Knowledge.k291")} className={styles.quickSwitcherField} autoFocus />
            </div>
            <div className={styles.quickSwitcherList}>
              {quickSwitcherResults.length === 0 ? <div className={styles.quickSwitcherEmpty}>{t("Knowledge.k222")}</div> : quickSwitcherResults.map((entry, idx) => <div key={entry.id} className={`${styles.quickSwitcherItem} ${idx === quickSwitcherIndex ? styles.quickSwitcherItemActive : ''}`} onClick={() => handleQuickSelect(entry)} onMouseEnter={() => setQuickSwitcherIndex(idx)}>
                    <span className={styles.quickSwitcherItemIcon}>
                      {entry.entry_type === 'link' ? '🔗' : entry.is_favorited ? '⭐' : '📄'}
                    </span>
                    <span className={styles.quickSwitcherItemName}>{entry.name}</span>
                    <span className={styles.quickSwitcherItemHint}>
                      {categories.find(c => c.id === entry.category_id)?.name || t("knowledge.GraphView.k7")}
                    </span>
                  </div>)}
            </div>
            <div className={styles.quickSwitcherFooter}>
              <span>{t("Knowledge.k292")}</span><span>{t("Knowledge.k293")}</span><span>{t("Knowledge.k294")}</span>
            </div>
          </div>
        </div>}

      </main>
    </div>;
}