import { t } from "i18next";
import { useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useAuthStore } from '@/stores/authStore';
import DashboardPanel from '@/components/intelligence/DashboardPanel';
import SuggestionsPanel from '@/components/intelligence/SuggestionsPanel';
import BehaviorPanel from '@/components/intelligence/BehaviorPanel';
import ActivityPanel from '@/components/intelligence/ActivityPanel';
import SettingsPanel from '@/components/intelligence/SettingsPanel';
import styles from './Intelligence.module.css';
type TabId = 'dashboard' | 'suggestions' | 'behavior' | 'activity' | 'settings';
const TABS: {
  id: TabId;
  label: string;
  icon: string;
}[] = [{
  id: 'dashboard',
  label: t("layout.k29"),
  icon: '📊'
}, {
  id: 'suggestions',
  label: t("components.intelligence.ActivityPanel.k17"),
  icon: '💡'
}, {
  id: 'behavior',
  label: t("components.intelligence.ActivityPanel.k18"),
  icon: '🧠'
}, {
  id: 'activity',
  label: t("layout.k30"),
  icon: '📋'
}, {
  id: 'settings',
  label: t("common.settings"),
  icon: '⚙️'
}];
export default function Intelligence() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const {
    isGuest
  } = useAuthStore();
  const queryTab = searchParams.get('tab') as TabId | null;
  const activeTab = queryTab && TABS.some(t => t.id === queryTab) ? queryTab : 'dashboard';
  const [dashboardPeriod, setDashboardPeriod] = useState<'day' | 'week' | 'month'>('day');

  // 临时账号无权访问底层智能
  if (isGuest()) {
    return <div className={styles.page}>
        <div className={styles.header}>
          <div className={styles.headerTitle}>
            <button className={styles.backBtn} onClick={() => navigate('/profile?tab=setting')} title={t("Intelligence.k1")}>
              {t("Intelligence.k2")}
            </button>
            <span className={styles.headerIcon}>{'🤖'}</span>
            <span>{t("Intelligence.k3")}</span>
          </div>
        </div>
        <div className={styles.deniedContainer}>
          <div className={styles.deniedIcon}>🔒</div>
          <div className={styles.deniedText}>{t("errors.permissionDenied")}</div>
          <div className={styles.deniedHint}>{t("Intelligence.k4")}</div>
        </div>
      </div>;
  }
  return <div className={styles.page}>
      <div className={styles.header}>
        <div className={styles.headerTitle}>
          <button className={styles.backBtn} onClick={() => navigate('/profile?tab=setting')} title={t("Intelligence.k1")}>
            {t("Intelligence.k2")}
          </button>
          <span className={styles.headerIcon}>{'🤖'}</span>
          <span>{t("Intelligence.k3")}</span>
        </div>
      </div>

      <div className={styles.content}>
        {activeTab === 'dashboard' && <DashboardPanel period={dashboardPeriod} onPeriodChange={setDashboardPeriod} />}
        {activeTab === 'suggestions' && <SuggestionsPanel />}
        {activeTab === 'behavior' && <BehaviorPanel />}
        {activeTab === 'activity' && <ActivityPanel />}
        {activeTab === 'settings' && <SettingsPanel />}
      </div>
    </div>;
}