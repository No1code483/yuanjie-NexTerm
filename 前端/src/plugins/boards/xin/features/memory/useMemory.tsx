// xin.memory L2 —— 记忆功能域（局部 state + handlers + JSX）。跨 tab 共享状态经 core 注入。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { xin } from '../../ipc';
import { time } from '@/lib/utils';
import styles from '../../Xin.module.css';
import { CATEGORY_LABELS } from '../../xin/types';
import type { Memory } from '../../xin/types';
import { getMockMemories } from '../../xin/mockData';
import type { XinCore } from '../../core';

export function useMemory(_core: XinCore, active: boolean) {
  const [memories, setMemories] = useState<Memory[]>([]);
  const [memorySearch, setMemorySearch] = useState('');
  const [memoryCategory, setMemoryCategory] = useState('all');
  const [memoryLoading, setMemoryLoading] = useState(false);

  const loadMemories = async () => {
    setMemoryLoading(true);
    try {
      const res = await xin.getMemories(50);
      if (res?.data && Array.isArray(res.data)) {
        setMemories(res.data);
      }
      if (memories.length === 0) {
        setMemories(getMockMemories());
      }
    } catch {
      if (memories.length === 0) setMemories(getMockMemories());
    } finally {
      setMemoryLoading(false);
    }
  };

  useEffect(() => {
    if (active && memories.length === 0) loadMemories();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const filteredMemories = memories.filter(m => {
    if (memorySearch && !m.key.includes(memorySearch) && !m.value.includes(memorySearch)) return false;
    if (memoryCategory !== 'all' && m.category !== memoryCategory) return false;
    return true;
  });
  const uniqueCategories = Array.from(new Set(memories.map(m => m.category)));

  const renderMemory = () => <div className={styles.tabContent}>
      <div className={styles.searchRow}>
        <input className={styles.input} placeholder={t("Xin.k39")} value={memorySearch} onChange={e => setMemorySearch(e.target.value)} />
        <select className={styles.select} value={memoryCategory} onChange={e => setMemoryCategory(e.target.value)}>
          <option value="all">{t("Xin.k40")}</option>
          {uniqueCategories.map(c => <option key={c} value={c}>{CATEGORY_LABELS[c] || c}</option>)}
        </select>
        <button className={styles.btn} onClick={loadMemories} disabled={memoryLoading}>
          {memoryLoading ? t("common.loading") : t("common.refresh")}
        </button>
      </div>

      {memoryLoading && <div className={styles.loading}>{t("Xin.k41")}</div>}

      {!memoryLoading && filteredMemories.length === 0 && <div style={{
      textAlign: 'center',
      padding: '40px',
      color: '#5a5a5a'
    }}>
          {memorySearch ? t("Xin.k42") : t("Xin.k43")}
        </div>}

      {!memoryLoading && <div className={styles.memoryGrid}>
          {filteredMemories.map(m => <div key={m.id} className={styles.memoryItem}>
              <div className={styles.memoryHeader}>
                <span className={styles.memoryCategory}>{CATEGORY_LABELS[m.category] || m.category}</span>
                <span className={styles.memoryImportance}>★ {m.importance.toFixed(1)}</span>
              </div>
              <span className={styles.memoryKey}>{m.key}</span>
              <p className={styles.memoryValue}>{m.value}</p>
              <div className={styles.memoryMeta}>
                <span>{time.formatCompact(m.created_at)}</span>
              </div>
            </div>)}
        </div>}
    </div>;

  return { renderMemory };
}
