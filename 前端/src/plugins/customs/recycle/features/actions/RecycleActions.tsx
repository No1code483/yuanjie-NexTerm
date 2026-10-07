import { t } from 'i18next';
import { useRecycleContext } from '../../context';
import styles from './RecycleActions.module.css';

export default function RecycleActions() {
  const {
    selectedFiles,
    clearSelection
  } = useRecycleContext();
  return <div className={styles.recycleActions}>
      <button className={styles.recycleActionPrimary} disabled={selectedFiles.length === 0} onClick={() => window.dispatchEvent(new CustomEvent('recycle-restore'))} aria-label={t("common.restore")}>
        {t("layout.k34")}{selectedFiles.length})
      </button>
      <button className={styles.recycleActionDanger} disabled={selectedFiles.length === 0} onClick={() => window.dispatchEvent(new CustomEvent('recycle-delete'))} aria-label={t("common.permanentDelete")}>
        {t("layout.k35")}{selectedFiles.length})
      </button>
      <button className={styles.recycleActionDanger} onClick={() => window.dispatchEvent(new CustomEvent('recycle-clear'))} aria-label={t("common.clearAll")}>
        {t("common.clearAll")}
      </button>
      <button className={styles.recycleActionSecondary} onClick={() => window.dispatchEvent(new CustomEvent('recycle-select-all'))} aria-label={t("common.selectAll")}>
        {t("common.selectAll")}
      </button>
      <button className={styles.recycleActionSecondary} disabled={selectedFiles.length === 0} onClick={() => clearSelection()} aria-label={t("common.deselect")}>
        {t("common.deselect")}
      </button>
    </div>;
}
