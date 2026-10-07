import { t } from "i18next";
import styles from '../Knowledge.module.css';
import type { KbEntry } from './types';
interface NameConflict {
  newName: string;
  categoryId: number;
  existingEntry: KbEntry;
  onRename: (renamed: string) => void;
  onReplace: () => void;
}
interface Props {
  conflict: NameConflict;
  resolveAutoRename: (name: string, categoryId: number) => string;
  onCancel: () => void;
}
export default function NameConflictModal({
  conflict,
  resolveAutoRename,
  onCancel
}: Props) {
  const renamed = resolveAutoRename(conflict.newName, conflict.categoryId);
  return <div className={styles.modalOverlay} onClick={onCancel}>
      <div className={styles.modal} onClick={e => e.stopPropagation()} style={{
      maxWidth: 440
    }}>
        <h3 className={styles.modalTitle}>{t("knowledge.NameConflictModal.k1")}</h3>
        <div className={styles.conflictBody}>
          <div className={styles.conflictRow}>
            <span className={styles.conflictLabel}>{t("knowledge.NameConflictModal.k2")}</span>
            <code className={styles.conflictName}>{conflict.newName}</code>
          </div>
          <div className={styles.conflictArrow}>{t("knowledge.NameConflictModal.k3")}</div>
          <div className={styles.conflictRow}>
            <span className={styles.conflictLabel}>{t("knowledge.NameConflictModal.k4")}</span>
            <code className={styles.conflictName}>{conflict.existingEntry.name}</code>
          </div>
          <p className={styles.conflictHint}>
            {t("knowledge.NameConflictModal.k5")}
          </p>
        </div>
        <div className={styles.modalActions}>
          <button className={styles.btnSm} onClick={() => {
          conflict.onRename(renamed);
        }}>
            {t("knowledge.NameConflictModal.k6")}{renamed}」
          </button>
          <button className={styles.btnDel} onClick={() => conflict.onReplace()}>
            {t("knowledge.NameConflictModal.k7")}
          </button>
        </div>
      </div>
    </div>;
}