import { t } from "i18next";
import { useState, useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/kernel/state/authStore';
import { ROUTES } from '@/routes/routes';
import DashboardPanel from '../features/dashboard/DashboardPanel';
import SuggestionsPanel from '../features/suggestions/SuggestionsPanel';
import BehaviorPanel from '../features/behavior/BehaviorPanel';
import ActivityPanel from '../features/activity/ActivityPanel';
import SettingsPanel from '../features/settings/SettingsPanel';
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
// tab -> 所属 L2 子插件；intelligence.dashboard/settings 为必备，不门控
const TAB_PLUGIN_ID: Record<TabId, string> = {
  dashboard: 'intelligence.dashboard',
  suggestions: 'intelligence.suggestions',
  behavior: 'intelligence.behavior',
  activity: 'intelligence.activity',
  settings: 'intelligence.settings'
};
const REQUIRED_TABS: TabId[] = ['dashboard', 'settings'];
export default function Intelligence() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const {
    isGuest
  } = useAuthStore();

  // 子插件启停门控（/spyglass 板块内 Tab 由注册表侧边栏驱动；此处按启停过滤可达性）
  const [enabledIds, setEnabledIds] = useState<string[] | null>(null);
  useEffect(() => {
    invoke<Array<{
      id: string;
    }>>('plugin:kernel|kernel_dispatch', {
      cmd: 'kernel:plugin:get_enabled',
      args: {}
    }).then(list => {
      setEnabledIds(list.map(p => p.id));
    }).catch(() => setEnabledIds(null));
  }, []);
  const isEnabled = (id: string) => enabledIds === null || enabledIds.includes(id);
  const tabEnabled = (id: TabId) => REQUIRED_TABS.includes(id) || isEnabled(TAB_PLUGIN_ID[id]);
  const queryTab = searchParams.get('tab') as TabId | null;
  // 停用的子插件对应 tab 不可达，回退 dashboard
  const activeTab = queryTab && TABS.some(t => t.id === queryTab) && tabEnabled(queryTab) ? queryTab : 'dashboard';
  const [dashboardPeriod, setDashboardPeriod] = useState<'day' | 'week' | 'month'>('day');

  // 临时账号无权访问底层智能
  if (isGuest()) {
    return <div className={styles.page}>
        <div className={styles.header}>
          <div className={styles.headerTitle}>
            <button className={styles.backBtn} onClick={() => navigate(`${ROUTES.PROFILE}?tab=setting`)} title={t("Intelligence.k1")}>
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
          <button className={styles.backBtn} onClick={() => navigate(`${ROUTES.PROFILE}?tab=setting`)} title={t("Intelligence.k1")}>
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
