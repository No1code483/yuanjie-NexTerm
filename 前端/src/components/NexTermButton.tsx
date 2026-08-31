import { t } from "i18next";
import { ReactNode } from 'react';
import styles from './NexTermButton.module.css';
interface NexTermButtonProps {
  children: ReactNode;
  onClick?: () => void;
  type?: 'default' | 'danger' | 'success';
  disabled?: boolean;
  loading?: boolean;
  size?: 'small' | 'medium' | 'large';
}
export default function NexTermButton({
  children,
  onClick,
  type = 'default',
  disabled = false,
  loading = false,
  size = 'medium'
}: NexTermButtonProps) {
  return <button className={`
        ${styles.button}
        ${styles[type]}
        ${styles[size]}
        ${disabled || loading ? styles.disabled : ''}
        ${loading ? styles.loading : ''}
      `} onClick={disabled || loading ? undefined : onClick} disabled={disabled || loading}>
      {loading ? t("common.loading") : children}
    </button>;
}