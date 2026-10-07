// xin.dream L2 —— 梦境记忆整合功能域（局部 state + handlers + JSX）。
// dream 需要 memories：原实现复用 memory tab 的 memories，拆后本 hook 自持（自行 xin.getMemories 加载）。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { xin, xinOrchestration } from '../../ipc';
import { time } from '@/lib/utils';
import styles from '../../Xin.module.css';
import type { Memory, DreamConfig, DreamState, DreamResult } from '../../xin/types';
import type { XinCore } from '../../core';

export function useDream(core: XinCore, active: boolean) {
  const { messages } = core;
  const [memories, setMemories] = useState<Memory[]>([]);
  const [dreamConfig, setDreamConfig] = useState<DreamConfig | null>(null);
  const [dreamState, setDreamState] = useState<DreamState | null>(null);
  const [dreamResult, setDreamResult] = useState<DreamResult | null>(null);
  const [dreamLoading, setDreamLoading] = useState(false);

  const loadMemoriesForDream = async (): Promise<Memory[]> => {
    try {
      const res = await xin.getMemories(50);
      if (res?.data && Array.isArray(res.data)) {
        setMemories(res.data);
        return res.data;
      }
    } catch {/* silent */}
    return [];
  };

  const loadDream = async () => {
    setDreamLoading(true);
    try {
      const mems = await loadMemoriesForDream();
      const [cfgRes, stateRes] = await Promise.all([xinOrchestration.dreamDefaultConfig(), xinOrchestration.dreamCalcHealth(mems)]);
      if (cfgRes?.data) setDreamConfig(cfgRes.data);
      if (stateRes?.data) setDreamState(stateRes.data);
    } catch {/* silent */} finally {
      setDreamLoading(false);
    }
  };
  const runDream = async (phase: 'light' | 'deep' | 'rem') => {
    setDreamLoading(true);
    try {
      let res: any;
      const dreamCfg = dreamConfig ?? await xinOrchestration.dreamDefaultConfig().then(r => r?.data ?? null);
      if (phase === 'light') {
        const texts = messages.map(m => `${m.role} ${m.content}`);
        res = await xinOrchestration.dreamRunLight(texts, dreamCfg);
      } else if (phase === 'deep') {
        const candidates = (dreamState as any)?.light_candidates_count ? (dreamState as any).light_candidates_slice || [] : [];
        res = await xinOrchestration.dreamRunDeep(candidates, memories, dreamCfg);
      } else {
        res = await xinOrchestration.dreamRunRem(memories, dreamCfg);
      }
      if (res?.data) setDreamResult(res.data);
      await loadDream();
    } catch {/* silent */} finally {
      setDreamLoading(false);
    }
  };

  useEffect(() => {
    if (active && !dreamConfig) loadDream();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const renderDream = () => <div className={styles.tabContent}>
      {dreamLoading ? <div className={styles.loading}>{t("Xin.k82")}</div> : <>
          {dreamConfig && <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k83")}</div>
              <div className={styles.settingsGrid}>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k84")}</span>
                  <span className={dreamConfig.enabled ? styles.badgeOn : styles.badgeOff}>
                    {dreamConfig.enabled ? t("Linux.k46") : t("Xin.k85")}
                  </span>
                </div>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k86")}</span>
                  <span>{dreamConfig.phases.light.interval_hours}h</span>
                </div>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k87")}</span>
                  <span>{dreamConfig.phases.deep.interval_hours}h</span>
                </div>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k88")}</span>
                  <span>{dreamConfig.phases.rem.interval_hours}h</span>
                </div>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k89")}</span>
                  <span>{dreamConfig.phases.light.max_candidates}</span>
                </div>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k90")}</span>
                  <span>{(dreamConfig.phases.deep.min_score * 100).toFixed(0)}%</span>
                </div>
              </div>
            </div>}

          {dreamState && <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k91")}</div>
              <div className={styles.settingsGrid}>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k92")}</span>
                  <span className={styles.badgeOn}>{dreamState.light_candidates_count}</span>
                </div>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k93")}</span>
                  <span className={styles.badgeOn}>{dreamState.deep_promotions_count}</span>
                </div>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k94")}</span>
                  <span className={styles.badgeOn}>{dreamState.rem_patterns_count}</span>
                </div>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k95")}</span>
                  <span>{dreamState.last_light_at ? time.formatCompact(dreamState.last_light_at) : t("Xin.k96")}</span>
                </div>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k97")}</span>
                  <span>{dreamState.last_deep_at ? time.formatCompact(dreamState.last_deep_at) : t("Xin.k96")}</span>
                </div>
                <div className={styles.settingItem}>
                  <span className={styles.inputLabel}>{t("Xin.k98")}</span>
                  <span>{dreamState.last_rem_at ? time.formatCompact(dreamState.last_rem_at) : t("Xin.k96")}</span>
                </div>
              </div>
            </div>}

          <div className={styles.card}>
            <div className={styles.panelTitle}>{t("Xin.k99")}</div>
            <div style={{
        display: 'flex',
        gap: '8px',
        flexWrap: 'wrap'
      }}>
              <button className={styles.btn} onClick={() => runDream('light')} disabled={dreamLoading}>
                {t("Xin.k100")}
              </button>
              <button className={styles.btn} onClick={() => runDream('deep')} disabled={dreamLoading}>
                {t("Xin.k101")}
              </button>
              <button className={styles.btn} onClick={() => runDream('rem')} disabled={dreamLoading}>
                {t("Xin.k102")}
              </button>
            </div>
          </div>

          {dreamResult && <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k103")}</div>
              {dreamResult.candidates && <div>
                  <span style={{
          fontSize: '10px',
          color: '#6a6a8a'
        }}>
                    {t("Xin.k104")} {dreamResult.candidates.length} {t("Xin.k105")} {dreamResult.deduped_count}{t("Xin.k106")} {dreamResult.sources_processed}, {dreamResult.duration_ms}ms)
                  </span>
                  <div className={styles.dreamCandidates}>
                    {dreamResult.candidates.slice(0, 10).map((c, i) => <div key={i} className={styles.dreamCandidateItem}>
                        <span className={styles.dreamCandidateKey}>{c.key}</span>
                        <span className={styles.dreamCandidateValue}>{c.value.slice(0, 80)}</span>
                        <div className={styles.dreamCandidateMeta}>
                          <span>{c.category}</span>
                          <span>{t("Xin.k107")} {(c.confidence * 100).toFixed(0)}%</span>
                          <span>{t("Xin.k108")} {c.importance.toFixed(2)}</span>
                        </div>
                      </div>)}
                  </div>
                </div>}
              {dreamResult.promoted !== undefined && <div style={{
        fontSize: '11px',
        color: '#c8c8dd'
      }}>
                  {t("Xin.k109")} {dreamResult.promoted} {t("Xin.k110")} {((dreamResult.health_score || 0) * 100).toFixed(0)}{t("Xin.k111")} {dreamResult.health_status}
                </div>}
              {dreamResult.pattern_count !== undefined && <div style={{
        fontSize: '11px',
        color: '#c8c8dd'
      }}>
                  {t("Xin.k112")} {dreamResult.pattern_count} {t("Xin.k113")} {dreamResult.cross_domain_links?.length || 0}
                </div>}
            </div>}

          {!dreamConfig && !dreamLoading && <div className={styles.card}>
              <p style={{
        color: '#6a6a8a',
        textAlign: 'center'
      }}>
                {t("Xin.k114")}
              </p>
            </div>}
        </>}
    </div>;

  return { renderDream };
}
