// xin.checkpoint L2 —— 会话检查点功能域（局部 state + handlers + JSX）。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { xinOrchestration } from '../../ipc';
import { time } from '@/lib/utils';
import styles from '../../Xin.module.css';
import type { CheckpointSummary } from '../../xin/types';
import type { XinCore } from '../../core';

export function useCheckpoint(core: XinCore, active: boolean) {
  const { activeConversationId, switchConversation } = core;
  const [checkpoints, setCheckpoints] = useState<CheckpointSummary[]>([]);
  const [checkpointLoading, setCheckpointLoading] = useState(false);

  const loadCheckpoints = async () => {
    if (!activeConversationId) return;
    setCheckpointLoading(true);
    try {
      const res = await xinOrchestration.checkpointList(activeConversationId);
      if (res?.data) setCheckpoints(res.data);
    } catch {/* silent */} finally {
      setCheckpointLoading(false);
    }
  };
  const saveCheckpoint = async () => {
    if (!activeConversationId) return;
    try {
      await xinOrchestration.checkpointSave(activeConversationId, t("Xin.k6", {
        arg0: new Date().toLocaleTimeString('zh-CN')
      }));
      loadCheckpoints();
    } catch {/* silent */}
  };
  const restoreCheckpoint = async (checkpointId: string) => {
    try {
      await xinOrchestration.checkpointRestore(checkpointId);
      if (activeConversationId) switchConversation(activeConversationId);
      loadCheckpoints();
    } catch {/* silent */}
  };
  const deleteCheckpoint = async (checkpointId: string) => {
    try {
      await xinOrchestration.checkpointDelete(checkpointId);
      setCheckpoints(prev => prev.filter(c => c.id !== checkpointId));
    } catch {/* silent */}
  };

  useEffect(() => {
    if (active && checkpoints.length === 0) loadCheckpoints();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const renderCheckpoint = () => <div className={styles.tabContent}>
      <div style={{
      display: 'flex',
      gap: '8px',
      marginBottom: '12px'
    }}>
        <button className={styles.btn} onClick={saveCheckpoint} disabled={!activeConversationId}>
          {t("Xin.k115")}
        </button>
        <button className={styles.btnSmall} onClick={loadCheckpoints}>
          {t("common.refresh")}
        </button>
      </div>

      {!activeConversationId && <div className={styles.card}>
          <p style={{
        color: '#6a6a8a',
        textAlign: 'center'
      }}>
            {t("Xin.k116")}
          </p>
        </div>}

      {activeConversationId && checkpoints.length === 0 && !checkpointLoading && <div className={styles.card}>
          <p style={{
        color: '#6a6a8a',
        textAlign: 'center'
      }}>{t("Xin.k117")}</p>
        </div>}

      {checkpointLoading && <div className={styles.loading}>{t("Xin.k118")}</div>}

      {checkpoints.map(cp => <div key={cp.id} className={styles.card} style={{
      marginBottom: '8px'
    }}>
          <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: '6px'
      }}>
            <span style={{
          fontSize: '12px',
          color: '#c8c8dd'
        }}>
              {cp.title || t("Xin.k6", {
            arg0: time.formatCompact(cp.created_at)
          })}
            </span>
            <span className={cp.checkpoint_type === 'auto' ? styles.badgeOn : styles.badgeOn} style={cp.checkpoint_type === 'user' ? {
          background: 'rgba(255,180,84,0.1)',
          border: '1px solid #FFD700',
          color: '#FFD700'
        } : {}}>
              {cp.checkpoint_type === 'auto' ? t("Xin.k119") : t("components.FloatingBall.k67")}
            </span>
          </div>
          <div style={{
        fontSize: '10px',
        color: '#6a6a8a',
        marginBottom: '8px'
      }}>
            {t("Xin.k79")} {cp.message_count} | Token: {cp.total_tokens.toLocaleString()} | {time.formatCompact(cp.created_at)}
          </div>
          <div style={{
        display: 'flex',
        gap: '6px'
      }}>
            <button className={styles.btnSmall} onClick={() => restoreCheckpoint(cp.id)}>
              {t("Xin.k120")}
            </button>
            <button className={styles.btnDangerSmall} onClick={() => deleteCheckpoint(cp.id)}>
              {t("common.delete")}
            </button>
          </div>
        </div>)}
    </div>;

  return { renderCheckpoint };
}
