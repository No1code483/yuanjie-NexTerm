// boards.home L1 板块页（阶段3 批次1b-1：由 pages/Home.tsx 迁入）。
// news / todo / journal / timer 四个 modal 分支全部改由 L2 插件经插槽渲染
// （SlotRenderer：home.todo / home.journal 属 1b-1、home.timer 属 1b-2a、
// home.news 属 1b-2b-1），L1 不再直接 import 任何未迁模块。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { profile } from '@/plugins/boards/profile/ipc/profile';
// 批C4：专注子插件（home.focus）组件物理迁入 features/focus/
import FocusMode from './features/focus/FocusMode';
import { SlotRenderer } from '@/kernel/slots/SlotRenderer';
import { formatTime, formatDateFull } from './utils';
import styles from './Home.module.css';
import { ROUTES } from '@/routes/routes';
export default function Home() {
  const location = useLocation();
  const navigate = useNavigate();
  const [showModal, setShowModal] = useState(false);
  const [modalContent, setModalContent] = useState<'news' | 'todo' | 'log' | 'timer' | undefined>(undefined);
  const [currentTime, setCurrentTime] = useState(new Date());
  const [quote, setQuote] = useState('');
  const [focusMode, setFocusMode] = useState(false);
  // 批C4：专注子插件（home.focus）启停门控——null 为加载中（放行，与 Layout enabledIds 口径一致）
  const [focusEnabled, setFocusEnabled] = useState<boolean | null>(null);
  useEffect(() => {
    invoke<Array<{ id: string }>>('plugin:kernel|kernel_dispatch', {
      cmd: 'kernel:plugin:get_enabled',
      args: {},
    }).then((list) => {
      setFocusEnabled(list.some((p) => p.id === 'home.focus'));
    }).catch(() => setFocusEnabled(true));
  }, []);
  useEffect(() => {
    const searchParams = new URLSearchParams(location.search);
    const modalParam = searchParams.get('modal');
    if (modalParam && ['news', 'todo', 'log', 'timer'].includes(modalParam)) {
      setModalContent(modalParam as 'news' | 'todo' | 'log' | 'timer');
      setShowModal(true);
    }
    const focusParam = searchParams.get('focus');
    if (focusParam === 'true') {
      // 批C4：home.focus 停用时忽略 URL 直达参数
      if (focusEnabled !== false) {
        setFocusMode(true);
      }
      navigate(ROUTES.HOME, {
        replace: true
      });
    }
  }, [location.search, focusEnabled]);

  // Listen for focus-mode-start custom event（批C4：home.focus 停用时忽略）
  useEffect(() => {
    const handleFocusStart = () => {
      if (focusEnabled !== false) setFocusMode(true);
    };
    window.addEventListener('focus-mode-start', handleFocusStart);
    return () => window.removeEventListener('focus-mode-start', handleFocusStart);
  }, [focusEnabled]);
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
        return <SlotRenderer slot="home.news" />;
      case 'todo':
        return <SlotRenderer slot="home.todo" />;
      case 'log':
        return <SlotRenderer slot="home.journal" />;
      case 'timer':
        return <SlotRenderer slot="home.timer" />;
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

      {/* 专注模式入口（批C4：home.focus 停用时隐藏） */}
      {focusEnabled !== false && <button className={styles.focusEntryBtn} onClick={() => setFocusMode(true)} title={t("Home.k1")}>
        <span className={styles.focusEntryIcon}>◉</span>
        <span className={styles.focusEntryLabel}>{t("Home.k2")}</span>
      </button>}

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