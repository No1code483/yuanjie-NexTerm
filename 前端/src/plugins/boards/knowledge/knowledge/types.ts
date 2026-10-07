import type { MouseEvent } from 'react'

export interface KbCategory {
  id: number
  name: string
  parent_id: number | null
  library?: string
  sort_order: number
  created_at: number
}

export interface KbEntry {
  id: number
  category_id: number
  name: string
  path_url: string
  entry_type: string
  created_at: number
  updated_at: number
  is_favorited?: number
  content?: string | null
  source_path?: string | null
  file_exists?: boolean
}

export interface TreeNode {
  type: 'category' | 'entry'
  id: number
  name: string
  categoryId?: number
  pathUrl?: string
  entryType?: string
  updatedAt?: number
  sourcePath?: string
  depth: number
  children: TreeNode[]
}

export interface KbDatabase {
  id: string
  name: string
  created_at: number
}

export type SortMode = 'name_asc' | 'name_desc' | 'time_desc' | 'time_asc'

export interface CategoryCount {
  category_id: number
  count: number
}

export interface KbTrackedPath {
  id: number
  path: string
  category_id: number
  library: string
  last_imported_at: number
  created_at: number
}

export interface KbTemplate {
  id: number
  name: string
  icon: string
  description: string
  entry_type: string
  content: string
}

export interface ConfirmDelete {
  type: 'category' | 'entry' | 'batch'
  id?: number
  name?: string
  subCount?: number
  batchCount?: number
}

export interface KbTag {
  id: number
  name: string
  color: string
  created_at: number
}

export type ViewMode = 'all' | 'favorite' | 'recent' | 'tag'

export interface ScanDirFile {
  name: string
  path: string
  size_bytes: number
  extension: string
  file_type: string
}

/** 树/预览区选中项（分类或条目） */
export interface SelectedId {
  type: 'category' | 'entry'
  id: number
}

/** 顶部状态提示 */
export interface StatusMsg {
  type: 'success' | 'error'
  text: string
}

/** 同名冲突弹层状态（browse 渲染，import 亦会触发） */
export interface NameConflictState {
  newName: string
  existingEntry: KbEntry
  categoryId: number
  onReplace: () => void
  onRename: (renamed: string) => void
}

/**
 * L1 核心数据层（壳）向各 L2 功能域组件/hook 传递的共享数据与回调。
 * 各功能域自持其局部 state + handlers + JSX；涉及全局共享的读写一律经此对象。
 */
export interface KnowledgeCore {
  // ---- 只读数据 ----
  categories: KbCategory[]
  allEntries: KbEntry[]
  missingFiles: Set<number>
  pinnedEntries: Set<number>
  selectedId: SelectedId | null
  currentLibrary: string
  isOnline: boolean
  sortMode: SortMode
  typeFilter: string
  viewMode: ViewMode
  viewFilterId: number | null
  tags: KbTag[]
  tagStats: Array<{ tag_id: number; tag_name: string; tag_color: string; entry_count: number }>
  entryTags: Map<number, KbTag[]>
  /** 各分类条目数（loadCategories 侧加载，browse 树节点计数使用） */
  categoryCounts: Map<number, number>
  /** 目录树展开集合（loadCategories 会自动展开根分类） */
  expandedIds: Set<number>
  setExpandedIds: (updater: (prev: Set<number>) => Set<number>) => void
  // ---- 回写/回调 ----
  setSelectedId: (value: SelectedId | null) => void
  setPinnedEntries: (updater: (prev: Set<number>) => Set<number>) => void
  setSortMode: (value: SortMode) => void
  setTypeFilter: (value: string) => void
  setViewMode: (value: ViewMode) => void
  setViewFilterId: (value: number | null) => void
  showStatus: (type: 'success' | 'error', text: string) => void
  reload: () => Promise<void>
  loadTags: () => Promise<void>
  loadTagStats: () => Promise<void>
  loadAllEntryTags: () => Promise<void>
  loadEntryTags: (entryId?: number) => Promise<void>
  // ---- 同名冲突 / 条目工具（browse 与 import 共用） ----
  nameConflict: NameConflictState | null
  setNameConflict: (value: NameConflictState | null) => void
  findNameConflict: (name: string, categoryId: number) => KbEntry | null
  resolveAutoRename: (name: string, categoryId: number) => string
  isExternalEntry: (entry: KbEntry) => boolean
  moveEntryToRecycle: (entryId: number) => Promise<boolean>
  getCategoryDepth: (id: number, depth?: number) => number
  getEntriesInCategory: (categoryId: number) => KbEntry[]
  getAllDescendantIds: (cats: KbCategory[], rootId: number) => number[]
  isPinnable: (entry: KbEntry) => boolean
  handleToggleFavorite: (entryId: number) => void
  handleTogglePin: (entry: KbEntry, e?: MouseEvent) => void
  // ---- 纯工具 ----
  formatTime: (ts: number) => string
  formatFileSize: (bytes: number) => string
  getTypeIcon: (type?: string) => string
  getTypeLabel: (type?: string) => string
}