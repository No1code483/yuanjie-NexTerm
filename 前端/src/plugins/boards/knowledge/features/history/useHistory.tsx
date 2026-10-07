// knowledge.history L2 功能域：快照与反向链接。
// 局部 state + handlers + JSX 物理落在本目录；核心共享数据经 KnowledgeCore 注入。
import { t } from "i18next";
import { useCallback, useEffect, useState } from 'react';
import { kb } from '../../ipc';
import styles from '../../Knowledge.module.css';
import type { KbEntry, KnowledgeCore } from '../../knowledge/types';

export function useHistory(core: KnowledgeCore, handleOpenEntry: (entry: KbEntry) => void) {
  const [backlinks, setBacklinks] = useState<Array<{ entry: KbEntry; snippet: string }>>([]);
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
  const [snapshotPreview, setSnapshotPreview] = useState<{ id: number; content: string } | null>(null);

  const loadBacklinks = useCallback(async (entryId: number) => {
    if (!entryId) {
      setBacklinks([]);
      return;
    }
    setBacklinksLoading(true);
    try {
      const res = await kb.kbGetBacklinks({ entryId });
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

  const selectedId = core.selectedId;
  const selectedEntry = selectedId?.type === 'entry' ? core.allEntries.find(e => e.id === selectedId.id) ?? null : null;

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
      const res = await kb.kbGetSnapshots({ entryId });
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
      const res = await kb.kbRestoreSnapshot({ snapshotId });
      if (res.code === 0 && res.data) {
        await core.reload();
        setShowSnapshots(false);
        setSnapshotPreview(null);
        await loadSnapshots(res.data.id);
        core.showStatus('success', t("Knowledge.k129"));
      } else {
        core.showStatus('error', res.message || t("Knowledge.k130"));
      }
    } catch (e: any) {
      core.showStatus('error', t("Knowledge.k131", { e: e }));
    }
  };

  /** 反向链接面板（预览元信息内） */
  const renderBacklinks = () => {
    if (backlinksLoading) return <div className={styles.backlinkHint}>{t("Knowledge.k237")}</div>;
    if (backlinks.length === 0) return null;
    return (
      <div className={styles.backlinkPanel}>
        <div className={styles.backlinkTitle}>{t("Knowledge.k238")} {backlinks.length} {t("Knowledge.k239")}</div>
        <div className={styles.backlinkList}>
          {backlinks.map((bl, i) => <div key={i} className={styles.backlinkItem} onClick={() => handleOpenEntry(bl.entry)} title={bl.snippet}>
              <span className={styles.backlinkItemIcon}>{core.getTypeIcon(bl.entry.entry_type)}</span>
              <span className={styles.backlinkItemName}>{bl.entry.name}</span>
              <span className={styles.backlinkItemSnippet}>{bl.snippet.slice(0, 60)}{bl.snippet.length > 60 ? '...' : ''}</span>
            </div>)}
        </div>
      </div>
    );
  };

  /** 快照面板（预览元信息内） */
  const renderSnapshots = (entry: KbEntry) => (
    <>
      <div className={styles.snapshotToggle}>
        <button className={styles.btnSm} onClick={() => {
          if (!showSnapshots) {
            loadSnapshots(entry.id);
          }
          setShowSnapshots(!showSnapshots);
          setSnapshotPreview(null);
        }}>
          {showSnapshots ? t("Knowledge.k240") : t("Knowledge.k241", { arg0: snapshots.length > 0 ? `(${snapshots.length})` : '' })}
        </button>
      </div>
      {showSnapshots && <div className={styles.snapshotPanel}>
          {snapshotsLoading ? <div className={styles.snapshotHint}>{t("Knowledge.k242")}</div> : snapshots.length === 0 ? <div className={styles.snapshotHint}>{t("Knowledge.k243")}</div> : <>
              <div className={styles.snapshotTitle}>{t("Knowledge.k244")} {snapshots.length} {t("components.Linux.k18")}</div>
              <div className={styles.snapshotList}>
                {snapshots.map(snap => <div key={snap.id} className={`${styles.snapshotItem} ${snapshotPreview?.id === snap.id ? styles.snapshotItemActive : ''}`}>
                    <div className={styles.snapshotMeta}>
                      <span className={styles.snapshotTime}>{core.formatTime(snap.created_at)}</span>
                      <span className={styles.snapshotSize}>{core.formatFileSize(snap.content_length)}</span>
                    </div>
                    <div className={styles.snapshotActions}>
                      <button className={styles.btnSmXs} onClick={() => setSnapshotPreview(snapshotPreview?.id === snap.id ? null : { id: snap.id, content: snap.content })}>
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
                {t("Knowledge.k246")} {core.formatTime(snapshotPreview.id > 0 ? snapshots.find(s => s.id === snapshotPreview.id)?.created_at || 0 : 0)}
              </div>
              <pre className={styles.snapshotPreviewContent}>{snapshotPreview.content}</pre>
            </div>}
        </div>}
    </>
  );

  return {
    renderBacklinks,
    renderSnapshots
  };
}
