import { t } from "i18next";
import { useEffect } from 'react';
import styles from './ConfirmDialog.module.css';
interface ConfirmDialogProps {
  /** 是否显示弹窗 */
  isOpen: boolean;
  /** 关闭回调（点击"否"/遮罩/× 均触发） */
  onClose: () => void;
  /** 确认回调（点击"是"触发） */
  onConfirm: () => void;
  /** 操作目标名称，如 "FreeBuf"、"该新闻源" */
  targetName?: string;
  /** 弹窗类型：danger=红色(删除)、warning=紫色、default=青色 */
  type?: 'danger' | 'warning' | 'default';
  /** 自定义提示文案（不传则自动拼接 targetName） */
  message?: string;
}
export default function ConfirmDialog({
  isOpen,
  onClose,
  onConfirm,
  targetName = '',
  type = 'danger',
  message
}: ConfirmDialogProps) {
  // ESC 关闭
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [isOpen, onClose]);
  if (!isOpen) return null;
  const displayMessage = message || t("components.ConfirmDialog.k1", {
    targetName: targetName
  });
  return <div className={styles.overlay} onClick={onClose}>
      <div className={`${styles.dialog} ${styles[type]}`} onClick={e => e.stopPropagation()}>
        {/* 标题栏 */}
        <div className={`${styles.header} ${styles[type]}`}>
          <span className={styles.title}>{t("components.ConfirmDialog.k2")}</span>
          <button className={`${styles.closeBtn} ${styles[type]}`} onClick={onClose}>
            ×
          </button>
        </div>

        {/* 提示内容 */}
        <div className={styles.body}>
          <p className={styles.message}>{displayMessage}</p>

          {/* 按钮组 */}
          <div className={styles.actions}>
            <button className={`${styles.btn} ${styles.confirmBtn} ${styles[type]}`} onClick={() => {
            onConfirm();
            onClose();
          }}>
              {t("common.yes")}
            </button>
            <button className={`${styles.btn} ${styles.cancelBtn}`} onClick={onClose}>
              {t("common.no")}
            </button>
          </div>
        </div>
      </div>
    </div>;
}