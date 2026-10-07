// knowledge.browse L2 功能域：目录树浏览 / 条目·目录 CRUD / 拖拽移动 / 批量操作 / 快捷切换器 / 预览区默认视图。
// 局部 state + handlers + JSX 物理落在本目录；核心共享数据经 KnowledgeCore 注入。
import { t } from "i18next";
import type { ReactNode } from 'react';
import { useCallback, useEffect, useState } from 'react';
import { ipc, intelligence } from '@/lib/ipc';
import { kb } from '../../ipc';
import styles from '../../Knowledge.module.css';
import { SkeletonList } from '@/components/ui/Skeleton';
import PdfViewer from '@/components/PdfViewer';
import CodePreview from '@/components/CodePreview';
import DeleteConfirmModal from '../../knowledge/DeleteConfirmModal';
import NameConflictModal from '../../knowledge/NameConflictModal';
import ContextMenu from '../../knowledge/ContextMenu';
import MoveTargetModal from '../../knowledge/MoveTargetModal';
import ViewModeList from '../../knowledge/ViewModeList';
import type { ConfirmDelete, KbCategory, KbEntry, KbTemplate, SortMode, TreeNode, ViewMode, KnowledgeCore } from '../../knowledge/types';

interface BrowseDeps {
  handleOpenEntry: (entry: KbEntry) => void;
  openFileViewer: (entry: KbEntry) => void;
  handleWikiLinkClick: (name: string) => void;
  mediaTextColor: string;
  mediaClose: () => void;
  editors: {
    fileEditMode: boolean;
    handleStopFileEdit: () => void;
    startMarkdownEdit: (content: string) => void;
    resetEditors: () => void;
  };
  searchIsSearching: boolean;
  resetSearch: () => void;
  tagsApi: { renderEntryTagRow: (entry: KbEntry) => ReactNode };
  historyApi: { renderBacklinks: () => ReactNode; renderSnapshots: (entry: KbEntry) => ReactNode };
  aiClassifyMenuItem?: (id: number, name: string) => void;
  isLoading: boolean;
  pendingTemplate: KbTemplate | null;
  setPendingTemplate: (v: KbTemplate | null) => void;
  setShowTemplateModal: (v: boolean) => void;
}

