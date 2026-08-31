// ConflictResolver.tsx — A5.2.6.4 手动冲突解决 UI
//
// 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.6.4
//
// 功能：
// - 列出所有 sync_status='conflict' 的队列记录
// - 展示本地 vs 远端 payload（JSON）
// - 三种解决方案：保留本地 / 采用远端 / 合并（手动编辑）
// - 解决后调用 sync_resolve_conflict 重新入队 pending
//
// 注：「远端 payload」当前实现为同一记录的 last_error 字段中嵌入的远端快照
//     （由后端 sync_service::mark_conflict 写入，格式为 "remote:<json>"）。
//     若 last_error 不含 remote: 前缀，则仅显示本地版本。

import { useEffect, useState, useCallback } from 'react';
import { useSyncStore } from '@/stores/syncStore';
import type { SyncQueueItem } from '@/lib/ipc';
import styles from './ConflictResolver.module.css';

interface ConflictResolverProps {
  /** 是否显示 */
  isOpen: boolean;
  /** 关闭回调 */
  onClose: () => void;
}

/**
 * 从 last_error 字段中提取远端 payload
 * 后端 mark_conflict 时写入格式："remote:<json>"
 */
function extractRemotePayload(item: SyncQueueItem): string | null {
  if (!item.last_error) return null;
  const prefix = 'remote:';
  if (item.last_error.startsWith(prefix)) {
    return item.last_error.slice(prefix.length);
  }
  return null;
}

/** 美化 JSON 字符串（失败则原样返回） */
function prettyJson(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

export default function ConflictResolver({ isOpen, onClose }: ConflictResolverProps) {
  const {
    conflicts,
    refreshConflicts,
    resolveConflict,
    lastError,
  } = useSyncStore();

  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [mergedPayload, setMergedPayload] = useState('');
  const [submitting, setSubmitting] = useState(false);

  // 弹窗打开时拉取冲突列表
  useEffect(() => {
    if (isOpen) {
      void refreshConflicts();
    }
  }, [isOpen, refreshConflicts]);

  // ESC 关闭
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [isOpen, onClose]);

  // 选中冲突项时初始化合并编辑器
  const selectedItem: SyncQueueItem | null = selectedId
    ? conflicts.find((c) => c.id === selectedId) ?? null
    : null;

  useEffect(() => {
    if (selectedItem) {
      const remote = extractRemotePayload(selectedItem);
      // 默认合并值为本地 payload（用户可在编辑器中修改）
      setMergedPayload(prettyJson(selectedItem.payload));
      void remote;
    }
  }, [selectedItem]);

  const handleResolve = useCallback(
    async (resolution: 'local' | 'remote' | 'merged') => {
      if (!selectedItem) return;
      setSubmitting(true);
      const payload = resolution === 'merged' ? mergedPayload : undefined;
      const ok = await resolveConflict(selectedItem.id, resolution, payload);
      setSubmitting(false);
      if (ok) {
        setSelectedId(null);
        setMergedPayload('');
      }
    },
    [selectedItem, mergedPayload, resolveConflict],
  );

  if (!isOpen) return null;

  return (
    <div className={styles.overlay} onClick={onClose}>
      <div className={styles.dialog} onClick={(e) => e.stopPropagation()}>
        {/* 标题栏 */}
        <div className={styles.header}>
          <h2 className={styles.title}>
            <span>{'<Sync Conflict />'}</span>
            {conflicts.length > 0 && (
              <span className={styles.badge}>{conflicts.length}</span>
            )}
          </h2>
          <button className={styles.closeBtn} onClick={onClose} aria-label="close">
            ×
          </button>
        </div>

        {/* 主体 */}
        <div className={styles.body}>
          <div className={styles.toolbar}>
            <span>
              共 {conflicts.length} 条冲突 · 点击条目查看详情并选择解决方案
            </span>
            <button
              className={styles.refreshBtn}
              onClick={() => void refreshConflicts()}
            >
              ↻ 刷新
            </button>
          </div>

          {lastError && <div className={styles.errorMsg}>{lastError}</div>}

          {conflicts.length === 0 ? (
            <div className={styles.empty}>暂无冲突记录</div>
          ) : (
            <>
              {/* 冲突列表 */}
              <div className={styles.conflictList}>
                {conflicts.map((item) => {
                  const op = (item.operation || 'UPDATE').toUpperCase();
                  return (
                    <div
                      key={item.id}
                      className={`${styles.conflictItem} ${
                        selectedId === item.id ? styles.active : ''
                      }`}
                      onClick={() => setSelectedId(item.id)}
                    >
                      <div className={styles.conflictHeader}>
                        <div className={styles.conflictMeta}>
                          <span className={styles.tableName}>
                            {item.table_name}
                          </span>
                          <span className={styles.recordId}>
                            #{item.record_id}
                          </span>
                          <span className={`${styles.operation} ${styles[op] || ''}`}>
                            {op}
                          </span>
                        </div>
                        <span className={styles.conflictTime}>
                          {new Date(item.created_at).toLocaleString()}
                        </span>
                      </div>
                      {item.last_error && (
                        <div
                          style={{
                            fontSize: 11,
                            color: '#888899',
                            whiteSpace: 'pre-wrap',
                            wordBreak: 'break-all',
                            maxHeight: 40,
                            overflow: 'hidden',
                          }}
                        >
                          {item.last_error.slice(0, 200)}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>

              {/* 版本对比 + 解决方案 */}
              {selectedItem && (
                <div className={styles.diffSection}>
                  <div className={styles.diffHeader}>版本对比</div>
                  <div className={styles.diffGrid}>
                    <div className={styles.diffCol}>
                      <div className={`${styles.diffColTitle} ${styles.local}`}>
                        本地版本
                      </div>
                      <div className={styles.diffBody}>
                        {prettyJson(selectedItem.payload)}
                      </div>
                    </div>
                    <div className={styles.diffCol}>
                      <div className={`${styles.diffColTitle} ${styles.remote}`}>
                        远端版本
                      </div>
                      <div className={styles.diffBody}>
                        {(() => {
                          const remote = extractRemotePayload(selectedItem);
                          return remote ? prettyJson(remote) : '（远端快照未记录）';
                        })()}
                      </div>
                    </div>
                  </div>

                  {/* 合并编辑器 */}
                  <div className={styles.diffHeader}>合并版本（可编辑）</div>
                  <textarea
                    className={styles.mergeEditor}
                    value={mergedPayload}
                    onChange={(e) => setMergedPayload(e.target.value)}
                    placeholder="编辑合并后的 JSON payload..."
                  />

                  {/* 操作按钮 */}
                  <div className={styles.actions}>
                    <button
                      className={`${styles.btn} ${styles.btnLocal}`}
                      onClick={() => void handleResolve('local')}
                      disabled={submitting}
                    >
                      保留本地
                    </button>
                    <button
                      className={`${styles.btn} ${styles.btnRemote}`}
                      onClick={() => void handleResolve('remote')}
                      disabled={submitting}
                    >
                      采用远端
                    </button>
                    <button
                      className={`${styles.btn} ${styles.btnMerge}`}
                      onClick={() => void handleResolve('merged')}
                      disabled={submitting || !mergedPayload.trim()}
                    >
                      应用合并
                    </button>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
