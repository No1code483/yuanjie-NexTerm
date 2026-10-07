import { t } from "i18next";
import styles from '../Knowledge.module.css';
import type { KbCategory } from './types';
interface MoveTarget {
  type: 'category' | 'entry';
  id: number;
}
interface Props {
  target: MoveTarget;
  categories: KbCategory[];
  getCategoryDepth: (id: number) => number;
  getAllDescendantIds: (cats: KbCategory[], parentId: number) => number[];
  onMoveEntry: (entryId: number, categoryId: number) => void;
  onMoveCategory: (catId: number, parentId: number | null) => void;
  onCancel: () => void;
}
export default function MoveTargetModal({
  target,
  categories,
  getCategoryDepth,
  getAllDescendantIds,
  onMoveEntry,
  onMoveCategory,
  onCancel
}: Props) {
  return <div className={styles.modalOverlay} onClick={onCancel}>
      <div className={styles.modal} onClick={e => e.stopPropagation()}>
        <h3 className={styles.modalTitle}>{t("knowledge.MoveTargetModal.k1")}</h3>
        <div className={styles.moveList}>
          {target.type === 'entry' ? categories.map(cat => <button key={cat.id} className={styles.moveItem} onClick={() => onMoveEntry(target.id, cat.id)}>
                {'　'.repeat(getCategoryDepth(cat.id))}📁 {cat.name}
              </button>) : <>
              <button className={styles.moveItem} onClick={() => onMoveCategory(target.id, null)}>
                {t("knowledge.MoveTargetModal.k2")}
              </button>
              {categories.filter(c => c.id !== target.id && !getAllDescendantIds(categories, target.id).includes(c.id)).map(cat => <button key={cat.id} className={styles.moveItem} onClick={() => onMoveCategory(target.id, cat.id)}>
                  {'　'.repeat(getCategoryDepth(cat.id))}📁 {cat.name}
                </button>)}
            </>}
        </div>
        <div className={styles.modalActions}>
          <button className={styles.btnSm} onClick={onCancel}>{t("common.cancel")}</button>
        </div>
      </div>
    </div>;
}