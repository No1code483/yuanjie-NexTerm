import { t } from "i18next";
import { useState, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { profile } from '@/lib/ipc';
import NewsPanel from './home/NewsPanel';
import TodoPanel from './home/TodoPanel';
import JournalPanel from './home/JournalPanel';
import TimerPanel from './home/TimerPanel';
import FocusMode from '@/components/FocusMode';
import { formatTime, formatDateFull } from './home/utils';
import styles from './Home.module.css';
import { ROUTES } from '../routes/routes';
export default function Home() {
  const location = useLocation();
  const navigate = useNavigate();
  const [showModal, setShowModal] = useState(false);
  const [modalContent, setModalContent] = useState<'news' | 'todo' | 'log' | 'timer' | undefined>(undefined);
  const [currentTime, setCurrentTime] = useState(new Date());
  const [quote, setQuote] = useState('');
  const [focusMode, setFocusMode] = useState(false);
  useEffect(() => {
    const searchParams = new URLSearchParams(location.search);
    const modalParam = searchParams.get('modal');
    if (modalParam && ['news', 'todo', 'log', 'timer'].includes(modalParam)) {
      setModalContent(modalParam as 'news' | 'todo' | 'log' | 'timer');
      setShowModal(true);
    }
    const focusParam = searchParams.get('focus');
    if (focusParam === 'true') {
      setFocusMode(true);
      navigate(ROUTES.HOME, {
        replace: true
      });
    }
  }, [location.search]);

  // Listen for focus-mode-start custom event from Layout header
  useEffect(() => {
    const handleFocusStart = () => setFocusMode(true);
    window.addEventListener('focus-mode-start', handleFocusStart);
    return () => window.removeEventListener('focus-mode-start', handleFocusStart);
  }, []);
  useEffect(() => {
    profile.getAllQuotes().then(res => {
      const list: Array<{
        id: number;
        content: string;
      }> = res?.data ?? [];
      if (list.length === 0) return;
      const now = new Date();
      const start = new Date(now.getFullYear(), 0, 0);
      const dayOfYear = Math.floor((now.getTime() - start.getTime()) / (1000 * 60 * 60 * 24));
      setQuote(list[dayOfYear % list.length].content);
    }).catch(() => {});
  }, []);
  useEffect(() => {
    const timer = setInterval(() => setCurrentTime(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);
  const closeModal = () => {
    setShowModal(false);
    navigate(ROUTES.HOME);
  };
  const renderModalContent = () => {
    switch (modalContent) {
      case 'news':
        return <NewsPanel />;
      case 'todo':
        return <TodoPanel />;
      case 'log':
        return <JournalPanel modalContent={modalContent} />;
      case 'timer':
        return <TimerPanel />;
      default:
        return null;
    }
  };
  if (focusMode) {
    return <FocusMode onExit={() => setFocusMode(false)} />;
  }
  return <div className={styles.container}>
      <div className={styles.main}>
        <div className={styles.centerContent}>
          <div className={styles.timeSection}>
            <div className={styles.timeDisplay}>{formatTime(currentTime)}</div>
            <div className={styles.dateDisplay}>{formatDateFull(currentTime)}</div>
            <div className={styles.quoteDisplay}>{quote}</div>
          </div>
        </div>
      </div>

      {/* 专注模式入口 */}
      <button className={styles.focusEntryBtn} onClick={() => setFocusMode(true)} title={t("Home.k1")}>
        <span className={styles.focusEntryIcon}>◉</span>
        <span className={styles.focusEntryLabel}>{t("Home.k2")}</span>
      </button>

      {showModal && <div className={styles.functionModal} onClick={closeModal}>
          <div className={styles.functionModalContent} onClick={e => e.stopPropagation()}>
            <div className={styles.functionModalHeader}>
              <h2 className={styles.functionModalTitle}>
                {modalContent === 'news' && t("Home.k3")}
                {modalContent === 'todo' && t("Home.k4")}
                {modalContent === 'log' && t("Home.k5")}
                {modalContent === 'timer' && t("Home.k6")}
              </h2>
              <button className={styles.closeButton} onClick={closeModal}>✕</button>
            </div>
            <div className={styles.functionModalBody}>
              {renderModalContent()}
            </div>
          </div>
        </div>}
    </div>;
}