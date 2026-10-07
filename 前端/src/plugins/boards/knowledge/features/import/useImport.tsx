// knowledge.import L2 功能域：导入 / 目录扫描 / 追踪路径。
// 局部 state + handlers + JSX 物理落在本目录；核心共享数据经 KnowledgeCore 注入。
import { t } from "i18next";
import type { ReactNode } from 'react';
import { useState } from 'react';
import { kb } from '../../ipc';
import { intelligence } from '@/lib/ipc';
import { time } from '@/lib/utils';
import styles from '../../Knowledge.module.css';
import type { KbEntry, KbTrackedPath, ScanDirFile, KnowledgeCore } from '../../knowledge/types';

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

export function useImport(core: KnowledgeCore, ai: { renderAiClassifyToolbar: (fileNames: string[]) => ReactNode }) {
  const [isImporting, setIsImporting] = useState(false);
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

  const handleImportFile = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        title: t("Knowledge.k91"),
        directory: false,
        filters: [{ name: t("Knowledge.k92"), extensions: ['*'] }]
      });
      if (!selected) return;
      const targetCategoryId = core.selectedId?.type === 'category' ? core.selectedId.id : core.categories[0]?.id ?? null;
      if (!targetCategoryId) {
        core.showStatus('error', t("Knowledge.k93"));
        return;
      }
      const pathStr = String(selected);
      const fileName = pathStr.split(/[\\/]/).pop() || t("Knowledge.k94");
      const ext = fileName.split('.').pop()?.toLowerCase() || '';
      const entryType = ['pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx'].includes(ext) ? 'file' : ['md', 'txt', 'rst'].includes(ext) ? 'text' : ['mp4', 'avi', 'mkv', 'mov', 'f4v', 'flv'].includes(ext) ? 'video' : ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg'].includes(ext) ? 'image' : ['mp3', 'wav', 'oga', 'ogg'].includes(ext) ? 'audio' : 'file';
      const existing = core.findNameConflict(fileName, targetCategoryId);
      if (existing) {
        core.setNameConflict({
          newName: fileName,
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
              const res = await kb.addKbEntry({
                request: { category_id: targetCategoryId, name: fileName, path_url: pathStr, entry_type: entryType }
              });
              if (res.code === 0) {
                core.showStatus('success', t("Knowledge.k95", { fileName: fileName }));
                core.reload();
              } else core.showStatus('error', res.message || t("Knowledge.k96"));
            } catch (e) {
              core.showStatus('error', t("Knowledge.k97", { e: e }));
            }
          },
          onRename: async renamed => {
            core.setNameConflict(null);
            try {
              const res = await kb.addKbEntry({
                request: { category_id: targetCategoryId, name: renamed, path_url: pathStr, entry_type: entryType }
              });
              if (res.code === 0) {
                core.showStatus('success', t("Knowledge.k98", { renamed: renamed }));
                core.reload();
              } else core.showStatus('error', res.message || t("Knowledge.k99"));
            } catch (e) {
              core.showStatus('error', t("Knowledge.k100", { e: e }));
            }
          }
        });
        return;
      }
      const res = await kb.addKbEntry({
        request: { category_id: targetCategoryId, name: fileName, path_url: pathStr, entry_type: entryType }
      });
      if (res.code === 0) {
        core.showStatus('success', t("Knowledge.k101", { fileName: fileName }));
        core.reload();
      } else core.showStatus('error', res.message || t("Knowledge.k99"));
    } catch (err) {
      core.showStatus('error', t("Knowledge.k102", { arg0: err instanceof Error ? err.message : String(err) }));
    }
  };

  const handleImportFolder = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: true,
        title: t("Knowledge.k103"),
        directory: true
      });
      if (!selected) return;
      const paths: string[] = Array.isArray(selected) ? selected.map(String) : [String(selected)];
      const targetCategoryId = core.selectedId?.type === 'category' ? core.selectedId.id : core.categories[0]?.id ?? null;
      if (!targetCategoryId) {
        core.showStatus('error', t("Knowledge.k104"));
        return;
      }
      setIsImporting(true);
      core.showStatus('success', t("Knowledge.k105", { length: paths.length }));
      const res = await kb.kbImportMultiFolders({
        folderPaths: paths,
        categoryId: targetCategoryId,
        library: core.currentLibrary
      });
      if (res.code === 0 && res.data) {
        await core.reload();
        const importedEntries = res.data.entries || [];
        const siblingsBeforeImport = core.getEntriesInCategory(targetCategoryId).filter(e => !importedEntries.some((ne: KbEntry) => ne.name === e.name));
        let renamedCount = 0;
        for (const newEntry of importedEntries) {
          const conflict = siblingsBeforeImport.find(e => e.name.toLowerCase() === newEntry.name.toLowerCase());
          if (conflict) {
            const renamed = core.resolveAutoRename(newEntry.name, targetCategoryId);
            try {
              await kb.updateKbEntry({ request: { id: newEntry.id, name: renamed } });
              renamedCount++;
            } catch {/* skip */}
          }
        }
        if (renamedCount > 0) {
          await core.reload();
        }
        const { categories: newCats, entries: newEntries, fail_count } = res.data;
        let msg = t("Knowledge.k106", { length: newCats.length, arg0: newEntries.length });
        if (renamedCount > 0) msg += t("Knowledge.k107", { renamedCount: renamedCount });
        if (fail_count > 0) msg += t("Knowledge.k108", { fail_count: fail_count });
        core.showStatus(fail_count > 0 ? 'error' : 'success', msg);
      } else core.showStatus('error', res.message || t("Knowledge.k99"));
    } catch (err) {
      core.showStatus('error', t("Knowledge.k109", { arg0: err instanceof Error ? err.message : String(err) }));
    } finally {
      setIsImporting(false);
    }
  };

  const loadTrackedPaths = async () => {
    setTrackedPathsLoading(true);
    try {
      const res = await kb.kbGetTrackedPaths({ library: core.currentLibrary });
      if (res.code === 0 && res.data) setTrackedPaths(res.data);
    } catch {/* ignore */} finally {
      setTrackedPathsLoading(false);
    }
  };

  const handleRemoveTrackedPath = async (id: number) => {
    try {
      const res = await kb.kbRemoveTrackedPath({ id });
      if (res.code === 0) {
        setTrackedPaths(prev => prev.filter(p => p.id !== id));
        core.showStatus('success', t("Knowledge.k110"));
      } else core.showStatus('error', res.message || t("Knowledge.k111"));
    } catch {
      core.showStatus('error', t("Knowledge.k112"));
    }
  };

  const handleReimportTracked = async (tp: KbTrackedPath) => {
    setIsImporting(true);
    try {
      const res = await kb.kbImportMultiFolders({
        folderPaths: [tp.path],
        categoryId: tp.category_id,
        library: core.currentLibrary
      });
      if (res.code === 0 && res.data) {
        await core.reload();
        core.showStatus('success', t("Knowledge.k113", { length: res.data.entries.length }));
      } else core.showStatus('error', res.message || t("Knowledge.k114"));
    } catch (err) {
      core.showStatus('error', t("Knowledge.k115", { arg0: err instanceof Error ? err.message : String(err) }));
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
      const res = await kb.kbCheckPaths();
      if (res.code === 0 && res.data) {
        setPathCheckResult(res.data);
        const invT = res.data.invalid_tracked.length;
        const invE = res.data.invalid_external_entries.length;
        if (invT + invE === 0) {
          core.showStatus('success', t("Knowledge.k116"));
        } else {
          const parts: string[] = [];
          if (invT > 0) parts.push(t("Knowledge.k117", { invT: invT }));
          if (invE > 0) parts.push(t("Knowledge.k118", { invE: invE }));
          core.showStatus('error', parts.join('，'));
        }
      }
    } catch {
      core.showStatus('error', t("Knowledge.k119"));
    } finally {
      setPathChecking(false);
    }
  };

  const handleDirectoryScan = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
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
      const res = await kb.kbScanDirectory({ folderPath: pathStr });
      if (res.code === 0 && res.data) {
        setDirScanFiles(res.data);
      } else {
        core.showStatus('error', res.message || t("Knowledge.k121"));
        setShowDirScanModal(false);
      }
    } catch (err) {
      core.showStatus('error', t("Knowledge.k122", { arg0: err instanceof Error ? err.message : String(err) }));
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
      core.showStatus('error', t("Knowledge.k123"));
      return;
    }
    const targetCategoryId = core.selectedId?.type === 'category' ? core.selectedId.id : core.categories[0]?.id ?? null;
    if (!targetCategoryId) {
      core.showStatus('error', t("Knowledge.k124"));
      return;
    }
    setDirImporting(true);
    const selectedIndices = Array.from(dirScanSelected);
    const selectedFiles = selectedIndices.map(i => filteredDirScanFiles[i]);
    const filePaths = selectedFiles.map(f => f.path);
    try {
      const res = await kb.kbAddScannedFiles({
        filePaths,
        categoryId: targetCategoryId,
        sourcePath: dirScanPath
      });
      if (res.code === 0 && res.data) {
        const imported = res.data.added.length;
        const skipped = res.data.total - imported;
        core.showStatus('success', t("Knowledge.k125", {
          imported: imported,
          arg0: skipped > 0 ? t("Knowledge.k126", { skipped: skipped }) : ''
        }));
      } else {
        core.showStatus('error', res.message || t("Knowledge.k99"));
      }
    } catch (err: any) {
      core.showStatus('error', t("Knowledge.k127", { arg0: err.message || err }));
    }
    setShowDirScanModal(false);
    core.reload();
    setDirImporting(false);
    intelligence.logActivity('user', new Date().toISOString(), 'knowledge', 'import_files', `${selectedFiles.length} files from ${dirScanPath}`).catch(() => {});
  };

  /** 侧边栏工具栏：导入相关按钮 */
  const renderImportToolbarButtons = () => (
    <>
      <button className={styles.toolIconBtn} onClick={handleImportFile} title={t("components.intelligence.ActivityPanel.k16")}>
        📄
      </button>
      <button className={styles.toolIconBtn} onClick={handleDirectoryScan} title={t("Knowledge.k154")}>
        🔍
      </button>
      <button className={styles.toolIconBtnPrimary} onClick={handleImportFolder} disabled={isImporting} title={t("Knowledge.k155")}>
        📥
      </button>
      <button className={styles.toolIconBtn} onClick={toggleTrackedPaths} title={t("Knowledge.k156")} style={showTrackedPaths ? { color: '#FFC107', borderColor: '#FFC107' } : undefined}>
        📌 {trackedPaths.length > 0 && <span style={{ fontSize: 9, background: '#FFC107', color: '#000', borderRadius: '50%', padding: '0 3px', marginLeft: 2 }}>{trackedPaths.length}</span>}
      </button>
    </>
  );

  /** 侧边栏追踪路径面板 */
  const renderTrackedPanel = () => {
    if (!showTrackedPaths) return null;
    return (
      <div className={styles.aiCategoryPanel}>
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 6, alignItems: 'center' }}>
          <span style={{ fontSize: 11, color: '#FFC107', fontFamily: 'var(--nt-font-mono)' }}>{t("Knowledge.k159")}</span>
          <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
            <button onClick={handleCheckPaths} disabled={pathChecking} style={{ background: 'none', border: '1px solid rgba(0,240,255,0.3)', color: '#00F0FF', cursor: pathChecking ? 'default' : 'pointer', fontSize: 10, padding: '1px 6px', borderRadius: 3 }} title={t("Knowledge.k160")}>{pathChecking ? '⏳' : t("Knowledge.k161")}</button>
            <button onClick={() => { setShowTrackedPaths(false); setPathCheckResult(null); }} style={{ background: 'none', border: 'none', color: '#FF5050', cursor: 'pointer', fontSize: 11 }}>✕</button>
          </div>
        </div>
        {pathCheckResult && (pathCheckResult.invalid_tracked.length > 0 || pathCheckResult.invalid_external_entries.length > 0) && <div style={{ fontSize: 10, color: '#FF5050', marginBottom: 6, padding: '2px 6px', background: 'rgba(255,80,80,0.1)', borderRadius: 3 }}>
            ⚠ {pathCheckResult.invalid_tracked.length > 0 && t("Knowledge.k162", { length: pathCheckResult.invalid_tracked.length })}
            {pathCheckResult.invalid_external_entries.length > 0 && t("Knowledge.k163", { length: pathCheckResult.invalid_external_entries.length })}
          </div>}
        {trackedPathsLoading ? <div style={{ fontSize: 11, color: '#888' }}>{t("common.loading")}</div> : trackedPaths.length === 0 ? <div style={{ fontSize: 11, color: '#888' }}>{t("Knowledge.k164")}</div> : <div style={{ display: 'flex', flexDirection: 'column', gap: 4, maxHeight: 200, overflowY: 'auto' }}>
            {trackedPaths.map(tp => {
              const isInvalid = pathCheckResult?.invalid_tracked.includes(tp.id);
              return <div key={tp.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '4px 6px', background: isInvalid ? 'rgba(255,80,80,0.15)' : 'rgba(255,193,7,0.08)', borderRadius: 4, fontSize: 11, border: isInvalid ? '1px solid rgba(255,80,80,0.3)' : 'none' }}>
                  <div style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', color: isInvalid ? '#FF6B6B' : 'rgba(200,200,220,0.85)' }}>
                    {isInvalid && <span style={{ marginRight: 4 }}>⚠️</span>}
                    <span style={{ color: isInvalid ? '#FF6B6B' : 'rgba(255,255,255,0.4)', marginRight: 6 }}>
                      {time.formatCompact(tp.last_imported_at)}
                    </span>
                    {tp.path}
                  </div>
                  <div style={{ display: 'flex', gap: 4, flexShrink: 0, marginLeft: 8 }}>
                    {isInvalid && <span style={{ fontSize: 9, color: '#FF5050' }}>{t("Knowledge.k165")}</span>}
                    <button onClick={() => handleReimportTracked(tp)} disabled={isImporting} style={{ background: 'none', border: isInvalid ? '1px solid rgba(255,80,80,0.4)' : '1px solid rgba(0,240,255,0.3)', color: isInvalid ? '#FF6B6B' : '#00F0FF', cursor: isImporting ? 'default' : 'pointer', fontSize: 10, padding: '1px 6px', borderRadius: 3 }} title={t("Knowledge.k166")}>🔄</button>
                    <button onClick={() => handleRemoveTrackedPath(tp.id)} style={{ background: 'none', border: '1px solid rgba(255,80,80,0.3)', color: '#FF5050', cursor: 'pointer', fontSize: 10, padding: '1px 6px', borderRadius: 3 }} title={t("Knowledge.k167")}>✕</button>
                  </div>
                </div>;
            })}
          </div>}
      </div>
    );
  };

  /** 目录扫描 Modal */
  const renderDirScanModal = () => {
    if (!showDirScanModal) return null;
    return (
      <div className={styles.modalOverlay} onClick={() => setShowDirScanModal(false)}>
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
                    return totalBytes > 0 ? core.formatFileSize(totalBytes) : '';
                  })()}
                </span>
              </div>

              {ai.renderAiClassifyToolbar(filteredDirScanFiles.map(f => f.name))}

              <div className={styles.scanFileList}>
                {filteredDirScanFiles.length === 0 ? <div className={styles.scanEmpty}>{t("Knowledge.k287")}</div> : filteredDirScanFiles.map((file, idx) => {
                  const isSelected = dirScanSelected.has(idx);
                  return <label key={idx} className={`${styles.scanFileItem} ${isSelected ? styles.scanFileItemSel : ''}`}>
                    <input type="checkbox" checked={isSelected} onChange={() => toggleDirScanSelect(idx)} />
                    <span className={styles.scanFileIcon}>
                      {file.file_type === 'image' ? '🖼' : file.file_type === 'video' ? '🎬' : file.file_type === 'audio' ? '🎵' : file.file_type === 'text' ? '📝' : file.file_type === 'document' ? '📑' : '📄'}
                    </span>
                    <span className={styles.scanFileName}>{file.name}</span>
                    <span className={styles.scanFileSize}>{core.formatFileSize(file.size_bytes)}</span>
                  </label>;
                })}
              </div>

              <div className={styles.modalActions} style={{ marginTop: 12 }}>
                <button className={styles.btnCancel} onClick={() => setShowDirScanModal(false)}>{t("common.cancel")}</button>
                <button className={styles.btnPrimary} onClick={handleBatchImportSelected} disabled={dirScanSelected.size === 0 || dirImporting}>
                  {dirImporting ? t("Knowledge.k288") : t("Knowledge.k289", { size: dirScanSelected.size })}
                </button>
              </div>
            </>}
        </div>
      </div>
    );
  };

  return {
    renderImportToolbarButtons,
    renderTrackedPanel,
    renderDirScanModal,
    setImporting: setIsImporting,
    openDirScan: handleDirectoryScan,
    closeDirScanModal: () => setShowDirScanModal(false)
  };
}
