import { t } from "i18next";
import { useState, useEffect, useCallback } from 'react';
import { intelligence } from '@/lib/ipc';
import { useAuthStore } from '@/stores/authStore';
import { useFloatingOrbStore } from '@/stores/floatingOrbStore';
import { time } from '@/lib/utils';
import type { Suggestion } from '@/types';
import styles from './Intelligence.module.css';
type SubTab = 'suggestions' | 'favorites';
type SuggestCategory = 'efficiency' | 'habit' | 'workflow' | 'health';
const CATEGORY_LABELS: Record<string, string> = {
  efficiency: t("components.intelligence.SuggestionsPanel.k1"),
  habit: t("components.intelligence.SuggestionsPanel.k2"),
  workflow: t("components.intelligence.SuggestionsPanel.k3"),
  health: t("components.intelligence.SuggestionsPanel.k4")
};
const STATUS_COLORS: Record<string, string> = {
  unread: '#39BAE6',
  read: '#8A919E',
  adopted: '#7FD962',
  ignored: '#555D68'
};
const ORB_TYPE_LABELS: Record<string, string> = {
  summary: t("components.intelligence.SuggestionsPanel.k5"),
  suggestion: t("components.intelligence.ActivityPanel.k17"),
  tag: t("components.intelligence.SuggestionsPanel.k6"),
  insight: t("components.intelligence.SuggestionsPanel.k7"),
  notification: t("components.intelligence.SuggestionsPanel.k8")
};
export default function SuggestionsPanel() {
  const {
    user
  } = useAuthStore();
  const {
    favorites,
    toggleFavorite
  } = useFloatingOrbStore();
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [loading, setLoading] = useState(false);
  const [filter, setFilter] = useState<SuggestCategory | 'all'>('all');
  const [subTab, setSubTab] = useState<SubTab>('suggestions');
  const load = useCallback(async () => {
    setLoading(true);
    try {
      const userId = user?.id ? String(user.id) : '1';
      const res = await intelligence.getSuggestions(userId, 'unread', undefined, 50, 0);
      if (res?.data?.suggestions) setSuggestions(res.data.suggestions);
    } catch (e) {
      console.error('加载建议失败:', e);
    } finally {
      setLoading(false);
    }
  }, [user]);
  useEffect(() => {
    load();
  }, [load]);
  const handleMark = useCallback(async (suggestionId: number, action: 'adopted' | 'ignored') => {
    try {
      await intelligence.markSuggestion(suggestionId, action);
      setSuggestions(prev => prev.map(s => s.id === suggestionId ? {
        ...s,
        status: action
      } : s));
    } catch (e) {
      console.error('标记建议失败:', e);
    }
  }, []);
  const filtered = filter === 'all' ? suggestions : suggestions.filter(s => s.category === filter);
  if (loading && !suggestions.length) {
    return <div className={styles.loading}>{t("components.intelligence.SuggestionsPanel.k9")}</div>;
  }
  return <div className={styles.panel}>
      {/* 子 Tab 切换 */}
      <div className={styles.suggestFilterBar} style={{
      marginBottom: 12
    }}>
        <button className={`${styles.filterBtn} ${subTab === 'suggestions' ? styles.filterBtnActive : ''}`} onClick={() => setSubTab('suggestions')}>{t("components.intelligence.ActivityPanel.k17")}</button>
        <button className={`${styles.filterBtn} ${subTab === 'favorites' ? styles.filterBtnActive : ''}`} onClick={() => setSubTab('favorites')}>
          {t("components.intelligence.SuggestionsPanel.k10")}
          {favorites.length > 0 && <span style={{
          marginLeft: 4,
          fontSize: 10,
          background: 'rgba(245,158,11,0.2)',
          color: '#F59E0B',
          padding: '0 5px',
          borderRadius: 8
        }}>
              {favorites.length}
            </span>}
        </button>
      </div>

      {/* ===== 智能建议 Tab ===== */}
      {subTab === 'suggestions' && <>
          <div className={styles.suggestFilterBar}>
            <button className={`${styles.filterBtn} ${filter === 'all' ? styles.filterBtnActive : ''}`} onClick={() => setFilter('all')}>{t("common.all")}</button>
            {(Object.keys(CATEGORY_LABELS) as SuggestCategory[]).map(cat => <button key={cat} className={`${styles.filterBtn} ${filter === cat ? styles.filterBtnActive : ''}`} onClick={() => setFilter(cat)}>
                {CATEGORY_LABELS[cat]}
              </button>)}
          </div>

          {filtered.length === 0 ? <div className={styles.emptyState}>
              <div className={styles.emptyIcon}>{'💡'}</div>
              <div className={styles.emptyText}>{t("components.intelligence.SuggestionsPanel.k11")}</div>
              <div className={styles.emptyHint}>{t("components.intelligence.SuggestionsPanel.k12")}</div>
            </div> : <div className={styles.suggestList}>
              {filtered.map(sug => <div key={sug.id} className={styles.suggestCard}>
                  <div className={styles.suggestMain}>
                    <div className={styles.suggestHeader}>
                      <span className={styles.suggestTitle}>{sug.title}</span>
                      <span className={styles.suggestCategory}>{CATEGORY_LABELS[sug.category] || sug.category}</span>
                      <span className={styles.suggestStatus} style={{
                color: STATUS_COLORS[sug.status] || '#8A919E'
              }}>
                        {sug.status === 'unread' ? t("components.intelligence.SuggestionsPanel.k13") : sug.status === 'read' ? t("components.intelligence.SuggestionsPanel.k14") : sug.status === 'adopted' ? t("components.intelligence.SuggestionsPanel.k15") : t("components.intelligence.SuggestionsPanel.k16")}
                      </span>
                    </div>
                    <div className={styles.suggestDesc}>{sug.description}</div>
                    <div style={{
              fontSize: 11,
              color: '#555D68',
              marginTop: 6
            }}>{time.timeAgo(sug.created_at || '')}</div>
                  </div>
                  {sug.status === 'unread' || sug.status === 'read' ? <div className={styles.suggestActions}>
                      <button className={styles.suggestBtn} onClick={() => handleMark(sug.id, 'adopted')}>
                        {t("components.intelligence.SuggestionsPanel.k17")}
                      </button>
                      <button className={`${styles.suggestBtn} ${styles.suggestBtnIgnore}`} onClick={() => handleMark(sug.id, 'ignored')}>
                        {t("components.intelligence.SuggestionsPanel.k18")}
                      </button>
                    </div> : null}
                </div>)}
            </div>}
        </>}

      {/* ===== 收藏夹 Tab ===== */}
      {subTab === 'favorites' && <>
          {favorites.length === 0 ? <div className={styles.emptyState}>
              <div className={styles.emptyIcon}>{'⭐'}</div>
              <div className={styles.emptyText}>{t("components.intelligence.SuggestionsPanel.k19")}</div>
              <div className={styles.emptyHint}>{t("components.intelligence.SuggestionsPanel.k20")}</div>
            </div> : <div className={styles.suggestList}>
              {favorites.map(orb => <div key={orb.id} className={styles.suggestCard} style={{
          borderLeftColor: orb.color || '#00F0FF'
        }}>
                  <div className={styles.suggestMain}>
                    <div className={styles.suggestHeader}>
                      <span style={{
                marginRight: 6
              }}>{orb.icon}</span>
                      <span className={styles.suggestTitle}>{orb.title}</span>
                      <span className={styles.suggestStatus} style={{
                color: orb.color || '#00F0FF',
                fontSize: 11
              }}>
                        {ORB_TYPE_LABELS[orb.type] || orb.type}
                      </span>
                      <span className={styles.suggestStatus} style={{
                color: '#555D68',
                fontSize: 10
              }}>
                        {time.timeAgo(String(orb.createdAt))}
                      </span>
                    </div>
                    <div className={styles.suggestDesc}>{orb.content}</div>
                    {orb.keyPoints && orb.keyPoints.length > 0 && <ul style={{
              margin: '6px 0 0',
              paddingLeft: 18,
              listStyle: 'none'
            }}>
                        {orb.keyPoints.map((p, i) => <li key={i} style={{
                fontSize: 12,
                color: 'rgba(180,195,215,0.8)',
                lineHeight: 1.6
              }}>
                            ▸ {p}
                          </li>)}
                      </ul>}
                  </div>
                  <div className={styles.suggestActions}>
                    <button className={`${styles.suggestBtn} ${styles.favRemoveBtn}`} onClick={() => toggleFavorite(orb.id)} title={t("components.FloatingBall.k56")}>
                      {t("components.FloatingBall.k56")}
                    </button>
                  </div>
                </div>)}
            </div>}
        </>}
    </div>;
}