export function useBrowse(core: KnowledgeCore, deps: BrowseDeps) {
  const [displayMode, setDisplayMode] = useState<'list' | 'card' | 'timeline'>('list');
  const [editingId, setEditingId] = useState<{ type: 'category' | 'entry'; id: number } | null>(null);
  const [editName, setEditName] = useState('');
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; node: TreeNode } | null>(null);
  const [moveTarget, setMoveTarget] = useState<{ type: 'entry' | 'category'; id: number } | null>(null);
  const [dragOverId, setDragOverId] = useState<{ type: 'entry' | 'category'; id: number } | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [dragOtherCategories, setDragOtherCategories] = useState<KbCategory[]>([]);
  const [dragOverLibraryRoot, setDragOverLibraryRoot] = useState<string | null>(null);
  const [showNewFolder, setShowNewFolder] = useState(false);
  const [newFolderName, setNewFolderName] = useState('');
  const [newFolderParentId, setNewFolderParentId] = useState<number | null>(null);
  const [showNewEntry, setShowNewEntry] = useState(false);
  const [newEntry, setNewEntry] = useState({ name: '', entry_type: 'text', path_url: '' });
  const [confirmDelete, setConfirmDelete] = useState<ConfirmDelete | null>(null);
  const [selectedEntryIds, setSelectedEntryIds] = useState<Set<number>>(new Set());
  const [isMultiMode, setIsMultiMode] = useState(false);
  const [showBatchMove, setShowBatchMove] = useState(false);
  const [showBatchTagPicker, setShowBatchTagPicker] = useState(false);
  const [batchTagAction, setBatchTagAction] = useState<'add' | 'remove'>('add');
  const [showQuickSwitcher, setShowQuickSwitcher] = useState(false);
  const [quickSwitcherQuery, setQuickSwitcherQuery] = useState('');
  const [quickSwitcherIndex, setQuickSwitcherIndex] = useState(0);
  const [editContent, setEditContent] = useState('');
  const [previewLoading, setPreviewLoading] = useState(false);
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);
  const [highlightTokens, setHighlightTokens] = useState<Array<{ text: string; scope: string }>>([]);
  const [csvPreviewData, setCsvPreviewData] = useState<{ headers: string[]; rows: string[][] } | null>(null);
  const [pdfPreviewUrl, setPdfPreviewUrl] = useState<string | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);

  const getCategoryDepth = core.getCategoryDepth;
  const selectedId = core.selectedId;
  const selectedEntry = selectedId?.type === 'entry' ? core.allEntries.find(e => e.id === selectedId.id) ?? null : null;
  const selectedCategoryEntries = selectedId?.type === 'category' ? core.allEntries.filter(e => e.category_id === selectedId.id).sort((a, b) => b.updated_at - a.updated_at) : [];
  const selectedSubCategories = selectedId?.type === 'category' ? core.categories.filter(c => c.parent_id === selectedId.id).sort((a, b) => a.name.localeCompare(b.name)) : [];

  // 选中文本条目时同步编辑内容
  useEffect(() => {
    if (selectedId?.type === 'entry' && selectedEntry?.entry_type === 'text') {
      setEditContent(selectedEntry.content || '');
    }
  }, [selectedId, selectedEntry]);

  // 默认预览（非媒体/非编辑器）加载：缩略图 / PDF / CSV / 代码高亮
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
      rs: 'rust', ts: 'typescript', tsx: 'typescriptreact', js: 'javascript', jsx: 'javascriptreact', py: 'python', go: 'go', java: 'java', c: 'c', cpp: 'cpp', h: 'c', hpp: 'cpp', cs: 'csharp', rb: 'ruby', php: 'php', swift: 'swift', kt: 'kotlin', scala: 'scala', lua: 'lua', sh: 'bash', bash: 'bash', zsh: 'bash', yaml: 'yaml', yml: 'yaml', toml: 'toml', json: 'json', xml: 'xml', css: 'css', scss: 'scss', less: 'less', html: 'html', htm: 'html', sql: 'sql'
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
          const readRes = await kb.kbReadExternalFile({ path: entry.path_url });
          if (readRes.code === 0 && readRes.data) {
            fileContent = readRes.data.content;
          }
        } catch {/* 读不到就算了 */}
      }
      try {
        if (imageExts.includes(ext)) {
          if (isExternal) {
            const readRes = await kb.kbReadFileBase64({ path: entry.path_url });
            if (readRes.code === 0 && readRes.data?.base64) {
              setThumbnailUrl(`data:${readRes.data.mime_type};base64,${readRes.data.base64}`);
            } else {
              setPreviewError(t("Knowledge.k6"));
            }
          } else {
            const res = await ipc.invoke<any>('editor_generate_thumbnail', {
              request: { doc_uuid: `kb-entry-${entry.id}`, max_width: 400, max_height: 300 }
            });
            if (res.code === 0 && res.data?.base64_png) {
              setThumbnailUrl(`data:image/png;base64,${res.data.base64_png}`);
            }
          }
        } else if (ext === 'pdf') {
          if (isExternal) {
            const readRes = await kb.kbReadFileBase64({ path: entry.path_url });
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
            setCsvPreviewData({ headers, rows });
          }
        } else if (codeExts.includes(ext) && fileContent) {
          const lang = languageMap[ext] || ext;
          const res = await ipc.invoke<any>('editor_highlight', {
            request: { content: fileContent, language: lang }
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedEntry]);

  const buildTree = useCallback((): TreeNode[] => {
    const getChildrenOfCategory = (catId: number): TreeNode[] => {
      const childCats = core.categories.filter(c => c.parent_id === catId).sort((a, b) => a.sort_order - b.sort_order);
      const entriesInCat = core.allEntries.filter(e => e.category_id === catId && (core.typeFilter === 'all' || e.entry_type === core.typeFilter));
      switch (core.sortMode) {
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
        nodes.push({ type: 'category', id: cat.id, name: cat.name, categoryId: cat.id, depth: getCategoryDepth(cat.id), children: getChildrenOfCategory(cat.id) });
      }
      for (const e of entriesInCat) {
        nodes.push({ type: 'entry', id: e.id, name: e.name, categoryId: e.category_id, pathUrl: e.path_url, sourcePath: e.source_path || undefined, entryType: e.entry_type, updatedAt: e.updated_at, depth: getCategoryDepth(catId) + 1, children: [] as TreeNode[] });
      }
      return nodes;
    };
    const rootCats = core.categories.filter(c => !c.parent_id).sort((a, b) => a.sort_order - b.sort_order);
    return rootCats.map(cat => ({ type: 'category' as const, id: cat.id, name: cat.name, categoryId: cat.id, depth: 0, children: getChildrenOfCategory(cat.id) }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [core.categories, core.allEntries, core.sortMode, core.typeFilter]);

  const toggleExpand = (id: number) => {
    core.setExpandedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);else next.add(id);
      return next;
    });
  };

  const handleSelect = (type: 'category' | 'entry', id: number) => {
    core.setSelectedId({ type, id });
    deps.resetSearch();
    // 切换选择时，重置所有查看器/编辑状态，使预览区立即切换到新选中的条目
    deps.mediaClose();
    deps.editors.resetEditors();
  };

  const handleViewModeChange = (mode: ViewMode) => {
    core.setViewMode(mode);
    core.setViewFilterId(null);
  };

  const handleAddCategory = async () => {
    if (!newFolderName.trim()) return;
    try {
      const res = await kb.addKbCategory({
        name: newFolderName.trim(),
        parentId: newFolderParentId,
        library: core.currentLibrary,
        sortOrder: 0
      });
      if (res.code === 0) {
        core.showStatus('success', t("Knowledge.k9", { newFolderName: newFolderName }));
        setNewFolderName('');
        setShowNewFolder(false);
        core.reload();
      } else core.showStatus('error', res.message || t("Knowledge.k10"));
    } catch {
      core.showStatus('error', t("Knowledge.k11"));
    }
  };

  const handleDeleteCategory = async (id: number) => {
    const cat = core.categories.find(c => c.id === id);
    if (!cat) return;
    const subCount = core.categories.filter(c => c.parent_id === id).length;
    const entryCount = core.categoryCounts.get(id) || 0;
    setConfirmDelete({ type: 'category', id, name: cat.name, subCount: subCount + entryCount });
  };

  const confirmDeleteCategory = async () => {
    if (!confirmDelete) return;
    const id = confirmDelete.id;
    try {
      const res = await kb.moveKbCategoryToRecycle({ id });
      if (res.code === 0) {
        const sub = res.data?.deleted_subfolders || 0;
        const ent = res.data?.deleted_entries || 0;
        core.showStatus('success', t("Knowledge.k12", { sub: sub, ent: ent }));
        if (core.selectedId?.id === id) core.setSelectedId(null);
        core.reload();
      } else core.showStatus('error', res.message || t("Knowledge.k13"));
    } catch (e) {
      core.showStatus('error', t("Knowledge.k14", { e: e }));
    }
    setConfirmDelete(null);
  };

  const handleAddEntry = async () => {
    if (!newEntry.name.trim()) {
      core.showStatus('error', t("Knowledge.k15"));
      return;
    }
    const targetCategoryId = core.selectedId?.type === 'category' ? core.selectedId.id : core.categories[0]?.id;
    if (!targetCategoryId) {
      core.showStatus('error', t("Knowledge.k16"));
      return;
    }
    const finalName = core.findNameConflict(newEntry.name.trim(), targetCategoryId) ? core.resolveAutoRename(newEntry.name.trim(), targetCategoryId) : newEntry.name.trim();
    try {
      const res = await kb.addKbEntry({
        request: { category_id: targetCategoryId, name: finalName, path_url: newEntry.path_url.trim() || `kb://${finalName}`, entry_type: newEntry.entry_type }
      });
      if (res.code === 0) {
        const tmpl = deps.pendingTemplate;
        if (tmpl?.content && res.data) {
          await kb.updateKbEntry({ request: { id: res.data.id, content: tmpl.content } });
        }
        core.showStatus('success', t("Knowledge.k17", { finalName: finalName, arg0: finalName !== newEntry.name.trim() ? t("Knowledge.k18") : '' }));
        setNewEntry({ name: '', entry_type: 'text', path_url: '' });
        setShowNewEntry(false);
        deps.setPendingTemplate(null);
        core.reload();
        intelligence.logActivity('user', new Date().toISOString(), 'knowledge', 'add_entry', finalName).catch(() => {});
      } else {
        core.showStatus('error', res.message || t("components.PptEditor.k4"));
        deps.setPendingTemplate(null);
      }
    } catch {
      core.showStatus('error', t("Knowledge.k19"));
      deps.setPendingTemplate(null);
    }
  };

  const handleDeleteEntry = async (id: number) => {
    const entry = core.allEntries.find(e => e.id === id);
    if (!entry) return;
    setConfirmDelete({ type: 'entry', id, name: entry.name });
  };

  const confirmDeleteEntry = async () => {
    if (!confirmDelete || confirmDelete.type !== 'entry') return;
    const id = confirmDelete.id!;
    const entry = core.allEntries.find(e => e.id === id);
    try {
      if (entry && core.isExternalEntry(entry)) {
        const res = await kb.deleteKbEntry({ id });
        if (res.code === 0) {
          core.showStatus('success', t("Knowledge.k20"));
          if (core.selectedId?.id === id) core.setSelectedId(null);
          core.reload();
          intelligence.logActivity('user', new Date().toISOString(), 'knowledge', 'delete_entry', entry?.name).catch(() => {});
        } else core.showStatus('error', res.message || t("errors.deleteFailed"));
      } else {
        const success = await core.moveEntryToRecycle(id);
        if (success) {
          core.showStatus('success', t("Knowledge.k21"));
          if (core.selectedId?.id === id) core.setSelectedId(null);
          core.reload();
          intelligence.logActivity('user', new Date().toISOString(), 'knowledge', 'delete_entry', entry?.name || String(id)).catch(() => {});
        } else core.showStatus('error', t("Knowledge.k13"));
      }
    } catch {
      core.showStatus('error', t("errors.deleteFailed"));
    }
    setConfirmDelete(null);
  };

  const startRename = (type: 'category' | 'entry', id: number, name: string) => {
    setEditingId({ type, id });
    setEditName(name);
    setContextMenu(null);
  };

  const submitRename = async () => {
    if (!editingId || !editName.trim()) {
      setEditingId(null);
      return;
    }
    const { type, id } = editingId;
    try {
      if (type === 'category') {
        const res = await kb.updateKbCategory({ id, name: editName.trim() });
        if (res.code === 0) core.reload();else core.showStatus('error', res.message || t("Knowledge.k22"));
      } else {
        const res = await kb.updateKbEntry({ request: { id, name: editName.trim() } });
        if (res.code === 0) core.reload();else core.showStatus('error', res.message || t("Knowledge.k22"));
      }
    } catch {
      core.showStatus('error', t("Knowledge.k22"));
    }
    setEditingId(null);
    setEditName('');
  };

  const handleMoveEntry = async (entryId: number, targetCategoryId: number) => {
    const entry = core.allEntries.find(e => e.id === entryId);
    if (!entry) {
      setContextMenu(null);
      setMoveTarget(null);
      return;
    }
    if (entry.category_id === targetCategoryId) {
      core.showStatus('error', t("Knowledge.k23"));
      setContextMenu(null);
      setMoveTarget(null);
      return;
    }
    const existing = core.findNameConflict(entry.name, targetCategoryId);
    if (existing) {
      core.setNameConflict({
        newName: entry.name,
        existingEntry: existing,
        categoryId: targetCategoryId,
        onReplace: async () => {
          core.setNameConflict(null);
          try {
            if (core.isExternalEntry(existing)) {
              await kb.deleteKbEntry({ id: existing.id });
            } else {
              await core.moveEntryToRecycle(existing.id);
            }
            const res = await kb.moveKbEntry({ request: { id: entryId, target_category_id: targetCategoryId } });
            if (res.code === 0) {
              core.showStatus('success', t("Knowledge.k24", { name: entry.name }));
              core.reload();
            } else core.showStatus('error', res.message || t("Knowledge.k25"));
          } catch (e) {
            core.showStatus('error', t("Knowledge.k26", { e: e }));
          }
          setContextMenu(null);
          setMoveTarget(null);
        },
        onRename: async renamed => {
          core.setNameConflict(null);
          try {
            const _updateRes = await kb.updateKbEntry({ request: { id: entryId, name: renamed } });
            if (_updateRes.code === 0) {
              const res = await kb.moveKbEntry({ request: { id: entryId, target_category_id: targetCategoryId } });
              if (res.code === 0) {
                core.showStatus('success', t("Knowledge.k27", { renamed: renamed }));
                core.reload();
              } else core.showStatus('error', res.message || t("Knowledge.k25"));
            } else core.showStatus('error', _updateRes.message || t("Knowledge.k22"));
          } catch (e) {
            core.showStatus('error', t("Knowledge.k26", { e: e }));
          }
          setContextMenu(null);
          setMoveTarget(null);
        }
      });
      return;
    }
    try {
      const res = await kb.moveKbEntry({ request: { id: entryId, target_category_id: targetCategoryId } });
      if (res.code === 0) {
        core.showStatus('success', t("Knowledge.k28"));
        core.reload();
      } else core.showStatus('error', res.message || t("Knowledge.k25"));
    } catch {
      core.showStatus('error', t("Knowledge.k25"));
    }
    setContextMenu(null);
    setMoveTarget(null);
  };

  const handleMoveCategory = async (categoryId: number, targetParentId: number | null, targetLibrary?: string) => {
    setContextMenu(null);
    setMoveTarget(null);
    try {
      const res = await kb.moveKbCategory({ request: { id: categoryId, target_parent_id: targetParentId, target_library: targetLibrary ?? null } });
      if (res.code === 0) {
        core.showStatus('success', t("Knowledge.k29"));
        core.reload();
      } else core.showStatus('error', res.message || t("Knowledge.k25"));
    } catch {
      core.showStatus('error', t("Knowledge.k30"));
    }
  };

  const handleRightClick = (e: React.MouseEvent, node: TreeNode) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, node });
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
    setConfirmDelete({ type: 'batch', batchCount: selectedEntryIds.size });
  };

  const confirmBatchDelete = async () => {
    if (!confirmDelete || confirmDelete.type !== 'batch') return;
    try {
      const ids = Array.from(selectedEntryIds);
      const entries = ids.map(id => core.allEntries.find(e => e.id === id)).filter(Boolean) as KbEntry[];
      const externalIds = entries.filter(core.isExternalEntry).map(e => e.id);
      const internalIds = entries.filter(e => !core.isExternalEntry(e)).map(e => e.id);
      let deletedCount = 0;
      let recycledCount = 0;
      if (externalIds.length > 0) {
        const res = await kb.batchDeleteKbEntries({ request: { ids: externalIds } });
        if (res.code === 0) deletedCount = externalIds.length;
      }
      for (const eid of internalIds) {
        const ok = await core.moveEntryToRecycle(eid);
        if (ok) recycledCount++;
      }
      const parts: string[] = [];
      if (deletedCount > 0) parts.push(t("Knowledge.k42", { deletedCount: deletedCount }));
      if (recycledCount > 0) parts.push(t("Knowledge.k43", { recycledCount: recycledCount }));
      if (parts.length > 0) core.showStatus('success', parts.join('，'));else core.showStatus('error', t("common.failed"));
      setSelectedEntryIds(new Set());
      core.reload();
    } catch {
      core.showStatus('error', t("Knowledge.k44"));
    }
    setConfirmDelete(null);
  };

  const handleBatchMove = async (targetCategoryId: number) => {
    if (selectedEntryIds.size === 0) return;
    try {
      const ids = Array.from(selectedEntryIds);
      const conflicts: { entryId: number; name: string; existingId: number }[] = [];
      for (const eid of ids) {
        const entry = core.allEntries.find(e => e.id === eid);
        if (!entry) continue;
        const existing = core.findNameConflict(entry.name, targetCategoryId);
        if (existing && !ids.includes(existing.id)) {
          conflicts.push({ entryId: eid, name: entry.name, existingId: existing.id });
        }
      }
      if (conflicts.length > 0) {
        const firstConflictEntry = core.allEntries.find(e => e.id === conflicts[0].existingId);
        if (!firstConflictEntry) {
          core.showStatus('error', t("Knowledge.k45"));
          setShowBatchMove(false);
          return;
        }
        core.setNameConflict({
          newName: t("Knowledge.k46", { length: conflicts.length }),
          existingEntry: firstConflictEntry,
          categoryId: targetCategoryId,
          onReplace: async () => {
            core.setNameConflict(null);
            try {
              for (const c of conflicts) {
                const existEntry = core.allEntries.find(e => e.id === c.existingId);
                if (existEntry && core.isExternalEntry(existEntry)) {
                  await kb.deleteKbEntry({ id: c.existingId });
                } else {
                  await core.moveEntryToRecycle(c.existingId);
                }
              }
              const res = await kb.batchMoveKbEntries({ request: { ids, category_id: targetCategoryId } });
              if (res.code === 0) {
                core.showStatus('success', t("Knowledge.k47", { length: ids.length, arg0: conflicts.length }));
                setSelectedEntryIds(new Set());
                setShowBatchMove(false);
                core.reload();
              } else core.showStatus('error', res.message || t("Knowledge.k25"));
            } catch (e) {
              core.showStatus('error', t("Knowledge.k48", { e: e }));
            }
            setShowBatchMove(false);
          },
          onRename: async () => {
            core.setNameConflict(null);
            try {
              const res = await kb.batchMoveKbEntries({ request: { ids, category_id: targetCategoryId } });
              if (res.code === 0) {
                let renamedCount = 0;
                for (const c of conflicts) {
                  const renamed = core.resolveAutoRename(c.name, targetCategoryId);
                  try {
                    await kb.updateKbEntry({ request: { id: c.entryId, name: renamed } });
                    renamedCount++;
                  } catch {/* skip */}
                }
                await core.reload();
                core.showStatus('success', t("Knowledge.k49", { length: ids.length, renamedCount: renamedCount }));
                setSelectedEntryIds(new Set());
                setShowBatchMove(false);
              } else core.showStatus('error', res.message || t("Knowledge.k25"));
            } catch (e) {
              core.showStatus('error', t("Knowledge.k48", { e: e }));
            }
            setShowBatchMove(false);
          }
        });
        return;
      }
      const res = await kb.batchMoveKbEntries({ request: { ids, category_id: targetCategoryId } });
      if (res.code === 0) {
        core.showStatus('success', t("Knowledge.k50", { length: ids.length }));
        setSelectedEntryIds(new Set());
        setShowBatchMove(false);
        core.reload();
      } else core.showStatus('error', res.message || t("Knowledge.k25"));
    } catch {
      core.showStatus('error', t("Knowledge.k51"));
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
      const res = await kb.batchAddKbTag({ entryIds: ids, tagId: tagId });
      if (res.code === 0) {
        core.showStatus('success', t("Knowledge.k52", { data: res.data }));
        await core.loadAllEntryTags();
        await core.loadTagStats();
        setShowBatchTagPicker(false);
      } else core.showStatus('error', res.message || t("Knowledge.k53"));
    } catch {
      core.showStatus('error', t("Knowledge.k54"));
    }
  };

  const handleBatchRemoveTag = async (tagId: number) => {
    if (selectedEntryIds.size === 0) return;
    try {
      const ids = Array.from(selectedEntryIds);
      const res = await kb.batchRemoveKbTag({ entryIds: ids, tagId: tagId });
      if (res.code === 0) {
        core.showStatus('success', t("Knowledge.k55", { data: res.data }));
        await core.loadAllEntryTags();
        await core.loadTagStats();
        setShowBatchTagPicker(false);
      } else core.showStatus('error', res.message || t("Knowledge.k56"));
    } catch {
      core.showStatus('error', t("Knowledge.k57"));
    }
  };

  const handleInsertTemplate = (template: KbTemplate) => {
    deps.setShowTemplateModal(false);
    deps.setPendingTemplate(template);
    setNewEntry({ name: template.name, entry_type: template.entry_type, path_url: '' });
    setShowNewEntry(true);
  };

  const cycleSort = () => {
    const modes: SortMode[] = ['time_desc', 'time_asc', 'name_asc', 'name_desc'];
    const idx = modes.indexOf(core.sortMode);
    const next = modes[(idx + 1) % modes.length];
    core.setSortMode(next);
    localStorage.setItem('kb_sort_mode', next);
  };

  const sortLabels: Record<SortMode, string> = {
    time_desc: t("Knowledge.k147"),
    time_asc: t("Knowledge.k148"),
    name_asc: t("Knowledge.k149"),
    name_desc: t("Knowledge.k150")
  };

  // 拖拽全局清理
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

  const tree = buildTree();

  const renderNode = (node: TreeNode): ReactNode => {
    const isExpanded = node.type === 'category' && core.expandedIds.has(node.id);
    const isSelected = core.selectedId?.id === node.id && core.selectedId?.type === node.type;
    const hasChildren = node.children.length > 0;
    const isEditing = editingId?.id === node.id && editingId?.type === node.type;
    const entryCount = node.type === 'category' ? core.categoryCounts.get(node.id) || 0 : 0;
    const isEntryMissing = node.type === 'entry' && core.missingFiles.has(node.id);
    return <div key={`${node.type}-${node.id}`}>
        <div className={`${styles.treeItem} ${isSelected ? styles.treeItemSelected : ''} ${dragOverId?.type === node.type && dragOverId?.id === node.id ? styles.treeItemDragOver : ''}`} style={{ paddingLeft: `${8 + node.depth * 16}px` }} draggable={!isEditing} onClick={() => handleSelect(node.type, node.id)} onDoubleClick={() => {
          if (node.type === 'entry' && selectedEntry) deps.handleOpenEntry(selectedEntry);else if (node.type === 'category' || node.type === 'entry') startRename(node.type, node.id, node.name);
        }} onContextMenu={e => handleRightClick(e, node)} onDragStart={e => {
          e.dataTransfer.setData('text/plain', JSON.stringify({ type: node.type, id: node.id, name: node.name }));
          e.dataTransfer.effectAllowed = 'move';
          setIsDragging(true);
          // 加载另一库的分类，以便跨库拖放
          const otherLib = core.currentLibrary === 'material' ? 'study' : 'material';
          kb.getKbCategories({ library: otherLib }).then(res => {
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
          setDragOverId({ type: node.type, id: node.id });
        }} onDragLeave={() => setDragOverId(null)} onDrop={e => {
          e.preventDefault();
          setDragOverId(null);
          const data = JSON.parse(e.dataTransfer.getData('text/plain'));
          if (data.type === 'entry') {
            const targetCatId = node.type === 'category' ? node.id : core.allEntries.find(en => en.id === node.id)?.category_id ?? node.categoryId ?? 0;
            if (data.id !== node.id) {
              handleMoveEntry(data.id, targetCatId);
            }
          } else if (data.type === 'category' && node.type === 'category') {
            if (data.id !== node.id && !core.getAllDescendantIds(core.categories, data.id).includes(node.id)) {
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
            {node.type === 'category' ? isExpanded ? '📂' : '📁' : core.getTypeIcon(node.entryType)}
          </span>
          {isEditing ? <input className={styles.inlineEdit} value={editName} onChange={e => setEditName(e.target.value)} onBlur={submitRename} onKeyDown={e => {
            if (e.key === 'Enter') submitRename();
            if (e.key === 'Escape') setEditingId(null);
          }} autoFocus onClick={e => e.stopPropagation()} /> : <span className={styles.treeLabel} style={isEntryMissing ? { color: '#FF4444', fontWeight: 'bold', textDecoration: 'underline' } : undefined}>
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

  const quickSwitcherResults = quickSwitcherQuery.trim() === '' ? core.allEntries.slice(0, 20) : core.allEntries.filter(e => e.name.toLowerCase().includes(quickSwitcherQuery.toLowerCase())).slice(0, 20);
  const handleQuickSelect = (entry: KbEntry) => {
    setShowQuickSwitcher(false);
    setQuickSwitcherQuery('');
    deps.handleOpenEntry(entry);
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

  /** 侧边栏工具栏：新建/排序按钮 */
  const renderSidebarButtons = () => (
    <>
      <button className={styles.toolIconBtn} onClick={() => setShowNewEntry(!showNewEntry)} title={t("Knowledge.k151")}>
        ✏️
      </button>
      <button className={styles.toolIconBtn} onClick={() => setShowNewFolder(!showNewFolder)} title={t("Knowledge.k152")}>
        📁
      </button>
      <button className={styles.toolIconBtn} onClick={cycleSort} title={t("Knowledge.k153", { sortMode: sortLabels[core.sortMode] })}>
        ↕️
      </button>
    </>
  );

  /** 侧边栏新建文件夹表单 */
  const renderNewFolderForm = () => {
    if (!showNewFolder) return null;
    return (
      <div className={styles.quickForm}>
        <input value={newFolderName} onChange={e => setNewFolderName(e.target.value)} placeholder={t("Knowledge.k169")} className={styles.inputSmall} onKeyDown={e => e.key === 'Enter' && handleAddCategory()} autoFocus />
        <select value={newFolderParentId ?? ''} onChange={e => setNewFolderParentId(e.target.value ? Number(e.target.value) : null)} className={styles.inputSmall}>
          <option value="">{t("Knowledge.k170")}</option>
          {core.categories.map(c => <option key={c.id} value={c.id}>{'　'.repeat(getCategoryDepth(c.id))}{c.name}</option>)}
        </select>
        <button onClick={handleAddCategory} className={styles.btnSm}>{t("common.confirm")}</button>
      </div>
    );
  };

  /** 侧边栏视图标签页 */
  const renderViewTabs = () => (
    <div className={styles.viewTabs}>
      <button className={`${styles.viewTab} ${core.viewMode === 'all' ? styles.viewTabActive : ''}`} onClick={() => handleViewModeChange('all')}>
        {t("Knowledge.k171")}
      </button>
      <button className={`${styles.viewTab} ${core.viewMode === 'favorite' ? styles.viewTabActive : ''}`} onClick={() => handleViewModeChange('favorite')}>
        {t("Knowledge.k172")}
      </button>
      <button className={`${styles.viewTab} ${core.viewMode === 'recent' ? styles.viewTabActive : ''}`} onClick={() => handleViewModeChange('recent')}>
        {t("Knowledge.k173")}
      </button>
      <button className={`${styles.viewTabMini} ${isMultiMode ? styles.viewTabMiniActive : ''}`} onClick={toggleMultiMode} title={t("Knowledge.k174")}>
        ☑
      </button>
    </div>
  );

  /** 侧边栏目录树 */
  const renderFileTree = () => (
    <nav className={styles.fileTree}>
      {deps.isLoading ? <div style={{ padding: '12px' }}>
          <SkeletonList count={8} />
        </div> : core.viewMode === 'all' ? <>
          {/* 拖放时显示跨库移动目标区域 */}
          {isDragging && dragOtherCategories.length > 0 && <div className={styles.crossLibSection}>
              <div className={styles.crossLibHeader}>
                {core.currentLibrary === 'material' ? t("Knowledge.k175") : t("Knowledge.k176")}
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
                const otherLib = core.currentLibrary === 'material' ? 'study' : 'material';
                if (data.type === 'entry') {
                  // 移动到另一库的根目录（无父分类）
                  const rootCat = dragOtherCategories.find(c => !c.parent_id);
                  if (rootCat) {
                    handleMoveEntry(data.id, rootCat.id);
                  } else {
                    core.showStatus('error', t("Knowledge.k177"));
                  }
                } else if (data.type === 'category') {
                  handleMoveCategory(data.id, null, otherLib);
                }
              }}>
                {t("Knowledge.k178")}
              </div>
              {/* 另一库的根分类 */}
              {dragOtherCategories.filter(c => !c.parent_id).map(otherCat => <div key={`other-${otherCat.id}`} className={`${styles.treeItem} ${dragOverId?.type === 'category' && dragOverId?.id === otherCat.id ? styles.treeItemDragOver : ''}`} style={{ paddingLeft: '24px' }} onDragOver={e => {
                e.preventDefault();
                e.dataTransfer.dropEffect = 'move';
                setDragOverId({ type: 'category', id: otherCat.id });
              }} onDragLeave={() => setDragOverId(null)} onDrop={e => {
                e.preventDefault();
                setDragOverId(null);
                const data = JSON.parse(e.dataTransfer.getData('text/plain'));
                const otherLib = core.currentLibrary === 'material' ? 'study' : 'material';
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
        </> : <ViewModeList mode={core.viewMode} filterId={core.viewFilterId} sortMode={core.sortMode} typeFilter={core.typeFilter} renderNode={renderNode} />}
    </nav>
  );

  /** 侧边栏批量操作条 */
  const renderBatchBar = () => {
    if (!isMultiMode || selectedEntryIds.size === 0) return null;
    return (
      <div className={styles.batchBar}>
        <span className={styles.batchInfo}>{t("ai.ChatPanel.k18")} {selectedEntryIds.size} {t("Knowledge.k179")}</span>
        <button className={styles.btnSm} onClick={() => setShowBatchMove(true)}>{t("Knowledge.k180")}</button>
        <button className={styles.btnSm} onClick={() => { setBatchTagAction('add'); setShowBatchTagPicker(true); }}>{t("Knowledge.k181")}</button>
        <button className={styles.btnSm} onClick={() => { setBatchTagAction('remove'); setShowBatchTagPicker(true); }}>{t("Knowledge.k182")}</button>
        <button className={styles.btnDel} onClick={handleBatchDelete}>{t("Knowledge.k183")}</button>
        <button className={styles.btnSm} onClick={() => setSelectedEntryIds(new Set())}>{t("common.deselect")}</button>
      </div>
    );
  };

  /** 批量移动 / 批量标签 Modal */
  const renderBatchModals = () => (
    <>
      {showBatchMove && <div className={styles.modalOverlay} onClick={() => setShowBatchMove(false)}>
          <div className={styles.modal} onClick={e => e.stopPropagation()}>
            <h3 className={styles.modalTitle}>{t("Knowledge.k184")}</h3>
            <div className={styles.moveList}>
              {core.categories.map(cat => <button key={cat.id} className={styles.moveItem} onClick={() => handleBatchMove(cat.id)}>
                  {'　'.repeat(getCategoryDepth(cat.id))}📁 {cat.name}
                </button>)}
            </div>
            <div className={styles.modalActions}>
              <button className={styles.btnSm} onClick={() => setShowBatchMove(false)}>{t("common.cancel")}</button>
            </div>
          </div>
        </div>}

      {showBatchTagPicker && <div className={styles.modalOverlay} onClick={() => setShowBatchTagPicker(false)}>
          <div className={styles.modal} onClick={e => e.stopPropagation()} style={{ maxWidth: 320 }}>
            <h3 className={styles.modalTitle}>{batchTagAction === 'add' ? t("Knowledge.k185") : t("Knowledge.k186")}（{selectedEntryIds.size} {t("Knowledge.k187")}</h3>
            <div className={styles.moveList}>
              {core.tags.length === 0 ? <div style={{ padding: 12, color: '#666', fontSize: 12 }}>{t("Knowledge.k188")}</div> : core.tags.map(tg => <button key={tg.id} className={styles.moveItem} style={{ display: 'flex', alignItems: 'center', gap: 6 }} onClick={() => batchTagAction === 'add' ? handleBatchAddTag(tg.id) : handleBatchRemoveTag(tg.id)}>
                  <span className={styles.tagColorDot} style={{ background: tg.color }} />
                  {tg.name}
                </button>)}
            </div>
            <div className={styles.modalActions}>
              <button className={styles.btnSm} onClick={() => setShowBatchTagPicker(false)}>{t("common.cancel")}</button>
            </div>
          </div>
        </div>}
    </>
  );

  /** 主内容区工具栏：视图切换（列表/卡片/时间线） */
  const renderDisplayToggle = (visible: boolean) => {
    if (!visible) return null;
    return (
      <div className={styles.viewToggle}>
        <button className={`${styles.viewToggleBtn} ${displayMode === 'list' ? styles.viewToggleBtnActive : ''}`} onClick={() => setDisplayMode('list')} title={t("Knowledge.k201")}>☰</button>
        <button className={`${styles.viewToggleBtn} ${displayMode === 'card' ? styles.viewToggleBtnActive : ''}`} onClick={() => setDisplayMode('card')} title={t("Knowledge.k202")}>▦</button>
        <button className={`${styles.viewToggleBtn} ${displayMode === 'timeline' ? styles.viewToggleBtnActive : ''}`} onClick={() => setDisplayMode('timeline')} title={t("Knowledge.k203")}>⏱</button>
      </div>
    );
  };

  /** 新建条目表单 */
  const renderNewEntryForm = () => {
    if (!showNewEntry) return null;
    const pendingTemplate = deps.pendingTemplate;
    return (
      <section className={styles.formPanel}>
        {pendingTemplate && <div className={styles.formRow} style={{ marginBottom: 6 }}>
            <span style={{ fontSize: 11, color: '#FFD700' }}>{t("Knowledge.k204")}{pendingTemplate.icon} {pendingTemplate.name}</span>
          </div>}
        <div className={styles.formRow}>
          <input value={newEntry.name} onChange={e => setNewEntry(p => ({ ...p, name: e.target.value }))} placeholder={t("Knowledge.k205")} className={styles.input} autoFocus />
          <select value={newEntry.entry_type} onChange={e => setNewEntry(p => ({ ...p, entry_type: e.target.value }))} className={styles.input}>
            <option value="text">{t("Knowledge.k206")}</option>
            <option value="link">{t("components.TextEditor.k9")}</option>
            <option value="file">{t("Knowledge.k207")}</option>
          </select>
        </div>
        <div className={styles.formRow}>
          <input value={newEntry.path_url} onChange={e => setNewEntry(p => ({ ...p, path_url: e.target.value }))} placeholder={newEntry.entry_type === 'link' ? t("Knowledge.k208") : t("Knowledge.k209")} className={styles.input} onKeyDown={e => e.key === 'Enter' && handleAddEntry()} />
          <button onClick={handleAddEntry} className={styles.btnPrimary}>{deps.pendingTemplate ? t("Knowledge.k210") : t("Knowledge.k211")}</button>
        </div>
      </section>
    );
  };

  /** 主内容区：分类文件夹视图 */
  const renderFolderContent = () => {
    if (selectedId?.type !== 'category') return null;
    return (
      <div className={styles.folderContentView}>
        <div className={styles.folderContentHeader}>
          <span className={styles.folderContentIcon}>📂</span>
          <span className={styles.folderContentName}>{core.categories.find(c => c.id === selectedId.id)?.name || t("knowledge.GraphView.k8")}</span>
          <span className={styles.folderContentCount}>{selectedSubCategories.length} {t("Knowledge.k269")} {selectedCategoryEntries.length} {t("knowledge.GraphView.k1")}</span>
        </div>
        {selectedSubCategories.length === 0 && selectedCategoryEntries.length === 0 ? <div className={styles.folderEmpty}>
            <span>{t("Knowledge.k270")}</span>
            <button className={styles.btnPrimary} onClick={handleAddEntry} style={{ marginTop: 12, fontSize: 12, padding: '6px 16px' }}>{t("Knowledge.k211")}</button>
          </div> : displayMode === 'card' ? <div className={styles.cardGrid}>
            {/* 子文件夹 */}
            {selectedSubCategories.map(subCat => <div key={`cat-${subCat.id}`} className={styles.cardItem} onClick={() => core.setSelectedId({ type: 'category', id: subCat.id })}>
                <div className={styles.cardIcon}>📁</div>
                <div className={styles.cardBody}>
                  <div className={styles.cardName}>{subCat.name}</div>
                  <div className={styles.cardMeta}>{t("Knowledge.k271")} {core.allEntries.filter(e => e.category_id === subCat.id).length} {t("Knowledge.k179")}</div>
                </div>
              </div>)}
            {/* 文件 */}
            {selectedCategoryEntries.map(entry => <div key={entry.id} className={`${styles.cardItem}${core.missingFiles.has(entry.id) ? ` ${styles.entryMissing}` : ''}`} onClick={() => deps.handleOpenEntry(entry)} onDoubleClick={() => deps.openFileViewer(entry)}>
                <div className={styles.cardIcon}>{core.getTypeIcon(entry.entry_type)}</div>
                <div className={styles.cardBody}>
                  <div className={styles.cardName}>{entry.name}</div>
                  <div className={styles.cardMeta}>
                    {core.getTypeLabel(entry.entry_type)} · {core.formatTime(entry.updated_at)}
                    {(entry as any).is_favorited === 1 && ' ⭐'}{core.pinnedEntries.has(entry.id) && ' 📶'}
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
                      {selectedSubCategories.map(subCat => <div key={`cat-${subCat.id}`} className={styles.timelineItem} onClick={() => core.setSelectedId({ type: 'category', id: subCat.id })}>
                          <div className={styles.timelineDot} style={{ background: '#00F0FF' }} />
                          <div className={styles.timelineContent}>
                            <div className={styles.timelineItemName}>📁 {subCat.name}</div>
                            <div className={styles.timelineItemMeta}>{t("Knowledge.k271")} {core.allEntries.filter(e => e.category_id === subCat.id).length} {t("Knowledge.k179")}</div>
                          </div>
                        </div>)}
                    </div>
                  </div>}
                {/* 文件按日期分组 */}
                {Object.entries(grouped).map(([date, entries]) => <div key={date} className={styles.timelineGroup}>
                    <div className={styles.timelineDate}>{date}</div>
                    <div className={styles.timelineItems}>
                      {entries.map(entry => <div key={entry.id} className={`${styles.timelineItem}${core.missingFiles.has(entry.id) ? ` ${styles.entryMissing}` : ''}`} onClick={() => deps.handleOpenEntry(entry)} onDoubleClick={() => deps.openFileViewer(entry)}>
                          <div className={styles.timelineDot} />
                          <div className={styles.timelineContent}>
                            <div className={styles.timelineItemName}>{core.getTypeIcon(entry.entry_type)} {entry.name}</div>
                            <div className={styles.timelineItemMeta}>
                              {core.getTypeLabel(entry.entry_type)} · {core.formatTime(entry.created_at)}
                              {(entry as any).is_favorited === 1 && ' ⭐'}{core.pinnedEntries.has(entry.id) && ' 📶'}
                            </div>
                          </div>
                        </div>)}
                    </div>
                  </div>)}
              </div>;
          })() : <div className={styles.folderEntryList}>
            {/* 子文件夹 */}
            {selectedSubCategories.map(subCat => <div key={`cat-${subCat.id}`} className={styles.folderEntryItem} onClick={() => core.setSelectedId({ type: 'category', id: subCat.id })}>
                <span className={styles.folderEntryIcon}>📁</span>
                <div className={styles.folderEntryInfo}>
                  <span className={styles.folderEntryName}>{subCat.name}</span>
                  <span className={styles.folderEntryMeta}>{t("Knowledge.k271")} {core.allEntries.filter(e => e.category_id === subCat.id).length} {t("Knowledge.k179")}</span>
                </div>
              </div>)}
            {/* 文件 */}
            {selectedCategoryEntries.map(entry => {
              const isMissing = core.missingFiles.has(entry.id);
              return <div key={entry.id} className={`${styles.folderEntryItem}${isMissing ? ` ${styles.entryMissing}` : ''}`} onClick={() => deps.handleOpenEntry(entry)} onDoubleClick={() => deps.openFileViewer(entry)}>
                <span className={styles.folderEntryIcon}>{core.getTypeIcon(entry.entry_type)}</span>
                <div className={styles.folderEntryInfo}>
                  <span className={styles.folderEntryName} style={isMissing ? { color: '#FF4444', fontWeight: 'bold', textDecoration: 'underline' } : undefined}>{entry.name}</span>
                  <span className={styles.folderEntryMeta}>
                    {core.getTypeLabel(entry.entry_type)} · {core.formatTime(entry.updated_at)}
                    {(entry as any).is_favorited === 1 && ' ⭐'}{core.pinnedEntries.has(entry.id) && ' 📶'}
                  </span>
                </div>
                <div className={styles.folderEntryActions}>
                  <button className={`${styles.btnSm} ${(entry as any).is_favorited === 1 ? styles.btnFavoriteActive : ''}`} onClick={e => {
                    e.stopPropagation();
                    core.handleToggleFavorite(entry.id);
                  }} title={t("components.FloatingBall.k57")}>⭐</button>
                  {core.isPinnable(entry) && <button className={`${styles.btnSm} ${core.pinnedEntries.has(entry.id) ? styles.btnFavoriteActive : ''}`} onClick={e => core.handleTogglePin(entry, e)} title={core.pinnedEntries.has(entry.id) ? t("Knowledge.k298") : t("Knowledge.k299")}>{core.pinnedEntries.has(entry.id) ? '📌' : '📍'}</button>}
                  <button className={styles.btnDel} onClick={e => {
                    e.stopPropagation();
                    handleDeleteEntry(entry.id);
                  }} title={t("common.delete")}>✕</button>
                </div>
              </div>;
            })}
          </div>}
      </div>
    );
  };

  /** 主内容区：全局（未选中）卡片/时间线/空态 */
  const renderGlobalContent = () => (
    displayMode === 'card' ? <div className={styles.cardView}>
        <div className={styles.cardViewHeader}>
          <span className={styles.cardViewTitle}>{t("Knowledge.k273")}</span>
          <span className={styles.cardViewCount}>{core.allEntries.filter(e => core.typeFilter === 'all' || e.entry_type === core.typeFilter).length} {t("Knowledge.k179")}</span>
        </div>
        {core.allEntries.filter(e => core.typeFilter === 'all' || e.entry_type === core.typeFilter).length === 0 ? <div className={styles.emptyTip}>{t("Knowledge.k274")}</div> : <div className={styles.cardGrid}>
            {core.allEntries.filter(e => core.typeFilter === 'all' || e.entry_type === core.typeFilter).map(entry => <div key={entry.id} className={`${styles.cardItem}${core.missingFiles.has(entry.id) ? ` ${styles.entryMissing}` : ''}`} onClick={() => deps.handleOpenEntry(entry)} onDoubleClick={() => deps.openFileViewer(entry)}>
                <div className={styles.cardIcon}>{core.getTypeIcon(entry.entry_type)}</div>
                <div className={styles.cardBody}>
                  <div className={styles.cardName}>{entry.name}</div>
                  <div className={styles.cardMeta}>
                    {core.getTypeLabel(entry.entry_type)} · {core.formatTime(entry.updated_at)}
                    {(entry as any).is_favorited === 1 && ' ⭐'}{core.pinnedEntries.has(entry.id) && ' 📶'}
                  </div>
                  {entry.source_path && <div className={styles.cardSource}>📌 {entry.source_path}</div>}
                </div>
              </div>)}
          </div>}
      </div> : displayMode === 'timeline' ? <div className={styles.timelineView}>
        <div className={styles.timelineHeader}>
          <span className={styles.timelineTitle}>{t("Knowledge.k275")}</span>
          <span className={styles.timelineCount}>{core.allEntries.filter(e => core.typeFilter === 'all' || e.entry_type === core.typeFilter).length} {t("Knowledge.k179")}</span>
        </div>
        {(() => {
          const filtered = core.allEntries.filter(e => core.typeFilter === 'all' || e.entry_type === core.typeFilter);
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
                    {entries.map(entry => <div key={entry.id} className={`${styles.timelineItem}${core.missingFiles.has(entry.id) ? ` ${styles.entryMissing}` : ''}`} onClick={() => deps.handleOpenEntry(entry)} onDoubleClick={() => deps.openFileViewer(entry)}>
                        <div className={styles.timelineDot} />
                        <div className={styles.timelineContent}>
                          <div className={styles.timelineItemName}>{core.getTypeIcon(entry.entry_type)} {entry.name}</div>
                          <div className={styles.timelineItemMeta}>
                            {core.getTypeLabel(entry.entry_type)} · {core.formatTime(entry.created_at)}
                            {(entry as any).is_favorited === 1 && ' ⭐'}{core.pinnedEntries.has(entry.id) && ' 📶'}
                          </div>
                        </div>
                      </div>)}
                  </div>
                </div>)}
            </div>;
        })()}
      </div> : <div className={styles.emptyTip}>
        <div style={{ fontSize: 36, marginBottom: 16 }}>📚</div>
        <strong style={{ fontSize: 15, color: '#aaa' }}>{t("components.intelligence.ActivityPanel.k1")}</strong><br /><br />
        <span style={{ opacity: 0.4, lineHeight: 2 }}>
          {t("Knowledge.k276")}<br /><br />
          {t("Knowledge.k277")}<br />
          {t("Knowledge.k278")}<br />
          {t("Knowledge.k279")}<br />
          {t("Knowledge.k280")}
        </span>
      </div>
  );

  /** 预览区默认视图（媒体/编辑器之外的条目详情） */
  const renderPreviewDefault = (entry: KbEntry) => (
    <>
      {core.missingFiles.has(entry.id) && <div style={{ margin: '12px 16px', padding: '12px 16px', borderRadius: '8px', background: 'rgba(244,67,54,0.12)', border: '1.5px solid #F44336', color: '#FF5252', fontSize: '13.5px', lineHeight: '1.6' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px', fontWeight: 600, marginBottom: '4px' }}>
            <span style={{ fontSize: '16px' }}>⚠️</span>
            {t("Knowledge.k233")}
          </div>
          <div style={{ opacity: 0.85, fontSize: '12.5px' }}>
            {t("Knowledge.k234")}{entry.source_path || entry.path_url}
          </div>
        </div>}
      <div className={styles.previewHeader}>
        <span className={styles.previewIcon}>{core.getTypeIcon(entry.entry_type)}</span>
        <div className={styles.previewMeta}>
          <h4 className={styles.previewTitle}>{entry.name}</h4>
          <div className={styles.previewInfo}>
            <span className={styles.tag}>{core.getTypeLabel(entry.entry_type)}</span>
            <span>{t("Knowledge.k235")} {core.formatTime(entry.updated_at)}</span>
            <span>{t("Knowledge.k236")} {core.formatTime(entry.created_at)}</span>
          </div>
          {deps.historyApi.renderBacklinks()}
          {deps.historyApi.renderSnapshots(entry)}
        </div>
        <div className={styles.previewActions}>
          {!core.missingFiles.has(entry.id) && <button className={styles.btnOpen} onClick={() => deps.handleOpenEntry(entry)}>
            {t("common.open")}{entry.entry_type === 'link' ? t("components.TextEditor.k9") : t("knowledge.GraphView.k1")}
          </button>}
          <button className={`${styles.btnSm} ${(entry as any).is_favorited === 1 ? styles.btnFavoriteActive : ''}`} onClick={() => core.handleToggleFavorite(entry.id)} title={t("Knowledge.k247")}>
            ⭐
          </button>
          {core.isPinnable(entry) && <button className={`${styles.btnSm} ${core.pinnedEntries.has(entry.id) ? styles.btnFavoriteActive : ''}`} onClick={e => core.handleTogglePin(entry, e)} title={core.pinnedEntries.has(entry.id) ? t("Knowledge.k258") : t("Knowledge.k259")}>
            {core.pinnedEntries.has(entry.id) ? '📌' : '📍'}
          </button>}
          <button className={styles.btnDel} onClick={() => handleDeleteEntry(entry.id)}>{t("common.delete")}</button>
        </div>
      </div>
      <div className={styles.previewBody}>
        {deps.tagsApi.renderEntryTagRow(entry)}
        <div className={styles.pathDisplay}>
          <span className={styles.pathLabel}>{t("Knowledge.k250")}</span>
          <code className={styles.pathValue}>{entry.path_url}</code>
        </div>
        {entry.source_path && <div className={styles.pathDisplay} style={{ color: '#FFC107' }}>
            <span className={styles.pathLabel}>{t("Knowledge.k251")}</span>
            <code className={styles.pathValue} style={{ color: 'rgba(255,193,7,0.85)' }}>{entry.source_path}</code>
          </div>}
        {(() => {
          const url = (entry.path_url || '').toLowerCase();
          const extMatch = url.match(/\.([a-zA-Z0-9]+)$/);
          const ext = extMatch ? extMatch[1] : '';
          const imageExts = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg', 'ico'];
          const codeExts = ['rs', 'ts', 'tsx', 'js', 'jsx', 'py', 'go', 'java', 'c', 'cpp', 'h', 'hpp', 'cs', 'rb', 'php', 'swift', 'kt', 'scala', 'lua', 'sh', 'bash', 'zsh', 'yaml', 'yml', 'toml', 'json', 'xml', 'css', 'scss', 'less', 'html', 'htm', 'sql'];
          const languageMapR: Record<string, string> = {
            rs: 'rust', ts: 'typescript', tsx: 'typescriptreact', js: 'javascript', jsx: 'javascriptreact', py: 'python', go: 'go', java: 'java', c: 'c', cpp: 'cpp', h: 'c', hpp: 'cpp', cs: 'csharp', rb: 'ruby', php: 'php', swift: 'swift', kt: 'kotlin', scala: 'scala', lua: 'lua', sh: 'bash', bash: 'bash', zsh: 'bash', yaml: 'yaml', yml: 'yaml', toml: 'toml', json: 'json', xml: 'xml', css: 'css', scss: 'scss', less: 'less', html: 'html', htm: 'html', sql: 'sql'
          };

          // 文件已删除：直接提示，不尝试加载
          if (core.missingFiles.has(entry.id) && entry.source_path) {
            return <div className={styles.previewMissing}>
                <div style={{ fontSize: 40, marginBottom: 16 }}>⚠️</div>
                <div style={{ color: '#E57373', fontSize: 15, fontWeight: 600, marginBottom: 8 }}>{t("Knowledge.k252")}</div>
                <div style={{ color: '#888', fontSize: 12, marginBottom: 4 }}>{t("Knowledge.k253")}</div>
                <code style={{ color: '#A55', fontSize: 11, background: 'rgba(229,115,115,0.08)', padding: '4px 8px', borderRadius: 4, display: 'inline-block', marginTop: 8, wordBreak: 'break-all' }}>{entry.source_path}</code>
                <div style={{ marginTop: 16, display: 'flex', gap: 8, justifyContent: 'center' }}>
                  <button className={styles.btnDel} onClick={() => {
                    if (confirm(t("Knowledge.k254"))) handleDeleteEntry(entry.id);
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
                <div className={styles.previewImageContainer} style={{ cursor: 'pointer', overflow: 'hidden', position: 'relative', borderRadius: 6 }} onClick={() => deps.handleOpenEntry(entry)}>
                  <img src={thumbnailUrl} alt={entry.name} className={styles.previewImage} />
                  <div style={{ position: 'absolute', inset: 0, background: 'rgba(0,0,0,0)', transition: 'background 0.2s', display: 'flex', alignItems: 'center', justifyContent: 'center' }} onMouseEnter={e => e.currentTarget.style.background = 'rgba(0,0,0,0.4)'} onMouseLeave={e => e.currentTarget.style.background = 'rgba(0,0,0,0)'}>
                    <span style={{ color: '#fff', fontSize: 14, opacity: 0, transition: 'opacity 0.2s', pointerEvents: 'none' }} onMouseEnter={e => {
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
                  <button className={styles.btnOpen} onClick={() => deps.handleOpenEntry(entry)}>{t("Knowledge.k257")}</button>
                </div>
              </div>;
          }
          if (ext === 'pdf' && pdfPreviewUrl) {
            return <div className={styles.previewPdfWrap}>
                <PdfViewer base64Data={pdfPreviewUrl} fileName={entry.name} />
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
          if (entry.entry_type === 'text' && (!entry.path_url || entry.path_url.startsWith('kb://'))) {
            return <div className={styles.contentEditor}>
                <div className={styles.contentEditorHeader}>
                  <span style={{ color: deps.mediaTextColor, fontSize: '13px' }}>{entry.name}</span>
                  <button className={styles.modeToggleIcon} onClick={() => {
                    if (deps.editors.fileEditMode) {
                      deps.editors.handleStopFileEdit();
                    } else {
                      deps.editors.startMarkdownEdit(entry.content || editContent || '');
                    }
                  }} title={deps.editors.fileEditMode ? t("Knowledge.k223") : t("Knowledge.k224")}>
                    {deps.editors.fileEditMode ? '✏️' : '📖'}
                  </button>
                </div>
                {deps.editors.fileEditMode ? null : <div className={styles.contentPreview}>
                    {(() => {
                      const raw = (editContent || entry.content || '').slice(0, 2000);
                      const suffix = (editContent || entry.content || '').length > 2000 ? '...' : '';
                      const parts = raw.split(/(\[\[[^\]]+\]\])/g);
                      return <span>
                          {parts.map((part, i) => {
                            const m = part.match(/^\[\[([^\]]+)\]\]$/);
                            if (m) {
                              return <span key={i} className={styles.wikiLinkInline} onClick={() => deps.handleWikiLinkClick(m[1])} title={t("Knowledge.k261", { arg0: m[1] })}>[[{m[1]}]]</span>;
                            }
                            return <span key={i}>{part}</span>;
                          })}
                          {suffix}
                        </span>;
                    })()}
                  </div>}
              </div>;
          }
          const isExternalFile = entry.path_url && !entry.path_url.startsWith('kb://') && !entry.path_url.startsWith('http');
          const videoExtsP = ['mp4', 'webm', 'avi', 'mkv', 'mov', 'wmv', 'flv'];
          const audioExtsP = ['mp3', 'wav', 'flac', 'aac', 'ogg', 'opus', 'wma'];
          const docExtsP = ['docx', 'doc', 'xlsx', 'xls', 'pptx', 'ppt', 'odt', 'ods', 'odp', 'rtf'];
          if (isExternalFile) {
            if (videoExtsP.includes(ext)) {
              return <div className={styles.previewMediaWrap}>
                  <div className={styles.previewMediaIcon}>🎬</div>
                  <span className={styles.previewMediaLabel}>{t("Knowledge.k262")} {ext.toUpperCase()}</span>
                  <button className={styles.btnOpen} onClick={() => deps.handleOpenEntry(entry)}>{t("Knowledge.k263")}</button>
                </div>;
            }
            if (audioExtsP.includes(ext)) {
              return <div className={styles.previewMediaWrap}>
                  <div className={styles.previewMediaIcon}>🎵</div>
                  <span className={styles.previewMediaLabel}>{t("Knowledge.k264")} {ext.toUpperCase()}</span>
                  <button className={styles.btnOpen} onClick={() => deps.handleOpenEntry(entry)}>{t("Knowledge.k263")}</button>
                </div>;
            }
            if (docExtsP.includes(ext)) {
              return <div className={styles.previewMediaWrap}>
                  <div className={styles.previewMediaIcon}>📄</div>
                  <span className={styles.previewMediaLabel}>{t("Knowledge.k265")} {ext.toUpperCase()}</span>
                  <button className={styles.btnOpen} onClick={() => deps.handleOpenEntry(entry)}>{t("Knowledge.k266")}</button>
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
    </>
  );

  /** 快捷切换器（Ctrl+P） */
  const renderQuickSwitcher = () => {
    if (!showQuickSwitcher) return null;
    return (
      <div className={styles.modalOverlay} onClick={() => setShowQuickSwitcher(false)}>
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
                  {core.categories.find(c => c.id === entry.category_id)?.name || t("knowledge.GraphView.k7")}
                </span>
              </div>)}
          </div>
          <div className={styles.quickSwitcherFooter}>
            <span>{t("Knowledge.k292")}</span><span>{t("Knowledge.k293")}</span><span>{t("Knowledge.k294")}</span>
          </div>
        </div>
      </div>
    );
  };

  /** 全局弹层：删除确认 / 同名冲突 / 右键菜单 / 移动目标 */
  const renderModals = () => (
    <>
      {confirmDelete && <DeleteConfirmModal confirmDelete={confirmDelete} allEntries={core.allEntries} isExternalEntry={core.isExternalEntry} onCancel={() => setConfirmDelete(null)} onConfirmCategory={confirmDeleteCategory} onConfirmEntry={confirmDeleteEntry} onConfirmBatch={confirmBatchDelete} />}
      {core.nameConflict && <NameConflictModal conflict={core.nameConflict} resolveAutoRename={core.resolveAutoRename} onCancel={() => core.setNameConflict(null)} />}
      {contextMenu && <ContextMenu menu={contextMenu} onClose={() => setContextMenu(null)} onRename={startRename} onMove={(type, id) => setMoveTarget({ type, id })} onDelete={(type, id) => type === 'category' ? handleDeleteCategory(id) : handleDeleteEntry(id)} onAiClassify={deps.aiClassifyMenuItem} />}
      {moveTarget !== null && <MoveTargetModal target={moveTarget} categories={core.categories} getCategoryDepth={getCategoryDepth} getAllDescendantIds={core.getAllDescendantIds} onMoveEntry={handleMoveEntry} onMoveCategory={handleMoveCategory} onCancel={() => setMoveTarget(null)} />}
    </>
  );

  return {
    renderSidebarButtons,
    renderNewFolderForm,
    renderViewTabs,
    renderFileTree,
    renderDisplayToggle,
    renderBatchBar,
    renderBatchModals,
    renderNewEntryForm,
    renderFolderContent,
    renderGlobalContent,
    renderPreviewDefault,
    renderQuickSwitcher,
    renderModals,
    handleInsertTemplate,
    handleQuickSelect,
    openQuickSwitcher: () => {
      setShowQuickSwitcher(true);
      setQuickSwitcherQuery('');
      setQuickSwitcherIndex(0);
    },
    closeQuickSwitcher: () => setShowQuickSwitcher(false)
  };
}
