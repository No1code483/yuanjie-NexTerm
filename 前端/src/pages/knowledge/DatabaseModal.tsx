import { t } from "i18next";
import styles from '../Knowledge.module.css';
interface Props {
  newDbName: string;
  onNameChange: (name: string) => void;
  onCreate: () => void;
  onCancel: () => void;
}
export default function DatabaseModal({
  newDbName,
  onNameChange,
  onCreate,
  onCancel
}: Props) {
  return <div className={styles.modalOverlay} onClick={onCancel}>
      <div className={styles.modal} onClick={e => e.stopPropagation()}>
        <h3 className={styles.modalTitle}>{t("knowledge.DatabaseModal.k1")}</h3>
        <p className={styles.modalBody}>{t("knowledge.DatabaseModal.k2")}</p>
        <div style={{
        marginBottom: 16
      }}>
          <input value={newDbName} onChange={e => onNameChange(e.target.value)} placeholder={t("knowledge.DatabaseModal.k3")} className={styles.input} autoFocus onKeyDown={e => e.key === 'Enter' && onCreate()} />
        </div>
        <div className={styles.modalActions}>
          <button className={styles.btnPrimary} onClick={onCreate}>{t("common.created")}</button>
          <button className={styles.btnSm} onClick={onCancel}>{t("common.cancel")}</button>
        </div>
      </div>
    </div>;
}