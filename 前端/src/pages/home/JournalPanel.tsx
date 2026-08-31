import { t } from "i18next";
import { useState, useEffect, useRef } from 'react';
import { home, intelligence } from '@/lib/ipc';
import { useIntelligence } from '@/hooks/useIntelligence';
import { SkeletonText } from '@/components/ui/Skeleton';
import type { JournalEntry, JournalFillResult } from './types';
import styles from '../Home.module.css';
interface Props {
  modalContent: string | undefined;
}
export default function JournalPanel({
  modalContent
}: Props) {
  const {
    aiOn,
    featureOn
  } = useIntelligence();
  const [journal, setJournal] = useState<JournalEntry | null>(null);
  const [journalDate, setJournalDate] = useState(() => new Date().toISOString().split('T')[0]);
  const [loadingJournal, setLoadingJournal] = useState(false);
  const today = new Date().toISOString().split('T')[0];
  const [aiJournalAnnotation, setAiJournalAnnotation] = useState('');
  const [aiJournalAnnotLoading, setAiJournalAnnotLoading] = useState(false);
  const [journalFillLoading, setJournalFillLoading] = useState(false);
  const [journalFillResult, setJournalFillResult] = useState<JournalFillResult | null>(null);
  const journalCheckTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const loadJournal = async () => {
    setLoadingJournal(true);
    try {
      const result = await home.getLogs(journalDate);
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
      const result = await home.saveLog(journalDate, content);
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
  useEffect(() => {
    const content = journal?.content || '';
    if (!aiOn || !featureOn('smart_complete') || modalContent !== 'log' || content.length < 20) {
      setAiJournalAnnotation('');
      return;
    }
    if (journalCheckTimerRef.current) clearTimeout(journalCheckTimerRef.current);
    setAiJournalAnnotLoading(true);
    journalCheckTimerRef.current = setTimeout(async () => {
      try {
        const res = await home.aiCheckJournal(journalDate, content);
        if (res.code === 0 && res.data) {
          setAiJournalAnnotation((res.data as any)?.template || '');
        } else {
          setAiJournalAnnotation('');
        }
      } catch {
        setAiJournalAnnotation('');
      } finally {
        setAiJournalAnnotLoading(false);
      }
    }, 2000);
    return () => {
      if (journalCheckTimerRef.current) clearTimeout(journalCheckTimerRef.current);
    };
  }, [journal?.content, journalDate, modalContent, aiOn, featureOn]);
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
              {aiJournalAnnotLoading && <span className={styles.aiCheckingInline}>{t("home.JournalPanel.k8")}</span>}
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
            {aiJournalAnnotation && <div className={styles.aiAnnotation}>
                <div className={styles.aiAnnotationHeader}>
                  <span>{t("home.JournalPanel.k9")}</span>
                  <button className={styles.aiAnnotationClose} onClick={() => setAiJournalAnnotation('')}>✕</button>
                </div>
                <pre className={styles.aiAnnotationContent}>{aiJournalAnnotation}</pre>
              </div>}
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