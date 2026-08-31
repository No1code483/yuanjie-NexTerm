import { useTranslation } from 'react-i18next';
import { memo, useId } from 'react';
import styles from './Modal.module.css';
import Button from './Button';
interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title?: string;
  children: React.ReactNode;
  variant?: 'default' | 'danger' | 'warning';
  showCloseButton?: boolean;
  width?: string;
  maxWidth?: string;
}
const Modal = memo(function Modal({
  isOpen,
  onClose,
  title,
  children,
  variant = 'default',
  showCloseButton = true,
  width = '400px',
  maxWidth = '600px'
}: ModalProps) {
  const { t } = useTranslation();
  const titleId = useId();
  if (!isOpen) return null;
  return <div className={styles.overlay} onClick={onClose} role="dialog" aria-modal="true" aria-labelledby={title ? titleId : undefined}>
      <div className={`${styles.content} ${styles[variant]}`} style={{
      width,
      maxWidth
    }} onClick={e => e.stopPropagation()}>
        {title && <div className={`${styles.header} ${styles[variant]}`}>
            <h3 id={titleId} className={`${styles.title} ${styles[variant]}`}>{title}</h3>
            {showCloseButton && <Button variant="ghost" size="small" onClick={onClose} className={styles.closeButton} ariaLabel={t("common.close")}>
                ×
              </Button>}
          </div>}
        <div className={styles.body}>{children}</div>
      </div>
    </div>;
});
export default Modal;