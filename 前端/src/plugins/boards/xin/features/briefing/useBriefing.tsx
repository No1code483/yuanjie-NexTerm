// xin.briefing L2 —— 每日简报功能域（局部 state + handlers + JSX）。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { xin } from '../../ipc';
import styles from '../../Xin.module.css';
import { getMockBriefing } from '../../xin/mockData';
import type { XinCore } from '../../core';

export function useBriefing(_core: XinCore, active: boolean) {
  const [briefing, setBriefing] = useState<any>(null);
  const [briefingLoading, setBriefingLoading] = useState(false);

  const loadBriefing = async () => {
    setBriefingLoading(true);
    try {
      const res = await xin.dailyBriefing();
      if (res?.data) {
        setBriefing(res.data);
        return;
      }
    } catch {/* fallback to mock */}
    setBriefing(getMockBriefing());
    setBriefingLoading(false);
  };

  useEffect(() => {
    if (active && !briefing) loadBriefing();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const renderBriefing = () => <div className={styles.tabContent}>
      {briefingLoading ? <div className={styles.loading}>{t("Xin.k53")}</div> : <div className={styles.briefingContent}>
          {briefing && <>
              <div className={styles.card}>
                <p className={styles.briefingDate}>{briefing.date || new Date().toLocaleDateString('zh-CN')}</p>
                <p className={styles.briefingGreeting}>{briefing.greeting || t("Xin.k54")}</p>
                {briefing.quote && <p className={styles.briefingQuote}>"{briefing.quote}"</p>}
              </div>

              {briefing.suggestions && <div className={styles.card}>
                  <div className={styles.panelTitle}>{t("Xin.k55")}</div>
                  <div className={styles.briefingSuggestions}>
                    {(briefing.suggestions as string[]).map((s: string, i: number) => <div key={i} className={styles.suggestionItem}>
                        <span className={styles.suggestionIcon}>✦</span>
                        <span>{s}</span>
                      </div>)}
                  </div>
                </div>}

              {briefing.memories && <div className={styles.card}>
                  <div className={styles.panelTitle}>{t("Xin.k56")}</div>
                  <div className={styles.briefingMemories}>
                    {(briefing.memories as any[]).map((m: any, i: number) => <div key={i} className={styles.briefingMemItem}>
                        <span>📌</span>
                        <span>{m.title || m.key || m.content?.slice(0, 60)}</span>
                      </div>)}
                  </div>
                </div>}
            </>}
          {!briefing && <div className={styles.card}>
              <p className={styles.briefingGreeting}>{t("Xin.k57")}</p>
              <p style={{
          fontSize: '12px',
          color: '#5a5a5a'
        }}>
                {t("Xin.k58")}
              </p>
            </div>}
        </div>}
    </div>;

  return { renderBriefing };
}
