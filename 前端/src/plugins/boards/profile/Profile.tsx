// boards.profile L1 壳 + 核心数据层（个人中心「一切皆插件」拆分）。
// 功能域已物理拆分至 features/<dir>/ 各 L2 插件（account/resume/quote/settings），
// 本文件仅保留：路由页骨架、tab 解析（?tab=）、未登录早退、标题区、
// 核心共享状态（user/isTempAccount/通知/确认弹窗/底层智能/formatTime）、概览统计计数，
// 以及按 deploy 门控分发各功能域渲染（停用的子插件回退到 account）。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { useLocation } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/kernel/state/authStore';
import { useNotifStore } from '@/kernel/state/notifStore';
import { ROUTES } from '@/routes/routes';
import { useIntelligence } from '@/hooks/useIntelligence';
import { profile } from './ipc/profile';
import { time } from '@/lib/utils';
import styles from './Profile.module.css';
import AccountPanel from './features/account/AccountPanel';
import { useResume } from './features/resume/useResume';
import { useQuote } from './features/quote/useQuote';
import { useSettings } from './features/settings/useSettings';
import type { ProfileCore, ConfirmDialogState } from './core';

/** tab → 对应子插件 id（用于启停门控：停用时回退 account） */
const TAB_PLUGIN_ID: Record<string, string> = {
  account: 'profile.account',
  resume: 'profile.resume',
  quote: 'profile.quote',
  setting: 'profile.settings'
};

export default function Profile() {
  const location = useLocation();
  const pathname = location.pathname;
  const searchParams = new URLSearchParams(location.search);
  const queryTab = searchParams.get('tab');
  // 优先使用 ?tab= 查询参数（Layout 全局侧边栏导航），其次用路径分段
  const rawTab = queryTab || (pathname === ROUTES.PROFILE ? 'account' : pathname.split('/').pop() || 'account');
  const validTabs = ['account', 'resume', 'quote', 'setting'];
  const currentTab = validTabs.includes(rawTab) ? rawTab : 'account';
  const {
    user,
    isAuthenticated
  } = useAuthStore();
  const isTempAccount = user?.is_permanent === false;
  const addToast = useNotifStore(s => s.addToast);

  /** 统一通知：桥接旧 showNotification → 全局 notifStore */
  const showNotify = (type: 'success' | 'error' | 'info', message: string) => {
    addToast({
      type,
      title: message
    });
  };

  // ===== 统一确认弹窗（多场景复用） =====
  const [confirmDialog, setConfirmDialog] = useState<ConfirmDialogState>({
    isOpen: false,
    targetName: '',
    onConfirm: () => {}
  });

  // ===== 概览统计计数：挂载即加载（修复原「仅对应 tab 加载致概览统计恒为 0」） =====
  const [resumeCount, setResumeCount] = useState(0);
  const [quoteCount, setQuoteCount] = useState(0);
  useEffect(() => {
    let cancelled = false;
    profile.getResumes().then(response => {
      if (!cancelled && response.code === 0 && response.data) setResumeCount(response.data.length);
    }).catch(() => {/* silent */});
    profile.getAllQuotes().then(response => {
      if (!cancelled && response.code === 0 && response.data) setQuoteCount((response.data as any[]).length);
    }).catch(() => {/* silent */});
    return () => {
      cancelled = true;
    };
  }, []);

  const {
    aiOn,
    featureOn,
    llmConfigured
  } = useIntelligence();

  const formatTime = (ts: number) => {
    return time.formatUtcToLocal(ts);
  };

  // ===== 子插件启停门控（参考 Home.tsx focusEnabled 写法；null 为加载中，放行） =====
  const [enabledIds, setEnabledIds] = useState<string[] | null>(null);
  useEffect(() => {
    invoke<Array<{ id: string }>>('plugin:kernel|kernel_dispatch', {
      cmd: 'kernel:plugin:get_enabled',
      args: {}
    }).then((list) => {
      setEnabledIds(list.map((p) => p.id));
    }).catch(() => setEnabledIds(null));
  }, []);
  // 对应子插件被停用时回退到 account（默认必备）
  const displayTab = enabledIds !== null && !enabledIds.includes(TAB_PLUGIN_ID[currentTab]) ? 'account' : currentTab;

  // ===== 核心共享契约（注入各功能域插件） =====
  const core: ProfileCore = {
    user: user ?? null,
    isTempAccount,
    showNotify,
    confirmDialog,
    setConfirmDialog,
    aiOn,
    featureOn,
    llmConfigured,
    formatTime,
    setResumeCount,
    setQuoteCount
  };

  // ===== 功能域 hook（各 L2 自持其 tab 局部 state + handlers） =====
  const resume = useResume(core, isAuthenticated && displayTab === 'resume');
  const quote = useQuote(core, isAuthenticated && displayTab === 'quote');
  const settings = useSettings(core, isAuthenticated && displayTab === 'setting');

  if (!isAuthenticated || !user) {
    return <div className={styles.container}>
        <main className={styles.main}>
          <div className={styles.contentSection}>
            <h3 className={styles.sectionTitle}>{t("layout.k9")}</h3>
            <div className={styles.infoCard}>
              <div className={styles.infoContent}>
                <p style={{
                color: '#FF0000'
              }}>{t("Profile.k79")}</p>
              </div>
            </div>
          </div>
        </main>
      </div>;
  }
  const isInSubPage = resume.resumeSubPage !== null || settings.settingSubPage !== null || resume.viewingResumeId !== null;
  return <div className={`${styles.container} profile-scrollable`}>
      <main className={styles.main}>
        <div className={`${styles.contentSection} profile-scrollable`}>
          {!isInSubPage && <h3 className={styles.sectionTitle} style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between'
        }}>
            {displayTab === 'account' && <span>{t("layout.k9")}</span>}
            {displayTab === 'resume' && <span>{t("components.intelligence.ActivityPanel.k5")}</span>}
            {displayTab === 'quote' && <>
                <span>{t("components.intelligence.ActivityPanel.k6")}</span>
                <button className={styles.editActionBtn} onClick={() => quote.setShowAddQuote(!quote.showAddQuote)} style={{
              padding: '4px 14px',
              height: 32,
              fontSize: 12,
              minWidth: 80
            }}>
                  <span className={styles.btnText}>{quote.showAddQuote ? t("common.cancel") : t("Profile.k148")}</span>
                  <span className={styles.btnIcon}>{quote.showAddQuote ? '✕' : '✎'}</span>
                </button>
              </>}
            {displayTab === 'setting' && <span>{t("common.settings")}</span>}
          </h3>}
          <div className={`${styles.contentText} profile-scrollable`}>
            {displayTab === 'account' && <AccountPanel isTempAccount={isTempAccount} user={user} formatTime={formatTime} resumeCount={resumeCount} quoteCount={quoteCount} kbCount={0} />}
            {displayTab === 'resume' && resume.renderResume()}
            {displayTab === 'quote' && quote.renderQuote()}
            {displayTab === 'setting' && settings.renderSettings()}
          </div>
        </div>
      </main>
    </div>;
}
