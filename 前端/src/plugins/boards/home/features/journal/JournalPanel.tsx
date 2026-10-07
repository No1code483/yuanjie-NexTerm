// plugins/boards/home/features/journal/JournalPanel.tsx — home.journal L2 面板
// （阶段3 批次1b-1：由 pages/home/JournalPanel.tsx 迁入，经 home.journal 插槽渲染）。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { intelligence } from '@/lib/ipc';
import { journal as journalIpc } from './ipc'; // 别名避免与局部变量 journal 冲突
import { useIntelligence } from '@/hooks/useIntelligence';
import { SkeletonText } from '@/components/ui/Skeleton';
import type { JournalEntry, JournalFillResult } from '../../types';
import styles from '../../Home.module.css';
export default function JournalPanel() {
  const {
    aiOn,
    featureOn
  } = useIntelligence();
  const [journal, setJournal] = useState<JournalEntry | null>(null);
  const [journalDate, setJournalDate] = useState(() => new Date().toISOString().split('T')[0]);
  const [loadingJournal, setLoadingJournal] = useState(false);
  const today = new Date().toISOString().split('T')[0];
  const [journalFillLoading, setJournalFillLoading] = useState(false);
  const [journalFillResult, setJournalFillResult] = useState<JournalFillResult | null>(null);
  const loadJournal = async () => {
    setLoadingJournal(true);
    try {
      const result = await journalIpc.getJournal(journalDate);
      if (result.code === 0 && result.data) {
        const journalData = result.data as any;
        setJournal({
          id: journalData.id,
          date: journalData.date,
          content: journalData.content
        });
      } else {
        setJournal(null);
      }
    } catch (error) {
      console.error('加载日志失败:', error);
    } finally {
      setLoadingJournal(false);
    }
  };
  useEffect(() => {
    loadJournal();
  }, [journalDate]);
  const saveJournal = async (content: string) => {
    try {
      const result = await journalIpc.saveJournal(journalDate, content);
      if (result.code === 0 && result.data) {
        setJournal({
          id: result.data.id,
          date: result.data.date,
          content: result.data.content
        });
      }
    } catch (error) {
      console.error('保存日志失败:', error);
    }
  };
  const navigateJournalDate = (direction: 'prev' | 'next') => {
    const currentDate = new Date(journalDate);
    if (direction === 'prev') currentDate.setDate(currentDate.getDate() - 1);else currentDate.setDate(currentDate.getDate() + 1);
    setJournalDate(currentDate.toISOString().split('T')[0]);
  };
  const goToday = () => setJournalDate(today);
  const handleJournalFill = async () => {
    const content = journal?.content?.slice(0, 500) || '';
    setJournalFillLoading(true);
    setJournalFillResult(null);
    try {
      const res = await intelligence.journalFill(content || undefined);
      if (res?.data) setJournalFillResult(res.data);
    } catch {/* ignore */} finally {
      setJournalFillLoading(false);
    }
  };
  return <div className={styles.subpage}>
      <div className={styles.journalNav}>
        <button className={styles.navDateButton} onClick={() => navigateJournalDate('prev')}>{t("home.JournalPanel.k1")}</button>
        <div className={styles.journalDateDisplay}>
          <input type="date" value={journalDate} onChange={e => setJournalDate(e.target.value)} className={styles.datePickerInput} />
          {journalDate === today && <span className={styles.todayBadge}>{t("common.today")}</span>}
        </div>
        <button className={styles.navDateButton} onClick={() => navigateJournalDate('next')} disabled={journalDate >= today}>{t("home.JournalPanel.k2")}</button>
        {journalDate !== today && <button className={styles.todayButton} onClick={goToday}>{t("home.JournalPanel.k3")}</button>}
      </div>
      <div className={styles.journalEditor}>
        {loadingJournal ? <div style={{
        padding: '20px'
      }}>
            <SkeletonText lines={4} />
          </div> : <>
            <textarea defaultValue={journal?.content || ''} placeholder={t("home.JournalPanel.k4", {
          arg0: journalDate === today ? t("common.today") : journalDate
        })} className={styles.journalTextarea} onBlur={e => saveJournal(e.target.value)} rows={15} />
            <div className={styles.journalStats}>
              <span className={styles.wordCount}>📝 {(journal?.content || '').length} {t("home.JournalPanel.k5")}</span>
              <span className={styles.estimatedTime}>{t("home.JournalPanel.k6")} {Math.ceil((journal?.content?.length || 0) / 400)} {t("home.JournalPanel.k7")}</span>
              {aiOn && featureOn('smart_complete') && <button onClick={handleJournalFill} disabled={journalFillLoading} style={{
            marginLeft: 12,
            padding: '2px 10px',
            fontSize: 11,
            borderRadius: 4,
            border: '1px solid rgba(0,240,255,0.3)',
            background: 'rgba(0,240,255,0.08)',
            color: '#00F0FF',
            cursor: 'pointer'
          }}>
                  {journalFillLoading ? '⏳' : '🧠'} {t("components.intelligence.SettingsPanel.k10")}
                </button>}
            </div>
            {journalFillResult && <div className={styles.aiAnnotation} style={{
          borderColor: 'rgba(0,240,255,0.2)'
        }}>
                <div className={styles.aiAnnotationHeader}>
                  <span>{t("home.JournalPanel.k10")}</span>
                  <button className={styles.aiAnnotationClose} onClick={() => setJournalFillResult(null)}>✕</button>
                </div>
                {journalFillResult.suggestions.length > 0 && <div style={{
            color: 'rgba(255,255,255,0.6)',
            fontSize: 12,
            marginBottom: 8
          }}>
                    {journalFillResult.suggestions.map((s, i) => <div key={i} style={{
              marginBottom: 2
            }}>· {s}</div>)}
                  </div>}
                <pre className={styles.aiAnnotationContent}>{journalFillResult.template}</pre>
              </div>}
          </>}
      </div>
      {!journal && !loadingJournal && <div className={styles.journalEmptyHint}>{t("home.JournalPanel.k11")}</div>}
    </div>;
}