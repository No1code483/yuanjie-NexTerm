import { t } from "i18next";
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import BrowserSearch from '../features/browser/BrowserSearch';
import GlobalSearch from '../features/global/GlobalSearch';
import styles from './Search.module.css';
type SearchMode = 'browser' | 'global';
/** customs.search 壳：模式切换（浏览器/全站）+ 子插件启停门控。
 *  事件契约（search-navigate / search-switch-engine / bookmark-add / bookmark-remove）由各域组件承载，保持不变。 */
export default function Search() {
  const [searchMode, setSearchMode] = useState<SearchMode>('browser');

  // 子插件启停门控（search.browser 必备；global/bookmarks 可选）
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
  const globalEnabled = isEnabled('search.global');
  const bookmarksEnabled = isEnabled('search.bookmarks');
  // 全站搜索被停用且当前处于该模式 → 回退浏览器模式
  const mode: SearchMode = searchMode === 'global' && !globalEnabled ? 'browser' : searchMode;
  return <div className={styles.page}>
      {/* 模式切换 */}
      <div className={styles.modeBar}>
        <button className={`${styles.modeBtn} ${mode === 'browser' ? styles.modeBtnActive : ''}`} onClick={() => setSearchMode('browser')}>
          {t("Search.k8")}
        </button>
        {globalEnabled && <button className={`${styles.modeBtn} ${mode === 'global' ? styles.modeBtnActive : ''}`} onClick={() => setSearchMode('global')}>
            {t("Search.k9")}
          </button>}
      </div>

      {mode === 'browser' ? <BrowserSearch bookmarksEnabled={bookmarksEnabled} /> : <GlobalSearch />}
    </div>;
}
