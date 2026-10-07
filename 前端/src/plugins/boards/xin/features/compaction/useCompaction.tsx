// xin.compaction L2 —— 上下文压缩功能域（局部 state + handlers + JSX）。
// compactionConfig/checkCompactionNeed/triggerCompactionAuto/needsCompaction 等核心项经 core 注入。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { xinOrchestration } from '../../ipc';
import { time } from '@/lib/utils';
import styles from '../../Xin.module.css';
import type { CompactionRecord } from '../../xin/types';
import type { XinCore } from '../../core';

export function useCompaction(core: XinCore, active: boolean) {
  const {
    activeConversationId,
    messages,
    selectedModel,
    loadConversations,
    compactionConfig,
    updateCompactionConfig,
    checkCompactionNeed,
    triggerCompactionAuto,
    needsCompaction
  } = core;

  const [compactionRecords, setCompactionRecords] = useState<CompactionRecord[]>([]);

  const loadCompactionRecords = async () => {
    try {
      const res = await xinOrchestration.compactionGetRecords(activeConversationId || '', 30);
      if (res?.data) setCompactionRecords(res.data);
    } catch {/* silent */}
  };
  const triggerCompactionManual = async () => {
    if (!activeConversationId) return;
    try {
      const contextJson = JSON.stringify(messages.map(m => ({
        id: m.id,
        role: m.role,
        content: m.content
      })));
      await xinOrchestration.compactionManual(activeConversationId, contextJson, String(selectedModel?.id ?? ''));
      loadCompactionRecords();
      loadConversations();
    } catch {/* silent */}
  };

  useEffect(() => {
    if (active && compactionRecords.length === 0) loadCompactionRecords();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const renderCompaction = () => <div className={styles.tabContent}>
      {compactionConfig && <div className={styles.card}>
          <div className={styles.panelTitle}>{t("Xin.k59")}</div>
          <div className={styles.settingsGrid}>
            <div className={styles.settingItem}>
              <span className={styles.inputLabel}>{t("Xin.k60")}</span>
              <button className={compactionConfig.enabled ? styles.toggleOn : styles.toggleOff} onClick={() => updateCompactionConfig({
          enabled: !compactionConfig.enabled
        })}>
                {compactionConfig.enabled ? 'ON' : 'OFF'}
              </button>
            </div>
            <div className={styles.settingItem}>
              <span className={styles.inputLabel}>{t("Xin.k61")}</span>
              <span>{(compactionConfig.trigger_threshold_ratio * 100).toFixed(0)}%</span>
            </div>
            <div className={styles.settingItem}>
              <span className={styles.inputLabel}>{t("Xin.k62")}</span>
              <span>{compactionConfig.keep_recent_tokens.toLocaleString()}</span>
            </div>
            <div className={styles.settingItem}>
              <span className={styles.inputLabel}>{t("Xin.k63")}</span>
              <span className={styles.badgeOn}>
                {compactionConfig.compaction_mode === 'sliding_window' ? t("Xin.k64") : t("Xin.k65")}
              </span>
            </div>
            <div className={styles.settingItem}>
              <span className={styles.inputLabel}>{t("Xin.k66")}</span>
              <button className={compactionConfig.memory_flush_enabled ? styles.toggleOn : styles.toggleOff} onClick={() => updateCompactionConfig({
          memory_flush_enabled: !compactionConfig.memory_flush_enabled
        })}>
                {compactionConfig.memory_flush_enabled ? 'ON' : 'OFF'}
              </button>
            </div>
            <div className={styles.settingItem}>
              <span className={styles.inputLabel}>{t("Xin.k67")}</span>
              <button className={compactionConfig.notify_user ? styles.toggleOn : styles.toggleOff} onClick={() => updateCompactionConfig({
          notify_user: !compactionConfig.notify_user
        })}>
                {compactionConfig.notify_user ? 'ON' : 'OFF'}
              </button>
            </div>
          </div>
        </div>}

      {!compactionConfig && <div className={styles.card}>
          <div className={styles.panelTitle}>{t("Xin.k59")}</div>
          <p style={{
      color: '#5a5a5a'
    }}>{t("Xin.k68")}</p>
        </div>}

      <div className={styles.card}>
        <div className={styles.panelTitle}>{t("Xin.k69")}</div>
        <div style={{
      display: 'flex',
      gap: '8px',
      flexWrap: 'wrap'
    }}>
          <button className={styles.btn} onClick={checkCompactionNeed} disabled={!activeConversationId}>
            {t("Xin.k70")}
          </button>
          <button className={styles.btn} onClick={triggerCompactionAuto} disabled={!activeConversationId}>
            {t("Xin.k71")}
          </button>
          <button className={styles.btn} onClick={triggerCompactionManual} disabled={!activeConversationId}>
            {t("Xin.k72")}
          </button>
        </div>
        {!activeConversationId && <p style={{
      fontSize: '10px',
      color: '#6a6a8a',
      marginTop: '8px'
    }}>
            {t("Xin.k73")}
          </p>}
        {needsCompaction && <p style={{
      fontSize: '10px',
      color: '#FFD700',
      marginTop: '8px'
    }}>
            {t("Xin.k74")}
          </p>}
      </div>

      <div className={styles.card}>
        <div className={styles.panelTitle}>{t("Xin.k75")}</div>
        {compactionRecords.length === 0 && <p style={{
      color: '#5a5a5a',
      textAlign: 'center',
      padding: '20px'
    }}>
            {t("Xin.k76")}
          </p>}
        {compactionRecords.map(r => <div key={r.id} className={styles.compactionRecord}>
            <div className={styles.compactionRecordHeader}>
              <span className={styles.compactionTrigger}>
                {r.trigger_type === 'auto' ? t("Xin.k77") : t("Xin.k78")}
              </span>
              <span className={styles.compactionTime}>{time.formatCompact(r.created_at)}</span>
            </div>
            <div className={styles.compactionStats}>
              <span>{t("Xin.k79")} {r.pre_message_count} → {r.post_message_count}</span>
              <span>Token: {r.pre_token_count.toLocaleString()} → {r.post_token_count.toLocaleString()}</span>
              <span>{t("Xin.k80")} {r.memory_flush_count} {t("ai.ChatPanel.k19")}</span>
            </div>
            {r.summary_text && <div className={styles.compactionSummary}>
                <span className={styles.summaryLabel}>{t("Linux.k101")}</span>
                <span>{r.summary_text.slice(0, 120)}{r.summary_text.length > 120 ? '...' : ''}</span>
              </div>}
            {r.guidance_text && <div className={styles.compactionGuidance}>
                <span className={styles.summaryLabel}>{t("Xin.k81")}</span>
                <span>"{r.guidance_text}"</span>
              </div>}
          </div>)}
      </div>
    </div>;

  return { renderCompaction };
}
