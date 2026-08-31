import { t } from "i18next";
import styles from '../Knowledge.module.css';
import type { KbEntry, ConfirmDelete } from './types';
interface Props {
  confirmDelete: ConfirmDelete;
  allEntries: KbEntry[];
  isExternalEntry: (entry: KbEntry) => boolean;
  onCancel: () => void;
  onConfirmCategory: () => void;
  onConfirmEntry: () => void;
  onConfirmBatch: () => void;
}
export default function DeleteConfirmModal({
  confirmDelete,
  allEntries,
  isExternalEntry,
  onCancel,
  onConfirmCategory,
  onConfirmEntry,
  onConfirmBatch
}: Props) {
  const entryForConfirm = confirmDelete.type === 'entry' ? allEntries.find(e => e.id === confirmDelete.id) : null;
  const isExt = entryForConfirm ? isExternalEntry(entryForConfirm) : false;
  const title = confirmDelete.type === 'category' ? t("knowledge.DeleteConfirmModal.k1") : confirmDelete.type === 'batch' ? t("ai.ChatPanel.k20") : isExt ? t("knowledge.DeleteConfirmModal.k2") : t("knowledge.DeleteConfirmModal.k1");
  const body = confirmDelete.type === 'category' ? t("knowledge.DeleteConfirmModal.k3", {
    name: confirmDelete.name,
    arg0: confirmDelete.subCount || 0
  }) : confirmDelete.type === 'batch' ? t("knowledge.DeleteConfirmModal.k4", {
    arg0: confirmDelete.batchCount || 0
  }) : isExt ? t("knowledge.DeleteConfirmModal.k5", {
    name: confirmDelete.name
  }) : t("knowledge.DeleteConfirmModal.k6", {
    name: confirmDelete.name
  });
  const handleConfirm = () => {
    if (confirmDelete.type === 'category') onConfirmCategory();else if (confirmDelete.type === 'batch') onConfirmBatch();else onConfirmEntry();
  };
  const confirmLabel = confirmDelete.type === 'category' ? t("knowledge.DeleteConfirmModal.k7") : confirmDelete.type === 'batch' ? t("common.confirm") : isExt ? t("knowledge.DeleteConfirmModal.k8") : t("knowledge.DeleteConfirmModal.k7");
  return <div className={styles.modalOverlay} onClick={onCancel}>
      <div className={styles.modal} onClick={e => e.stopPropagation()}>
        <h3 className={styles.modalTitle}>{title}</h3>
        <p className={styles.modalBody}>{body}</p>
        <div className={styles.modalActions}>
          <button className={styles.btnSm} onClick={onCancel}>{t("common.cancel")}</button>
          <button className={styles.btnDel} onClick={handleConfirm}>{confirmLabel}</button>
        </div>
      </div>
    </div>;
}