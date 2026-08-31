// ipc/knowledge.ts — knowledge 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）
import { ipc } from './core';

export const knowledge = {
  getCategories: () => ipc.invoke<any[]>('get_kb_categories'),
  addCategory: (category: any) => ipc.invoke('add_kb_category', category),
  deleteCategory: (id: number, recursive?: boolean) => ipc.invoke('delete_kb_category', {
    id,
    recursive
  }),
  renameCategory: (id: number, name: string) => ipc.invoke('rename_kb_category', {
    id,
    name
  }),
  moveCategory: (id: number, parentId: number | null) => ipc.invoke('move_kb_category', {
    id,
    parent_id: parentId
  }),
  getItems: (categoryId?: number) => ipc.invoke<any[]>('get_kb_entries', {
    category_id: categoryId
  }),
  getAllItems: () => ipc.invoke<any[]>('get_all_kb_entries'),
  getCategoryCounts: () => ipc.invoke<any[]>('get_kb_category_counts'),
  createItem: (item: any) => ipc.invoke('add_kb_entry', {
    request: item
  }),
  updateItem: (id: number, request: any) => ipc.invoke('update_kb_entry', {
    id,
    request
  }),
  deleteItem: (id: number) => ipc.invoke('delete_kb_entry', {
    id
  }),
  search: (query: string) => ipc.invoke<any[]>('search_kb_entries', {
    query
  }),
  getTags: () => ipc.invoke<any[]>('get_kb_tags'),
  addTag: (name: string, color?: string) => ipc.invoke('add_kb_tag', {
    name,
    color
  }),
  updateTag: (id: number, name: string, color: string) => ipc.invoke('update_kb_tag', {
    id,
    name,
    color
  }),
  deleteTag: (id: number) => ipc.invoke('delete_kb_tag', {
    id
  }),
  getTagStats: () => ipc.invoke<any[]>('get_kb_tag_stats'),
  getEntryTags: (entryId: number) => ipc.invoke<any[]>('get_kb_entry_tags', {
    entry_id: entryId
  }),
  setEntryTags: (entryId: number, tagIds: number[]) => ipc.invoke('set_kb_entry_tags', {
    entry_id: entryId,
    tag_ids: tagIds
  }),
  getEntriesByTag: (tagId: number) => ipc.invoke<any[]>('get_kb_entries_by_tag', {
    tag_id: tagId
  }),
  toggleFavorite: (entryId: number) => ipc.invoke<boolean>('toggle_kb_favorite', {
    entry_id: entryId
  }),
  getFavorites: () => ipc.invoke<any[]>('get_kb_favorites'),
  recordAccess: (entryId: number) => ipc.invoke('record_kb_access', {
    entry_id: entryId
  }),
  getRecent: (limit?: number) => ipc.invoke<any[]>('get_kb_recent', {
    limit
  }),
  batchDeleteEntries: (ids: number[]) => ipc.invoke<number>('batch_delete_kb_entries', {
    request: {
      ids
    }
  }),
  batchMoveEntries: (ids: number[], categoryId: number) => ipc.invoke<number>('batch_move_kb_entries', {
    request: {
      ids,
      category_id: categoryId
    }
  }),
  batchAddTag: (entryIds: number[], tagId: number) => ipc.invoke<number>('batch_add_kb_tag', {
    entry_ids: entryIds,
    tag_id: tagId
  }),
  batchRemoveTag: (entryIds: number[], tagId: number) => ipc.invoke<number>('batch_remove_kb_tag', {
    entry_ids: entryIds,
    tag_id: tagId
  }),
  addTrackedPath: (path: string, categoryId: number, library: string) => ipc.invoke('kb_add_tracked_path', {
    path,
    categoryId,
    library
  }),
  getTrackedPaths: () => ipc.invoke<any[]>('kb_get_tracked_paths'),
  removeTrackedPath: (id: number) => ipc.invoke('kb_remove_tracked_path', {
    id
  }),
  addScannedFiles: (categoryId: number, files: any[]) => ipc.invoke('kb_add_scanned_files', {
    categoryId,
    files
  }),
  scanDirectory: (path: string) => ipc.invoke<any[]>('kb_scan_directory', {
    path
  }),
  checkPaths: () => ipc.invoke<any>('kb_check_paths'),
  checkFilesExistence: (paths: string[]) => ipc.invoke<Record<string, boolean>>('kb_check_files_existence', {
    paths
  }),
  getTemplates: () => ipc.invoke<any[]>('kb_get_templates'),
  createTemplate: (request: {
    name: string;
    icon: string;
    description: string;
    entry_type: string;
    content: string;
  }) => ipc.invoke('kb_create_template', {
    request
  }),
  updateTemplate: (request: {
    id: number;
    name?: string;
    icon?: string;
    description?: string;
    entry_type?: string;
    content?: string;
  }) => ipc.invoke('kb_update_template', {
    request
  }),
  deleteTemplate: (id: number) => ipc.invoke('kb_delete_template', {
    id
  }),
  getBacklinks: (entryId: number) => ipc.invoke<Array<{
    entry: any;
    snippet: string;
  }>>('kb_get_backlinks', {
    entryId
  }),
  getOutgoingLinks: (entryId: number) => ipc.invoke<any[]>('kb_get_outgoing_links', {
    entryId
  }),
  getSnapshots: (entryId: number) => ipc.invoke<Array<{
    id: number;
    entry_id: number;
    entry_name: string;
    content: string;
    content_length: number;
    created_at: number;
  }>>('kb_get_snapshots', {
    entryId
  }),
  restoreSnapshot: (snapshotId: number) => ipc.invoke<any>('kb_restore_snapshot', {
    snapshotId
  }),
  // A5 离线同步 Phase 3 Task 4: 知识库附件预加载
  attachmentPin: (entryId: number, filePath: string) =>
    ipc.invoke<any>('kb_attachment_pin', { entryId, filePath }),
  attachmentUnpin: (entryId: number) =>
    ipc.invoke<any>('kb_attachment_unpin', { entryId }),
  attachmentPreload: (entryId: number) =>
    ipc.invoke<any>('kb_attachment_preload', { entryId }),
  attachmentListPinned: () =>
    ipc.invoke<any[]>('kb_attachment_list_pinned'),
  attachmentGetCachedPath: (entryId: number) =>
    ipc.invoke<string | null>('kb_attachment_get_cached_path', { entryId }),
};
