import { t } from "i18next";
import { useState, useEffect } from 'react';
import { ipc } from '@/lib/ipc';
import type { ApiResponse } from '@/lib/ipc';
import type { KbEntry, SortMode, ViewMode, TreeNode } from './types';
import styles from '../Knowledge.module.css';
interface ViewModeListProps {
  mode: ViewMode;
  filterId: number | null;
  sortMode: SortMode;
  typeFilter: string;
  renderNode: (node: TreeNode) => React.ReactNode;
}
export default function ViewModeList({
  mode,
  filterId,
  sortMode,
  typeFilter,
  renderNode
}: ViewModeListProps) {
  const [entries, setEntries] = useState<KbEntry[]>([]);
  const [loading, setLoading] = useState(false);
  useEffect(() => {
    const load = async () => {
      setLoading(true);
      try {
        let res: ApiResponse<KbEntry[]> | null = null;
        if (mode === 'favorite') {
          res = await ipc.invoke<KbEntry[]>('get_kb_favorites');
        } else if (mode === 'recent') {
          res = await ipc.invoke<KbEntry[]>('get_kb_recent', {
            limit: 50
          });
        } else if (mode === 'tag' && filterId) {
          res = await ipc.invoke<KbEntry[]>('get_kb_entries_by_tag', {
            tag_id: filterId
          });
        }
        if (res?.code === 0 && res.data) setEntries(res.data);
      } catch {/* silent */}
      setLoading(false);
    };
    load();
  }, [mode, filterId]);
  let sorted = [...entries];
  if (typeFilter !== 'all') sorted = sorted.filter(e => e.entry_type === typeFilter);
  switch (sortMode) {
    case 'name_asc':
      sorted.sort((a, b) => a.name.localeCompare(b.name));
      break;
    case 'name_desc':
      sorted.sort((a, b) => b.name.localeCompare(a.name));
      break;
    case 'time_desc':
      sorted.sort((a, b) => b.updated_at - a.updated_at);
      break;
    case 'time_asc':
      sorted.sort((a, b) => a.updated_at - b.updated_at);
      break;
  }
  if (loading) return <div className={styles.emptyTipSmall}>{t("common.loading")}</div>;
  if (sorted.length === 0) return <div className={styles.emptyTipSmall}>{t("knowledge.ViewModeList.k1")}</div>;
  return <>
      {sorted.map(entry => renderNode({
      type: 'entry',
      id: entry.id,
      name: entry.name,
      categoryId: entry.category_id,
      pathUrl: entry.path_url,
      entryType: entry.entry_type,
      updatedAt: entry.updated_at,
      depth: 0,
      children: []
    }))}
    </>;
}