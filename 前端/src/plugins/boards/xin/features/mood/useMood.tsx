// xin.mood L2 —— 心情功能域（局部 state + handlers + JSX）。mood 核心状态经 core 注入。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { xin } from '../../ipc';
import styles from '../../Xin.module.css';
import { MOOD_EMOJI } from '../../xin/types';
import type { MoodEntry } from '../../xin/types';
import { getMockMoodTimeline } from '../../xin/mockData';
import type { XinCore } from '../../core';

export function useMood(core: XinCore, active: boolean) {
  const { mood, setMood } = core;
  const [moodTimeline, setMoodTimeline] = useState<MoodEntry[]>([]);
  const [moodLoading, setMoodLoading] = useState(false);

  const loadMoodData = async () => {
    setMoodLoading(true);
    try {
      const res = await xin.getMood();
      if (res?.data) setMood(res.data);
      setMoodTimeline(getMockMoodTimeline());
    } catch {
      setMood({
        category: 'curious',
        intensity: 0.75,
        trigger: t("Xin.k7"),
        updated_at: new Date().toISOString()
      });
      setMoodTimeline(getMockMoodTimeline());
    } finally {
      setMoodLoading(false);
    }
  };

  useEffect(() => {
    if (active && moodTimeline.length === 0) loadMoodData();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const renderMood = () => <div className={styles.tabContent}>
      {moodLoading ? <div className={styles.loading}>{t("Xin.k44")}</div> : <>
          {mood && <div className={styles.card} style={{
        marginBottom: '16px'
      }}>
              <div className={styles.moodDisplay}>
                <span className={styles.moodEmoji}>{MOOD_EMOJI[mood.category] || '🤖'}</span>
                <div className={styles.moodInfo}>
                  <span className={styles.moodLabel}>{mood.category}</span>
                  <span className={styles.moodIntensity}>{t("Xin.k45")} {((mood.intensity || 0) * 100).toFixed(0)}%</span>
                  {mood.trigger && <span className={styles.moodTrigger}>{t("Xin.k46")} {mood.trigger}</span>}
                </div>
              </div>
            </div>}

          <div className={styles.card}>
            <div className={styles.panelTitle}>{t("Xin.k47")}</div>
            <div className={styles.moodTimeline}>
              {moodTimeline.map(m => <div key={m.id} className={styles.moodTimelineItem}>
                  <span className={styles.moodTimelineEmoji}>{m.emoji}</span>
                  <div className={styles.moodTimelineInfo}>
                    <span className={styles.moodTimelineCategory}>{m.category}</span>
                    <span className={styles.moodTimelineContext}>{m.context}</span>
                  </div>
                  <span className={styles.moodTimelineTime}>{m.time}</span>
                </div>)}
            </div>
          </div>
        </>}
    </div>;

  return { renderMood };
}
