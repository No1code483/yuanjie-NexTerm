import { t } from "i18next";
import styles from '../Knowledge.module.css';
import type { KbTag } from './types';
interface TagStat {
  tag_id: number;
  entry_count: number;
}
interface Props {
  tags: KbTag[];
  tagStats: TagStat[];
  activeTagId: number | null;
  editingTagId: number | null;
  editingTagName: string;
  editingTagColor: string;
  onTagFilter: (tagId: number) => void;
  onStartEdit: (tag: KbTag) => void;
  onDeleteTag: (tagId: number) => void;
  onUpdateTag: () => void;
  onCancelEdit: () => void;
  onEditingNameChange: (name: string) => void;
  onEditingColorChange: (color: string) => void;
  onAddTag: (name: string) => void;
}
export default function TagBar({
  tags,
  tagStats,
  activeTagId,
  editingTagId,
  editingTagName,
  editingTagColor,
  onTagFilter,
  onStartEdit,
  onDeleteTag,
  onUpdateTag,
  onCancelEdit,
  onEditingNameChange,
  onEditingColorChange,
  onAddTag
}: Props) {
  return <>
      {tags.length > 0 && <div className={styles.tagSection}>
          {tags.map(tag => {
        const st = tagStats.find(s => s.tag_id === tag.id);
        const cnt = st ? st.entry_count : 0;
        return <span key={tag.id} className={`${styles.tagChip} ${activeTagId === tag.id ? styles.tagChipActive : ''}`} style={{
          borderColor: tag.color + '55',
          background: tag.color + '15'
        }} onClick={() => onTagFilter(tag.id)} onDoubleClick={() => onStartEdit(tag)} title={t("knowledge.TagBar.k1", {
          cnt: cnt
        })}>
                <span className={styles.tagColorDot} style={{
            background: tag.color
          }} />
                {tag.name}
                {cnt > 0 && <span className={styles.tagCount}>{cnt}</span>}
                <button className={styles.tagEditBtn} onClick={e => {
            e.stopPropagation();
            onStartEdit(tag);
          }} title={t("common.edit")}>✎</button>
                <button className={styles.tagDelBtn} onClick={e => {
            e.stopPropagation();
            onDeleteTag(tag.id);
          }}>×</button>
              </span>;
      })}
        </div>}

      {editingTagId !== null && <div className={styles.tagEditPanel}>
          <input className={styles.addTagInput} value={editingTagName} onChange={e => onEditingNameChange(e.target.value)} onKeyDown={e => e.key === 'Enter' && onUpdateTag()} autoFocus />
          <input type="color" className={styles.colorPicker} value={editingTagColor} onChange={e => onEditingColorChange(e.target.value)} />
          <button className={styles.btnSm} onClick={onUpdateTag}>✓</button>
          <button className={styles.btnSm} onClick={onCancelEdit} style={{
        background: 'rgba(255,60,60,0.15)'
      }}>✗</button>
        </div>}

      <div className={styles.tagSection}>
        <input className={styles.addTagInput} placeholder={t("knowledge.TagBar.k2")} onKeyDown={e => {
        if (e.key === 'Enter') {
          onAddTag((e.target as HTMLInputElement).value);
          (e.target as HTMLInputElement).value = '';
        }
      }} />
      </div>
    </>;
}