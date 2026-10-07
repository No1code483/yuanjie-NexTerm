// plugins/boards/knowledge/ipc.ts — boards.knowledge 插件 IPC 客户端（短码 kb）。
// 契约：统一经内核 dispatcher，逻辑名 `kb:plugin:<旧命令名>`；参数形状与迁移前
// 旧 Tauri 命令逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 批次2a-1：核心数据面 29 条；批次2a-2：导入与解析 21 条（4 条模版方法随批C3 迁出至
// knowledge.templates L2）；批次2a-3：附件与语义 5 条（有消费才迁——裁定 35-A② 同口径）。
// 零消费判「留」有 alias 无前端方法 2 条：`kb_get_outgoing_links`（功能缺口，42-A）、
// `kb_attachment_preload`（A5 命令面完整保留，43-A）。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const KNOWLEDGE_IPC_METHODS = {
  getKbCategories: { cmd: 'get_kb_categories' },
  getKbCategoryCounts: { cmd: 'get_kb_category_counts' },
  addKbCategory: { cmd: 'add_kb_category' },
  updateKbCategory: { cmd: 'update_kb_category' },
  addKbEntry: { cmd: 'add_kb_entry' },
  deleteKbEntry: { cmd: 'delete_kb_entry' },
  moveKbEntry: { cmd: 'move_kb_entry' },
  moveKbCategory: { cmd: 'move_kb_category' },
  updateKbEntry: { cmd: 'update_kb_entry' },
  searchKbEntries: { cmd: 'search_kb_entries' },
  getAllKbEntries: { cmd: 'get_all_kb_entries' },
  getKbTags: { cmd: 'get_kb_tags' },
  addKbTag: { cmd: 'add_kb_tag' },
  updateKbTag: { cmd: 'update_kb_tag' },
  deleteKbTag: { cmd: 'delete_kb_tag' },
  getKbEntryTags: { cmd: 'get_kb_entry_tags' },
  getKbAllEntryTags: { cmd: 'get_kb_all_entry_tags' },
  setKbEntryTags: { cmd: 'set_kb_entry_tags' },
  getKbTagStats: { cmd: 'get_kb_tag_stats' },
  getKbEntriesByTag: { cmd: 'get_kb_entries_by_tag' },
  batchAddKbTag: { cmd: 'batch_add_kb_tag' },
  batchRemoveKbTag: { cmd: 'batch_remove_kb_tag' },
  toggleKbFavorite: { cmd: 'toggle_kb_favorite' },
  getKbFavorites: { cmd: 'get_kb_favorites' },
  moveKbCategoryToRecycle: { cmd: 'move_kb_category_to_recycle' },
  recordKbAccess: { cmd: 'record_kb_access' },
  getKbRecent: { cmd: 'get_kb_recent' },
  batchDeleteKbEntries: { cmd: 'batch_delete_kb_entries' },
  batchMoveKbEntries: { cmd: 'batch_move_kb_entries' },
  // ========== 2a-2 段（导入与解析，25 条有消费方法） ==========
  kbImportMultiFolders: { cmd: 'kb_import_multi_folders' },
  kbCheckFilesExistence: { cmd: 'kb_check_files_existence' },
  kbScanDirectory: { cmd: 'kb_scan_directory' },
  kbExtractTableData: { cmd: 'kb_extract_table_data' },
  kbReadExternalFile: { cmd: 'kb_read_external_file' },
  kbReadFileBase64: { cmd: 'kb_read_file_base64' },
  kbExtractDocxText: { cmd: 'kb_extract_docx_text' },
  kbExtractPsdInfo: { cmd: 'kb_extract_psd_info' },
  kbExtractAiInfo: { cmd: 'kb_extract_ai_info' },
  kbListZipContents: { cmd: 'kb_list_zip_contents' },
  kbExtractEpubText: { cmd: 'kb_extract_epub_text' },
  kbExtractOdpText: { cmd: 'kb_extract_odp_text' },
  kbExtractDocText: { cmd: 'kb_extract_doc_text' },
  kbExtractRtfText: { cmd: 'kb_extract_rtf_text' },
  kbGetTrackedPaths: { cmd: 'kb_get_tracked_paths' },
  kbRemoveTrackedPath: { cmd: 'kb_remove_tracked_path' },
  kbAddScannedFiles: { cmd: 'kb_add_scanned_files' },
  kbCheckPaths: { cmd: 'kb_check_paths' },
  kbGetSnapshots: { cmd: 'kb_get_snapshots' },
  kbRestoreSnapshot: { cmd: 'kb_restore_snapshot' },
  // 批C3：4 条模版方法迁出至 knowledge.templates L2（features/templates/ipc.ts，短码 kt）
  kbGetBacklinks: { cmd: 'kb_get_backlinks' },
  // ========== 2a-3 段（附件与语义，5 条有消费方法） ==========
  kbSemanticSearch: { cmd: 'kb_semantic_search' },
  kbAttachmentPin: { cmd: 'kb_attachment_pin' },
  kbAttachmentUnpin: { cmd: 'kb_attachment_unpin' },
  kbAttachmentListPinned: { cmd: 'kb_attachment_list_pinned' },
  kbAttachmentGetCachedPath: { cmd: 'kb_attachment_get_cached_path' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherKb = defineIpcNamespace('kb', KNOWLEDGE_IPC_METHODS);
type KbMethod = keyof typeof KNOWLEDGE_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeKb<T = any>(method: KbMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherKb[method](args) as Promise<ApiResponse<T>>;
}

/** 参数键按裁定 37-A 统一 camelCase（`entryId` / `tagIds` / `entryIds` / `tagId` /
 *  `parentId` / `sortOrder`）；单字键（`id` / `request` / `query` / `limit`）不受影响。
 *  后端 dispatcher 以 snake_case 优先、camelCase 回退，两种口径均可绑定。 */
export const kb = {
  getKbCategories: (args?: any) => invokeKb<any[]>('getKbCategories', args),
  getKbCategoryCounts: () => invokeKb<any[]>('getKbCategoryCounts'),
  addKbCategory: (args: any) => invokeKb<any>('addKbCategory', args),
  updateKbCategory: (args: any) => invokeKb<any>('updateKbCategory', args),
  addKbEntry: (args: any) => invokeKb<any>('addKbEntry', args),
  deleteKbEntry: (args: any) => invokeKb<any>('deleteKbEntry', args),
  moveKbEntry: (args: any) => invokeKb<any>('moveKbEntry', args),
  moveKbCategory: (args: any) => invokeKb<any>('moveKbCategory', args),
  updateKbEntry: (args: any) => invokeKb<any>('updateKbEntry', args),
  searchKbEntries: (args: any) => invokeKb<any[]>('searchKbEntries', args),
  getAllKbEntries: () => invokeKb<any[]>('getAllKbEntries'),
  getKbTags: () => invokeKb<any[]>('getKbTags'),
  addKbTag: (args: any) => invokeKb<any>('addKbTag', args),
  updateKbTag: (args: any) => invokeKb<any>('updateKbTag', args),
  deleteKbTag: (args: any) => invokeKb<any>('deleteKbTag', args),
  getKbEntryTags: (args: any) => invokeKb<any[]>('getKbEntryTags', args),
  getKbAllEntryTags: () => invokeKb<any[]>('getKbAllEntryTags'),
  setKbEntryTags: (args: any) => invokeKb<any>('setKbEntryTags', args),
  getKbTagStats: () => invokeKb<any[]>('getKbTagStats'),
  getKbEntriesByTag: (args: any) => invokeKb<any[]>('getKbEntriesByTag', args),
  batchAddKbTag: (args: any) => invokeKb<number>('batchAddKbTag', args),
  batchRemoveKbTag: (args: any) => invokeKb<number>('batchRemoveKbTag', args),
  toggleKbFavorite: (args: any) => invokeKb<boolean>('toggleKbFavorite', args),
  getKbFavorites: () => invokeKb<any[]>('getKbFavorites'),
  moveKbCategoryToRecycle: (args: any) => invokeKb<any>('moveKbCategoryToRecycle', args),
  recordKbAccess: (args: any) => invokeKb<any>('recordKbAccess', args),
  getKbRecent: (args?: any) => invokeKb<any[]>('getKbRecent', args),
  batchDeleteKbEntries: (args: any) => invokeKb<number>('batchDeleteKbEntries', args),
  batchMoveKbEntries: (args: any) => invokeKb<number>('batchMoveKbEntries', args),
  // ========== 2a-2 段（导入与解析，25 条） ==========
  kbImportMultiFolders: (args: any) => invokeKb<any>('kbImportMultiFolders', args),
  kbCheckFilesExistence: (args: any) => invokeKb<Record<string, boolean>>('kbCheckFilesExistence', args),
  kbScanDirectory: (args: any) => invokeKb<any[]>('kbScanDirectory', args),
  kbExtractTableData: (args: any) => invokeKb<any>('kbExtractTableData', args),
  kbReadExternalFile: (args: any) => invokeKb<any>('kbReadExternalFile', args),
  kbReadFileBase64: (args: any) => invokeKb<any>('kbReadFileBase64', args),
  kbExtractDocxText: (args: any) => invokeKb<any>('kbExtractDocxText', args),
  kbExtractPsdInfo: (args: any) => invokeKb<any>('kbExtractPsdInfo', args),
  kbExtractAiInfo: (args: any) => invokeKb<any>('kbExtractAiInfo', args),
  kbListZipContents: (args: any) => invokeKb<any>('kbListZipContents', args),
  kbExtractEpubText: (args: any) => invokeKb<any>('kbExtractEpubText', args),
  kbExtractOdpText: (args: any) => invokeKb<any>('kbExtractOdpText', args),
  kbExtractDocText: (args: any) => invokeKb<any>('kbExtractDocText', args),
  kbExtractRtfText: (args: any) => invokeKb<any>('kbExtractRtfText', args),
  kbGetTrackedPaths: (args?: any) => invokeKb<any[]>('kbGetTrackedPaths', args),
  kbRemoveTrackedPath: (args: any) => invokeKb<any>('kbRemoveTrackedPath', args),
  kbAddScannedFiles: (args: any) => invokeKb<any>('kbAddScannedFiles', args),
  kbCheckPaths: () => invokeKb<any>('kbCheckPaths'),
  kbGetSnapshots: (args: any) => invokeKb<any[]>('kbGetSnapshots', args),
  kbRestoreSnapshot: (args: any) => invokeKb<any>('kbRestoreSnapshot', args),
  // 批C3：模版 4 方法迁出至 knowledge.templates L2（features/templates/ipc.ts）
  kbGetBacklinks: (args: any) => invokeKb<any[]>('kbGetBacklinks', args),
  // ========== 2a-3 段（附件与语义，5 条） ==========
  kbSemanticSearch: (args: any) => invokeKb<any>('kbSemanticSearch', args),
  kbAttachmentPin: (args: any) => invokeKb<any>('kbAttachmentPin', args),
  kbAttachmentUnpin: (args: any) => invokeKb<any>('kbAttachmentUnpin', args),
  kbAttachmentListPinned: () => invokeKb<any[]>('kbAttachmentListPinned'),
  kbAttachmentGetCachedPath: (args: any) => invokeKb<string | null>('kbAttachmentGetCachedPath', args),
};
